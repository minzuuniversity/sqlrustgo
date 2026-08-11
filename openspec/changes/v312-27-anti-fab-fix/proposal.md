# V312-27 Anti-Fab Fix — Retroactive Reconstruction

**Status**: RECONSTRUCTED 2026-08-10
**Source evidence**:
- git commit `2b9a0d8d55` (V312-27 anti-fab fixes)
- git commit `2a5d806840` (V312-27 closure report)
- `docs/releases/v3.12.0/V312-27_anti_fab_fix_report.md`

This file was created retroactively to satisfy V312-24 governance
(per `openspec/README.md` requirement that every V312-NN change
have proposal/tasks/design). No new behavior is introduced.

## Motivation

37 stale + 3 union + 1 phantom `#[ignore]` entries drifted from
`tests/baseline/ignore_registry.json`. Five `if is_e2e_disabled()`
early-return blocks in `tests/e2e/e2e_beta_test.rs` masked test
execution behind a runtime check.

## Scope

- Registry reconciliation in `tests/baseline/ignore_registry.json`
- Removal of 5 e2e_beta_test.rs early-return blocks
- Tightening of two assertions
- Removal of one `#[ignore]` (replaced with active test)

## Non-Goals

- Fixing underlying failing tests
- Lowering `total_allowed` further (preserved at 73)
- New `#[ignore]` markers
