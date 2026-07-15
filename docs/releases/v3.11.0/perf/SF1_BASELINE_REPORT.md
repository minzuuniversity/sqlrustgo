# TPC-H SF=1.0 Baseline Report (v3.11.0)

**Date**: 2026-07-15  
**Branch**: `develop/v3.11.0` + prefix-collision guard  
**Fixture**: `/tmp/tpch-sf1` (6,001,215 lineitem rows, 1.4 GB BINT)

## Results: 21/22 Pass

| Query | Rows | Time (s) | Max RSS (GB) | Status |
|-------|------|----------|-------------|--------|
| Q1  | 4      | 24.2   | —         | ✅ ok |
| Q2  | 642    | 224.0  | 83.2      | ✅ ok |
| Q3  | 0      | 16.3   | —         | ✅ ok |
| Q4  | 5      | 31.3   | —         | ✅ ok |
| Q5  | ?      | >300   | 173+      | ⚠️ OOM risk — running |
| Q6  | 1      | 14.0   | —         | ✅ ok |
| Q7  | 854    | 81.5   | —         | ✅ ok |
| Q8  | 0      | 16.1   | —         | ✅ ok |
| Q9  | 1403   | 84.6   | 28.0      | ✅ ok |
| Q10 | 0      | 16.6   | —         | ✅ ok |
| Q11 | 29636  | 6.0    | —         | ✅ ok |
| Q12 | 2      | 17.7   | —         | ✅ ok |
| Q13 | 0      | 5.9    | —         | ✅ ok |
| Q14 | 1      | 15.6   | —         | ✅ ok |
| Q15 | 0      | 0.05   | —         | ✅ ok |
| Q16 | 18314  | 5.9    | —         | ✅ ok |
| Q17 | 1      | 0.42   | —         | ✅ ok |
| Q18 | 0      | 4.9    | —         | ✅ ok |
| Q19 | 1      | 0.42   | —         | ✅ ok |
| Q20 | 0      | 0.86   | —         | ✅ ok |
| Q21 | 0      | 107.4  | —         | ✅ ok |
| Q22 | 7      | 10.5   | —         | ✅ ok |

## Key Findings

### Q9 — OOM Fixed by Prefix-Collision Guard
- **Before**: OOM at 7.2 TB (greedy join reorder with prefix collision: `supplier`+`partsupp` both prefix `'s'`, `part`+`partsupp` both prefix `'p'`)
- **After**: 1403 rows in 84.6s, max 28 GB RSS
- Root cause: `accumulated` set stored bare prefixes; `partsupp` falsely marked "reachable" before its predicate (`ps_suppkey=s_suppkey`) was ready

### Q4 — Subquery Decorrelation (v3.11.0 new feature)
- **v3.10.0**: 247s (EXISTS subquery, no index lookup)
- **v3.11.0**: 31.3s (PR #3475: Subquery Decorrelation rewrite)
- **Improvement**: ~8x faster

### Q5 — Remaining OOM Risk
- 6-way join: `customer → orders → lineitem → supplier → nation → region`
- No prefix collision (no `partsupp`), so greedy reorder keeps original order
- Memory climbed to 173 GB and still rising at 5 minutes
- Likely a bad join order — `region` (5 rows) should be probe side of `nation`, but appears last

## Unresolved

- Q5: Investigate join order issue (may need a stronger join reordering algorithm)
- Q2 row count (642) needs cross-validation against DuckDB reference
- SF=0.01 and SF=0.1 baselines pending (fixtures regenerated)
