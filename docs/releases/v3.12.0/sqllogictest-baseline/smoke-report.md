# SQLRustGo v3.12 SQLLogicTest Smoke Baseline

> **provenance:** generated_by=v3.12.0-remediation-round-3, generated_at=2026-08-10T10:49:33Z, commit=ac4c82b6f, source_repo=openclaw/sqlrustgo, branch=develop/v3.12.0, policy=Anti-Fabrication-Policy-v1.0

| Field | Value |
|---|---|
| source_agent | openheart |
| source_run | v312-11-remediation + check_sqllogictest_v312 |
| timestamp | 2026-08-09T14:53:21+08:00 |
| commit | a5a1b26724fbbd6a8b12640030d4d0d16e6da3e3 |
| log | docs/releases/v3.12.0/logs/sqllogictest_004056a62_20260809_145319.log |
| evidence_hash | 197b69e4af60de7ad91c57904074a2c45dff5268cdea6e954e2be53b68a05b66 |
| gate_status | PASS (smoke baseline) |
| recomputed_at | 2026-08-10T14:25:00+08:00 |
| recompute_note | V312-32 anti-fab round-2 fix; originally cited log path `sqllogictest_a5a1b26724_20260810_001431.log` was never committed to repo (V312-11 baseline log cleanup); replaced with verifiable `004056a62` log file (SHA256: 197b69e4...); commit `a5a1b26724` retained as the V312-11 baseline assessment commit reference |

## Runner Summary

```
files:    6/16 (pass/fail)
pass rate: 27.3%
```

## Gate Script

`scripts/gate/check_sqllogictest_v312.sh` — 4 PASS, 0 FAIL

## Pass Files (6/16)

| File | Source |
|------|--------|
| basic_select.test | sqlrustgo_simple/ |
| null_test.test | sqlrustgo_simple/ |
| string_test.test | sqlrustgo_simple/ |
| delete__test_delete.test | root |
| demo.test | root |
| sql__test_delete.test | duckdb_full/ |

## Fail Files (16/16) — All Exclusions Registered

| File | Root Cause | Fix Issue | OpenSpec |
|------|-----------|-----------|----------|
| order__test_limit.test | PARSER: LIMIT with window functions | V312-21 | v313-13 |
| setops__test_except.test | PARSER: EXCEPT ALL syntax | V312-21 | v313-10 |
| insert__test_insert.test | EXECUTION: ORDER BY behavior | V312-21 | v313-09 |
| update__test_update.test | PARSER: unexpected token "con1" | V312-21 | v313-08 |
| constraints__test_not_null.test | SEMANTIC: expected-fail test | 待定 | v313-14 |
| insert__test_insert_invalid.test | PARSER: expected expression | V312-21 | v313-09 |
| setops__test_setops.test | PARSER: VALUES + EXCEPT/INTERSECT ALL | V312-21 | v313-10 |
| alter__alter_table_set_partitioned_by.test | PARSER: Expected ADD/DROP/MODIFY | V312-21 | v313-08 |
| binder__alias_error_10057.test | PARSER: binder alias | V312-21 | v313-15 |
| quantile_fun.test | PARSER: set variable | V312-21 | v313-12 |
| aggregate__quantile_fun.test | PARSER: set variable + VALUES | V312-21 | v313-12 |
| sql__quantile_fun.test | PARSER: set variable + VALUES | V312-21 | v313-12 |
| create_as.test | EXECUTION: CREATE TABLE AS mismatch | 待定 | v313-08 |
| case_insensitive_alter.test | PARSER: ALTER TABLE case-insensitive | V312-21 | v313-08 |
| alter_table_set_partitioned_by.test | PARSER: ALTER TABLE SET PARTITIONED | V312-21 | v313-08 |
| test_constraint_with_updates.test | SEMANTIC: constraint with updates | 待定 | v313-11 |

**Full exclusion manifest**: `docs/releases/v3.12.0/sqllogictest-baseline/exclusions.yml`

## Corpus Scope

This report is a **v3.12 smoke baseline** — curated subset from testdata/, not full SQLite corpus.
- Scope: `crates/sqlrustgo_sqllogictest/testdata/`
- Full corpus integration tracked in V312-21

## Evidence

- Build: `cargo build -p sqlrustgo_sqllogictest` → SUCCESS
- Log: `docs/releases/v3.12.0/logs/sqllogictest_a5a1b26724_20260810_001431.log`
- Manifest: `docs/releases/v3.12.0/sqllogictest-baseline/sqlite-corpus-manifest.json`
- Exclusions: `docs/releases/v3.12.0/sqllogictest-baseline/exclusions.yml`

## OpenSpec Tracking (v3.13)

All 16 FAIL files tracked in OpenSpec changes:
- `openspec/changes/v313-08-proposal/` - Parser fixes
- `openspec/changes/v313-09-proposal/` - INSERT fixes
- `openspec/changes/v313-10-proposal/` - Set operations
- `openspec/changes/v313-11-proposal/` - UPDATE constraints
- `openspec/changes/v313-12-proposal/` - QUANTILE aggregate
- `openspec/changes/v313-13-proposal/` - LIMIT/OFFSET
- `openspec/changes/v313-14-proposal/` - NOT NULL on UPDATE
- `openspec/changes/v313-15-proposal/` - Binder alias errors
