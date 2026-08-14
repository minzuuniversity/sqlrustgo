# Spec — V312-56E: Optimizer/EXPLAIN Teaching

## Overview

补齐 EXPLAIN、统计信息、histogram、hash join、semi/anti join 和简单 CBO 选择实验。

## Specification

### EXPLAIN Output Format

```
EXPLAIN [FORMAT=TEXT]
{
  id: 1
  select_type: SIMPLE
  table: t1
  type: ALL
  possible_keys: NULL
  key: NULL
  rows: 1000
  extra: NULL
}
```

#### Key Fields

| Field | Description |
|---|---|
| id | Query block identifier |
| select_type | SIMPLE / PRIMARY / SUBQUERY / DERIVED |
| table | Table name or alias |
| type | ALL / index / range / ref / eq_ref |
| possible_keys | Available indexes |
| key | Chosen index |
| rows | Estimated rows |
| extra | Access method details |

### Teaching Fixtures (5+)

#### Fixture 1: Full Table Scan vs Index

```sql
-- Without index: type=ALL
SELECT * FROM t WHERE col = 100;

-- With index: type=ref
CREATE INDEX idx ON t(col);
EXPLAIN SELECT * FROM t WHERE col = 100;
```

#### Fixture 2: Nested Loop vs Hash Join

```sql
-- Nested loop join
EXPLAIN SELECT * FROM t1 JOIN t2 ON t1.id = t2.t1_id;

-- Hash join (large tables)
EXPLAIN SELECT * FROM t1 JOIN t2 ON t1.id = t2.t1_id WHERE t1.value > 1000;
```

#### Fixture 3: Semi Join

```sql
EXPLAIN SELECT * FROM t1 WHERE id IN (SELECT t1_id FROM t2);
```

#### Fixture 4: Anti Join

```sql
EXPLAIN SELECT * FROM t1 WHERE id NOT IN (SELECT t1_id FROM t2);
```

#### Fixture 5: Histogram Impact

```sql
ANALYZE TABLE t;
EXPLAIN SELECT * FROM t WHERE col = 'value';
```

### Statistics Update

- `ANALYZE TABLE` updates statistics
- Statistics persisted and used for CBO
- Plan changes reproducible before/after ANALYZE

## Boundaries

- 不得把 heuristic plan 误写成成本模型严格正确
- 明确标注哪些是 heuristic vs cost-based decisions
