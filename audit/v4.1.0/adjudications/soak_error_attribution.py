#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""SOAK 错误率离线归因分析 —— 最终版（v3）。

两轮方法学纠错，都留在这份文件里，避免下一个人重犯：

v1 的假象
  把错误行的第 2 个字段当 elapsed_s 分桶，得到「72.9% 的错误发生在高负载窗」。
  **错**：该字段根本不是时间戳。

v2 的残余问题
  改成只用 metrics.csv 做时间维（正确），但仍把 errno=2013 的 f2 当成 elapsed_s 去做
  「爆发窗口」分析 ⇒ 又错了一次。

v3 的定论（仓内源码定案）
  `scripts/soak/pymysql_driver.py`：
    第 17 行 注释： Q,<elapsed_ms>,<thread_id>,<query_kind>,<row_count>
    第 18 行 注释： E,<elapsed_ms>,<thread_id>,query,<error_msg>
    第 95 行      output_fp.write(f"E,0,{thread_id},connect,{str(e)[:200]}\\n")   ← 连接错误写 elapsed_ms=0
    第 112 行     output_fp.write(f"Q,{ms:.1f},{thread_id},{q_fn.__name__},{row_count}\\n")
    第 118 行     output_fp.write(f"E,{ms:.1f},{thread_id},query,{err_str}\\n")   ← 查询错误写真实耗时
  ⇒ **f2 = 单次操作的耗时（毫秒），不是时间戳**；错误日志**没有任何墙钟时间轴**。
  ⇒ 时间维归因**只能**用 metrics.csv（每 15 s 一行，带 elapsed_s 与 load1）。

  同文件第 87 行：connect_timeout=5, read_timeout=15, write_timeout=15
  ⇒ 错误里耗时落在 15000–16914 ms 的那一类，**正是 15 秒读超时的指纹**。

因此：
  · 时间维结论（负载↔错误率的剂量-反应关系）只来自 metrics.csv；
  · 类型维结论（两类错误的形态）来自错误日志，但它**不可时间对齐**；
  · 剩下「类型 × 时间」的交叉**证据不足**，如实记为未归因（AFP：缺失证据保持 incomplete）。
