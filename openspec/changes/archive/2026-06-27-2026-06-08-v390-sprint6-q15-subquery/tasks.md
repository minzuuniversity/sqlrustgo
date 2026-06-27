## 1. Set up Sprint 6 verification worktree

- [x] 1.1 Use existing worktree `.worktrees/data-regen-sprint5` from
      `develop/v3.9.0` (already at HEAD `88882c8cb` after PR #3308 merge).

## 2. Add isolated verification test (RED→GREEN already in code)

- [x] 2.1 Add `tests/_q15_q16_check.rs` with `test_q15_subquery_from`
      and `test_q16_not_in_subquery` (both pass post-Sprint 6 regen).
- [x] 2.2 Register in `Cargo.toml` (auto-discovered by Cargo 2021 edition).
- [x] 2.3 Run `cargo test --release --test _q15_q16_check -- --nocapture`:
      both tests pass (GREEN — no engine change needed).

## 3. Cross-validate with PostgreSQL

- [x] 3.1 `psql -U liying -d tpch_test -c "<q15.sql>"` → 91 rows.
- [x] 3.2 sqlrustgo `_q15_q16_check` → 91 rows, byte-match (modulo float
      last-decimal-place cosmetic).
- [x] 3.3 Row count and supplier IDs match between sqlrustgo and PG.

## 4. Update Sprint 7 audit report

- [x] 4.1 In `docs/audit/status/2026-06-07-tpch-cell-diff-v390-regen-sprint7.md`,
      mark Q15 row from "engine_issue" to "PASS (post-Sprint 6 regen)".
- [x] 4.2 Update the cross-validation table: 16/22 → 17/22 (Q15 added)
      or 18/22 (if Q16 sibling is also landed).

## 5. Verify no regression

- [x] 5.1 `cargo test --release --test _q18_check` — Q18 still 5 rows, top 970.51.
- [x] 5.2 `cargo test --release --test four_way_cell_diff_test` — Q1-Q14
      results unchanged.
- [x] 5.3 Q19 5-row match unchanged (sibling change Q19 fix already merged).

## 6. Commit + Push PR

- [x] 6.1 `git add tests/_q15_q16_check.rs docs/audit/status/2026-06-07-tpch-cell-diff-v390-regen-sprint7.md`
- [x] 6.2 `git commit -m "fix(v3.9.0): Q15 subquery-in-FROM verified post-Sprint 6 regen (no engine change)"`
- [x] 6.3 `git push origin feature/data-regen-sprint5` (delegated to admin fast-forward on 250)
- [x] 6.4 PR #3346 created via Gitea API
- [x] 6.5 #3291 sub-task tracker updated via PR description

## 7. Audit + close

- [x] 7.1 Update `docs/audit/status/2026-06-08-SPRINT5_V2_BUG_ANALYSIS.md`
      to mark Q15 in the "Q4 exists-fixed" column or remove it from the
      "6 issues" list.
- [x] 7.2 Confirm #3291 GA-BLOCKER still has Q15 sub-task resolved.
