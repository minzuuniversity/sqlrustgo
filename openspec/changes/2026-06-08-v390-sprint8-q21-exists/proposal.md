## Why

TPC-H Q21 (Suppliers Who Kept Orders Waiting Query) is a 4-way JOIN
(supplier × lineitem l1 × orders × nation) with two correlated
EXISTS subqueries on lineitem. At SF=0.1, the in-process executor
times out (1.5+ min) because each correlated EXISTS subquery is
O(N²) (per outer lineitem, scan all 60K lineitems again to check
the EXISTS).

The Q21 N² EXISTS issue is a **Sprint 8 deferred** engine
issue, similar to Q3 and Q8. The fix requires a hash-semi-join
executor that builds a hash table on l_orderkey once and probes
it in O(1) per outer row.

## What Changes

- **No engine code change** for v3.9.0 GA. Q21 deferred to Sprint 8.
- **SPEC-only** under `openspec/changes/2026-06-08-v390-sprint8-q21-exists/`.
- **GA-BLOCKER impact**: 19/22 (with Q15+Q16 sibling) minus Q21 = 18/22,
  same as Q3-only / Q8-only deferred.

## Acceptance Criteria

- [ ] Q21 SPEC recorded with 4 files (proposal, design, tasks, spec).
- [ ] Sprint 8 backlog: hash-semi-join for correlated EXISTS.
- [ ] #3291 sub-task tracker: Q21 deferred.
- [ ] No code change in v3.9.0.

## Sprint 8 Target State

- [ ] `crates/executor/src/join/semi_join.rs` — new file.
- [ ] Hash-semi-join builds hash on inner l_orderkey, probes
      outer l_orderkey in O(1) avg.
- [ ] Q21 in-process < 5s at SF=0.1, < 60s at SF=1.0.
- [ ] No regression on Q3, Q4, Q8, Q10, Q13, Q15, Q16 (all
      queries that use correlated subqueries).
