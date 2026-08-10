# V312-33: STAGE_CONFIG explicit V312-32 wiring — tasks

> **Status**: 🟡 OPEN (in worktree, 2 commits in flight)
> **Owner**: minimax

- [ ] 1.1 Add `scripts/gate/check_anti_fabrication.sh` to BETA required_gates after `check_beta_gate.sh` in `docs/governance/STAGE_CONFIG.yaml` (with inline comment: "2026-08-09 V312-32: AFP v4 CHECK 1.5 forces target/sqlancer-report.json + target/test-runner-report.json")
- [ ] 1.2 Verify `grep -A 5 "BETA:" docs/governance/STAGE_CONFIG.yaml` shows new line
- [ ] 1.3 Create `openspec/changes/v312-33-staging-gate-wiring/{proposal,tasks}.md`
- [ ] 1.4 Commit + push to gitea
- [ ] 1.5 Re-compute composite evidence_hash (with V312-33 OpenSpec in input set)
- [ ] 1.6 Post comment to PR #3950 + ISSUE #3911 with V312-33 details + 5 gate 实跑 + 25→22 remaining `|| true` count
