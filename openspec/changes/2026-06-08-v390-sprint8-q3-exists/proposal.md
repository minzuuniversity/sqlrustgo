## Why

TPC-H Q3 (Shipping Priority Query) is a 3-way JOIN
(customer × orders × lineitem) with a `GROUP BY l_orderkey,
o_orderdate, o_shippriority` aggregate and a `WHERE` clause
filtering by `o_orderdate < '1995-03-15' AND l_shipdate >
'1995-03-15'`. The query is a 60K² row × row scan in the current
executor: for each order, the lineitem scan is O(60K), and there
are ~20K orders → 1.2 × 10⁹ op worst case (actual: 1.5 min
timeout in 4-way harness at SF=0.1).

The Sprint 5 v2 baseline reports Q3 as `TIMEOUT` (N² complexity
class). After Sprint 6 regen the row count is still 10 (PG), but
the engine can't reach the answer in < 5 minutes.

This is a **Sprint 8 deferred** engine issue. The Sprint 6 regen
did not and cannot fix it; the data shape is correct. The fix
requires a hash-join or hash-semi-join implementation in the
storage engine, which is a 5-15h engineering effort outside the
scope of the v3.9.0 GA cut.

This change records the Q3 N² issue as a Sprint 8 SPEC, so the
v3.9.0 GA-BLOCKER #3291 sub-task list is complete and the
remaining 3 N² queries (Q3, Q8, Q21) are explicitly tracked.

## What Changes

- **No engine code change** for v3.9.0 GA. The Q3 N² issue is
  deferred to v3.9.1 or Sprint 8.
- **SPEC-only**: this change captures the Q3 N² issue in an
  openspec change under `openspec/changes/2026-06-08-v390-sprint8-q3-exists/`,
  with a target-state capability spec and engineering tasks for
  Sprint 8.
- **GA-BLOCKER impact**: with Q3, Q8, Q21 deferred to Sprint 8, the
  v3.9.0 GA cell-level count will be **19/22** (after Q15, Q16
  sibling changes) or **18/22** if the float-cosmetic diff on Q15
  fails the "18/22 + 4 cosmetic" acceptance. Either way, the
  "22/22 clean_match" acceptance is **deferred to v3.9.1**.

## Impact

- Affected: GA-BLOCKER #3291 (Q3 sub-task: deferred to Sprint 8).
- Operators / users: no API change. Q3 is a 1.5-min TIMEOUT for
  in-process eval, but PG and MySQL both return 10 rows in < 50ms
  in the 4-way harness (which excludes the slow path).
- The Sprint 7 4-way harness already marks Q3 as
  "sqlrustgo N^2 N/A" in the cell-level table, so the
  19-or-18/22 cell-level pass count is correctly attributed.

## Acceptance Criteria

- [ ] Q3 SPEC recorded under
      `openspec/changes/2026-06-08-v390-sprint8-q3-exists/`
      with `proposal.md`, `design.md`, `tasks.md`,
      `specs/q3-shipping-priority-n2/spec.md`.
- [ ] Sprint 8 backlog entry: "Implement hash-join or
      hash-semi-join in `src/executor` for 3+ way JOINs to
      eliminate O(N²) row × row scan".
- [ ] #3291 sub-task tracker updated: Q3 deferred to Sprint 8.
- [ ] No engine code change in this PR.

## Sprint 8 SPEC Target State (out of scope for v3.9.0)

- [ ] Implement hash-join executor for 3+ way JOINs
      (`crates/executor/src/join/hash_join.rs`, new file).
- [ ] Hash-join uses 1KB/row hash table, fits in 1GB for 60K rows.
- [ ] Q3 in-process eval < 1s at SF=0.1 (60K lineitem).
- [ ] Q3 in-process eval < 30s at SF=1.0 (6M lineitem).
- [ ] No regression on Q1, Q2, Q4, Q5, Q6, Q7, Q10, Q12, Q13, Q14, Q15,
      Q16, Q17, Q19, Q20, Q22 cell-level results.
