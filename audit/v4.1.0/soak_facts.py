#!/usr/bin/env python3
"""从 SOAK 实测快照算出全部事实 —— 仓内自包含版。

为什么要有这一层：SOAK 是**活跑**，它的数字每分钟都在变。若材料里写死
「错误数冻结在 744 不再增长」，几小时后这句话就是假的 —— 而材料一旦落进
看板与报告，假的句子比没有句子更糟。

所以：所有 SOAK 数字一律从快照（metrics.csv / driver.log / driver_errors.log /
server.log / heartbeat.jsonl / STARTED.json）现算，材料里只出现**算出来的**结论。

本脚本原先只存在于 `/tmp`（依赖 `/tmp/_soak_snap`），无法复核、也会随系统清理
消失 ⇒ 落进仓内，改成**路径参数化**：

- `--snapshot-dir DIR`：快照目录（放 metrics.csv / driver.log / driver_errors.log /
  server.log / fsync_probe.json / host_contention.json / STARTED.json）。
  缺省取 `$SOAK_SNAPSHOT_DIR`，再缺省取仓内已提交的快照
  `audit/v4.1.0/runs/20261116-168h-v410/artifacts`。
- `--manifest FILE`：该轮的 `manifest.yaml`（可选）。当快照目录里没有
  `STARTED.json` 时，起跑字段（started_utc / planned_end_utc / frozen_commit /
  binary sha256 / host_at_start）改从 manifest 的 `start_record` 取。
- `--out FILE`：把算出的事实写成 JSON（默认打到 stdout）。

**缺失即披露**：算不出来的输入不会被悄悄当成 0，而是在输出的
`missing_inputs` 里逐条列出 —— 宁可标「未读到」，不编一个数（AFP）。
"""
import argparse
import csv
import json
import os
import pathlib
import re
import statistics
import sys

REPO_ROOT = pathlib.Path(__file__).resolve().parents[2]
DEFAULT_SNAPSHOT = REPO_ROOT / "audit/v4.1.0/runs/20261116-168h-v410/artifacts"
DEFAULT_MANIFEST = REPO_ROOT / "audit/v4.1.0/runs/20261116-168h-v410/manifest.yaml"

# 事实计算要用到的输入文件。缺一个就在 missing_inputs 里点名。
INPUTS = (
    "metrics.csv",
    "driver.log",
    "driver_errors.log",
    "server.log",
    "STARTED.json",
)


class Snapshot:
    """快照目录 + manifest 的解析结果。"""

    def __init__(self, snapshot_dir: pathlib.Path, manifest: pathlib.Path | None):
        self.dir = snapshot_dir
        self.manifest = manifest
        self._manifest_data = None
        self.missing: list[str] = []

    # ---- 路径解析 ----
    def candidates(self, name: str) -> list[pathlib.Path]:
        out = []
        if self.dir:
            out.append(self.dir / name)
            # 快照有时把驱动日志放在 armA/ 子目录下。
            out.append(self.dir / "armA" / name)
            # driver.log 在仓内快照里是压缩过的。
            if name == "driver.log":
                out.append(self.dir / "driver.log.gz")
        return out

    def path(self, name: str) -> pathlib.Path | None:
        for c in self.candidates(name):
            if c.exists():
                return c
        return None

    def read_text(self, name: str) -> str:
        p = self.path(name)
        if p is None:
            return ""
        if p.suffix == ".gz":
            import gzip

            with gzip.open(p, "rt", errors="replace") as fh:
                return fh.read()
        return p.read_text(errors="replace")

    # ---- manifest ----
    def manifest_data(self) -> dict:
        if self._manifest_data is not None:
            return self._manifest_data
        data = {}
        if self.manifest and self.manifest.exists():
            try:
                import yaml

                data = yaml.safe_load(self.manifest.read_text(encoding="utf-8")) or {}
            except Exception as exc:  # 解析失败 ⇒ 当作没有，不猜
                print(f"[WARN] 读 manifest 失败（{exc}）—— 起跑字段改用 STARTED.json",
                      file=sys.stderr)
                data = {}
        self._manifest_data = data
        return data

    def started(self) -> dict:
        """起跑记录：STARTED.json 优先，其次 manifest 的 start_record。"""
        p = self.path("STARTED.json")
        if p is not None:
            try:
                return json.loads(p.read_text(encoding="utf-8"))
            except Exception:
                pass
        rec = self.manifest_data().get("start_record") or {}
        if rec:
            return dict(rec)
        return {}


