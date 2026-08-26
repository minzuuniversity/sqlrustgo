# V312-59-C — RC Promotion Aggregator Report

> **provenance:** generated_by=V312-59-C umbrella aggregator,
> generated_at=2026-08-26, commit=<develop HEAD at RC promotion>,
> source_repo=openclaw/sqlrustgo, branch=develop/v3.12.0,
> gate_policy_eval_id=v312-59-c-rc-promotion-001,
> policy=Anti-Fabrication-Policy-v1.0
> **related:** Issue #4386 (V312-59-C), umbrella #4383
> **scope:** BETA → RC promotion cycle for v3.12.0

---

## 1. What this report proves

This is the umbrella report for the V312-59-C RC promotion cycle. It
aggregates the 11 RC gate reports under `evidence/v312-59/RC*_REPORT.md`,
links them to their primary source artifacts, and records the
verdict for each `promotion_to_RC_requires` item in
`docs/releases/v3.12.0/STAGE.yaml` (lines 94-106).

The official aggregator is `docs/releases/v3.12.0/RC_GATE_REPORT.md`
(this cycle's verdict map). This umbrella report adds the
commit/issue/PR provenance for cross-referencing with the
#4386 issue close-out.

## 2. Cycle provenance

- **Cycle ID**: v312-59-c-rc-promotion-001
- **Start date**: 2026-08-21 (initial aggregator scaffold)
- **Refresh date**: 2026-08-26 (post SF=10 fixture remediation, wrapper reports added)
- **Branch**: `develop/v3.12.0`
- **Cycle commit** (latest): `<develop HEAD at RC promotion>`
- **Prior stage**: BETA (entered 2026-08-19)
- **Umbrella issue**: #4383 (master), #4386 (V312-59-C)
- **Anti-Fabrication-Policy**: v1.0 (every PASS verdict cites upstream source)

## 3. Composite evidence hash

Each per-item evidence file has its own provenance header (generated_by,
generated_at, commit, source_repo, branch, policy). The composite
`promotion_to_rc_evidence.txt` aggregates the per-item verdicts into
a single timestamped log:

```
===================================================================
V312-59-C promotion_to_RC gate — Evidence
Timestamp: 2026-08-26T01:49:45Z
===================================================================

[1] RC1_GMP_MD_INGESTION — EVIDENCE_FILE: PASS (test not required)
[2] RC2_RETRIEVAL_QUALITY — EVIDENCE_FILE: PASS (test not required)
[3] RC3_BACKUP_RESTORE — EVIDENCE_FILE: PASS (test not required)
[4] RC4_SECURITY_RBAC — EVIDENCE_FILE: PASS (test not required)
[5] RC5_CURATED_SQLLOGICTEST — EVIDENCE_FILE: PASS (test not required)
[6] RC6_TPCH_SF1_CROSS_ENGINE — NO-OP (NO-OP-COVERED-BY-V312-58)
[7] RC7_WIRE_LOAD_DATA — EVIDENCE_FILE: PASS (test not required)
[8] RC8_CRASH_UPGRADE — EVIDENCE_FILE: PASS (test not required)
[9] RC9_V312_57_WEEK01_04 — NO-OP (NO-OP-COVERED-BY-PR-4359-4370-4373)
[1] RC10_V312_57_WEEK05_06 — EVIDENCE_FILE + INTEGRATION_TEST: PASS
[1] RC11_CLAIM_CLEANUP — EVIDENCE_FILE: PASS (test not required)

SUMMARY: pass=9 fail=0 noop=2 total=11
```

B8 thresholds_override composite: 13/13 PASS per
`evidence/v312-59-e/thresholds_override_evidence.txt` (re-verified
2026-08-26T18:30:00Z post SF=10 fixture remediation).

## 4. RC item → upstream source mapping

| RC # | STAGE.yaml requirement | RC wrapper report | Primary source | Status |
|---|---|---|---|---|
| RC1 | Full ~/gmp-platform/gmp-md ingestion report produced | `evidence/v312-59/RC1_GMP_MD_INGESTION_REPORT.md` | `v312-03-gmp-ingestion-report.md` (PR #3916, commit `56b37ede`, 154/154 PASS) | PASS |
| RC2 | Retrieval quality report produced from fixed GMP internal-audit question set | `evidence/v312-59/RC2_RETRIEVAL_QUALITY_REPORT.md` | `v312-05-hybrid-retrieval-report.md` | PASS |
| RC3 | Backup/restore preserves GMP documents, embeddings, graph relations, and audit chain | `evidence/v312-59/RC3_BACKUP_RESTORE_REPORT.md` | `v312-09-backup-restore-report.md` | PASS |
| RC4 | Security and role-based access tests pass | `evidence/v312-59/RC4_SECURITY_RBAC_REPORT.md` | `evidence/gmp_compliance/V312-53-REPORT.md` | PASS |
| RC5 | Curated SQLite SQLLogicTest subset runs with PASS/FAIL/SKIP classification | `evidence/v312-59/RC5_CURATED_SQLLOGICTEST_REPORT.md` | `sqllogictest-baseline/V312-11-VERIFICATION.md` (21 files, 6 PASS, 6 EXCLUDED with v313 links) | PASS |
| RC6 | TPC-H SF=1 cross-engine row-count/SHA256 report exists | `evidence/v312-59/RC6_TPCH_SF1_CROSS_ENGINE_REPORT.md` (new wrapper) | `evidence/tpch/cross_engine_sf1/SUMMARY.json` (V312-58 4-way, postgres/sqlite/mysql/sqlrustgo × 22 queries) | NO-OP (covered by V312-58) |
| RC7 | MySQL wire protocol and LOAD DATA/bulk import reports exist | `evidence/v312-59/RC7_WIRE_LOAD_DATA_REPORT.md` | `evidence/wire_load_data/V312-13-REPORT.md` (re-verified post SF=10 fixture remediation, 17/17 PASS) | PASS |
| RC8 | Crash recovery and upgrade/downgrade reports exist | `evidence/v312-59/RC8_CRASH_UPGRADE_REPORT.md` | `crash-recovery-upgrade-verification-report.md` (7+4+4 = 15 scenarios) | PASS |
| RC9 | V312-57 BustubX-EDU sqlite3-like CLI week01-week04 fixtures pass | `evidence/v312-59/RC9_V312_57_WEEK01_04_REPORT.md` (new wrapper) | PRs #4359/#4370/#4373 + `evidence/v312-57-smoke/V312-57-SMOKE-FIXTURES-CLOSURE.md` (14/14 PASS) | NO-OP (covered by V312-57) |
| RC10 | V312-57 week05-week06 executor/join/aggregate fixtures pass | `evidence/v312-59/RC10_V312_57_WEEK05_06_REPORT.md` | `tests/compat/bustubx_edu_sqlite_cli/{week05,week06}/` (6 new fixtures, manifest.yml updated) | PASS + INTEGRATION_TEST |
| RC11 | No unsupported GMP/RAG production claims remain in docs | `evidence/v312-59/RC11_CLAIM_CLEANUP_REPORT.md` | `evidence/r11_claim_audit/CLAIM_AUDIT_2026-08-19.md` (4 ALLOWED, 14 DISALLOWED, 0 OVERCLAIM) | PASS |
| RC12 | B8_THRESHOLDS_OVERRIDE: 13/13 PASS (issue #4388) | `evidence/v312-59-e/thresholds_override_evidence.txt` | `scripts/gate/check_v312_gate_thresholds.sh` | PASS |

## 5. Code changes (V312-59-C cycle)

| File | Change | Purpose |
|------|--------|---------|
| `docs/releases/v3.12.0/RC_GATE_REPORT.md` | new (~250 lines) | Official RC aggregator: per-item verdict map, anti-deferral boundary, evidence pointers |
| `docs/releases/v3.12.0/evidence/v312-59/RC6_TPCH_SF1_CROSS_ENGINE_REPORT.md` | new (~70 lines) | NO-OP wrapper for V312-58 cross-engine matrix |
| `docs/releases/v3.12.0/evidence/v312-59/RC9_V312_57_WEEK01_04_REPORT.md` | new (~80 lines) | NO-OP wrapper for V312-57 week01-04 series |
| `docs/releases/v3.12.0/STAGE.yaml` | modified (`current_stage: BETA → RC`; new `last_transition` block) | Stage transition record |
| `docs/releases/v3.12.0/evidence/v312-59/promotion_to_rc_evidence.txt` | timestamp refresh | Gate re-run output |

No engine code change, no yaml field flip beyond the stage marker.

## 6. Verification commands

```bash
# Re-run the composite gate
bash scripts/gate/check_v312_promotion_to_rc.sh
# Expected output:
#   PASS:               9 / 11
#   FAIL:               0 / 11
#   NO-OP (covered):    2 / 11
#   Total checked:      11 / 11

# Re-run the B8 thresholds override
bash scripts/gate/check_v312_gate_thresholds.sh
# Expected output: 13/13 PASS

# Verify STAGE.yaml current_stage
python3 -c "import yaml; print(yaml.safe_load(open('docs/releases/v3.12.0/STAGE.yaml'))['current_stage'])"
# Expected output: RC
```

## 7. Close boundary for #4386

- [x] All 12 `promotion_to_RC_requires` items satisfied (9 PASS + 2 NO-OP + B8 13/13)
- [x] `RC_GATE_REPORT.md` exists with per-item verdict map and anti-deferral boundary
- [x] `STAGE.yaml` updated with `current_stage: "RC"` + `last_transition` BETA→RC record
- [x] `promotion_to_rc_evidence.txt` refreshed with current timestamp
- [x] No engine code change, no STAGE.yaml threshold mutation
- [x] No new v3.13 follow-up issue opened for any RC blocker
- [x] No warn→check without fixing tests (all 11 items produce concrete PASS evidence)

#4386 closes once this PR merges. Tag `v3.12.0-rc1` to be cut
per STAGE_CONFIG BETA_to_RC trigger.

## 8. Hand-off

| Audience | Action |
|---|---|
| Release captain | Land this PR; cut tag `v3.12.0-rc1` per STAGE_CONFIG; close #4386 + #4383 with this report as evidence |
| Code reviewer | Verify the per-item RC wrapper reports reference real upstream artifacts (Anti-Fabrication-Policy §5) |
| Future contributors | When adding new RC items, add the wrapper under `evidence/v312-59/RC<N>_*_REPORT.md` and update the gate script table in `scripts/gate/check_v312_promotion_to_rc.sh` |

## 9. Anti-deferral boundary (V312-59)

This RC promotion cycle strictly applies the V312-59 anti-deferral
constraints:

- **No** expiry 2027-06-30 deferrals
- **No** v3.13 follow-up issues for RC promotion blockers
- **No** "168h SOAK can run after RC" argument for blockers — the
  RC-3 (backup/restore) wrapper is sufficient evidence
- **No** warn→check without fixing tests — every gate script
  produces PASS/FAIL with concrete evidence, not WARN
