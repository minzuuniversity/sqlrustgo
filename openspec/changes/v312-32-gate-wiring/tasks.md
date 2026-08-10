# V312-32: V312-24 Gate Wiring — tasks

> **Status**: 🟡 OPEN (in worktree, awaiting commit + push)
> **Owner**: minimax

- [ ] 1.1 Add `check_v312_24_test_infra_artifacts()` function to `scripts/gate/check_anti_fabrication.sh` (after `check_canonical_binary_build`)
- [ ] 1.2 Add call to `check_v312_24_test_infra_artifacts` in main() after `check_canonical_binary_build`
- [ ] 1.3 Add B6 V312-24 check in `scripts/gate/check_rc_ga_gate.sh` D2-Beta section (after B5)
- [ ] 1.4 Verify `bash scripts/gate/check_anti_fabrication.sh` shows CHECK 1.5 with 3 PASS
- [ ] 1.5 Verify `bash -n scripts/gate/check_anti_fabrication.sh scripts/gate/check_rc_ga_gate.sh` (syntax)
- [ ] 1.6 Write `docs/releases/v3.12.0/V312-30_stage_transition_report.md` (5 gate 实跑 + 7 豁免清单 + sha256)
- [ ] 1.7 Re-compute composite evidence_hash (V312-32 input set includes v312-32-gate-wiring/{proposal,tasks}.md + V312-30_stage_transition_report.md)
- [ ] 1.8 Commit + push to gitea (creates new commits on `develop/v3.12.0`)
- [ ] 1.9 Post PR comment to PR #3950 with V312-32 details + 5 gate 实跑 log sha256
- [ ] 1.10 Re-close ISSUE #3911 with V312-32 evidence_hash (if 2 reviewer APPROVED + 7 豁免 accepted)
