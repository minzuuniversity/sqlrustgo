#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""soak_evidence_axes.py — 从 SOAK 产物算出 §7.25 要求的三条证据轴。

为什么需要它
------------
`docs/releases/v4.1.0/GA_GATE_REPORT.md` §7.25（`GA-SOAK-21`）在 r2 轮之后
把 168h SOAK 的错误成因拆成了**两层**，并写明合格证据需要三者一起：

    「独占宿主 + 长时跑满 + 监控 RSS/表规模/单查询 p99」
    ——「后两项需 `scripts/soak/` 采集增强，属主批准后另行执行」

但仓内现有的两个报告工具**都覆盖不到这三轴**：

| 工具 | 覆盖 | 缺口 |
|---|---|---|
| `extract_soak_report.py` | RSS 首尾值、FD 首尾值 | p50/p99 **只从 mysqlslap 的输出正则里取**，认不了 v400 驱动写的 `Q,<latency_ms>,...` 行；也没有按时间分段 |
| `soak_steady_window.py` | 稳态窗口的 QPS | 没有 p99、没有 RSS 趋势、没有与负载的分段相关性 |

⇒ 结果是：**r3 轮即使跑满 0 错，也无法回答「这 0 错是不是只是因为负载档够低」**
以及「RSS/查询延迟是否随时间劣化」这两个 GA 报告已经点名的问题。

本工具补上这个缺口。它**只读产物、只算不改判据**，输出：

1. **RSS 趋势** —— 首/中位/末/峰值、每小时增长速率、是否单调；
2. **单查询延迟分位** —— p50/p90/p99/p99.9/max，按**时间桶**（默认 1 h）给出，
   以便看出「延迟随时间劣化」而不是只有一个总分位数；
3. **表规模** —— 来自配套采样器 `soak_data_size_sampler.py` 的 jsonl
   （数据目录占用 + 行数）；没有采样器时如实报 NOT-COLLECTED；
4. **分段相关性** —— 每个时间桶的 (errors, load1, host_busy, rss_mb, p99)，
   用来区分「宿主争用」与「服务端侧劣化」这两条成因。

判定口径
--------
**本工具不产出 PASS / FAIL 判定**。它产出的是**事实**。
§7.25 的因果归因是评审材料，不是自动判据 —— 门禁判据一个都不动。

用法
----
    python3 scripts/soak/soak_evidence_axes.py \
        --attempt-dir /home/liying/soak-artifacts/<run_id>/attempt-<ts> \
        --bucket-sec 3600 \
        --out-json  <attempt-dir>/evidence_axes.json \
        --out-md    <attempt-dir>/EVIDENCE_AXES.md

    # 没有采样器时（表规模轴缺失，如实报）
    ... --data-size-jsonl /path/to/data_size.jsonl