def _rows(snap: Snapshot):
    out = []
    text = snap.read_text("metrics.csv")
    for r in csv.DictReader(text.splitlines()):
        try:
            out.append({
                "ts": r.get("ts", ""),
                "elapsed": int(r.get("elapsed_s") or 0),
                "q": int(r.get("driver_q") or 0),
                "e": int(r.get("driver_e") or 0),
                "cpu": float(r.get("cpu_pct") or 0),
                "rss": float(r.get("rss_mb") or 0),
                "panics": (r.get("panic_count") or "").strip(),
                "load1": float(r.get("host_load1") or 0),
                "busy": int(r.get("host_busy") or 0),
            })
        except (TypeError, ValueError):
            continue
    return out


def _err_types(snap: Snapshot):
    """客户端 driver.log 里错误的类型分布 + 服务端日志里的对应痕迹。

    错误是**客户端**读超时，所以要看 driver.log（服务端日志里看不到这些 2013）。
    这条分布是「变的是环境、不是引擎」的关键证据：若是引擎的锁/并发问题，
    错误形态会是 server gone / deadlock / panic，而不是整齐的 read timeout。
    """
    txt = snap.read_text("driver.log")
    e_lines = [l.strip() for l in txt.splitlines() if l.startswith("E,")]
    timeout = sum(1 for l in e_lines if "read operation timed out" in l)
    lost = sum(1 for l in e_lines if "Lost connection" in l)
    errno0 = sum(1 for l in e_lines if re.search(r"\(0,\s*''\)", l))
    lats = [float(m) for m in re.findall(r"^E,([0-9.]+),", txt, re.M)]
    kinds = {
        "客户端读超时 (2013, read operation timed out)": timeout,
        "客户端连接丢失 (Lost connection, 其它形态)": max(0, lost - timeout),
        "连接层 errno=0 空消息 ((0, ''))": errno0,
        "E 行合计": len(e_lines),
    }
    if lats:
        kinds["_timeout_latency_max_ms"] = round(max(lats), 1)
        kinds["_timeout_latency_min_ms"] = round(min(lats), 1)
    return {k: v for k, v in kinds.items() if v}


def _server_side(snap: Snapshot):
    """服务端日志里的痕迹：崩溃迹象必须为 0；断管与背压是**结果**不是原因。"""
    txt = snap.read_text("server.log")
    return {
        "crash_markers": len(re.findall(
            r"panic|deadlock|assertion|abort|segmentation|fatal", txt, re.I)),
        "broken_pipe": len(re.findall(r"Broken pipe", txt)),
        "backpressure_rejects": len(re.findall(r"BACKPRESSURE_REJECTED_COUNT", txt)),
        "worker_pool_full": len(re.findall(r"worker pool full", txt)),
    }


def _pct(sorted_vals, p):
    """分位数（入参必须已排序）。"""
    if not sorted_vals:
        return 0.0
    n = len(sorted_vals)
    idx = int(round(p * (n - 1)))
    idx = max(0, min(n - 1, idx))
    return round(float(sorted_vals[idx]), 1)


