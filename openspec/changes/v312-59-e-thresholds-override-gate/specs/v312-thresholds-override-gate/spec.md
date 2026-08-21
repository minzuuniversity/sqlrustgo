# Spec — v312-thresholds-override-gate

> **Capability**: Hard enforcement of `docs/releases/v3.12.0/STAGE.yaml` `thresholds_override` section as a single composite gate `B8_THRESHOLDS_OVERRIDE` in the v3.12.0 BETA gate pipeline.
> **Status**: Proposed (Issue #4388, V312-59-E)
> **Default state**: B8 absent from `check_beta_v3.12.0.sh` (current 40/40 gate count)

## ADDED Requirements

### Requirement: thresholds_override field validity check

The system SHALL provide a script `scripts/gate/check_v312_gate_thresholds.sh` that asserts every field under `thresholds_override` in `docs/releases/v3.12.0/STAGE.yaml` is present, non-empty, and has a value matching the field's expected type:
- `COVERAGE_MIN_PER_CRATE`: integer ≥ 0
- `GMP_CORPUS_UNCLASSIFIED_FAILURE_MAX`: integer ≥ 0
- `MIXED_SOAK_HOURS`: integer ≥ 0
- 10 boolean fields: literal `true` (case-insensitive)

#### Scenario: All 13 fields present and well-formed
- **WHEN** STAGE.yaml contains all 13 thresholds_override fields with valid values
- **THEN** Section 1 reports `13/13 FIELD_VALIDITY PASS`
- **AND** the script proceeds to Section 2

#### Scenario: Missing boolean field
- **WHEN** STAGE.yaml omits `MYSQL_WIRE_E2E_REQUIRED`
- **THEN** Section 1 reports `[MYSQL_WIRE_E2E_REQUIRED] FAIL: field missing`
- **AND** the script exits non-zero
- **AND** reports `12/13 FIELD_VALIDITY PASS, 1 FAIL`

#### Scenario: Boolean field set to false
- **WHEN** `GMP_AUDIT_TAMPER_TEST_REQUIRED: false`
- **THEN** Section 1 reports `[GMP_AUDIT_TAMPER_TEST_REQUIRED] FAIL: expected true, got false`
- **AND** Anti-pattern flag emitted: "boolean=true field flipped to false" (Anti-Fabrication-Policy-v1.0 §5)

### Requirement: thresholds_override executable gate checks

The system SHALL, for each boolean/numeric field where an executable gate exists, invoke that gate and verify PASS:

| Field | Required Executable Gate |
|---|---|
| COVERAGE_MIN_PER_CRATE | `scripts/gate/check_coverage_v312.sh` (per-crate ≥80%) |
| GMP_AUDIT_TAMPER_TEST_REQUIRED | `B6_AUDIT_HASH_CHAIN` |
| GMP_RETRIEVAL_CITATION_REQUIRED | `B6_HYBRID_RETRIEVAL` |
| MIXED_SOAK_HOURS | grep SOAK report for ≥168h marker |
| SQLLOGICTEST_SMOKE_REQUIRED | `B6_SQLLOGICTEST_SMOKE_GATE` |
| SQLLOGICTEST_SELECTED_TARGETS_REQUIRED | curated subset PASS or issue-link each SKIP |
| TPCH_SF1_CORRECTNESS_REQUIRED | `B6_TPCH_SF1_G4` 22/22 row-count assertion |
| MYSQL_WIRE_E2E_REQUIRED | `scripts/gate/check_mysql_wire_e2e.sh` |
| LOAD_DATA_BULK_IMPORT_REQUIRED | `scripts/gate/check_load_data_bulk.sh` |
| CRASH_RECOVERY_REQUIRED | `scripts/gate/check_crash_recovery.sh` |
| UPGRADE_DOWNGRADE_REQUIRED | `scripts/gate/check_upgrade_downgrade.sh` |
| BUSTUBX_EDU_SQLITE_CLI_REQUIRED | `scripts/gate/check_bustubx_edu_cli_v312.sh` |
| GMP_CORPUS_UNCLASSIFIED_FAILURE_MAX | grep unclassified_count ≤0 |

#### Scenario: All 13 executable gates PASS
- **WHEN** all 13 inner gates return exit code 0
- **THEN** Section 2 reports `13/13 EXECUTABLE_GATES PASS`
- **AND** the script exits 0
- **AND** outputs `13/13 PASS` summary

#### Scenario: Missing infrastructure for a sub-gate
- **WHEN** `MYSQL_WIRE_E2E` sub-script does not exist
- **THEN** Section 2 reports `[MYSQL_WIRE_E2E_REQUIRED] INFRASTRUCTURE_MISSING (issue #XXXX)`
- **AND** the failure is non-veto if linked to a tracked blocker issue with valid expiry

#### Scenario: TPCH SF=1 row-count mismatch
- **WHEN** `B6_TPCH_SF1_G4` reports 21/22 PASS (one query zero-row without explanation)
- **THEN** Section 2 reports `[TPCH_SF1_CORRECTNESS_REQUIRED] FAIL: 21/22 row-count, query 18 unexplained zero-row`
- **AND** the script exits non-zero
- **AND** Anti-pattern flag emitted per `promotion_to_GA_requires` §5

### Requirement: STAGE.yaml / gate script field-set sync

The system SHALL provide a script `scripts/gate/check_v312_stage_yaml_sync.sh` that verifies the set of fields declared in `thresholds_override` (STAGE.yaml) is a superset of the set of fields checked by `check_v312_gate_thresholds.sh`:

```
yq '.thresholds_override | keys' docs/releases/v3.12.0/STAGE.yaml
   → set A (13 fields)
grep '^\s*\[' scripts/gate/check_v312_gate_thresholds.sh
   → set B (declared field names)
diff A B → must be empty (FAIL if missing from gate)
diff B A → WARN if extra in gate
```

#### Scenario: STAGE.yaml and gate script agree
- **WHEN** `set A == set B`
- **THEN** the script reports `[STAGE_YAML_SYNC] PASS`
- **AND** exits 0

#### Scenario: STAGE.yaml field not checked by gate
- **WHEN** STAGE.yaml declares a 14th field not handled by the gate
- **THEN** the script reports `[STAGE_YAML_SYNC] FAIL: drift in 1 field (MIXED_SOAK_HOURS in YAML, missing in gate)`
- **AND** exits non-zero

#### Scenario: Gate checks field not declared in STAGE.yaml
- **WHEN** the gate script declares a 14th check not present in STAGE.yaml
- **THEN** the script reports `[STAGE_YAML_SYNC] WARN: gate has undeclared field (X)`
- **AND** exits 0 (WARN only, not FAIL)

### Requirement: BETA gate integration as B8_THRESHOLDS_OVERRIDE

The system SHALL integrate the new gates into `scripts/gate/check_beta_v3.12.0.sh` as section `B8_THRESHOLDS_OVERRIDE`, placed after the existing B1-B7 sections and before the final summary, matching the invocation style of existing sections.

#### Scenario: All B1-B8 gates PASS
- **WHEN** B1-B7 still PASS and B8 reports `13/13 PASS` + `STAGE_YAML_SYNC PASS`
- **THEN** `check_beta_v3.12.0.sh` reports `41/41 PASS, 0 BLOCKERS, 0 WARN`
- **AND** exits 0

#### Scenario: B8 fails
- **WHEN** B8 reports any FAIL
- **THEN** `check_beta_v3.12.0.sh` reports `40/41 PASS, 1 BLOCKER (B8)`
- **AND** exits non-zero

### Requirement: STAGE.yaml promotion gate references

The system SHALL update `docs/releases/v3.12.0/STAGE.yaml` to reference `B8_THRESHOLDS_OVERRIDE` as an explicit gate in both `promotion_to_RC_requires` and `promotion_to_GA_requires`:

```
promotion_to_RC_requires:
  - ...existing items...
  - "B8_THRESHOLDS_OVERRIDE: 13/13 boolean + executable gates PASS"

promotion_to_GA_requires:
  - ...existing items...
  - "B8_THRESHOLDS_OVERRIDE: 13/13 PASS at GA cut time"
```

#### Scenario: STAGE.yaml sync check still passes after reference added
- **WHEN** the `B8_THRESHOLDS_OVERRIDE` text references are added to STAGE.yaml
- **AND** `check_v312_stage_yaml_sync.sh` is re-run
- **THEN** the script reports PASS (the references are outside `thresholds_override`, so they don't trigger drift)