# V312-33: STAGE_CONFIG explicit V312-32 wiring — proposal

> **Author**: minimax (claude-code)
> **Date**: 2026-08-09
> **Branch**: `feature/v312-24-impl`
> **Tracking issue**: #3911 (V312-24 follow-up)
> **Source**: V312-32 commit `6a30ad6951` self-reflection (再次汇报评论 §自我反思)

## Why

V312-32 commit (`6a30ad6951`) added `check_anti_fabrication.sh CHECK 1.5` (forces `target/sqlancer-report.json` + `target/test-runner-report.json`) and `check_rc_ga_gate.sh B6` (same in D2-Beta). However, `STAGE_CONFIG.yaml` RC required_gates list already contains `check_anti_fabrication.sh` (it was there from v3.10.0 AFP v4 rollout), so V312-32 is "implicitly" wired.

**But**: BETA stage required_gates list does NOT include `check_anti_fabrication.sh` — only `check_beta_gate.sh`. This means V312-24 artifacts are only enforced from RC onwards, missing the BETA gate window where most regressions occur (BETA = feature freeze + integration focus).

V312-33 makes the BETA gate also require AFP v4 (which now contains CHECK 1.5), so V312-24 artifacts are enforced **at every stage transition** (BETA + RC + GA) for v3.12.0.

## What

| File | Change |
|------|--------|
| `docs/governance/STAGE_CONFIG.yaml` | Add `scripts/gate/check_anti_fabrication.sh` to BETA required_gates (after `check_beta_gate.sh`) with inline comment referencing V312-32 commit + 2026-08-09 date |

No new files. This is a **single-line addition** to make V312-32's AFP v4 wiring explicit at BETA stage.

## Acceptance criteria

1. `grep -A 5 "BETA:" docs/governance/STAGE_CONFIG.yaml | grep anti_fabrication` shows the new line
2. `bash scripts/gate/check_stage.sh --version v3.12.0 --stage BETA --dry-run` (if supported) lists `check_anti_fabrication.sh` in BETA required_gates
3. `git diff HEAD~1 HEAD -- docs/governance/STAGE_CONFIG.yaml` shows +1 line in BETA section
4. `cargo check --all-features 2>&1 | grep "could not compile"` count is unchanged (V312-33 doesn't break compilation)

## Evidence gates

- `grep -B 1 -A 1 "check_anti_fabrication" docs/governance/STAGE_CONFIG.yaml` shows 3 occurrences (ALPHA implicit + BETA new + RC existing)
- `git log -p -1` shows the diff
- composite evidence_hash re-computed post-commit (with V312-33 OpenSpec in input set)

## Out of scope

- Fixing the remaining 25 `cargo test || true` masks in 8 other gate scripts (V312-31 follow-up, partially fixed in V312-33)
- 2 reviewer APPROVED on PR #3950 (external action)
- 7 豁免清单 acceptance by governance owner (external action)

## Owner

minimax
