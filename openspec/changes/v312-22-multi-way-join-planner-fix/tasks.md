# Tasks

## 1. Implement DFS chain builder

- [x] Read `src/engine_select.rs:1812-2200` to confirm current behavior
- [x] Identify greedy failure mode on TPC-H Q7/Q8/Q9 SF=1 (Q9: chain_order=6 vs join_tables=7)
- [ ] Replace greedy loop with DFS over `pair_key` adjacency in `try_comma_join_hash_chain`
- [ ] Add explicit "disconnected" diagnostic when DFS exhausts without full chain
- [ ] `cargo clippy --all-features -- -D warnings` clean
- [ ] `cargo fmt --check --all` clean

## 2. Add regression test

- [ ] Create `tests/integration/planner_multi_way_join_test.rs`
- [ ] Construct 6 in-memory tables with explicit key columns
- [ ] Assert chain_order.len() == join_tables.len() after planner call
- [ ] Assert result row count matches hand-computed reference
- [ ] `cargo test --test planner_multi_way_join_test --all-features` PASS

## 3. Extend openspec spec

- [ ] Update `openspec/specs/multi-join-3-table-resolution/spec.md` with N-table
      Scenario under Requirement: "Multi-table JOIN column resolution via
      qualifier"

## 4. Verify against SF=1 fixture

- [ ] Ensure SF=1 fixture present at `/tmp/tpch-sf1` (dbgen -s 1 -f)
- [ ] Run `cargo test --test tpch_sf1_22_vs_3engines_test -- --ignored` with
      `TPCH_SKIP_PANIC=1`
- [ ] Confirm Q7 returns non-zero rows matching SQLite oracle
- [ ] Confirm Q9 returns non-zero rows matching SQLite oracle
- [ ] Confirm Q10-Q22 produce non-error result (no panic)

## 5. Evidence + close

- [ ] Write `docs/releases/v3.12.0/evidence/issue-4181/4181_closeout.md`
- [ ] Compute `evidence_hash = sha256(report)` and record
- [ ] Update `docs/releases/v3.12.0/perf/SF1_BASELINE_REPORT.md` to mark
      Q7/Q9 as PASS instead of FAIL
- [ ] Comment on issue #4181 with: branch, commit, evidence_hash, log path
- [ ] PR into `develop/v3.12.0`, verify on merged HEAD before claiming done