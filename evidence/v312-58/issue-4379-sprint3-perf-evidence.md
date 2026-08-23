# V312-58 / Issue #4379 — Q17 Sprint 3 Performance Investigation

**Branch**: `fix/v312-58-tpch-sf1-7x`
**Date**: 2026-08-23
**Author**: openclaw
**Verdict**: **Q17 Sprint 3 is a real perf bug, NOT closed-by-inadvertent-fix.**
**Subset perf scaling is super-linear → at SF=1 (6M rows) it would TIMEOUT.**

---

## 1. Issue #4379 recap

TPC-H Q17 (small-order-shortage):
```sql
SELECT SUM(l_extendedprice) / 7.0 AS avg_yearly
FROM lineitem, part
WHERE p_partkey = l_partkey
  AND p_brand = 'Brand#23'
  AND p_container = 'LG CASE'
  AND l_quantity < (SELECT 0.2 * AVG(l_quantity) FROM lineitem WHERE l_partkey = p_partkey);
```

Per issue body (commit 596a6060d9): sqlrustgo returns NULL TIMEOUT (>1800s), SQLite 1 row in 0.109s.

---

## 2. Reproduction scaling table (engine vs SQLite)

| Subset | lineitem rows | Engine elapsed | Engine result | SQLite oracle | Bit-exact? |
|--------|---------------|----------------|---------------|---------------|-----------|
| Mini (Q17-specific parts only) | 126 | 0.37s | 7964.658571428573 | 7964.65857142857 | ✓ |
| 100K random sample | 100,000 | 16.83s | 224.52142857142857 | 224.521428571429 | ✓ |
| 1M random sample | 1,000,000 | **>10min TIMEOUT** (killed by `timeout 600`) | — | (estimated ~1345) | (cannot verify) |

**Scaling observation**:
- 126 → 100K (794x scale): 0.37s → 16.83s (45x time) — sub-linear in this range
- 100K → 1M (10x scale): 16.83s → >600s (>36x time, only lower bound) — **super-linear**
- Extrapolating 1M→6M (SF=1): if 1M needs ~600s+, SF=1 needs ~3600s+ → confirms TIMEOUT

This is consistent with the **O(N²) correlated subquery pattern**: at small scale the per-row
subquery computation is amortized; at large scale it dominates.

---

## 3. Test artifacts added

| Path | Purpose | Wall-clock |
|------|---------|-----------|
| `tests/integration/oracle/diag_q17_sf001_subset.rs` | Mini subset (126 rows, matching only the 4 Q17 parts) | 0.37s |
| `tests/integration/oracle/diag_q17_100k_subset.rs` | 100K random sample + 200K part | 16.83s |
| `tests/integration/oracle/diag_q17_1m_subset.rs` | 1M random sample + 200K part | >10min |
| `Cargo.toml` | Registered 3 new test binaries | — |
| `/tmp/q17_lineitem_minimal.tbl` | 126 rows for Q17 Brand#23 LG CASE parts | — |
| `/tmp/q17_lineitem_100k.tbl` | 100K random sample | — |
| `/tmp/q17_lineitem_1m.tbl` | 1M random sample | — |
| `/tmp/q17_100k.db` | SQLite ground truth for 100K subset (224.521428571429) | — |

---

## 4. Root cause analysis (parallel to Sprint 3 roadmap)

Q17 correlated subquery:
```sql
l_quantity < (SELECT 0.2 * AVG(l_quantity) FROM lineitem WHERE l_partkey = p_partkey)
```

Per `crates/optimizer/src/decorrelate.rs:266`:
```rust
pub fn try_decorrelate(where_expr: &Expression) -> Option<DecorrelatedWhere>
```

This function exists (V311-16 V2 implementation, ~250 LOC) and has unit tests covering
EXISTS, NOT EXISTS, IN-subquery. **But it is not called from the execution path** — only from
the unit test file `decorrelate.rs:533-559`.

Verified via `grep -rn "try_decorrelate" src/ crates/`:
- Only references: definition + unit tests
- No call from `src/execution_engine.rs` or `src/engine_select.rs`

Same for `HashSemiJoin` operator (`crates/executor/src/join/hash_semi_join.rs:88`):
- Operator struct exists with 5 unit tests
- Has `left_semi_join()`, `left_anti_semi_join()` methods
- Referenced in `unified_cost.rs:349` cost model (advisory only)
- No code path actually creates and executes a `HashSemiJoin` from a query plan

