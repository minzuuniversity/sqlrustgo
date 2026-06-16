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

- [ ] 4.1 In `docs/audit/status/2026-06-07-tpch-cell-diff-v390-regen-sprint7.md`,
      mark Q16 row from "engine_issue" to "PASS (post-Sprint 6 regen)".
- [ ] 4.2 Update the cross-validation table: 17/22 → 18/22 (Q16 added).

## 5. Verify no regression

- [ ] 5.1 `cargo test --release --test _q18_check` — Q18 still 5 rows, top 970.51.
- [ ] 5.2 `cargo test --release --test four_way_cell_diff_test` — Q1-Q15,
      Q17, Q19 results unchanged.
- [ ] 5.3 Q15 sibling test still passes (91 rows).

## 6. Commit + Push PR (combined with Q15 sibling)

- [ ] 6.1 `git add tests/_q15_q16_check.rs docs/audit/status/2026-06-07-tpch-cell-diff-v390-regen-sprint7.md`
- [ ] 6.2 `git commit -m "fix(v3.9.0): Q15+Q16 subquery verification (no engine change post-Sprint 6 regen)"`
- [ ] 6.3 `GIT_SSH_COMMAND="ssh -i /home/openclaw/.ssh/id_ed25519 -o IdentitiesOnly=yes" git push origin feature/data-regen-sprint5`
- [ ] 6.4 `gh pr create --repo http://192.168.0.252:3000/openclaw/sqlrustgo --title "Q15 + Q16 subquery verification post-Sprint 6 regen" --body "..."`
- [ ] 6.5 Closes the Gitea issue (TBD) if a Q16-specific issue exists;
      otherwise updates the v3.9.0 #3291 sub-task tracker.

## 7. Audit + close

- [ ] 7.1 Update `docs/audit/status/2026-06-08-SPRINT5_V2_BUG_ANALYSIS.md`
      to mark Q16 in the "Q4 exists-fixed" column or remove it from the
      "6 issues" list.
- [ ] 7.2 Confirm #3291 GA-BLOCKER still has Q16 sub-task resolved.
