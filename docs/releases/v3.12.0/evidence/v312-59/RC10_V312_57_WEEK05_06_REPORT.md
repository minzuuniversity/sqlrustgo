# V312-59-C RC10 — V312-57 Week05-06 Fixtures Report

**Issue**: #4386 (V312-59-C)
**STAGE.yaml key**: `promotion_to_RC_requires[10]` — "V312-57 BustubX-EDU sqlite3-like CLI week05-week06 executor/join/aggregate fixtures pass"
**Verdict**: ✅ PASS (after this PR adds the missing fixtures)
**Status**: 6 new fixtures created and registered

---

## Background

V312-57 was the parent issue covering BustubX-EDU sqlite-style CLI teaching
fixtures. PRs #4359, #4370, and #4373 covered week01-04 (entry-level cases).
Week05 (executor) and week06 (join + aggregate) fixtures were missing.

## New fixtures added (3 executor + 3 join/aggregate)

### Week 5 — Executor

| Case | SQL | Golden |
|---|---|---|
| `week05_exec_create_table` | CREATE TABLE + 3-row INSERT + SELECT | 3 rows (id\|val\|label) |
| `week05_exec_filter` | 5-row table + WHERE val > 25 | 3 rows (id) |
| `week05_exec_aggregate` | 5-row table + GROUP BY SUM/COUNT | 3 rows (group_id\|count\|sum) |

### Week 6 — Join + Aggregate

| Case | SQL | Golden |
|---|---|---|
| `week06_join_inner` | 3 customers + 3 orders INNER JOIN | 3 rows (name\|total) |
| `week06_aggregate_group` | 5 products + GROUP BY AVG/MIN/MAX | 3 rows (cat\|avg\|min\|max) |
| `week06_join_three_tables` | 3-table INNER JOIN chain | 2 rows (id\|id\|val) |

## Files added

```
tests/compat/bustubx_edu_sqlite_cli/week05/week05_exec_create_table.sql  (+4 lines)
tests/compat/bustubx_edu_sqlite_cli/week05/week05_exec_create_table.golden  (+4 lines)
tests/compat/bustubx_edu_sqlite_cli/week05/week05_exec_filter.sql  (+3 lines)
tests/compat/bustubx_edu_sqlite_cli/week05/week05_exec_filter.golden  (+4 lines)
tests/compat/bustubx_edu_sqlite_cli/week05/week05_exec_aggregate.sql  (+4 lines)
tests/compat/bustubx_edu_sqlite_cli/week05/week05_exec_aggregate.golden  (+4 lines)
tests/compat/bustubx_edu_sqlite_cli/week06/week06_join_inner.sql  (+5 lines)
tests/compat/bustubx_edu_sqlite_cli/week06/week06_join_inner.golden  (+4 lines)
tests/compat/bustubx_edu_sqlite_cli/week06/week06_aggregate_group.sql  (+4 lines)
tests/compat/bustubx_edu_sqlite_cli/week06/week06_aggregate_group.golden  (+4 lines)
tests/compat/bustubx_edu_sqlite_cli/week06/week06_join_three_tables.sql  (+6 lines)
tests/compat/bustubx_edu_sqlite_cli/week06/week06_join_three_tables.golden  (+3 lines)
tests/compat/bustubx_edu_sqlite_cli/manifest.yml  (+6 cases appended)
```

## Manifest registration

All 6 new cases appended to `manifest.yml` after the existing week04 cases:

```yaml
# ─── Week 5: executor (CREATE/INSERT/SELECT/WHERE) ───
- case_id: week05_exec_create_table
  ...
- case_id: week05_exec_filter
  ...
- case_id: week05_exec_aggregate
  ...

# ─── Week 6: JOIN + GROUP BY ───
- case_id: week06_join_inner
  ...
- case_id: week06_aggregate_group
  ...
- case_id: week06_join_three_tables
  ...
```

## Oracle mode

All 6 fixtures use `oracle_mode: golden` — the expected output is encoded
in a sibling `.golden` file. The gate `check_bustubx_edu_cli_v312.sh` runs
each `.sql` against `target/debug/sqlrustgo` and diffs output against the
golden file.

## Cross-reference to B8

The B8_THRESHOLDS_OVERRIDE gate references this evidence via
`BUSTUBX_EDU_SQLITE_CLI_REQUIRED=true` (PASS via PR #4398).

## RC10 verdict for V312-59-C composite gate

```
[10/11] RC10_V312_57_WEEK05_06
  [EVIDENCE_FILE]      PASS (this file)
  [INTEGRATION_TEST]   runs check_bustubx_edu_cli_v312.sh (all 6 fixtures)
  → PASS
```

This gate is now PASS for `promotion_to_RC_requires[10]`.