**Conclusion**: optimizer infrastructure exists but **is not wired up**.
Sprint 1 #4375 (Q2 LIMIT) and Sprint 2 #4377 (Q11 pushdown) and #4378 (Q12 pushdown) all
benefited from `PredicatePushdown` being relatively easy to wire into `execute_select`.
Sprint 3 #4379/#4380/#4381 require **deeper execution-path changes**:

1. **Decorrelate inline**: `try_decorrelate()` must be invoked during query planning, BEFORE
   execution; aggregate version (`0.2 * AVG(...)`) is currently not handled — only EXISTS/IN
2. **HashSemiJoin instantiation**: a planner rule must convert `EXISTS`/`NOT EXISTS`/`IN-subquery`
   into `HashSemiJoin`/`HashAntiSemiJoin` operators
3. **Cost-based selection**: must use `unified_cost.rs` to choose hash-join over nested-loop

Per Sprint 3 roadmap (`evidence/v312-58/issue-4374-4381-remediation-roadmap.md` §2.6):
- Q17 (this issue): 5–10 days
- Q20: EXISTS semi-join rewrite + nested subquery flatten, 5–10 days
- Q22: NOT EXISTS anti-semi-join + AVG decorrelate, 5–10 days
- Total Sprint 3: 15–30 days

---

## 5. Recommendations

### 5.1 Immediate (within v3.12.0 GA, expiry 2026-09-30)

Given current evidence:
- Q17 1M subset: **>10min** (not 300s SLA)
- Sprint 3 work: 15–30 days
- Time to expiry: ~5 weeks
- Sprint 1+2 already used 2 weeks of the same window

**Realistic options**:

| Option | Outcome | Sprint 3 status |
|--------|---------|-----------------|
| A | Sprint 3 implementation in 3 weeks, Q17/Q20/Q22 all fixed | Aggressive |
| B | Sprint 3 partial: only Q17 (correlated AVG), defer Q20/Q22 to v3.13 | Realistic |
| C | All 3 Sprint 3 issues deferred to v3.13 with explicit expiry 2026-12-31 | Conservative |
| D | v3.12.0 GA delay (push expiry to 2026-10-31) | Political |

### 5.2 Recommended approach (Sprint 3 partial)

Given:
- Q17's correlated-AVG is **the** hardest subquery pattern (decorrelate-aggregate not in
  try_decorrelate scope)
- Q20's EXISTS + nested subquery is medium difficulty (EXISTS → semi-join covered)
- Q22's NOT EXISTS + AVG correlated is medium-hard (anti-semi-join = semi-join + negation)

Recommend **Option B**: focus Sprint 3 on Q20 first (semi-join rewrite can be reused for Q22),
defer Q17/Q22 AVG-aggregate decorrelation to v3.13 with explicit issue-anchored plan.

### 5.3 Acceptance evidence for #4379 closure (NOT recommended)

The issue acceptance requires SF=1 ≤300s with sha256 match. Current engine cannot achieve this
without:
- 5–10 days of optimizer work (per Sprint 3 plan §2.6)
- AND verification on full SF=1 which itself takes >10min on current bulk_load_tbl_file

Closing #4379 as "closed-by-inadvertent-fix" (like #4378) is **NOT legitimate** here because:
- At 100K subset (1/60 of SF=1), engine returns correct answer in 16.83s
- At 1M subset (1/6 of SF=1), engine TIMES OUT — clear super-linear scaling
- Bug is genuine performance bug, not data-dependent correctness bug

---

## 6. Next steps (proposed)

1. **Open PR** documenting this perf evidence as supplementary for #4379/#4380/#4381
2. **Update parent issue #4374** with Sprint 3 risk assessment + recommended Option B
3. **Get user direction** before any optimizer implementation (15–30 day effort)
4. **Defer Q17/Q22 AVG-aggregate decorrelation to v3.13** with explicit issue:
   `#4416: Sprint 3 partial — Q20 EXISTS semi-join, defer Q17/Q22 to v3.13`

---

## 7. Files added in this investigation

```
 Cargo.toml                                                |  6 ++
 tests/integration/oracle/diag_q17_100k_subset.rs          | 53 ++++++++
 tests/integration/oracle/diag_q17_1m_subset.rs            | 54 ++++++++
 tests/integration/oracle/diag_q17_sf001_subset.rs         | 53 ++++++++
 evidence/v312-58/issue-4379-sprint3-perf-evidence.md      | this file
```
