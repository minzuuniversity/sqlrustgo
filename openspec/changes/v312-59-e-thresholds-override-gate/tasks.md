# Tasks — V312-59-E Thresholds Override Gate

## 1. Pre-work

- [ ] 1.1 Read `docs/releases/v3.12.0/STAGE.yaml` lines 118-132 (thresholds_override section)
- [ ] 1.2 Inventory existing gate scripts in `scripts/gate/` that map to boolean fields:
  - [ ] 1.2.1 Confirm `check_coverage_v312.sh` exists and is wired to `B6_COVERAGE`
  - [ ] 1.2.2 Confirm `B6_SQLLOGICTEST_SMOKE_GATE` exists
  - [ ] 1.2.3 Confirm `B6_AUDIT_HASH_CHAIN` / `B6_HYBRID_RETRIEVAL` exist
  - [ ] 1.2.4 Confirm `B6_TPCH_SF1_G4` exists with row-count output
  - [ ] 1.2.5 Check whether `check_mysql_wire_e2e.sh` exists (likely NOT — new)
  - [ ] 1.2.6 Check whether `check_load_data_bulk.sh` exists
  - [ ] 1.2.7 Check whether `check_crash_recovery.sh` exists (likely NOT — new)
  - [ ] 1.2.8 Check whether `check_upgrade_downgrade.sh` exists (likely NOT — new)
  - [ ] 1.2.9 Confirm `check_bustubx_edu_cli_v312.sh` exists
- [ ] 1.3 Identify SOAK evidence path for `MIXED_SOAK_HOURS=168` check
- [ ] 1.4 Identify curated SQLLogicTest target list path for `SQLLOGICTEST_SELECTED_TARGETS_REQUIRED`

## 2. Implement `check_v312_gate_thresholds.sh`

- [ ] 2.1 Create file `scripts/gate/check_v312_gate_thresholds.sh` with `set -euo pipefail` header
- [ ] 2.2 Add 13 field constants (matching STAGE.yaml key names exactly):
  ```bash
  declare -A FIELDS=(
    [COVERAGE_MIN_PER_CRATE]=80
    [GMP_CORPUS_UNCLASSIFIED_FAILURE_MAX]=0
    [GMP_AUDIT_TAMPER_TEST_REQUIRED]=true
    [GMP_RETRIEVAL_CITATION_REQUIRED]=true
    [MIXED_SOAK_HOURS]=168
    [SQLLOGICTEST_SMOKE_REQUIRED]=true
    [SQLLOGICTEST_SELECTED_TARGETS_REQUIRED]=true
    [TPCH_SF1_CORRECTNESS_REQUIRED]=true
    [MYSQL_WIRE_E2E_REQUIRED]=true
    [LOAD_DATA_BULK_IMPORT_REQUIRED]=true
    [CRASH_RECOVERY_REQUIRED]=true
    [UPGRADE_DOWNGRADE_REQUIRED]=true
    [BUSTUBX_EDU_SQLITE_CLI_REQUIRED]=true
  )
  ```
- [ ] 2.3 Implement `_parse_stage_yaml_field()` helper using yq (with python3 fallback)
- [ ] 2.4 Implement `_check_field_present_and_valid()` for each boolean/numeric field
- [ ] 2.5 Implement Section 1: 13 field-validity checks (output `[1/13 ...] PASS|FAIL`)
- [ ] 2.6 Implement Section 2: 8 executable gate invocations
  - [ ] 2.6.1 COVERAGE — call `check_coverage_v312.sh` and assert per-crate ≥80%
  - [ ] 2.6.2 GMP_AUDIT_TAMPER — call `B6_AUDIT_HASH_CHAIN`
  - [ ] 2.6.3 GMP_RETRIEVAL_CITATION — call `B6_HYBRID_RETRIEVAL`
  - [ ] 2.6.4 MIXED_SOAK_HOURS — parse SOAK report for ≥168h marker
  - [ ] 2.6.5 SQLLOGICTEST_SMOKE — call `B6_SQLLOGICTEST_SMOKE_GATE`
  - [ ] 2.6.6 SQLLOGICTEST_SELECTED — assert curated subset PASS or issue-link each SKIP
  - [ ] 2.6.7 TPCH_SF1_CORRECTNESS — assert 22/22 row-count = SQLite oracle (no unexplained zero-row)
  - [ ] 2.6.8 MYSQL_WIRE_E2E — call gate (or report INFRASTRUCTURE_MISSING with issue link)
  - [ ] 2.6.9 LOAD_DATA_BULK_IMPORT — assert LOAD DATA SF=1 row_count
  - [ ] 2.6.10 CRASH_RECOVERY — call gate (or INFRASTRUCTURE_MISSING)
  - [ ] 2.6.11 UPGRADE_DOWNGRADE — call gate (or INFRASTRUCTURE_MISSING)
  - [ ] 2.6.12 BUSTUBX_EDU_SQLITE_CLI — call `check_bustubx_edu_cli_v312.sh`
  - [ ] 2.6.13 GMP_CORPUS_UNCLASSIFIED — grep unclassified_count ≤0