def _latency_profile(snap: Snapshot):
    """按查询类型算成功查询的延迟分布。

    **列序陷阱**：`driver.log` 的行格式是
    `Q,<query_latency_ms>,<thread_id>,<query_kind>,<row_count>`
    —— 第 2 列是**这条查询自己的耗时**，不是「自起跑以来的 elapsed」。
    （`scripts/soak/v400_soak_driver.py` 头部有同样的告警：把它当成 elapsed
    会让「稳态窗口」永远是空的。）
    本函数按**正确列序**解析，并把按类型的延迟分布交给材料脚本。
    """
    txt = snap.read_text("driver.log")
    qpat = re.compile(r"^Q,([0-9.]+),([0-9]+),([a-z_]+),([0-9]+)$")
    lat = {}
    for line in txt.splitlines():
        m = qpat.match(line.strip())
        if m:
            lat.setdefault(m.group(3), []).append(float(m.group(1)))
    out = {}
    for kind, vals in lat.items():
        s = sorted(vals)
        out[kind] = {
            "n": len(s),
            "p50_ms": _pct(s, 0.50), "p95_ms": _pct(s, 0.95), "p99_ms": _pct(s, 0.99),
            "max_ms": round(s[-1], 1),
        }
    return out


def _error_latency_profile(snap: Snapshot):
    """错误查询的延迟分布 —— 全部贴住 15 s 读超时上限，就是「等不到回复」的形态。"""
    txt = snap.read_text("driver_errors.log") or snap.read_text("driver.log")
    epat = re.compile(r"^E,([0-9.]+),([0-9]+),([a-z_]+),(.*)$")
    lats = []
    for line in txt.splitlines():
        m = epat.match(line.strip())
        if m:
            lats.append(float(m.group(1)))
    if not lats:
        return {}
    s = sorted(lats)
    near = sum(1 for x in s if x >= 14000)
    return {
        "n": len(s),
        "p50_ms": _pct(s, 0.50), "p95_ms": _pct(s, 0.95), "max_ms": round(s[-1], 1),
        "ge_14s": near,
        "ge_14s_pct": round(near / len(s) * 100, 1),
    }


def _json_file(snap: Snapshot, name: str):
    p = snap.path(name)
    if p is None:
        return {}
    try:
        return json.loads(p.read_text(encoding="utf-8"))
    except Exception:
        return {}


def _fsync_probe(snap: Snapshot):
    return _json_file(snap, "fsync_probe.json")


def _fsync_probe_history(snap: Snapshot):
    """探针历史 —— 保留每一次实测。

    为什么要留历史：只有一次探测时，秒级尾部只能说明「现在慢」；
    两次相隔 54 分钟的探测（p90 3305 ms → 0.133 ms）才能证明
    **尾部恶化是宿主争用的瞬态**，不是被测引擎的固有属性。
    """
    hist = _json_file(snap, "fsync_probe_history.json")
    probes = list(hist.get("probes") or [])
    cur = _fsync_probe(snap)
    if cur and not any(x.get("measured_at_utc") == cur.get("measured_at_utc")
                       for x in probes):
        probes.append(cur)
    return probes


def _host_contention(snap: Snapshot):
    """宿主争用现状 —— 独占窗口判据的证据。"""
    return _json_file(snap, "host_contention.json")


def _quiet_window(rows):
    """当前这段「零新增错误」的安静窗口（宿主安静 ⇒ 没有新的读超时）。

    注意这**不是**『通过』：它只说明宿主此刻没在抢盘。独占窗口要的是
    整个窗口期独占，而 root 的编译进程仍在 ⇒ 该窗口不满足独占要求。
    """
    if not rows:
        return None
    last_e = rows[-1]["e"]
    idx = len(rows) - 1
    while idx > 0 and rows[idx]["e"] == last_e:
        idx -= 1
    # idx 现在指向「最后一个 E 还不是当前值」的采样点；安静窗口从**下一个**开始，
    # 否则会把边界那一笔错误算进窗口，得出「安静窗口里还涨了 N 个错」的错话。
    if idx < len(rows) - 1:
        idx += 1
    seg = rows[idx:]
    if len(seg) < 2:
        return None
    loads = [r["load1"] for r in seg]
    span = max(1, seg[-1]["elapsed"] - seg[0]["elapsed"])
    return {
        "from_elapsed": seg[0]["elapsed"],
        "to_elapsed": seg[-1]["elapsed"],
        "span_s": span,
        "samples": len(seg),
        "e_frozen_at": last_e,
        "new_errors": last_e - seg[0]["e"],
        "load_min": round(min(loads), 2),
        "load_max": round(max(loads), 2),
        "load_mean": round(sum(loads) / len(loads), 2),
        "qps": round((seg[-1]["q"] - seg[0]["q"]) / span, 2),
    }


