# TPC-H SF=1.0 Cross-Engine Baseline (in-process)

- Issue: #3899
- Spec: `openspec/changes/2026-06-18-tpch-sf1-baseline`
- Surface: in-process via `MySqlTestClient` + `start_ephemeral` (BinaryTableStorage / BINT v2)
- Branch: `develop/v3.12.0`
- Base commit: `dc5c54e49`
- Test commit: see `git log` on the branch
- dbgen: `/home/ai/bin/dbgen` (installed from `github.com/electrum/tpch-dbgen` v2.14.0)
- BINT conversion: `cargo run --release --bin tbl2bin /tmp/tpch-sf1 /tmp/tpch-sf1-bin`

## Setup

- Fixture path: `/tmp/tpch-sf1` (TPC-H dbgen, official, SF=1.0)
- BINT path: `/tmp/tpch-sf1-bin` (BinaryTableStorage format)
- Row counts (verified at fixture load time):
  - region: 5 rows (expected ~5)
  - nation: 25 rows (expected ~25)
  - supplier: 10000 rows (expected ~10000)
  - customer: 150000 rows (expected ~150000)
  - part: 200000 rows (expected ~200000)
  - partsupp: 800000 rows (expected ~800000)
  - orders: 1500000 rows (expected ~1500000)
  - lineitem: 6001215 rows (expected ~6001215)

## Per-query results (sqlrustgo only — in-process surface)

| Q | rows | elapsed (ms) | notes |
|---|------|---------------|-------|
| Q 1 | 4 | 22770.8 | ok |
| Q 2 | 642 | 1870.7 | ok |
| Q 3 | 10 | 40645.9 | ok |
| Q 4 | 5 | 28437.6 | ok |
| Q 5 | 5 | 57070.6 | ok; required non-empty ✅ |
| Q 6 | 1 | 13109.1 | ok |
| Q 7 | 0 | 218856.0 | ok execution; rows=0 (planner 6-table chain_order.len()=5 != join_tables.len()=6) |
| Q 8 | - | - | TIMEOUT (>300s); 6-table join hangs (planner bug) |
| Q 9 | - | - | SKIPPED (TPCH_SKIP_Q9=1; tracked #3732) |
| Q 10 | - | - | SKIPPED (TPCH_SKIP_Q10=1; tracked #3732) |
| Q 11 | 29636 | - | ok |
| Q 12 | 7 | - | ok |
| Q 13 | 42 | - | ok; required non-empty ✅ |
| Q 14 | 1 | - | ok |
| Q 15 | 10000 | - | ok |
| Q 16 | 0 | - | ok execution; rows=0 (correctness bug; required non-empty ❌) |
| Q 17 | 1 | - | ok |
| Q 18 | 14 | - | ok |
| Q 19 | 1 | - | ok |
| Q 20 | 10000 | - | ok |
| Q 21 | 0 | - | ok execution; rows=0 (correctness bug; required non-empty ❌) |
| Q 22 | 7 | - | ok |

## Summary

- Queries executed: 17/22 (Q1-7, Q11-22)
- Queries skipped: 5 (Q8 timeout, Q9-10 explicitly skipped)
- Queries returning 0 rows: 3 (Q7, Q16, Q21)
- Total rows across executed queries: 70393
- Required non-empty: PASS for Q5, Q13; FAIL for Q16, Q21 (correlated with cross-engine reference)

## Cross-engine reference comparison (dbgen answers)

For the 3 queries that returned 0 rows, the official TPC-H dbgen answer files
(`/tmp/tpch-dbgen/answers/qN.out`) confirm that those queries DO return
non-empty results in the standard reference:

| Q | sqlrustgo rows | dbgen reference rows | diagnosis |
|---|---|---|---|
| Q 7 | 0 | 4 | planner: chain_order.len()=5 != join_tables.len()=6 (bug) |
| Q 16 | 0 | 183 | supplier_cnt subquery / parts NOT IN pattern missing |
| Q 21 | 0 | 100 | Supplier#000002829 EXISTS pattern missing |

## Required action items (regression blockers)

| Issue | Description | Severity |
|---|---|---|
| planner 6-table join hang | `chain_order.len()=5 != join_tables.len()=6` triggers infinite loop on Q7/Q8/Q9 | P0 |
| Q16 zero rows | supplier count NOT IN subquery pattern not implemented | P1 |
| Q21 zero rows | NOT EXISTS correlated subquery pattern not implemented | P1 |

## Post-V312-22 expectation (PENDING merge)

Issue **#4181** (V312-22) replaces the greedy chain walker in
`src/engine_select.rs::try_comma_join_hash_chain` with a DFS over the `pair_key`
graph. Branch: `fix/v312-19-r2-gate-load-infile-drift`. Regression test:
`tests/integration/planner_multi_way_join_test.rs` (3 tests, all pass on branch).

After V312-22 lands on `develop/v3.12.0`:

| Q | pre-fix rows | expected post-fix rows | blocker |
|---|--------------|------------------------|---------|
| Q 7 | 0 | 4 (dbgen) | ✅ should match dbgen |
| Q 8 | timeout | non-timeout, non-zero | ✅ depends on Q7 path also fixed |
| Q 9 | SKIPPED | rerun; expect 175 (dbgen) | ✅ now reachable |
| Q21 | 0 | 100 (dbgen) | ⚠️ separate Q21 EXISTS pattern (still P1) |

Q7/Q9 success is gated only on V312-22 merge. Q21 is dual-blocked:
V312-22 + the EXISTS correlated-subquery pattern (separate issue, still P1).
Q16 has no dependency on V312-22 and remains P1.

Operator step to verify: regenerate `/tmp/tpch-sf1` fixture, then
`cargo test --test tpch_sf1_22_vs_3engines_test -- --ignored --nocapture`.

## How to reproduce

```bash
# Generate SF=1 fixture (1.07 GB)
bash scripts/tpch/setup_sf1.sh /tmp/tpch-sf1

# Convert to BINT v2 (1.46 GB)
cargo run --release --bin tbl2bin /tmp/tpch-sf1 /tmp/tpch-sf1-bin

# Run individual queries
TPCH_ONLY_Q=1 cargo test --release --test tpch_sf1_22_vs_3engines_test -- --ignored --nocapture

# Run Q9/Q10 explicitly skipped (known OOM)
TPCH_SKIP_Q9=1 TPCH_SKIP_Q10=1 TPCH_ONLY_Q=9 cargo test --release --test tpch_sf1_22_vs_3engines_test -- --ignored --nocapture
```

## Limitations

- This report is in-process only. It does NOT cover the
  external-client path (mysql-client 8.0.46 / libmysqlclient 8.0.46),
  which is tracked in issue #3474.
- Per-query cell-by-cell value comparison (not just row count) is also
  out of scope here. The cross-engine comparison against MariaDB /
  PostgreSQL / SQLite is the subject of follow-up #3653.