- [ ] 2.7 Final summary: `13/13 PASS` or `12/13 PASS, 1 FAIL (MYSQL_WIRE_E2E: INFRASTRUCTURE_MISSING → issue #XXXX)`
- [ ] 2.8 Exit 0 iff all 13 PASS; exit 1 otherwise
- [ ] 2.9 Make executable: `chmod +x scripts/gate/check_v312_gate_thresholds.sh`

## 3. Implement `check_v312_stage_yaml_sync.sh`

- [ ] 3.1 Create file with `set -euo pipefail` header
- [ ] 3.2 yq query: extract keys under `thresholds_override` from STAGE.yaml → set A
- [ ] 3.3 grep query: extract field names declared in `check_v312_gate_thresholds.sh` (the `FIELDS` array) → set B
- [ ] 3.4 diff: `comm -23 <(sort -u A) <(sort -u B)` — fields in YAML but missing in gate → FAIL
- [ ] 3.5 diff: `comm -13 <(sort -u A) <(sort -u B)` — fields in gate but missing in YAML → WARN
- [ ] 3.6 Output `[STAGE_YAML_SYNC] PASS|FAIL` and exit accordingly
- [ ] 3.7 Make executable

## 4. Wire into BETA gate

- [ ] 4.1 Read current `scripts/gate/check_beta_v3.12.0.sh` structure
- [ ] 4.2 Add new section `B8_THRESHOLDS_OVERRIDE` after `B7_*` (matching style)
- [ ] 4.3 In B8: invoke `bash scripts/gate/check_v312_gate_thresholds.sh`
- [ ] 4.4 In B8: invoke `bash scripts/gate/check_v312_stage_yaml_sync.sh`
- [ ] 4.5 Update overall gate count from 40 → 41 in summary line

## 5. Update STAGE.yaml

- [ ] 5.1 Append to `promotion_to_RC_requires`:
  `- "B8_THRESHOLDS_OVERRIDE: 13/13 boolean + executable gates PASS"`
- [ ] 5.2 Append to `promotion_to_GA_requires`:
  `- "B8_THRESHOLDS_OVERRIDE: 13/13 PASS at GA cut time"`
- [ ] 5.3 Verify `check_v312_stage_yaml_sync.sh` still passes (no drift)

## 6. Generate evidence

- [ ] 6.1 Create `docs/releases/v3.12.0/evidence/v312-59-e/` directory
- [ ] 6.2 Run `check_v312_gate_thresholds.sh` and capture full output
- [ ] 6.3 Run `check_v312_stage_yaml_sync.sh` and capture full output
- [ ] 6.4 Run `check_beta_v3.12.0.sh` and confirm 41/41 (or document failures)
- [ ] 6.5 Write `EVIDENCE.md` with 13-field verdict table

## 7. Validation

- [ ] 7.1 `bash scripts/gate/check_v312_gate_thresholds.sh` exits 0
- [ ] 7.2 `bash scripts/gate/check_v312_stage_yaml_sync.sh` exits 0
- [ ] 7.3 `bash scripts/gate/check_beta_v3.12.0.sh` reports B8 PASS
- [ ] 7.4 No regression: other 40 gates still PASS
- [ ] 7.5 openspec validate `v312-59-e-thresholds-override-gate` exits 0

## 8. PR & close

- [ ] 8.1 Branch: `fix/v312-59-e-thresholds-override-gate` from `develop/v3.12.0`
- [ ] 8.2 Commit with detailed message referencing issue #4388
- [ ] 8.3 Push to origin
- [ ] 8.4 Create PR #4395+ to `develop/v3.12.0`
- [ ] 8.5 Reference issue #4388 in PR body
- [ ] 8.6 After merge: comment on #4388 with PR link + verification output
- [ ] 8.7 Close #4388