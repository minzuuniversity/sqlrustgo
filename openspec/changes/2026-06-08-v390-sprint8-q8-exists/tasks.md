## 1. Set up Sprint 8 SPEC worktree

- [x] 1.1 Use existing worktree from `develop/v3.9.0`.

## 2. Add SPEC scaffolding (no code change)

- [x] 2.1 `openspec/changes/2026-06-08-v390-sprint8-q8-exists/proposal.md`
- [x] 2.2 `openspec/changes/2026-06-08-v390-sprint8-q8-exists/design.md`
- [x] 2.3 `openspec/changes/2026-06-08-v390-sprint8-q8-exists/tasks.md`
- [x] 2.4 `openspec/changes/2026-06-08-v390-sprint8-q8-exists/specs/q8-national-market-share-n2/spec.md`

## 3. Update v3.9.0 #3291 sub-task tracker

- [ ] 3.1 Add Q8 to "Sprint 8 deferred" in
      `docs/audit/status/2026-06-08-SPRINT5_V2_BUG_ANALYSIS.md`.

## 4. Update Sprint 7 audit report

- [ ] 4.1 Confirm Q8 marked "N² N/A (Sprint 8 deferred)" in
      `docs/audit/status/2026-06-07-tpch-cell-diff-v390-regen-sprint7.md`.

## 5. Commit + Push PR (SPEC-only)

- [ ] 5.1 `git add openspec/changes/2026-06-08-v390-sprint8-q8-exists/`
- [ ] 5.2 `git commit -m "spec(v3.9.0): defer Q8 8-way JOIN to Sprint 8 hash-join + optimizer"`
- [ ] 5.3 `git push origin feature/data-regen-sprint5`
- [ ] 5.4 `gh pr create --repo http://192.168.0.252:3000/openclaw/sqlrustgo --title "Q8 N² deferred to Sprint 8 (SPEC only)"`

## 6. Sprint 8 (out of scope for this PR)

- [ ] 6.1 Hash-join for 8-way JOINs.
- [ ] 6.2 CASE WHEN short-circuit.
- [ ] 6.3 EXTRACT YEAR function support.
- [ ] 6.4 Q8 cell-level test.
- [ ] 6.5 Q8 perf bench.

## 7. Gitea issue tracking

- [ ] 7.1 Add a Gitea comment on the Q8-related issue: "Deferred to
      Sprint 8. Tracking: openspec/changes/2026-06-08-v390-sprint8-q8-exists/."
- [ ] 7.2 Do NOT close the issue.