"""
import csv
import json
import os
import re
import statistics
import sys
from collections import Counter, defaultdict

SNAP = sys.argv[1] if len(sys.argv) > 1 else "/tmp/_soak_snap"
OUT = sys.argv[2] if len(sys.argv) > 2 else "/tmp/_soak_attribution_v3.json"
REPO = sys.argv[3] if len(sys.argv) > 3 else "/Volumes/workspace/dev/srg-gate-d1"

LINE_RE = re.compile(r"^E,([^,]*),([^,]*),([^,]*),\((.+)\)\s*$")

# ---------------------------------------------------------------- metrics.csv
rows = []
with open(os.path.join(SNAP, "metrics.csv"), encoding="utf-8", newline="") as fh:
    for r in csv.DictReader(fh):
        try:
            rows.append({
                "ts": r["ts"], "elapsed_s": int(r["elapsed_s"]),
                "cpu_pct": float(r["cpu_pct"]), "driver_q": int(r["driver_q"]),
                "driver_e": int(r["driver_e"]), "panic_count": int(r["panic_count"]),
                "host_load1": float(r["host_load1"]), "host_busy": int(r["host_busy"]),
            })
        except (ValueError, KeyError):
            continue
rows.sort(key=lambda x: x["elapsed_s"])
prev = None
for r in rows:
    r["d_e"] = 0 if prev is None else r["driver_e"] - prev["driver_e"]
    r["d_q"] = 0 if prev is None else r["driver_q"] - prev["driver_q"]
    prev = r

# ---------------------------------------------------------------- 错误日志
classes = Counter()
per_class = defaultdict(list)
ops = Counter()
total_lines = unparsed = 0
with open(os.path.join(SNAP, "driver_errors.log"), encoding="utf-8", errors="replace") as fh:
    for line in fh:
        line = line.strip()
        if not line:
            continue
        total_lines += 1
        m = LINE_RE.match(line)
        if not m:
            unparsed += 1
            continue
        f2, f3, op, rest = m.groups()
        key = (op, rest[:80])
        classes[key] += 1
        ops[op] += 1
        try:
            per_class[key].append(float(f2))
        except ValueError:
            pass

# ---------------------------------------------------------------- 驱动源码定案
src_path = os.path.join(REPO, "scripts/soak/pymysql_driver.py")
src_evidence = {}
if os.path.isfile(src_path):
    src = open(src_path, encoding="utf-8").read().split("\n")
    for i, line in enumerate(src, 1):
        if re.search(r'read_timeout\s*=\s*15', line):
            src_evidence["read_timeout_15_line"] = i
            src_evidence["read_timeout_15_text"] = line.strip()
        if 'f"E,0,{thread_id},connect' in line:
            src_evidence["connect_err_elapsed0_line"] = i
        if 'f"E,{ms:.1f},{thread_id},query' in line:
            src_evidence["query_err_elapsedms_line"] = i
        if line.strip().startswith("Q,<elapsed_ms>"):
            src_evidence["format_doc_line"] = i
            src_evidence["format_doc_text"] = line.strip()

# ---------------------------------------------------------------- 剂量-反应
TIERS = [(0, 5), (5, 10), (10, 20), (20, 50), (50, 1e9)]
tier_out = []
for lo, hi in TIERS:
    sel = [r for r in rows if lo <= r["host_load1"] < hi]
    e = sum(r["d_e"] for r in sel)
    q = sum(r["d_q"] for r in sel)
    tier_out.append({
        "load1_range": [lo, (None if hi > 1e8 else hi)],
        "samples": len(sel),
        "errors": e, "queries": q,
        "err_rate_pct": round(100.0 * e / max(q, 1), 4),
    })

def pearson(a, b):
    n = len(a)
    if n < 3:
        return None
    ma, mb = sum(a) / n, sum(b) / n
    va = sum((x - ma) ** 2 for x in a)
    vb = sum((y - mb) ** 2 for y in b)
    if va <= 0 or vb <= 0:
        return None
    return sum((a[i] - ma) * (b[i] - mb) for i in range(n)) / ((va ** 0.5) * (vb ** 0.5))

xs = [r["host_load1"] for r in rows]
ys = [r["d_e"] for r in rows]
qs = [r["d_q"] for r in rows]

loads = sorted(xs)
p50 = statistics.median(loads)
p75 = loads[int(len(loads) * 0.75)]
hi = [r for r in rows if r["host_load1"] >= p75]
lo_ = [r for r in rows if r["host_load1"] < p75]
def agg(sel):
    e = sum(r["d_e"] for r in sel); q = sum(r["d_q"] for r in sel)
    return {"samples": len(sel), "errors": e, "queries": q,
            "err_rate_pct": round(100.0 * e / max(q, 1), 4),
            "mean_load1": round(statistics.mean([r["host_load1"] for r in sel]), 2) if sel else None}

# ---------------------------------------------------------------- 类型维
class_out = []
for k, c in classes.most_common():
    v = per_class[k]
    class_out.append({
        "op": k[0], "error_text": k[1], "count": c,
        "pct_of_errors": round(100.0 * c / max(total_lines, 1), 3),
        "duration_ms_min": min(v) if v else None,
        "duration_ms_median": statistics.median(v) if v else None,
        "duration_ms_max": max(v) if v else None,
        "duration_ms_zero_fraction": round(sum(1 for x in v if x == 0.0) / max(len(v), 1), 4),
        "matches_15s_read_timeout": (
            bool(v) and statistics.median(v) >= 14000 and statistics.median(v) <= 20000
        ),
        "is_instant_failure": bool(v) and statistics.median(v) < 50,
    })

out = {
    "snapshot_dir": SNAP,
    "method": "v3 —— 时间维只信 metrics.csv；类型维来自错误日志；f2 语义由仓内源码定案",
    "field_semantics_proof": {
        "source": "scripts/soak/pymysql_driver.py",
        "evidence": src_evidence,
        "conclusion": "f2 = 单次操作耗时(ms)，**不是时间戳** ⇒ 错误日志无墙钟时间轴 ⇒ "
                      "「类型 × 时间」交叉不可做（未归因）",
    },
    "error_lines_total": total_lines,
    "error_lines_unparsed": unparsed,
    "metrics_rows": len(rows),
    "metrics_cumulative_driver_e": rows[-1]["driver_e"] if rows else None,
    "metrics_cumulative_driver_q": rows[-1]["driver_q"] if rows else None,
    "totals_reconcile": total_lines == (rows[-1]["driver_e"] if rows else -1),
    "panic_count_max": max((r["panic_count"] for r in rows), default=None),
    "op_distribution": dict(ops),
    "classes": class_out,
    "load_dose_response": tier_out,
    "load_split_measured_quantiles": {
        "p50_load1": p50, "p75_load1": p75,
        "high": agg(hi), "low": agg(lo_),
        "rate_ratio_high_over_low": (
            round((agg(hi)["err_rate_pct"]) / max(agg(lo_)["err_rate_pct"], 1e-12), 2)
            if agg(lo_)["err_rate_pct"] else None
        ),
    },
    "correlations": {
        "pearson_delta_e_vs_load1": pearson(xs, ys),
        "pearson_delta_e_vs_delta_q": pearson(qs, ys),
    },
    "attribution": {
        "attributed_to_load_pressure": {
            "basis": "metrics.csv 逐采样 Δe/Δq 对 host_load1 的剂量-反应（分档错误率随 load1 单调上升）"
                     " + Pearson 相关系数",
            "high_load_error_rate_pct": agg(hi)["err_rate_pct"],
            "low_load_error_rate_pct": agg(lo_)["err_rate_pct"],
            "ratio": (round(agg(hi)["err_rate_pct"] / max(agg(lo_)["err_rate_pct"], 1e-12), 2)
                      if agg(lo_)["err_rate_pct"] else None),
        },
        "unattributed_residual": {
            "what": "错误日志无墙钟时间轴 ⇒ 两类错误（瞬时失败 / 15s 读超时）各自落在哪些负载区间**不可考**；"
                    "且无法排除「只在高负载下才显现的产品缺陷」这一可能",
            "errors_without_wallclock": total_lines,
            "pct": 100.0,
            "not_a_claim": "本行**不**宣称全部错误均由争用造成；AFP：缺失证据保持 incomplete",
        },
    },
    "no_threshold_or_judgement_change": "本次分析不剔除争用窗口、不改验收阈值、不改判据、不重启 SOAK",
}
with open(OUT, "w", encoding="utf-8") as fh:
    json.dump(out, fh, ensure_ascii=False, indent=1)

print("字段语义定案（仓内源码）:", json.dumps(src_evidence, ensure_ascii=False))
print("错误行 =", total_lines, "｜ metrics 累计 driver_e =", out["metrics_cumulative_driver_e"],
      "｜ 总数核对:", out["totals_reconcile"], "｜ panic_max =", out["panic_count_max"])
print("op 分布:", out["op_distribution"])
print()
print("错误类型（按耗时形态）:")
for c in class_out:
    print("  op=%-7s count=%-6d (%5.2f%%) 耗时ms: min=%s p50=%s max=%s | 瞬时失败=%s 命中15s读超时=%s"
          % (c["op"], c["count"], c["pct_of_errors"], c["duration_ms_min"], c["duration_ms_median"],
             c["duration_ms_max"], c["is_instant_failure"], c["matches_15s_read_timeout"]))
print()
print("剂量-反应（load1 分档 → 错误率）:")
for t in tier_out:
    print("  load1 %-12s samples=%-5d errors=%-6d queries=%-8d err_rate=%.4f%%"
          % (str(t["load1_range"]), t["samples"], t["errors"], t["queries"], t["err_rate_pct"]))
print()
print("高/低负载分档:", json.dumps(out["load_split_measured_quantiles"], ensure_ascii=False))
print("相关系数:", out["correlations"])
print()
print("归因结论:", json.dumps(out["attribution"], ensure_ascii=False, indent=1))
print("→ 完整结果:", OUT)
