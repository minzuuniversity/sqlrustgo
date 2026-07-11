## 1. Set up Sprint 6 verification worktree

- [x] 1.1 Use existing worktree `.worktrees/data-regen-sprint5` from
      `develop/v3.9.0` (already at HEAD `88882c8cb` after PR #3308 merge).

## 2. Add isolated verification test (RED→GREEN already in code)

- [x] 2.1 `tests/_q15_q16_check.rs::test_q16_not_in_subquery` (added in
      same commit as Q15 sibling test).
- [x] 2.2 Run `cargo test --release --test _q15_q16_check -- --nocapture`:
      test passes (GREEN — no engine change needed).

## 3. Cross-validate with PostgreSQL

- [x] 3.1 `psql -U liying -d tpch_test -c "<q16.sql>"` → 284 rows.
- [x] 3.2 sqlrustgo `_q15_q16_check` → 284 rows, byte-match on all 4 columns.
- [x] 3.3 Row count and column values match between sqlrustgo and PG.

## 4. Update Sprint 7 audit report

- [x] 4.1 In `docs/audit/status/2026-06-07-tpch-cell-diff-v390-regen-sprint7.md`,
      mark Q16 row from "engine_issue" to "PASS (post-Sprint 6 regen)".
- [x] 4.2 Update the cross-validation table: 17/22 → 18/22 (Q16 added).

## 5. Verify no regression

- [x] 5.1 `cargo test --release --test _q18_check` — Q18 still 5 rows, top 970.51.
- [x] 5.2 `cargo test --release --test four_way_cell_diff_test` — Q1-Q15,
      Q17, Q19 results unchanged.
- [x] 5.3 Q15 sibling test still passes (91 rows).

## 6. Commit + Push PR (combined with Q15 sibling)

- [x] 6.1 `git add tests/_q15_q16_check.rs docs/audit/status/2026-06-07-tpch-cell-diff-v390-regen-sprint7.md`
- [x] 6.2 `git commit -m "fix(v3.9.0): Q15+Q16 subquery verification (no engine change post-Sprint 6 regen)"`
- [x] 6.3 `git push origin feature/data-regen-sprint5` (delegated to admin fast-forward on 250)
- [x] 6.4 PR #3346 created via Gitea API (combined Q15+Q16)
- [x] 6.5 #3291 sub-task tracker updated via PR description

## 7. Audit + close

- [x] 7.1 Update `docs/audit/status/2026-06-08-SPRINT5_V2_BUG_ANALYSIS.md`
      to mark Q16 in the "Q4 exists-fixed" column or remove it from the
      "6 issues" list.
- [x] 7.2 Confirm #3291 GA-BLOCKER still has Q16 sub-task resolved.
