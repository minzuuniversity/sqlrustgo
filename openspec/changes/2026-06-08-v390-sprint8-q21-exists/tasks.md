## 1. Set up Sprint 8 SPEC worktree

- [x] 1.1 Use existing worktree from `develop/v3.9.0`.

## 2. Add SPEC scaffolding (no code change)

- [x] 2.1 `openspec/changes/2026-06-08-v390-sprint8-q21-exists/proposal.md`
- [x] 2.2 `openspec/changes/2026-06-08-v390-sprint8-q21-exists/design.md`
- [x] 2.3 `openspec/changes/2026-06-08-v390-sprint8-q21-exists/tasks.md`
- [x] 2.4 `openspec/changes/2026-06-08-v390-sprint8-q21-exists/specs/q21-suppliers-waiting-n2/spec.md`

## 3. Update v3.9.0 #3291 sub-task tracker

- [x] 3.1 Add Q21 to "Sprint 8 deferred" in
      `docs/audit/status/2026-06-08-SPRINT5_V2_BUG_ANALYSIS.md`.

## 4. Update Sprint 7 audit report

- [x] 4.1 Confirm Q21 marked "N² N/A (Sprint 8 deferred)" in
      `docs/audit/status/2026-06-07-tpch-cell-diff-v390-regen-sprint7.md`.

## 5. Commit + Push PR (SPEC-only)

- [x] 5.1 `git add openspec/changes/2026-06-08-v390-sprint8-q21-exists/`
- [x] 5.2 `git commit -m "spec(v3.9.0): defer Q21 4-way EXISTS to Sprint 8 hash-semi-join"`
- [x] 5.3 `git push origin feature/data-regen-sprint5` (delegated to admin fast-forward on 250)
- [x] 5.4 PR #3347 created via Gitea API (combined SPEC-only batch)

## 6. Sprint 8 (out of scope for this PR)

- [x] 6.1 Hash-semi-join for EXISTS. **Foundation landed in
      `crates/executor/src/join/hash_join.rs` (PR #3346, SHA
      `e1fb0eba6c`)**. The in-tree semi-join path is
      `build_subquery_index` at `src/engine_select.rs:2922-2982`,
      which builds a `HashSet<Value>` per (table, col) pair (an
      effective hash-join index of inner keys) and reuses it per
      outer row in O(1) average. 8 unit tests PASS for the general
      hash-join helper.
- [ ] 6.2 Hash-anti-semi-join for NOT EXISTS. Mirror of §6.1 with
      `NOT match` semantics. Used by Q21's `NOT EXISTS` arm. See
      `tests/q21_exists_hash_path_test.rs::not_exists_fast_path_filters_late_receipts`
      (currently `#[ignore]`d) for the failure-mode documentation;
      the SubqueryIndex needs to carry the qualifying rows so the
      residual predicate is checked per outer-row substitution.
- [ ] 6.3 Q21 cell-level regression test (in-process eval at SF=0.1;
      compare cell-level vs PG).
- [ ] 6.4 Q21 perf bench (target < 5s at SF=0.1, < 60s at SF=1.0).

## 7. Gitea issue tracking

- [x] 7.1 Add a Gitea comment on the Q21-related issue: "Deferred to
      Sprint 8. Tracking: openspec/changes/2026-06-08-v390-sprint8-q21-exists/."
- [x] 7.2 Do NOT close the issue.
