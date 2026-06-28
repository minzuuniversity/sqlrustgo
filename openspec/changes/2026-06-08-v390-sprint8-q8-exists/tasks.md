## 1. Set up Sprint 8 SPEC worktree

- [x] 1.1 Use existing worktree from `develop/v3.9.0`.

## 2. Add SPEC scaffolding (no code change)

- [x] 2.1 `openspec/changes/2026-06-08-v390-sprint8-q8-exists/proposal.md`
- [x] 2.2 `openspec/changes/2026-06-08-v390-sprint8-q8-exists/design.md`
- [x] 2.3 `openspec/changes/2026-06-08-v390-sprint8-q8-exists/tasks.md`
- [x] 2.4 `openspec/changes/2026-06-08-v390-sprint8-q8-exists/specs/q8-national-market-share-n2/spec.md`

## 3. Update v3.9.0 #3291 sub-task tracker

- [x] 3.1 Add Q8 to "Sprint 8 deferred" in
      `docs/audit/status/2026-06-08-SPRINT5_V2_BUG_ANALYSIS.md`.

## 4. Update Sprint 7 audit report

- [x] 4.1 Confirm Q8 marked "N² N/A (Sprint 8 deferred)" in
      `docs/audit/status/2026-06-07-tpch-cell-diff-v390-regen-sprint7.md`.

## 5. Commit + Push PR (SPEC-only)

- [x] 5.1 `git add openspec/changes/2026-06-08-v390-sprint8-q8-exists/`
- [x] 5.2 `git commit -m "spec(v3.9.0): defer Q8 8-way JOIN to Sprint 8 hash-join + optimizer"`
- [x] 5.3 `git push origin feature/data-regen-sprint5` (delegated to admin fast-forward on 250)
- [x] 5.4 PR #3347 created via Gitea API (combined SPEC-only batch)

## 6. Sprint 8 (out of scope for this PR)

- [x] 6.1 Hash-join for 8-way JOINs. **Foundation landed in
      `crates/executor/src/join/hash_join.rs` (PR #3346, SHA
      `e1fb0eba6c`)** + **Wired in `engine_select.rs` (PR
      `11ea816d5b`, this branch)** via `try_comma_join_hash_chain`.
      8 unit tests PASS for the hash-join helper. The wiring covers
      the Q3 (3-table) and Q21 (4-table) chains end-to-end.
- [ ] 6.2 CASE WHEN short-circuit. Pre-compute `n2.n_name =
      'GERMANY'` per row of the nation n2 subquery (or the
      materialized `__subq_N` if the parser encoded it), avoiding
      per-joined-row CASE re-evaluation. **Plus: 8-way cartesian
      path bug** — Q8's 8-way join (7 cross-table join keys + 1
      single-table filter on nation n2) currently returns 0 rows
      from the per-clause cartesian fallback even when the wiring
      returns None. Verified pre-existing on commit `11ea816d5b^`.
      Fix requires either (a) extending the chain build to include
      "single-table-filter" tables via the existing
      `pre_filter_cartesian_right_table` push-down, or (b) a
      separate "cartesian-of-filtered-sources" helper.
- [ ] 6.3 EXTRACT YEAR function support. Some Q8 paths use
      `EXTRACT(YEAR FROM o_orderdate)`; confirm the parser +
      `executor::expr::eval_fn` path returns the same year as PG for
      the test fixture's date range.
- [ ] 6.4 Q8 cell-level test (in-process eval at SF=0.1; compare
      cell-level vs PG).
- [ ] 6.5 Q8 perf bench (target < 5s at SF=0.1, < 60s at SF=1.0).

## 7. Gitea issue tracking

- [x] 7.1 Add a Gitea comment on the Q8-related issue: "Deferred to
      Sprint 8. Tracking: openspec/changes/2026-06-08-v390-sprint8-q8-exists/."
- [x] 7.2 Do NOT close the issue.
