# Tasks — V312-59-C promotion_to_RC gate

## 1. Pre-work

- [x] 1.1 Read issue #4386 + STAGE.yaml `promotion_to_RC_requires` (lines 94-105)
- [x] 1.2 Inventory existing evidence per RC item
- [x] 1.3 Verify OpenSpec change validation (`openspec validate`)

## 2. Implement `check_v312_promotion_to_rc.sh`

- [ ] 2.1 Create file with `set -euo pipefail` header
- [ ] 2.2 11-item associative array: RC-N → (kind, evidence_path, optional test_cmd)
- [ ] 2.3 Per-item verdict functions
- [ ] 2.4 Final summary: `11/11 RC_ITEMS PASS` or N/11 FAIL
- [ ] 2.5 Exit 0 iff 11/11 PASS; exit 1 otherwise
- [ ] 2.6 Make executable

## 3. Create wrapper evidence reports (8 new files)

- [ ] 3.1 `evidence/v312-59/RC1_GMP_MD_INGESTION_REPORT.md` (wrap v312-03)
- [ ] 3.2 `evidence/v312-59/RC2_RETRIEVAL_QUALITY_REPORT.md` (wrap v312-05)
- [ ] 3.3 `evidence/v312-59/RC3_BACKUP_RESTORE_REPORT.md` (wrap v312-09)
- [ ] 3.4 `evidence/v312-59/RC4_SECURITY_RBAC_REPORT.md` (wrap V312-53)
- [ ] 3.5 `evidence/v312-59/RC5_CURATED_SQLLOGICTEST_REPORT.md` (wrap sqllogictest-baseline)
- [ ] 3.6 `evidence/v312-59/RC7_WIRE_LOAD_DATA_REPORT.md` (wrap V312-13)
- [ ] 3.7 `evidence/v312-59/RC8_CRASH_UPGRADE_REPORT.md` (wrap crash-recovery report)
- [ ] 3.8 `evidence/v312-59/RC11_CLAIM_CLEANUP_REPORT.md` (wrap r11_claim_audit)

## 4. Implement RC-10 V312-57 week05-06 fixtures

- [ ] 4.1 Create `tests/compat/bustubx_edu_sqlite_cli/week05/` directory
- [ ] 4.2 Add 3 executor fixtures (week05_exec_create.sql + golden files)
- [ ] 4.3 Create `tests/compat/bustubx_edu_sqlite_cli/week06/` directory
- [ ] 4.4 Add 3 join+aggregate fixtures (week06_join.sql + week06_agg.sql + golden files)
- [ ] 4.5 Register 6 cases in `manifest.yml`
- [ ] 4.6 Verify `bash scripts/gate/check_bustubx_edu_cli_v312.sh` runs all 6 PASS

## 5. Wire into stage boundary

- [ ] 5.1 Read `scripts/gate/check_v312_stage_boundary.sh`
- [ ] 5.2 Add `bash scripts/gate/check_v312_promotion_to_rc.sh` invocation
- [ ] 5.3 Block BETA→RC transition on FAIL

## 6. STAGE.yaml updates

- [ ] 6.1 Reference `check_v312_promotion_to_rc.sh` in `promotion_to_RC_requires`
- [ ] 6.2 Verify STAGE.yaml sync check still passes

## 7. Validation

- [ ] 7.1 `bash scripts/gate/check_v312_promotion_to_rc.sh` reports 11/11 PASS
- [ ] 7.2 `openspec validate v312-59-c-promotion-to-rc` exits 0
- [ ] 7.3 No regression in BETA gate (B1-B8 still PASS)

## 8. PR & close

- [ ] 8.1 Branch: `fix/v312-59-c-promotion-to-rc` from `develop/v3.12.0`
- [ ] 8.2 Single commit per RC item (or grouped by family)
- [ ] 8.3 Push to origin
- [ ] 8.4 Create PR to `develop/v3.12.0`
- [ ] 8.5 Reference issue #4386 in PR body
- [ ] 8.6 After merge: comment on #4386 with PR link + verification output
- [ ] 8.7 Close #4386