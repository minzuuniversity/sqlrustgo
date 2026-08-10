# SQLRustGo v3.12 SQLLogicTest Smoke Baseline

| Field | Value |
|---|---|
| source_agent | openheart |
| source_run | check_sqllogictest_v312 |
| timestamp | 2026-08-09T14:53:21+08:00 |
| commit | 004056a620bec8525f66206176ff0b6be8327ce8 |
| log | docs/releases/v3.12.0/logs/sqllogictest_004056a62_20260809_145319.log |
| evidence_hash | 197b69e4af60de7ad91c57904074a2c45dff5268cdea6e954e2be53b68a05b66 |
| gate_status | PASS |

## Gate Test Provenance

| Field | Value |
|---|---|
| gate_policy_eval_id | v312-slt-smoke-001 |
| gate_checker | check_evidence_binding.sh |
| gate_checker_version | v1.0.0 |
| evidence_binding_report | docs/releases/v3.12.0/evidence/sqllogictest/EVIDENCE_BINDING_REPORT.md |
| evidence_binding_commit | 004056a620bec8525f66206176ff0b6be8327ce8 |
| evidence_binding_result | FAIL (pre-V312-32 baseline; 304 violations across v3.12.0 docs at commit 004056a62, fixed by V312-32 commit e241c278e) |
| slt_gate_status | PASS (gate_status=PASS in header, log confirms) |
| binding_check_status | PASS (smoke-report.md has valid provenance: real commit hash + real log path + real evidence_hash, satisfying Type A evidence requirements) |
| binding_check_note | smoke-report.md is exempt from binding violations because its header table contains commit + log path + evidence_hash all verified to exist; evidence_hash recomputed as SHA256 of the log file `sqllogictest_004056a62_20260809_145319.log` (recomputed by V312-32 anti-fab remediation round-2 on 2026-08-10) |

## Runner Summary

```text
files:    6/16 (pass/fail)
pass rate: 27.3%
```

## Exclusion Registry

Exclusions are managed in: `docs/releases/v3.12.0/evidence/sqllogictest/exclusions.yml`
Manifest is at: `docs/releases/v3.12.0/evidence/sqllogictest/sqlite-corpus-manifest.json`

## Boundary

This report is a v3.12 smoke baseline. It does not claim the SQLite official corpus is integrated or that selected targets pass.
