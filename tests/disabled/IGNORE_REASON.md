# tests/disabled/ — Disabled Test Files with Reasons

> **Status (2026-07-13)**: Empty directory (no test files currently in `tests/disabled/`).
> **Per**: TEST_PLAN.md §4.3 (`tests/disabled/` 目录规则)
> **DeepSeek review**: local://attachment-1 §阶段 2

## Purpose

This directory hosts test files that are **NOT compileable in the current
build** because their underlying feature has been **deferred, abandoned, or
superseded**. Each file in this directory must have an accompanying entry
in the `Reason` table below explaining:

1. **Origin** — when and why the test was added
2. **Reason for disable** — what makes it uncompileable or invalid
3. **Re-enable criteria** — what needs to happen for the test to return to `tests/`
4. **Issue / Tracking** — linked issue or commit

## 规则 (Per TEST_PLAN §4.3)

- Tests in `tests/disabled/` are **skipped by `check_test_inventory.sh`**.
- Each disabled test file MUST be paired with an entry below.
- No test file may be added to `tests/disabled/` without updating this README.
- `git log -- tests/disabled/<file>` shows the commit that originally moved the
  test out of `tests/`.

## 迁移规则

To move a test to `tests/disabled/`:

1. `git mv tests/X.rs tests/disabled/X.rs`
2. Add an entry to the Reason table below with the 4 fields
3. Verify `cargo build --tests` no longer fails on the test
4. Verify `check_test_inventory.sh` correctly excludes the test
5. Commit with a `tests: disable X (reason: ...)` message

## 移除规则 (re-enable)

To re-enable a test from `tests/disabled/`:

1. `git mv tests/disabled/X.rs tests/<subdir>/X.rs` (use the new dir structure)
2. Update the Reason table → mark re-enabled (with date and PR)
3. Run the test in isolation to verify it passes:
   `cargo test --test <name> --all-features`
4. Remove the entry from this README's Reason table (move to history)
5. Commit with a `tests: re-enable X (issue: ...)` message

## Reason Table

Currently empty. As tests are disabled (typically during the `#[ignore]`
debt closure in v3.10.0 BETA→RC phase), they will be listed here:

| File | Origin | Reason | Re-enable criteria | Tracking |
|------|--------|--------|-------------------|----------|
| (none yet) | | | | |

## Historical Relocations (informational only — not currently disabled)

The following test files were moved during the v3.10.0 test directory
restructure (commit 3e4cd4accc, Phase 2). They were NOT disabled — they
were simply reclassified:

| Old path | New path | Reason for move |
|----------|----------|------------------|
| (224 files) | tests/integration/{sql,dml,ddl,transaction,wire,tpch,migration,oracle,optimizer,stress}/ | Phase 2 subdir classification |

The original list of 224 files is preserved in `git log 3e4cd4accc`
under the rename detection.

## Implementation Reference

- `git log --diff-filter=R --name-only 3e4cd4accc | grep "^tests/"`
  → All 224 renames from Phase 2
- `cargo test --tests --all-features 2>&1 | grep "could not compile"`
  → Pre-existing compile errors (parking_lot migration, F-XX obsolete tests)
- `scripts/gate/check_test_inventory.sh`
  → Validates the active test count matches what STAGE_CONFIG expects

## Related Documentation

- `docs/releases/v3.10.0/TEST_PLAN.md` §4.1-4.3 (49 #[ignore] debt plan)
- `docs/releases/v3.10.0/V310_TEST_BINARY_GATE_AUDIT_REPORT.md` (audit context)
- `docs/governance/debt/debt-registry.yaml` (debt items, sync with this list)

---

*Last updated: 2026-07-13 by Claude Code (hermes-agent) per DeepSeek review (local://attachment-1) §阶段 2*