def compute(snap: Snapshot) -> dict:
    rows = _rows(snap)
    started = snap.started()
    if not rows:
        return {"missing_inputs": ["metrics.csv（没有可用采样）"], "facts": {}}

    last = rows[-1]
    rate = (last["e"] / last["q"] * 100) if last["q"] else 0.0
    peak = max(rows, key=lambda r: (r["e"] / r["q"] * 100) if r["q"] else 0)

    # 累计错误率**首次**越过 1 % 阈值的采样点（阈值来自 STARTED.json 的 max_error_rate_pct）
    thr = float(started.get("max_error_rate_pct") or 1)
    first_over = None
    for r in rows:
        rr = (r["e"] / r["q"] * 100) if r["q"] else 0.0
        if rr > thr:
            first_over = {"elapsed": r["elapsed"], "q": r["q"], "e": r["e"],
                          "rate_pct": round(rr, 3)}
            break

    # 最长的一段「错误数不变」窗口（宿主安静 ⇒ 零新增读超时）
    best = cur_start = None
    best_len = cur_len = 0
    prev_e = rows[0]["e"]
    cur_start = rows[0]
    for r in rows[1:]:
        if r["e"] == prev_e:
            cur_len += 1
        else:
            if cur_len > best_len:
                best_len, best = cur_len, cur_start
            cur_start, cur_len = r, 0
            prev_e = r["e"]
    if cur_len > best_len:
        best_len, best = cur_len, cur_start
    freeze = None
    if best is not None and best_len > 0:
        freeze = {"from_elapsed": best["elapsed"], "e": best["e"],
                  "len_samples": best_len}

    # 冻结窗口之后，宿主是否再次变忙、错误是否重新增长
    regrowth = None
    if freeze:
        after = [r for r in rows if r["elapsed"] > freeze["from_elapsed"]]
        # 「重新增长」区间 = 从错误数第一次超过冻结值、到**最后一笔错误**为止。
        # 不能取「最近 N 个采样点」：宿主会在安静与争用之间来回切，最近的一段
        # 往往恰好是安静的（错误不涨），那样算出来的「涨了多少」是 0 ⇒ 没有信息。
        over = next((r for r in after if r["e"] > freeze["e"]), None)
        if over is not None:
            last_change = over
            for r in after:
                if r["e"] != last_change["e"]:
                    last_change = r
            window = [r for r in after
                      if over["elapsed"] <= r["elapsed"] <= last_change["elapsed"]]
            if len(window) >= 2:
                span = max(1, window[-1]["elapsed"] - window[0]["elapsed"])
                regrowth = {
                    "at_elapsed": over["elapsed"],
                    "e_at_start": window[0]["e"],
                    "e_at_end": window[-1]["e"],
                    "q_at_start": window[0]["q"],
                    "q_at_end": window[-1]["q"],
                    "span_s": span,
                    "qps": round((window[-1]["q"] - window[0]["q"]) / span, 2),
                    "load_max": max(r["load1"] for r in window),
                    "load_min": min(r["load1"] for r in window),
                    "errors_added": window[-1]["e"] - window[0]["e"],
                }
            tail = after[-30:]
            if tail:
                span = max(1, tail[-1]["elapsed"] - tail[0]["elapsed"])
                regrowth = regrowth or {}
                regrowth["tail_qps"] = round((tail[-1]["q"] - tail[0]["q"]) / span, 2)
                regrowth["tail_load_max"] = max(r["load1"] for r in tail)
                regrowth["tail_e_from"] = tail[0]["e"]
                regrowth["tail_e_to"] = tail[-1]["e"]

    loads = [r["load1"] for r in rows]
    qps_window = 300
    tail_rows = [r for r in rows if r["elapsed"] >= last["elapsed"] - qps_window]
    if len(tail_rows) >= 2:
        span = max(1, tail_rows[-1]["elapsed"] - tail_rows[0]["elapsed"])
        qps_now = (tail_rows[-1]["q"] - tail_rows[0]["q"]) / span
    else:
        qps_now = 0.0

    panics = "".join(sorted({r["panics"] for r in rows if r["panics"]}))

    facts = {
        "samples": len(rows),
        "snapshot_at": last["ts"],
        "elapsed_s": last["elapsed"],
        "q": last["q"],
        "e": last["e"],
        "rate_pct": round(rate, 3),
        "peak_rate_pct": round((peak["e"] / peak["q"] * 100) if peak["q"] else 0, 3),
        "peak_at_elapsed": peak["elapsed"],
        "peak_q": peak["q"],
        "peak_e": peak["e"],
        "threshold_pct": thr,
        "first_over_threshold": first_over,
        "freeze": freeze,
        "regrowth": regrowth,
        "load_median": round(statistics.median(loads), 1),
        "load_gt40_pct": round(sum(1 for l in loads if l > 40) / len(loads) * 100, 1),
        "load_gt100_pct": round(sum(1 for l in loads if l > 100) / len(loads) * 100, 1),
        "load_now": last["load1"],
        "qps_now": round(qps_now, 2),
        "qps_at_start": started.get("driver_qps_at_verify"),
        "engine_cpu_now": last["cpu"],
        "engine_cpu_mean": round(statistics.mean(r["cpu"] for r in rows), 1),
        "rss_now": last["rss"],
        "panics": panics,
        "err_types": _err_types(snap),
        "server_side": _server_side(snap),
        "started_utc": started.get("started_utc"),
        "planned_end_utc": started.get("planned_end_utc"),
        "frozen_commit": started.get("frozen_commit"),
        "binary_sha256": started.get("soaked_binary_sha256"),
        "start_load1": (started.get("host_at_start") or {}).get("load1"),
        "fsync_p50_at_start": (started.get("host_at_start") or {}).get("fsync_p50_ms"),
        "attempt": started.get("attempt"),
        "total_planned_s": 604800,
        "fsync_probe": _fsync_probe(snap),
        "fsync_probe_history": _fsync_probe_history(snap),
        "host_contention": _host_contention(snap),
        "quiet_window": _quiet_window(rows),
        "latency_profile": _latency_profile(snap),
        "error_latency_profile": _error_latency_profile(snap),
    }

    # 缺失即披露：算不出来的输入逐条点名，不悄悄当成 0。
    missing = []
    for name in INPUTS:
        if snap.path(name) is None:
            missing.append(name)
    if not facts["host_contention"]:
        missing.append("host_contention.json")
    if not facts["fsync_probe"]:
        missing.append("fsync_probe.json")
    facts["missing_inputs"] = missing
    facts["snapshot_dir"] = str(snap.dir) if snap.dir else None
    return facts


def main() -> int:
    ap = argparse.ArgumentParser(description="从 SOAK 快照现算事实")
    ap.add_argument("--snapshot-dir", default=None,
                    help="快照目录（缺省 $SOAK_SNAPSHOT_DIR，再缺省仓内已提交快照）")
    ap.add_argument("--manifest", default=None,
                    help="该轮的 manifest.yaml（缺省仓内 168h 那一轮的）")
    ap.add_argument("--out", default=None, help="把事实写成 JSON 到该文件")
    args = ap.parse_args()

    snap_dir = args.snapshot_dir or os.environ.get("SOAK_SNAPSHOT_DIR")
    snap_dir = pathlib.Path(snap_dir) if snap_dir else DEFAULT_SNAPSHOT
    manifest = args.manifest or os.environ.get("SOAK_MANIFEST")
    manifest = pathlib.Path(manifest) if manifest else DEFAULT_MANIFEST

    facts = compute(Snapshot(snap_dir, manifest))
    text = json.dumps(facts, ensure_ascii=False, indent=2)
    if args.out:
        pathlib.Path(args.out).write_text(text + "\n", encoding="utf-8")
        print(f"已写出 {args.out}")
    else:
        print(text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
