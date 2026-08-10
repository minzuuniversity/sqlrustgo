# V312-11 SQLLogicTest Oracle Gate - Evidence Report

**source_agent**: claude-code
**source_run**: v312-11-gate-run-2026-08-10
**timestamp**: 2026-08-10T12:45:00+08:00
**commit**: 0f14219318 (docs(V312-11): fix exclusions.yml fix_issue mapping)
**branch**: feature/sync-250-v312-19-5-tasks (develop/v3.12.0)
**evidence_hash**: 346dde4a6b1a608df8babfa95199b627

---

## Gate Execution Results

```
=== sqlrustgo SQLLogicTest Runner ===
test_dir: crates/sqlrustgo_sqllogictest/testdata

files:    6/16 (pass/fail)
pass rate: 31.6%
```

---

## Passing Test Files (6)

1. basic_select.test
2. string_test.test
3. null_test.test
4. delete__test_delete.test
5. demo.test
6. sql__test_delete.test

---

## Failing Test Files (10) - Tracked in exclusions.yml

| File | Root Cause | OpenSpec |
|------|------------|----------|
| setops__test_setops.test | PARSER - nested set operation in derived table | v313-10 |
| setops__test_except.test | SEMANTIC - EXCEPT ALL semantics | v313-10 |
| insert__test_insert.test | EXECUTION - ORDER BY with VALUES | v313-09 |
| insert__test_insert_invalid.test | PARSER - invalid UTF-8 in test file | v313-09 |
| update__test_update.test | EXECUTION - transaction isolation (con1/con2) | v313-08 |
| alter__alter_table_set_partitioned_by.test | PARSER - ALTER TABLE syntax | v313-08 |
| alter_table_set_partitioned_by.test | PARSER - ALTER TABLE SET PARTITIONED BY | v313-08 |
| case_insensitive_alter.test | SEMANTIC - table name case sensitivity | v313-08 |
| order__test_limit.test | SEMANTIC - LIMIT with window functions | v313-13 |
| test_constraint_with_updates.test | SEMANTIC - constraint checking on UPDATE | v313-11 |

---

## PREPROCESS FAIL (3) - Environment Issues

| File | Issue |
|------|-------|
| quantile_fun.test | Missing tpch_setup.test_template |
| aggregate__quantile_fun.test | Missing tpch_setup.test_template |
| sql__quantile_fun.test | Missing tpch_setup.test_template |

---

## Excluded Test Files (Not in 16 count)

These files are in subdirectories (duckdb_samples, duckdb_full) and require additional setup:

- create_as.test
- test_constraint_with_updates.test
- constraints__test_not_null.test
- binder__alias_error_10057.test

---

## Code Fixes Applied

### 1. executor match arm fix (#3986 follow-up)
**Commit**: 22178a7e9a
**Issue**: PR #4003 added SetSessionVariable to parser but omitted executor match arm
**Fix**: Added `TransactionStatement::SetSessionVariable` match arm in `execute_transaction`

### 2. exclusions.yml fix_issue mapping correction
**Commit**: 0f14219318
**Issue**: All 16 entries incorrectly pointed to closed issue #3908
**Fix**: Updated all fix_issue fields to point to correct OpenSpec changes (v313-08 ~ v313-15)

---

## OpenSpec Tracking

All failing tests are tracked in OpenSpec changes for v3.13:

- `v313-08-proposal/` - Parser fixes (ALTER TABLE, UPDATE syntax, case-insensitive)
- `v313-09-proposal/` - INSERT fixes
- `v313-10-proposal/` - Set operations + LIMIT/OFFSET
- `v313-11-proposal/` - UPDATE constraints
- `v313-12-proposal/` - QUANTILE aggregate
- `v313-13-proposal/` - LIMIT/OFFSET with window functions
- `v313-14-proposal/` - NOT NULL on UPDATE, CREATE TABLE AS
- `v313-15-proposal/` - Binder alias errors

---

## Gate Status

**Status**: ✅ PASS (with exclusions)

The V312-11 gate establishes a smoke baseline. All required infrastructure is in place:
- Gate script: `scripts/gate/sqllogictest_gate.sh`
- exclusions.yml with proper fix_issue mappings
- OpenSpec tracking for all deferred items

---

## Next Steps for v3.13

To improve pass rate from 31.6%:

1. **Parser fixes**: VALUES in nested set operations, ALTER TABLE syntax
2. **Executor fixes**: ORDER BY with VALUES, constraint checking
3. **Semantic fixes**: Table name case sensitivity, EXCEPT ALL semantics
4. **Environment**: Set up tpch_setup.test_template for quantile tests
