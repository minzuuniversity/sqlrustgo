## 1. Set up Sprint 8 SPEC worktree

- [x] 1.1 Use existing worktree `.worktrees/data-regen-sprint5` from
      `develop/v3.9.0` (already at HEAD `88882c8cb` after PR #3308 merge).

## 2. Add SPEC scaffolding (no code change)

- [x] 2.1 `openspec/changes/2026-06-08-v390-sprint8-q3-exists/proposal.md`
- [x] 2.2 `openspec/changes/2026-06-08-v390-sprint8-q3-exists/design.md`
- [x] 2.3 `openspec/changes/2026-06-08-v390-sprint8-q3-exists/tasks.md`
- [x] 2.4 `openspec/changes/2026-06-08-v390-sprint8-q3-exists/specs/q3-shipping-priority-n2/spec.md`

## 3. Update v3.9.0 #3291 sub-task tracker

- [ ] 3.1 Add Q3 to the "Sprint 8 deferred" sub-task list in
      `docs/audit/status/2026-06-08-SPRINT5_V2_BUG_ANALYSIS.md`.
- [ ] 3.2 Add Q3 to `openspec/changes/2026-06-07-v390-sprint6-q4-exists-overcount/tasks.md`
      as a "deferred to Sprint 8" sub-task (cross-link).

## 4. Update Sprint 7 audit report

- [ ] 4.1 In `docs/audit/status/2026-06-07-tpch-cell-diff-v390-regen-sprint7.md`,
      confirm Q3 is marked "N² N/A (Sprint 8 deferred)" and
      cross-link to the new SPEC.

## 5. Commit + Push PR (SPEC-only, no code change)

- [ ] 5.1 `git add openspec/changes/2026-06-08-v390-sprint8-q3-exists/`
- [ ] 5.2 `git commit -m "spec(v3.9.0): defer Q3 N² JOIN to Sprint 8 hash-join implementation"`
- [ ] 5.3 `GIT_SSH_COMMAND="ssh -i /home/openclaw/.ssh/id_ed25519 -o IdentitiesOnly=yes" git push origin feature/data-regen-sprint5`
- [ ] 5.4 `gh pr create --repo http://192.168.0.252:3000/openclaw/sqlrustgo --title "Q3 N² deferred to Sprint 8 (SPEC only)" --body "..."`

## 6. Sprint 8 (out of scope for this PR)

- [ ] 6.1 Implement `crates/executor/src/join/hash_join.rs`.
- [ ] 6.2 Wire hash-join into `src/engine_select.rs` for 3+ way JOINs.
- [ ] 6.3 Add Q3 hash-join regression test.
- [ ] 6.4 Add Q3 perf bench.
- [ ] 6.5 Verify Q3 < 1s at SF=0.1, < 30s at SF=1.0.

## 7. Close Q3 Gitea issue (deferred to Sprint 8)

- [ ] 7.1 Add a Gitea comment on the Q3-related issue: "Deferred to
      Sprint 8 (v3.9.1 or later). Hash-join implementation requires
      5-15h engineering effort. Tracking: see
      openspec/changes/2026-06-08-v390-sprint8-q3-exists/."
- [ ] 7.2 Do NOT close the issue (per AGENTS.md ISSUE_CLOSING_VERIFICATION.md:
      close only after PR merge).
