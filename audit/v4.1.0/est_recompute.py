#!/usr/bin/env python3
"""est_recompute —— 把 v4.1.0 剩余人日区间绑回实测 census 与计划表（配套 run 20261105-est2-008）。

原则：**不复制数字**，两边都现场解析，派生值必须能在估算文档里找到对应字符串。

  census（stdin: `gov_probe.py issues` 的 `issue_summary` 行）
    × 三档权重（本文件常量 TIERS，即 LB-1 台账的成本档）      → 三档 issue 清账人日
  计划文档 §9.3 表格（四块非 issue 工作）                     → 非 issue 合计
  两者相加（四舍五入）                                        → 「档位中值 / 下沿」合计区间
  估算文档 §3 表里带 `人日` 的席位读数                        → 并集区间
  排期基线 × 吞吐带（文档里的 `1.0–1.3 有效人日/天`）          → 日历周区间

席位读数本身**不可复算**（LLM 非确定）。所以只做两件事：
  (1) 文档必须有披露句（`--disclosure`，默认「不可复算」）并指名证据 run（`--require-run-id`）；
  (2) 给 `--fresh-read <本次 run 的席位 stdout>` 时，新读数必须与文档声称的并集**有交集**
      且每端不超过对方 2 倍，否则判 finding（= 文档已过期）。

退出码：findings 为空 ⇒ 0；否则 1；用法/解析错误 ⇒ 2。
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

TIERS = {
    "lower": (0.25, 0.7, 1.5),
    "mid": (0.5, 1.25, 3.5),
    "upper": (0.75, 2.0, 5.0),
}
BLOCK_KEYS = ("判定系统", "声明与测量绑定", "文档事实面", "闭环与自演进")
DASH = "–—‒─‐-"
RANGE_RE = re.compile(r"(\d+(?:\.\d+)?)\s*[" + DASH + r"]\s*(\d+(?:\.\d+)?)")
RATE_RE = re.compile(
    r"(\d+(?:\.\d+)?)\s*[" + DASH + r"]\s*(\d+(?:\.\d+)?)\s*(?:有效)?人日\s*/\s*天"
)
BASELINE_RE = re.compile(r"排期基线[^0-9]*(\d+(?:\.\d+)?)\s*[" + DASH + r"]\s*(\d+(?:\.\d+)?)")
PROMISE_RE = re.compile(r"[≥>=]+\s*(\d+(?:\.\d+)?)\s*人日")


def die(msg: str) -> None:
    print(f"est_recompute: {msg}", file=sys.stderr)
    sys.exit(2)


def hround(x: float) -> int:
    """四舍五入（half up），与文档口径一致。"""
    return int(x + 0.5)


def fmt(x: float) -> str:
    return f"{x:g}"


def unit_ranges(text: str, unit: str) -> list[tuple[float, float]]:
    """只取紧跟 `unit` 的区间（`160–300 人日 / 20–40 周` 只回 160–300）。"""
    out: list[tuple[float, float]] = []
    for m in RANGE_RE.finditer(text):
        tail = text[m.end():].lstrip()
        if tail.startswith(unit):
            out.append((float(m.group(1)), float(m.group(2))))
    return out


def ranges(text: str) -> list[tuple[float, float]]:
    return [(float(a), float(b)) for a, b in RANGE_RE.findall(text)]


def table_rows(text: str, header: str) -> list[list[str]]:
    """取 `header` 小节里的 markdown 表格行（跳过表头与分隔行）。"""
    sec = text.split(header, 1)
    if len(sec) != 2:
        die(f"文档里找不到小节: {header}")
    body = sec[1].split("\n###", 1)[0]
    rows: list[list[str]] = []
    for line in body.splitlines():
        line = line.strip()
        if not line.startswith("|"):
            continue
        cells = [c.strip() for c in line.strip("|").split("|")]
        if len(cells) < 2:
            continue
        if all(set(c) <= set("-: ") for c in cells):
            continue
        rows.append(cells)
    return rows


def parse_plan_blocks(plan_text: str) -> dict[str, tuple[float, float]]:
    blocks: dict[str, tuple[float, float]] = {}
    for cells in table_rows(plan_text, "### 9.3"):
        label, val = cells[0].replace("**", "").strip(), cells[1].replace("**", "")
        if not label:
            continue
        found = ranges(val)
        if not found:
            continue
        if label.startswith("合计"):
            continue
        for key in BLOCK_KEYS:
            if label.startswith(key):
                blocks[key] = found[0]
    missing = [k for k in BLOCK_KEYS if k not in blocks]
    if missing:
        die(f"§9.3 解析不到这些块: {missing}")
    return blocks


def parse_plan_total(plan_text: str) -> tuple[float, float]:
    for cells in table_rows(plan_text, "### 9.3"):
        if cells[0].replace("**", "").strip().startswith("合计"):
            found = ranges(cells[1].replace("**", ""))
            if found:
                return found[0]
    die("§9.3 解析不到「合计」行")


def parse_doc_readings(doc_text: str) -> list[tuple[str, float, float]]:
    """估算文档 §3 表：读数列里带 `人日` 的那些区间。"""
    out: list[tuple[str, float, float]] = []
    for cells in table_rows(doc_text, "## 3"):
        if len(cells) < 4:
            continue
        for cell in cells[1:]:
            if "人日" not in cell:
                continue
            for a, b in unit_ranges(cell.replace("**", ""), "人日"):
                if a < b:
                    out.append((cells[0], a, b))
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--plan", required=True, help="GOVERNANCE_REMEDIATION_PLAN_*.md")
    ap.add_argument("--doc", required=True, help="ESTIMATE_DOUBLE_LINEAGE_*.md")
    ap.add_argument("--require-run-id", required=True)
    ap.add_argument("--disclosure", default="不可复算")
    ap.add_argument("--fresh-read", action="append", default=[],
                    help="本次 run 归档的席位 stdout（可重复）")
    args = ap.parse_args()

    summary = None
    for line in sys.stdin.read().splitlines():
        try:
            rec = json.loads(line)
        except json.JSONDecodeError:
            continue
        if rec.get("kind") == "issue_summary":
            summary = rec
    if summary is None:
        die("stdin 里没有 issue_summary")

    cost = summary.get("cost", {})
    t0, t1, t2 = (cost.get("cost/t0", 0), cost.get("cost/t1", 0), cost.get("cost/t2", 0))
    total = summary.get("open_total", 0)
    if t0 + t1 + t2 != total:
        die(f"cost 标签合计 {t0}+{t1}+{t2} != open_total {total}")

    plan_text = Path(args.plan).read_text(encoding="utf-8")
    doc_text = Path(args.doc).read_text(encoding="utf-8")

    findings: list[str] = []
    tier_vals = {
        name: round(w0 * t0 + w1 * t1 + w2 * t2, 1)
        for name, (w0, w1, w2) in TIERS.items()
    }
    for name, val in sorted(tier_vals.items()):
        if f"{val:.1f}" not in doc_text:
            findings.append(f"doc-missing-tier-{name}: {val:.1f}")

    blocks = parse_plan_blocks(plan_text)
    blo = sum(v[0] for v in blocks.values())
    bhi = sum(v[1] for v in blocks.values())
    block_str = f"{fmt(blo)}–{fmt(bhi)}"
    if block_str not in doc_text:
        findings.append(f"doc-missing-blocksum: {block_str}")

    totals: dict[str, tuple[int, int]] = {}
    for name in ("lower", "mid"):
        lo, hi = hround(tier_vals[name] + blo), hround(tier_vals[name] + bhi)
        totals[name] = (lo, hi)
        want = f"{lo}–{hi}"
        if want not in doc_text:
            findings.append(f"doc-missing-total-{name}: {want}")

    plan_total = parse_plan_total(plan_text)
    readings = parse_doc_readings(doc_text)
    if len(readings) < 2:
        findings.append(f"doc-seat-readings-too-few: {len(readings)}")
    bounds = [plan_total] + [(a, b) for _, a, b in readings]
    u_lo = min(a for a, _ in bounds)
    u_hi = max(b for _, b in bounds)
    union_str = f"{fmt(u_lo)}–{fmt(u_hi)}"
    if union_str not in doc_text:
        findings.append(f"doc-missing-union: {union_str}")

    m = BASELINE_RE.search(doc_text)
    if not m:
        findings.append("doc-missing-baseline-interval")
    else:
        b_lo, b_hi = float(m.group(1)), float(m.group(2))
        if b_lo >= b_hi:
            findings.append(f"baseline-inverted: {fmt(b_lo)}-{fmt(b_hi)}")
        pm = PROMISE_RE.search(doc_text)
        if not pm:
            findings.append("doc-missing-promise")
        else:
            pv = float(pm.group(1))
            if not (b_lo <= pv <= b_hi):
                findings.append(
                    f"promise-out-of-baseline: {fmt(pv)} not in {fmt(b_lo)}-{fmt(b_hi)}"
                )
        rate = RATE_RE.search(doc_text)
        if not rate:
            findings.append("doc-missing-throughput-rate")
        else:
            r_lo, r_hi = float(rate.group(1)), float(rate.group(2))
            band = f"{hround(b_lo / (r_hi * 5))}–{hround(b_hi / (r_lo * 5))} 周"
            if band not in doc_text:
                findings.append(f"doc-missing-calendar-band: {band}")
            if pm:
                pband = f"{hround(float(pm.group(1)) / (r_hi * 5))}–{hround(float(pm.group(1)) / (r_lo * 5))} 周"
                if pband not in doc_text:
                    findings.append(f"doc-missing-promise-calendar: {pband}")

    if args.disclosure not in doc_text:
        findings.append(f"doc-missing-drift-disclosure: {args.disclosure}")
    if args.require_run_id not in doc_text:
        findings.append(f"doc-missing-run-ref: {args.require_run_id}")

    fresh_report: list[list[float]] = []
    for path in args.fresh_read:
        text = Path(path).read_text(encoding="utf-8", errors="replace")
        cand = [(a, b) for a, b in ranges(text) if 10 <= a < b <= 2000]
        if not cand:
            findings.append(f"fresh-read-no-interval: {path}")
            continue
        a, b = cand[0]
        fresh_report.append([a, b])
        if b < u_lo or a > u_hi or a * 2 < u_lo or b > u_hi * 2:
            findings.append(
                f"fresh-read-drift-too-large: {fmt(a)}-{fmt(b)} vs union {union_str} ({path})"
            )

    print(json.dumps({
        "kind": "est_recompute",
        "census": {"open_total": total, "t0": t0, "t1": t1, "t2": t2},
        "tiers": {k: round(v, 1) for k, v in sorted(tier_vals.items())},
        "non_issue_blocks": {k: [fmt(v[0]), fmt(v[1])] for k, v in sorted(blocks.items())},
        "block_sum": [fmt(blo), fmt(bhi)],
        "totals": {k: [v[0], v[1]] for k, v in sorted(totals.items())},
        "plan_total": [fmt(plan_total[0]), fmt(plan_total[1])],
        "doc_seat_readings": [[s, fmt(a), fmt(b)] for s, a, b in readings],
        "union": [fmt(u_lo), fmt(u_hi)],
        "fresh_read": fresh_report,
        "findings": findings,
    }, ensure_ascii=False, sort_keys=True))
    return 0 if not findings else 1


if __name__ == "__main__":
    sys.exit(main())
