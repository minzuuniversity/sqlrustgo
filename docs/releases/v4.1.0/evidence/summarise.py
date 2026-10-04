#!/usr/bin/env python3
"""Summarise the #4956 A/B raw results.

Reports per (scenario, side): median-of-rounds for each metric, plus the
after/before ratio. A ratio > 1 means #4956 (flush in the commit path)
made things slower / lower-throughput.
"""
import collections
import re
import statistics
import sys

LINE = re.compile(r"side=(\w+) round=(\d+) RESULT\|([^|]+)\|(.*)")
NUM = re.compile(r"(\w+)=([\d.]+)")

rows = []
for raw in open(sys.argv[1]):
    m = LINE.search(raw)
    if not m:
        continue
    side, rnd, scen, rest = m.groups()
    d = {k: float(v) for k, v in NUM.findall(rest)}
    d["side"] = side
    d["round"] = int(rnd)
    d["scen"] = scen
    rows.append(d)

# Which metrics are "higher is better".
BETTER_HIGH = {"tps"}
METRICS = ["tps", "mean_us", "p50_us", "p90_us", "p99_us", "max_us", "wall_ms", "errors"]

by = collections.defaultdict(lambda: collections.defaultdict(list))
for r in rows:
    by[r["scen"]][r["side"]].append(r)

print("=" * 104)
print("#4956 A/B — flush on the commit path")
print("after  = develop/v4.1.0 (flush before WAL truncation)")
print("before = same tree, `self.inner_mut().flush()` removed from commit_transaction")
print("ratio  = after / before   (>1 on a latency row = #4956 is SLOWER)")
print("=" * 104)

for scen in sorted(by):
    sides = by[scen]
    if "after" not in sides or "before" not in sides:
        continue
    n_a, n_b = len(sides["after"]), len(sides["before"])
    print(f"\n--- {scen}  (rounds: after={n_a} before={n_b}) ---")
    print(f"{'metric':<10} {'after(med)':>12} {'before(med)':>13} {'ratio':>8}   verdict")
    for m in METRICS:
        a = [r[m] for r in sides["after"]]
        b = [r[m] for r in sides["before"]]
        if not a or not b:
            continue
        ma, mb = statistics.median(a), statistics.median(b)
        if mb == 0:
            continue
        ratio = ma / mb
        if m in BETTER_HIGH:
            verdict = "after FASTER" if ratio > 1.02 else ("before faster" if ratio < 0.98 else "~neutral")
        elif m == "errors":
            verdict = "OK" if ma == 0 and mb == 0 else "ERRORS PRESENT"
        else:
            verdict = "after SLOWER" if ratio > 1.02 else ("before slower" if ratio < 0.98 else "~neutral")
        print(f"{m:<10} {ma:>12.1f} {mb:>13.1f} {ratio:>8.3f}   {verdict}")

    # Round-to-round spread: if the two sides' ranges overlap heavily, the
    # difference is inside run-to-run noise and must not be reported as a
    # regression.
    for m in ("tps", "p50_us", "p99_us"):
        a = [r[m] for r in sides["after"]]
        b = [r[m] for r in sides["before"]]
        if len(a) > 1 and len(b) > 1:
            spread_a = (max(a) - min(a)) / statistics.median(a) * 100
            spread_b = (max(b) - min(b)) / statistics.median(b) * 100
            print(f"    {m:<8} round-to-round spread: after {spread_a:5.1f}%  before {spread_b:5.1f}%")

print("\n" + "=" * 104)
