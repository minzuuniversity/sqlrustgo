# Spec — v312-promotion-to-rc

> **Capability**: Hard enforcement of all 11 `promotion_to_RC_requires` items in `docs/releases/v3.12.0/STAGE.yaml` (lines 94-105) as a composite gate, so that BETA→RC promotion cannot occur without each item having documented evidence + a passing integration test (or explicit issue-link deferral).
> **Status**: Proposed (Issue #4386, V312-59-C)
> **Default state**: gate script absent; manual verification per item.

## ADDED Requirements

### Requirement: composite RC gate

The system SHALL provide `scripts/gate/check_v312_promotion_to_rc.sh` that asserts each of the 11 `promotion_to_RC_requires` items has either a passing integration test OR a documented evidence file (with commit hash + exit code + evidence_hash).

#### Scenario: 11/11 RC items have evidence or passing test
- **WHEN** every RC-1..RC-11 item has a PASS verdict
- **THEN** the script reports `11/11 RC_ITEMS PASS`
- **AND** exits 0

#### Scenario: missing evidence for any RC item
- **WHEN** `RC<N>_*_REPORT.md` is absent OR its referenced integration test fails
- **THEN** the script reports `[RC<N>] FAIL: <reason>`
- **AND** exits non-zero

### Requirement: RC-1 GMP-MD ingestion report

The system SHALL produce `docs/releases/v3.12.0/evidence/v312-59/RC1_GMP_MD_INGESTION_REPORT.md` referencing the existing `v312-03-gmp-ingestion-report.md` with: total docs/chunks/embeddings, idempotent re-run diff=0, per-step timing, memory peak, evidence_hash.

#### Scenario: existing V312-03 report sufficient
- **WHEN** `v312-03-gmp-ingestion-report.md` exists with 154/154 tests PASS and idempotency verified
- **THEN** the RC-1 wrapper can reference it without re-running the ingest

### Requirement: RC-2 retrieval quality report

The system SHALL produce `docs/releases/v3.12.0/evidence/v312-59/RC2_RETRIEVAL_QUALITY_REPORT.md` referencing the existing `v312-05-hybrid-retrieval-report.md`.

#### Scenario: existing V312-05 report sufficient
- **WHEN** `v312-05-hybrid-retrieval-report.md` exists with hybrid retrieval (vector + keyword) returning source path/version/chunk hash
- **THEN** the RC-2 wrapper can reference it

### Requirement: RC-3 backup/restore verification

The system SHALL produce `docs/releases/v3.12.0/evidence/v312-59/RC3_BACKUP_RESTORE_REPORT.md` referencing the existing `v312-09-backup-restore-report.md`.

#### Scenario: existing V312-09 report sufficient
- **WHEN** `v312-09-backup-restore-report.md` exists with backup SHA256 + restore row-count parity
- **THEN** the RC-3 wrapper can reference it

### Requirement: RC-4 security / RBAC report

The system SHALL produce `docs/releases/v3.12.0/evidence/v312-59/RC4_SECURITY_RBAC_REPORT.md` referencing `evidence/gmp_compliance/V312-53-REPORT.md` (12 ACL tests PASS, 5×11=55-cell matrix).

#### Scenario: V312-53 ACL coverage sufficient
- **WHEN** `test_acl_full_matrix_5_roles_x_11_operations` PASS (55 cells: 28 allowed + 27 denied)
- **AND** `test_permission_guard_fail_closed` PASS
- **THEN** the RC-4 wrapper can reference V312-53 evidence

### Requirement: RC-5 curated SQLite SQLLogicTest

The system SHALL produce `docs/releases/v3.12.0/evidence/v312-59/RC5_CURATED_SQLLOGICTEST_REPORT.md` referencing `sqllogictest-baseline/sqlite-corpus-manifest.json` (21 files, 6 PASS / 16 FAIL with v313-08..v313-15 exclusions).

#### Scenario: curated manifest sufficient
- **WHEN** `sqllogictest-baseline/sqlite-corpus-manifest.json` exists with per-target status
- **AND** `exclusions.yml` has every SKIP linked to a v313-* issue
- **THEN** the RC-5 wrapper can reference the manifest

### Requirement: RC-7 wire protocol + LOAD DATA report

The system SHALL produce `docs/releases/v3.12.0/evidence/v312-59/RC7_WIRE_LOAD_DATA_REPORT.md` referencing `evidence/wire_load_data/V312-13-REPORT.md` (10 steps, 9 pass, 1 SF=10 fixture-deferred).

#### Scenario: V312-13 report sufficient
- **WHEN** `evidence/wire_load_data/V312-13-REPORT.md` has all 10 steps recorded with status
- **THEN** the RC-7 wrapper can reference it

### Requirement: RC-8 crash recovery + upgrade/downgrade reports

The system SHALL produce `docs/releases/v3.12.0/evidence/v312-59/RC8_CRASH_UPGRADE_REPORT.md` referencing `crash-recovery-upgrade-verification-report.md`.

#### Scenario: crash+upgrade report sufficient
- **WHEN** `crash-recovery-upgrade-verification-report.md` exists with 7 crash + 4 upgrade + 4 downgrade scenarios
- **AND** each scenario reports recovered row-count = original
- **THEN** the RC-8 wrapper can reference it

### Requirement: RC-10 V312-57 week05-06 fixtures

The system SHALL provide 6 new fixtures (3 executor + 3 join/aggregate) under `tests/compat/bustubx_edu_sqlite_cli/week05/` and `week06/`, each registered in `manifest.yml`.

#### Scenario: week05-06 fixtures created
- **WHEN** 6 fixtures exist (3 in week05 executor + 3 in week06 join/aggregate)
- **AND** `manifest.yml` lists all 6 with `case_id`, `week`, `sql_file`, `expected_exit_code`, `oracle_mode`
- **AND** `bash scripts/gate/check_bustubx_edu_cli_v312.sh` reports all 6 PASS
- **THEN** RC-10 is satisfied

#### Scenario: missing fixtures
- **WHEN** week05/ or week06/ directories are empty or absent
- **THEN** RC-10 reports FAIL with "missing fixtures" reason

### Requirement: RC-11 claim cleanup

The system SHALL produce `docs/releases/v3.12.0/evidence/v312-59/RC11_CLAIM_CLEANUP_REPORT.md` referencing `evidence/r11_claim_audit/CLAIM_AUDIT_2026-08-19.md` (already audited).

#### Scenario: existing claim audit sufficient
- **WHEN** `r11_claim_audit/CLAIM_AUDIT_2026-08-19.md` exists with PASS verdict
- **THEN** the RC-11 wrapper can reference it

### Requirement: integration into BETA→RC boundary

The system SHALL invoke `scripts/gate/check_v312_promotion_to_rc.sh` from `scripts/gate/check_v312_stage_boundary.sh` so RC boundary verification is automatic.

#### Scenario: RC promotion blocked
- **WHEN** any RC-1..RC-11 item has FAIL verdict
- **THEN** `check_v312_stage_boundary.sh` blocks BETA→RC transition
- **AND** outputs `[V312-59-C RC_BLOCKER] RC-<N>: <reason>`