## Why

TPC-H Q8 (National Market Share Query) is an 8-way JOIN
(customer × orders × lineitem × supplier × nation n1 × nation n2
× region) with a WHERE filter and a CASE WHEN aggregate. At
SF=0.1, the in-process executor times out (1.5+ min) because:
1. The 8-way nested-loop join is O(N⁸) in the worst case.
2. The CASE WHEN `n2.n_name = 'GERMANY'` aggregate requires
   evaluating the row-by-row join result, which is O(N) per row.

After Sprint 6 regen and Q19 CASE WHEN fix (my commit), Q8's
row count would still be 2 (PG), but the engine can't reach
the answer in < 5 min.

The Q8 N² EXISTS issue is a **Sprint 8 deferred** engine
issue, similar to Q3. The fix requires a hash-join executor
plus a CASE WHEN short-circuit when `n2.n_name` is filtered
out early.

## What Changes

- **No engine code change** for v3.9.0 GA. Q8 deferred to Sprint 8.
- **SPEC-only** under `openspec/changes/2026-06-08-v390-sprint8-q8-exists/`.
- **GA-BLOCKER impact**: 19/22 (with Q15+Q16 sibling) minus Q8 = 18/22,
  same as Q3-only deferred.

## Acceptance Criteria

- [ ] Q8 SPEC recorded with 4 files (proposal, design, tasks, spec).
- [ ] Sprint 8 backlog: hash-join + 8-way JOIN optimizer + CASE WHEN
      short-circuit.
- [ ] #3291 sub-task tracker: Q8 deferred.
- [ ] No code change in v3.9.0.

## Sprint 8 Target State

- [ ] `src/engine_select.rs`: 8-way JOIN hash-join path.
- [ ] `crates/executor/src/aggregate/case_when_optimizer.rs`:
      pre-compute `n_name = 'GERMANY'` flag per row, avoid
      evaluating CASE WHEN twice.
- [ ] Q8 in-process < 5s at SF=0.1, < 60s at SF=1.0.
- [ ] No regression on Q1-Q7, Q10-Q22.

## Sprint 8 Effort

Estimated 8-12h engineering (more than Q3 because of the 8-way
join optimization complexity).
