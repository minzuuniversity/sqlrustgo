# V312-11 SQLLogicTest Oracle Gate — Verification Report

> **provenance:** generated_by=v3.12.0-remediation-round-3, generated_at=2026-08-10T10:49:33Z, commit=ac4c82b6f, source_repo=openclaw/sqlrustgo, branch=develop/v3.12.0, policy=Anti-Fabrication-Policy-v1.0

| Field | Value |
|---|---|
| source_agent | minimax-m2.7 |
| source_run | v312-11-final-remediation |
| timestamp | 2026-08-10T00:54:25+08:00 |
| commit | a5a1b26724fbbd6a8b12640030d4d0d16e6da3e3 |
| PR | #3938 (merged) |
| Merge Commit | 71fc33a9d287f79e17147cca086fb2e6cc4f7b39 |

## Gate Execution

```bash
bash scripts/gate/check_sqllogictest_v312.sh
```

**Result**: 4 PASS, 0 FAIL (exit 0)

## Evidence Hashes

| File | SHA256 |
|------|--------|
| Log | `a963c59da01c275362ea0fc5b524139b5c7247d58ee9c503052095bc09250c66` |
| exclusions.yml | `3f63e5a5b721b923a2d1e0816a4a8b5b07d1e260fc70ce69c03ef46a00d25e05` |
| sqlite-corpus-manifest.json | `05e59ffae4c7a9f163f66e4a1d43928017f0cdf669379ea810f2db2d1d346e16` |
| smoke-report.md | `cdb0420e8061960926fe37a94f1981f709a374d78e62c8175d668e194375f460` |

**Log file**: `docs/releases/v3.12.0/logs/sqllogictest_a5a1b26724_20260810_005425.log`

## Test Results

```
files:    6/22 (pass/fail)
pass rate: 27.3%
```

### PASS Files (6/22)

| File | Source |
|------|--------|
| basic_select.test | sqlrustgo_simple/ |
| null_test.test | sqlrustgo_simple/ |
| string_test.test | sqlrustgo_simple/ |
| delete__test_delete.test | root |
| demo.test | root |
| sql__test_delete.test | duckdb_full/ |

### FAIL Files (16/22) — All Registered in exclusions.yml

| File | Root Cause | OpenSpec |
|------|-----------|----------|
| insert__test_insert_invalid.test | PARSER | v313-08 |
| insert__test_insert.test | EXECUTION | v313-08 |
| update__test_update.test | PARSER | v313-08 |
| setops__test_except.test | PARSER | v313-09 |
| setops__test_setops.test | PARSER | v313-09 |
| order__test_limit.test | PARSER | v313-10 |
| alter__alter_table_set_partitioned_by.test | PARSER | v313-11 |
| alter_table_set_partitioned_by.test | PARSER | v313-11 |
| case_insensitive_alter.test | PARSER | v313-11 |
| constraints__test_not_null.test | SEMANTIC | v313-12 |
| test_constraint_with_updates.test | SEMANTIC | v313-12 |
| binder__alias_error_10057.test | SEMANTIC | v313-13 |
| create_as.test | EXECUTION | v313-14 |
| aggregate__quantile_fun.test | HARNESS | v313-15 |
| sql__quantile_fun.test | HARNESS | v313-15 |
| quantile_fun.test | HARNESS | v313-15 |

## Exclusion Registry Summary

- `status: active`
- `total_fail_files: 16`
- `smoke_scope_pass_files: 6`
- All 16 FAIL files have `root_cause`, `owner`, `expiry`, `follow_up_issue_or_openspec`

## OpenSpec Follow-ups (8 changes)

| Change | Coverage |
|--------|---------|
| v313-08-sql-logictest-insert-update-fix | 3 files |
| v313-09-sql-logictest-setops-fix | 2 files |
| v313-10-sql-logictest-order-limit-fix | 1 file |
| v313-11-sql-logictest-alter-table-fix | 3 files |
| v313-12-sql-logictest-constraint-semantics | 2 files |
| v313-13-sql-logictest-binder-alias | 1 file |
| v313-14-sql-logictest-create-as-execution | 1 file |
| v313-15-sql-logictest-duckdb-harness | 3 files |

## Closure Criteria

- [x] PR merged to `develop/v3.12.0` (PR #3938, commit 71fc33a9)
- [x] Gate script executes exit 0
- [x] All 16 FAIL files have OpenSpec follow-up
- [x] exclusions.yml: `status: active`, all items have required fields
- [x] Manifest stats consistent (22 total, 6 pass, 16 fail)
- [x] Evidence hashes computed from actual files
- [x] Issue comment posted with PR/Commit/command/log/PASS-FAIL/evidence_hash

## Status

**DEFERRED to v3.13.0 GA** — smoke baseline 6/22 PASS, 16 FAIL with OpenSpec tracking (owner: openclaw)
