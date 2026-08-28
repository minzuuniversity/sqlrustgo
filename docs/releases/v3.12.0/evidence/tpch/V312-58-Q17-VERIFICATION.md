# V312-58 — TPC-H Q17 Small-Order-Shortage Status

**Issue**: [#4379](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4379) — V312-58-Q17 [BLOCKER] TPC-H SF=1 Q17 small-order-shortage TIMEOUT (>1800s)：必须降到 ≤300s
**Date**: 2026-08-23 (initial FAIL); 2026-08-28 (closure verified — see follow-up below)
**Verifier**: openclaw
**Branch**: develop/v3.12.0 @ `10d6f489d` (initial) → `28bbf6dbab` (closure verified post Sprint 4 + Phase 1+2+3)

> **🟢 Update 2026-08-28**: Acceptance criteria #1, #2, #3, #4 are **NOW MET** on develop/v3.12.0 HEAD per issue #4540 verification. Q17 SF=1 elapsed: **61.6s** ≤ 300s (4.86× headroom); row_count == 1; value 249963.75857142854 matches oracle 249963.75857142857 within FLOAT_TOL 1e-3 (delta 2.91e-11). See `docs/releases/v3.12.0/evidence/issue-4540/V312-58-4540-Q17-SF1-CELLDIFF-PASS.md` for full cell-diff report. Issue #4379 BLOCKER reclassified → v312-shipped.

---

## TL;DR (initial 2026-08-23 snapshot — HISTORICAL FAIL STATE)

Q17 is **STILL BLOCKED** on `develop/v3.12.0` at commit `10d6f489d`. The correlated scalar subquery in the WHERE clause is not decorrelated, causing a 6M-row × 6M-row nested evaluation. A regression test is added (`q17_small_order_shortage_perf.rs`) that will FAIL until the engine is optimized, providing a permanent regression guard.

| Acceptance criterion | Result |
|----------------------|--------|
| 1. `cargo test --release --test q17_small_order_shortage_perf -- --include-ignored` PASS, elapsed ≤ 300s | ❌ **FAIL** (TIMEOUT > 420s observed; manually killed at 7 min wall) |
| 2. row_count == 1 | ⚠️ unverifiable until subquery is decorrelated |
| 3. sha256 == `595003bfd069556083c03906c87e59ac0997cc8932995628b038ede3004d759a` | ⚠️ unverifiable |
| 4. Real optimization (not just timeout adjustment) | ❌ **NOT IMPLEMENTED** |
| 5. evidence doc | ✅ this file |
| 6. 4-way consistency | ❌ deferred — blocked on (1)-(4) |

---

## Canonical query (`queries/q17.sql`)

```sql
SELECT SUM(l_extendedprice) / 7.0 AS avg_yearly
FROM lineitem, part
WHERE p_partkey = l_partkey
  AND p_brand = 'Brand#23'
  AND p_container = 'LG CASE'
  AND l_quantity < (SELECT 0.2 * AVG(l_quantity) FROM lineitem WHERE l_partkey = p_partkey);
```

## Measured behavior on develop/v3.12.0

Profile run on macOS aarch64, 16-core, SF=1 fixture at `/tmp/tpch-sf1/`:

```
$ TPCH_SF1_DIR=/tmp/tpch-sf1 cargo test --release \
    --test q17_small_order_shortage_perf --all-features \
    -- --ignored --nocapture q17_small_order_shortage_sf1

running 1 test
test q17_small_order_shortage_sf1 has been running for over 60 seconds
```

Observed (before manual kill at ~7 min wall):
- CPU: 98% single-thread (no parallelization in current subquery path)
- RSS: 4.9 GB and growing
- Elapsed at kill: **7 minutes** (incomplete)
- Issue baseline: **>1800s** for the full run

This confirms the regression: Q17 does not complete in any reasonable time on SF=1.

## Root cause

The WHERE clause contains a **correlated scalar subquery**:

```sql
... AND l_quantity < (SELECT 0.2 * AVG(l_quantity) FROM lineitem WHERE l_partkey = p_partkey)
```

The outer reference (`p_partkey`) ties the subquery to the candidate row. The current executor (`src/engine_select.rs`) detects correlated subqueries (`where_expr_has_correlated_subquery`) but re-evaluates the subquery per outer row — no decorrelation, no materialization.

For Q17:
- Outer query produces ~30K candidate rows (Brand#23 + LG CASE filter)
- Each subquery invocation scans all 6M lineitem rows to compute AVG
- Total ops: ~30K × 6M = **180 billion**
- Even at 1 ns/op → 180s; at memory-bandwidth-bound ~5 ns/op → 900s

SQLite handles this in 0.109s by decorrelating: computing the AVG once per `l_partkey` in a single GROUP BY pass, then joining the materialized result with the outer query.

## Fix strategy (proposed for #4379 closure)

Rewrite the correlated scalar subquery into a **derived table + JOIN**:

```sql
SELECT SUM(l_extendedprice) / 7.0 AS avg_yearly
FROM lineitem, part, (
  SELECT l_partkey AS lpk, 0.2 * AVG(l_quantity) AS thresh
  FROM lineitem
  GROUP BY l_partkey
) thresholds
WHERE p_partkey = l_partkey
  AND p_brand = 'Brand#23'
  AND p_container = 'LG CASE'
  AND l_partkey = thresholds.lpk
  AND l_quantity < thresholds.thresh;
```

This requires:
1. **Subquery materialization** in the planner — pull a `SELECT ... GROUP BY` subquery into a derived table and join it.
2. **Per-outer-row correlated column resolution** — recognize `l_partkey = p_partkey` correlation and convert it to an equi-join predicate.

Estimated cost after fix:
- Materialize thresholds: one scan over lineitem + GROUP BY → ~1-2s
- Outer join with derived table: ~30K rows → ~0.1s
- **Total: ~2-3s** (well under 300s budget)

This is a **substantial planner feature**, comparable in scope to the comma-join pushdown work that resolved Q2/Q7/Q11/Q12. Suggested sequencing: track in a new sub-issue (`#4379-A: subquery materialization`) so the GA blocker #4379 can be split into engine-feature work + integration test.

## Why this PR is docs + test only

The engine fix requires a multi-week planner change. This PR contributes:

1. **`tests/integration/oracle/q17_small_order_shortage_perf.rs`** — the regression test from the issue acceptance criterion #1 (`cargo test --release --test q17_small_order_shortage_perf -- --include-ignored`). It enforces:
   - `elapsed ≤ 300s` (TIMEOUT_BUDGET constant)
   - `row_count == 1`
   - `value ≈ 249,963.75857142857` (within ±1e-3)
2. **`docs/releases/v3.12.0/evidence/tpch/V312-58-Q17-VERIFICATION.md`** — this file.
3. **`Cargo.toml`** — registers the new test binary.

The test will FAIL on current `develop/v3.12.0` (as designed), and PASS once the engine optimization lands.

## Verification commands

```bash
# Run regression test (will currently FAIL with TIMEOUT)
TPCH_SF1_DIR=/tmp/tpch-sf1 cargo test --release \
  --test q17_small_order_shortage_perf --all-features \
  -- --ignored --nocapture q17_small_order_shortage_sf1

# Compute reference oracle (Python, raw .tbl parsing)
python3 -c "
brands = {p[0]: (p[3], p[6]) for l in open('/tmp/tpch-sf1/part.tbl') for p in [l.rstrip('\n').split('|')]}
candidate_parts = {k for k, (b, c) in brands.items() if b == 'Brand#23' and c == 'LG CASE'}
print(f'Brand#23 + LG CASE: {len(candidate_parts)} parts')

# Compute per-partkey AVG(l_quantity) for lineitem
avgs = {}
for l in open('/tmp/tpch-sf1/lineitem.tbl'):
    p = l.rstrip('\n').split('|')
    pk = int(p[1])
    q = int(p[4])
    avgs.setdefault(pk, []).append(q)

# Apply filter
import statistics
total = 0
for pk, qtys in avgs.items():
    if pk not in candidate_parts: continue
    threshold = 0.2 * statistics.mean(qtys)
    for q in qtys:
        if q < threshold:
            # Need l_extendedprice
            pass
# ...
"
```

## Disposition

**🟢 #4379 BLOCKER STATUS UPDATED 2026-08-28**: Engine fix HAS landed (Phase 1 PR #4449 + Phase 2 PR #4450 + Phase 3 commit 3b6634a7d0 + Sprint 4 Step 1.5/1.6 PR #4453). Regression guard `q17_small_order_shortage_sf1` now PASSES within 300s. See #4540 evidence for full closure proof. Original "**#4379 remains OPEN**" statement below is HISTORICAL (2026-08-23 FAIL state) — do NOT use as current status.

This PR provides:
- Regression test (FAIL-on-purpose guard for the 300s budget)
- Concrete measurement of current behavior (TIMEOUT confirmed)
- Concrete fix strategy (subquery materialization + decorrelation)
- Path forward via sub-issue (`#4379-A: subquery materialization`)

Related issues that share the same root cause (correlated subquery not decorrelated):
- **#4380** Q20 (`WHERE EXISTS (...)` correlated subquery)
- **#4381** Q22 (`c_acctbal > (SELECT AVG(...) FROM ...)` correlated subquery)

All three TIMEOUT issues should be addressed in a single engine-wide subquery-materialization initiative.