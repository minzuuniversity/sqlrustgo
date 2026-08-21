# V312-59-E — Thresholds Override Gate Evidence

**Issue**: #4388
**Branch**: `fix/v312-59-e-thresholds-override-gate` (from `develop/v3.12.0` @ 8c952a7b5)
**Date**: 2026-08-21
**Anti-Fabrication-Policy-v1.0**: §5 fully enforced (no boolean=false, no expiry push).

---

## Deliverables

| File | Purpose |
|---|---|
| `scripts/gate/check_v312_gate_thresholds.sh` | Composite gate enforcing 13 fields with both YAML-validity AND executable sub-gate checks |
| `scripts/gate/check_v312_stage_yaml_sync.sh` | Drift detector between STAGE.yaml `thresholds_override` keys and gate script field declarations |
| `scripts/gate/check_beta_v3.12.0.sh` (modified) | New `B8_THRESHOLDS_OVERRIDE` + `B8_STAGE_YAML_SYNC` sections (gate count 40 → 42) |
| `docs/releases/v3.12.0/STAGE.yaml` (modified) | References to B8 in `promotion_to_RC_requires` and `promotion_to_GA_requires` |
| `docs/releases/v3.12.0/evidence/v312-59-E/EVIDENCE.md` | This file |
| `docs/releases/v3.12.0/evidence/v312-59-e/thresholds_override_evidence.txt` | Machine-readable per-field verdict log |

---

## Field-by-field verdict

| # | Field | YAML value | Sub-gate | Verdict |
|---|---|---|---|---|
| 1 | COVERAGE_MIN_PER_CRATE | 80 | `check_coverage_v312.sh` | ✅ PASS |
| 2 | GMP_CORPUS_UNCLASSIFIED_FAILURE_MAX | 0 | (evidence file pending) | ⚠️ INFRASTRUCTURE_MISSING |
| 3 | GMP_AUDIT_TAMPER_TEST_REQUIRED | true | `v312-08-compliance-audit-report.md` | ✅ PASS |
| 4 | GMP_RETRIEVAL_CITATION_REQUIRED | true | `v312-05-hybrid-retrieval-report.md` | ✅ PASS |
| 5 | MIXED_SOAK_HOURS | 168 | (evidence file pending) | ⚠️ INFRASTRUCTURE_MISSING |
| 6 | SQLLOGICTEST_SMOKE_REQUIRED | true | `check_sqllogictest_v312.sh` | ✅ PASS |
| 7 | SQLLOGICTEST_SELECTED_TARGETS_REQUIRED | true | (file pending) | ❌ FAIL |
| 8 | TPCH_SF1_CORRECTNESS_REQUIRED | true | `evidence/G4_tpch_sf1.txt` | ✅ PASS |
| 9 | MYSQL_WIRE_E2E_REQUIRED | true | `check_v312_13_wire_load_data.sh` | ❌ FAIL |
| 10 | LOAD_DATA_BULK_IMPORT_REQUIRED | true | `check_load_data_infile.sh` | ✅ PASS |
| 11 | CRASH_RECOVERY_REQUIRED | true | `check_v312_14_crash_recovery.sh` | ✅ PASS |
| 12 | UPGRADE_DOWNGRADE_REQUIRED | true | `check_p14_upgrade_test.sh` | ❌ FAIL |
| 13 | BUSTUBX_EDU_SQLITE_CLI_REQUIRED | true | `check_bustubx_edu_cli_v312.sh` | ❌ FAIL |

**Totals**: 7 PASS · 4 FAIL · 2 INFRASTRUCTURE_MISSING · 13/13 checked.

---

## Action items surfaced by B8

These are the failures this gate is designed to catch. None of them are blockers for this PR — they are pre-existing gate failures that B8 now exposes structurally.

| Verdict | Field | Remediation |
|---|---|---|
| ❌ FAIL | SQLLOGICTEST_SELECTED_TARGETS_REQUIRED | Curate SQLLogicTest subset; write evidence to `docs/releases/v3.12.0/evidence/SQLLOGICTEST_SELECTED_TARGETS.txt` (tracked issue to be filed) |
| ❌ FAIL | MYSQL_WIRE_E2E_REQUIRED | `check_v312_13_wire_load_data.sh` failing — needs investigation; potentially fix tests or relax test environment expectations |
| ❌ FAIL | UPGRADE_DOWNGRADE_REQUIRED | `check_p14_upgrade_test.sh` failing — pending upgrade test fixes |
| ❌ FAIL | BUSTUBX_EDU_SQLITE_CLI_REQUIRED | `check_bustubx_edu_cli_v312.sh` failing — golden files / fixture mismatches; tracked under V312-57 |
| ⚠️ INFRASTRUCTURE | GMP_CORPUS_UNCLASSIFIED_FAILURE_MAX | No `GMP_CORPUS_REPORT.md` evidence file yet — generate from corpus ingestion run |
| ⚠️ INFRASTRUCTURE | MIXED_SOAK_HOURS | No `MIXED_SOAK_REPORT.md` evidence file yet — 168h soak still pending per Anti-Fabrication-Policy |

---

## Anti-pattern checks (Anti-Fabrication-Policy-v1.0 §5)

- [x] No boolean=true → false flip (script would have hard-FAILed)
- [x] No expiry push on any boolean gate
- [x] Per-crate coverage check required (NOT L1_8 average)
- [x] SOAK = 168h, not reduced
- [x] TPCH_SF1_CORRECTNESS requires 22/22 with no unexplained zero-row

---

## Reproducible verification

```bash
# 1. Field-set sync (drift check)
bash scripts/gate/check_v312_stage_yaml_sync.sh

# 2. Composite gate (13 fields)
bash scripts/gate/check_v312_gate_thresholds.sh

# 3. Full BETA gate (includes B8)
bash scripts/gate/check_beta_v3.12.0.sh
```

The third invocation will surface additional pre-existing B1-B7 issues, but B8 is now
structurally enforced for every `thresholds_override` field, which was the issue's
acceptance criterion E-1, E-2, E-3.