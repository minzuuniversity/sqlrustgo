# Spec — V312-56B: SQL Teaching Corpus

## Overview

建立面向教学的 SQL corpus，覆盖核心 SQL 功能，不与 full SQLite official corpus 混同。

## Specification

### Directory Structure

```
tests/compat/teaching_sql_v3_12/
├── manifest.yml
├── select/
│   ├── basic.sql
│   ├── where.sql
│   ├── distinct.sql
│   └── alias.sql
├── join/
│   ├── inner_join.sql
│   ├── left_join.sql
│   └── multi_join.sql
├── group/
│   ├── group_by.sql
│   ├── having.sql
│   └── aggregate.sql
├── null/
│   ├── is_null.sql
│   ├── coalesce.sql
│   └── null_in_where.sql
├── order_limit/
│   ├── order_by.sql
│   ├── limit_offset.sql
│   └── order_limit.sql
├── subquery/
│   ├── scalar_subquery.sql
│   ├── in_subquery.sql
│   └── exists_subquery.sql
├── ddl/
│   ├── create_table.sql
│   ├── alter_table.sql
│   └── drop_table.sql
├── dml/
│   ├── insert.sql
│   ├── update.sql
│   └── delete.sql
├── error/
│   ├── division_by_zero.sql
│   └── type_mismatch.sql
└── transaction/
    ├── begin_commit.sql
    └── rollback.sql
```

### manifest.yml Schema

```yaml
files:
  - path: select/basic.sql
    oracle: sqlite
    expected: PASS
    owner: tbd
    stage: teaching
  - path: join/inner_join.sql
    oracle: sqlite
    expected: PASS
    owner: tbd
    stage: teaching
  # ... more files
```

### Oracle Matrix

| Feature | SQLite | MySQL | PostgreSQL |
|---|---|---|---|
| SELECT | ✓ | ✓ | ✓ |
| JOIN | ✓ | ✓ | ✓ |
| GROUP BY | ✓ | ✓ | ✓ |
| NULL | ✓ | ✓ | ✓ |
| ORDER BY | ✓ | ✓ | ✓ |
| LIMIT | ✓ | ✓ | ✓ |
| Subquery | ✓ | ✓ | ✓ |
| DDL | ✓ | - | - |
| DML | ✓ | ✓ | - |
| Transaction | ✓ | ✓ | - |

## Boundaries

- teaching corpus 不允许 `#[ignore]` 静默通过
- FAIL/SKIP 必须有关联 issue/owner/expiry
- 不宣称完整 SQLite official corpus 完成