"""
from __future__ import annotations

import argparse
import json
import math
import os
import statistics
import sys
from collections import defaultdict
from typing import Dict, List, Optional, Tuple

METRICS_HEADER = [
    "ts", "elapsed_s", "rss_mb", "fd_count", "cpu_pct",
    "driver_q", "driver_e", "panic_count", "error_high",
    "host_load1", "host_busy",
]


# ---------------------------------------------------------------------------
# 读取
# ---------------------------------------------------------------------------
def read_metrics(path: str) -> List[dict]:
    """读 metrics.csv → 逐行 dict（表头按 METRICS_HEADER 解析，缺列用 None）。"""
    rows: List[dict] = []
    if not os.path.exists(path):
        return rows
    with open(path, encoding="utf-8", errors="replace") as f:
        for lineno, raw in enumerate(f, 1):
            line = raw.strip()
            if not line:
                continue
            parts = line.split(",")
            if lineno == 1 and parts and parts[0] == "ts":
                continue  # 表头
            rec: dict = {}
            for i, key in enumerate(METRICS_HEADER):
                if i >= len(parts):
                    rec[key] = None
                    continue
                v = parts[i].strip()
                if key in ("ts",):
                    rec[key] = v
                    continue
                try:
                    rec[key] = int(v) if key in (
                        "elapsed_s", "fd_count", "driver_q", "driver_e",
                        "panic_count", "error_high",
                    ) else float(v)
                except ValueError:
                    rec[key] = None
            rows.append(rec)
    return rows


def read_driver_log(path: str) -> Tuple[List[Tuple[float, str]], List[Tuple[float, str]]]:
    """读 driver.log → (Q 行列表, E 行列表)。

    行格式（v400 驱动）：
        Q,<latency_ms>,<thread_id>,<query_kind>,<row_count>
        E,<latency_ms>,<thread_id>,<query_kind>,<error_msg>
    `error_msg` 自身可能含逗号 ⇒ 用 maxsplit 截断。
    """
    qs: List[Tuple[float, str]] = []
    es: List[Tuple[float, str]] = []
    if not os.path.exists(path):
        return qs, es
    with open(path, encoding="utf-8", errors="replace") as f:
        for raw in f:
            line = raw.strip()
            if not line:
                continue
            tag = line[:1]
            if tag not in ("Q", "E"):
                continue
            parts = line.split(",", 3)
            if len(parts) < 2:
                continue
            try:
                latency = float(parts[1])
            except ValueError:
                continue
            kind = parts[3] if len(parts) > 3 else ""
            if tag == "Q":
                qs.append((latency, kind))
            else:
                es.append((latency, kind))
    return qs, es


def read_data_size(path: str) -> List[dict]:
    """读配套采样器写的 jsonl（每行一个采样）。"""
    out: List[dict] = []
    if not path or not os.path.exists(path):
        return out
    with open(path, encoding="utf-8", errors="replace") as f:
        for raw in f:
            raw = raw.strip()
            if not raw:
                continue
            try:
                out.append(json.loads(raw))
            except json.JSONDecodeError:
                continue
    return out


# ---------------------------------------------------------------------------
# 统计
# ---------------------------------------------------------------------------
def percentiles(values: List[float]) -> Dict[str, Optional[float]]:
    """p50/p90/p99/p99.9/max —— 最近秩法（nearest-rank），不插值。"""
    if not values:
        return {k: None for k in ("p50", "p90", "p99", "p999", "max", "n")}
    vs = sorted(values)
    n = len(vs)

    def nr(p: float) -> float:
        idx = max(1, min(n, math.ceil(p * n)))
        return vs[idx - 1]

    return {
        "p50": nr(0.50), "p90": nr(0.90), "p99": nr(0.99),
        "p999": nr(0.999), "max": vs[-1], "n": n,
    }


def bucket_of(elapsed: float, bucket_sec: float) -> int:
    return int(elapsed // bucket_sec)


def main(argv: List[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--attempt-dir", required=True,
                    help="attempt 目录（含 armA/metrics.csv 与 armA/driver.log）")
    ap.add_argument("--bucket-sec", type=float, default=3600.0,
                    help="时间桶宽度（秒），默认 3600 = 1 h")
    ap.add_argument("--data-size-jsonl", default="",
                    help="soak_data_size_sampler.py 的输出 jsonl（可选）")
    ap.add_argument("--out-json", default="")
    ap.add_argument("--out-md", default="")
    args = ap.parse_args(argv)

    ad = args.attempt_dir
    metrics_path = os.path.join(ad, "armA", "metrics.csv")
    driver_path = os.path.join(ad, "armA", "driver.log")
    server_log = os.path.join(ad, "armA", "server.log")

    metrics = read_metrics(metrics_path)
    qs, es = read_driver_log(driver_path)
    dsize = read_data_size(args.data_size_jsonl)

    if not metrics and not qs:
        print("FAIL: 既读不到 metrics.csv 也读不到 driver.log —— 无产物可算", file=sys.stderr)
        return 2

    # ---- 1. RSS 趋势 ------------------------------------------------------
    rss_series = [(m["elapsed_s"], m["rss_mb"]) for m in metrics
                  if m.get("elapsed_s") is not None and m.get("rss_mb") is not None]
    rss_axis: Dict[str, object] = {"collected": bool(rss_series)}
    if rss_series:
        vals = [v for _, v in rss_series]
        first_t, first_v = rss_series[0]
        last_t, last_v = rss_series[-1]
        span = max(1e-9, last_t - first_t)
        rss_axis.update({
            "samples": len(vals),
            "first_mb": first_v, "last_mb": last_v,
            "median_mb": statistics.median(vals),
            "max_mb": max(vals),
            "growth_mb": round(last_v - first_v, 2),
            "growth_mb_per_hour": round((last_v - first_v) / span * 3600.0, 3),
            "elapsed_span_s": round(span, 1),
            # 单调性：相邻样本中 RSS 下降的次数（0 = 完全单调不减）
            "decreasing_steps": sum(1 for i in range(1, len(vals)) if vals[i] < vals[i - 1]),
        })

    # ---- 2 & 4. 分桶：延迟分位 + 错误 + 负载 + RSS ------------------------
    buckets: Dict[int, Dict[str, object]] = defaultdict(
        lambda: {"q_lat": [], "e_lat": [], "e_count": 0, "load1": [], "busy": [],
                 "rss": [], "fd": [], "elapsed": []}
    )
    for lat, _kind in qs:
        # 用 metrics 的 elapsed→time 映射把 Q 行归桶：驱动不写时间戳，
        # 只能按**顺序**近似 —— 这里用 Q/E 的累计计数与 metrics 的 driver_q 对齐。
        pass
    # Q/E 行没有时间戳 ⇒ 用 metrics 的 driver_q/driver_e 累计值做游标归桶。
    q_cursor = 0
    e_cursor = 0
    for m in metrics:
        b = bucket_of(m["elapsed_s"], args.bucket_sec)
        bk = buckets[b]
        bk["elapsed"].append(m["elapsed_s"])
        if m.get("host_load1") is not None:
            bk["load1"].append(m["host_load1"])
        if m.get("host_busy") is not None:
            bk["busy"].append(m["host_busy"])
        if m.get("rss_mb") is not None:
            bk["rss"].append(m["rss_mb"])
        if m.get("fd_count") is not None:
            bk["fd"].append(m["fd_count"])
        # 游标推进：把本桶新增的 Q / E 归入本桶
        target_q = m.get("driver_q") or 0
        target_e = m.get("driver_e") or 0
        while q_cursor < target_q and q_cursor < len(qs):
            bk["q_lat"].append(qs[q_cursor][0])
            q_cursor += 1
        while e_cursor < target_e and e_cursor < len(es):
            bk["e_count"] += 1
            bk["e_lat"].append(es[e_cursor][0])
            e_cursor += 1

    bucket_rows: List[dict] = []
    for b in sorted(buckets):
        bk = buckets[b]
        pct = percentiles(list(bk["q_lat"]))
        bucket_rows.append({
            "bucket": b,
            "elapsed_from_s": int(b * args.bucket_sec),
            "elapsed_to_s": int((b + 1) * args.bucket_sec),
            "queries": len(bk["q_lat"]),
            "errors": bk["e_count"],
            "latency_ms": pct,
            "rss_mb": {
                "first": bk["rss"][0] if bk["rss"] else None,
                "last": bk["rss"][-1] if bk["rss"] else None,
                "max": max(bk["rss"]) if bk["rss"] else None,
            },
            "fd_count": {"first": bk["fd"][0] if bk["fd"] else None,
                         "last": bk["fd"][-1] if bk["fd"] else None},
            "host_load1": {"max": max(bk["load1"]) if bk["load1"] else None,
                           "median": statistics.median(bk["load1"]) if bk["load1"] else None},
            "host_busy": {"max": max(bk["busy"]) if bk["busy"] else None,
                          "median": statistics.median(bk["busy"]) if bk["busy"] else None},
        })

    latency_axis = {
        "collected": bool(qs),
        "overall": percentiles([x for x, _ in qs]),
        "note": "Q 行不带时间戳；按 metrics 的 driver_q 累计值做游标归桶（顺序对齐）",
    }

    # ---- 3. 表规模 --------------------------------------------------------
    table_axis: Dict[str, object] = {"collected": bool(dsize)}
    if dsize:
        sizes = [d.get("bytes") for d in dsize if isinstance(d.get("bytes"), (int, float))]
        rows_ = [d.get("rows") for d in dsize if isinstance(d.get("rows"), (int, float))]
        table_axis.update({
            "samples": len(dsize),
            "bytes_first": sizes[0] if sizes else None,
            "bytes_last": sizes[-1] if sizes else None,
            "bytes_max": max(sizes) if sizes else None,
            "rows_first": rows_[0] if rows_ else None,
            "rows_last": rows_[-1] if rows_ else None,
        })
    else:
        table_axis["reason"] = "未提供 --data-size-jsonl（配套采样器未运行或产物不在仓外路径）"

    # ---- 5. 汇总事实 ------------------------------------------------------
    total_q = len(qs)
    total_e = len(es)
    panic_hits = 0
    if os.path.exists(server_log):
        with open(server_log, encoding="utf-8", errors="replace") as f:
            for raw in f:
                if "panic" in raw.lower():
                    panic_hits += 1

    out = {
        "kind": "soak_evidence_axes",
        "attempt_dir": ad,
        "bucket_sec": args.bucket_sec,
        "totals": {
            "queries": total_q,
            "errors": total_e,
            "error_rate_pct": round(100.0 * total_e / total_q, 6) if total_q else None,
            "server_log_panic_lines": panic_hits,
            "metrics_samples": len(metrics),
        },
        "rss_axis": rss_axis,
        "latency_axis": latency_axis,
        "table_axis": table_axis,
        "buckets": bucket_rows,
        "note": "本文件只产出事实，不产出 PASS/FAIL 判定；门禁判据一个都没动。",
    }

    if args.out_json:
        with open(args.out_json, "w", encoding="utf-8") as f:
            json.dump(out, f, ensure_ascii=False, indent=2)
    if args.out_md:
        with open(args.out_md, "w", encoding="utf-8") as f:
            f.write(render_md(out))
    print(json.dumps({k: out[k] for k in ("kind", "totals")}, ensure_ascii=False, indent=2))
    print("rss_axis    :", json.dumps(out["rss_axis"], ensure_ascii=False))
    print("latency_axis:", json.dumps(out["latency_axis"]["overall"], ensure_ascii=False))
    print("table_axis  :", json.dumps(out["table_axis"], ensure_ascii=False))
    if args.out_json:
        print("wrote:", args.out_json)
    if args.out_md:
        print("wrote:", args.out_md)
    return 0


def render_md(out: dict) -> str:
    L: List[str] = []
    a = L.append
    a("# SOAK 证据三轴（§7.25 要求）")
    a("")
    a(f"- attempt 目录：`{out['attempt_dir']}`")
    a(f"- 时间桶宽度：{out['bucket_sec']:.0f} s")
    t = out["totals"]
    a(f"- 查询 {t['queries']} / 错误 {t['errors']}"
      f"（{t['error_rate_pct']}%）/ server.log 中 panic 行 {t['server_log_panic_lines']}")
    a("")
    a("> 本文件**只产出事实，不产出 PASS/FAIL 判定**；门禁判据一个都没动。")
    a("")
    a("## 轴 1：RSS 趋势")
    a("")
    r = out["rss_axis"]
    if not r.get("collected"):
        a("**NOT-COLLECTED** —— metrics.csv 无 rss 列。")
    else:
        a("| 项 | 值 |")
        a("|---|---|")
        a(f"| 采样数 | {r['samples']} |")
        a(f"| 首值 | {r['first_mb']} MB |")
        a(f"| 末值 | {r['last_mb']} MB |")
        a(f"| 中位 | {r['median_mb']} MB |")
        a(f"| 峰值 | {r['max_mb']} MB |")
        a(f"| 净增长 | {r['growth_mb']} MB（{r['growth_mb_per_hour']} MB/h）|")
        a(f"| 相邻样本中 RSS 下降次数 | {r['decreasing_steps']} |")
        a("")
        a(f"⇒ §7.25 的对照点：**RSS {r['first_mb']} MB 时累计错误** vs "
          f"**RSS {r['last_mb']} MB 时累计错误** —— 与 r2 轮"
          "「RSS 414 MB ⇒ 0 错 / RSS 1,018 MB ⇒ 715 错」做同口径对照。")
    a("")
    a("## 轴 2：单查询延迟分位")
    a("")
    lo = out["latency_axis"]
    if not lo.get("collected"):
        a("**NOT-COLLECTED** —— driver.log 无 Q 行。")
    else:
        o = lo["overall"]
        a("| 分位 | 延迟 (ms) |")
        a("|---|---|")
        for k in ("p50", "p90", "p99", "p999", "max"):
            a(f"| {k} | {o.get(k)} |")
        a(f"| 样本数 | {o.get('n')} |")
        a("")
        a("## 分桶明细（每桶：错误 / 负载 / RSS / 延迟）")
        a("")
        a("| 桶 | 区间 (s) | 查询 | 错误 | p99 (ms) | p50 (ms) | RSS 首→末 (MB) | load1 max | busy max |")
        a("|---|---|---|---|---|---|---|---|---|")
        for b in out["buckets"]:
            p = b["latency_ms"]
            rr = b["rss_mb"]
            l1 = b["host_load1"]
            bs = b["host_busy"]
            a("| %d | %d–%d | %d | %d | %s | %s | %s → %s | %s | %s |" % (
                b["bucket"], b["elapsed_from_s"], b["elapsed_to_s"],
                b["queries"], b["errors"], p.get("p99"), p.get("p50"),
                rr.get("first"), rr.get("last"), l1.get("max"), bs.get("max")))
    a("")
    a("## 轴 3：表规模")
    a("")
    t2 = out["table_axis"]
    if not t2.get("collected"):
        a("**NOT-COLLECTED** —— %s" % t2.get("reason", ""))
    else:
        a("| 项 | 值 |")
        a("|---|---|")
        a(f"| 采样数 | {t2['samples']} |")
        a(f"| 数据目录占用 首→末 | {t2.get('bytes_first')} → {t2.get('bytes_last')} bytes |")
        a(f"| 峰值 | {t2.get('bytes_max')} bytes |")
        a(f"| 行数 首→末 | {t2.get('rows_first')} → {t2.get('rows_last')} |")
    a("")
    a("---")
    a("")
    a("*生成器：`scripts/soak/soak_evidence_axes.py`；配套采样器 `scripts/soak/soak_data_size_sampler.py`。*")
    return "\n".join(L) + "\n"


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
