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

- [ ] 4.1 In `docs/audit/status/2026-06-07-tpch-cell-diff-v390-regen-sprint7.md`,
      mark Q15 row from "engine_issue" to "PASS (post-Sprint 6 regen)".
- [ ] 4.2 Update the cross-validation table: 16/22 → 17/22 (Q15 added)
      or 18/22 (if Q16 sibling is also landed).

## 5. Verify no regression

- [ ] 5.1 `cargo test --release --test _q18_check` — Q18 still 5 rows, top 970.51.
- [ ] 5.2 `cargo test --release --test four_way_cell_diff_test` — Q1-Q14
      results unchanged.
- [ ] 5.3 Q19 5-row match unchanged (sibling change Q19 fix already merged).

## 6. Commit + Push PR

- [ ] 6.1 `git add tests/_q15_q16_check.rs docs/audit/status/2026-06-07-tpch-cell-diff-v390-regen-sprint7.md`
- [ ] 6.2 `git commit -m "fix(v3.9.0): Q15 subquery-in-FROM verified post-Sprint 6 regen (no engine change)"`
- [ ] 6.3 `GIT_SSH_COMMAND="ssh -i /home/openclaw/.ssh/id_ed25519 -o IdentitiesOnly=yes" git push origin feature/data-regen-sprint5`
- [ ] 6.4 `gh pr create --repo http://192.168.0.252:3000/openclaw/sqlrustgo --title "Q15 subquery-in-FROM verification (no engine change)" --body "..."`
- [ ] 6.5 Closes the Gitea issue (TBD) if a Q15-specific issue exists;
      otherwise updates the v3.9.0 #3291 sub-task tracker.

## 7. Audit + close

- [ ] 7.1 Update `docs/audit/status/2026-06-08-SPRINT5_V2_BUG_ANALYSIS.md`
      to mark Q15 in the "Q4 exists-fixed" column or remove it from the
      "6 issues" list.
- [ ] 7.2 Confirm #3291 GA-BLOCKER still has Q15 sub-task resolved.
