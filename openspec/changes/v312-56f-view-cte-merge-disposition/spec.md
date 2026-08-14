# Spec — V312-56F: VIEW/CTE/MERGE Disposition

## Overview

明确 VIEW、CTE、MERGE 在 v3.12、4.0.0、教学场景中的真实边界。

## Specification

### CTE Teaching Coverage

#### Basic CTE

```sql
WITH cte AS (
  SELECT id, name FROM users WHERE active = 1
)
SELECT * FROM cte WHERE id > 10;
```

#### Multiple CTEs

```sql
WITH
  active_users AS (SELECT * FROM users WHERE active = 1),
  recent_orders AS (SELECT * FROM orders WHERE date > '2024-01-01')
SELECT u.name, o.total
FROM active_users u
JOIN recent_orders o ON u.id = o.user_id;
```

#### Recursive CTE

```sql
WITH RECURSIVE
  cnt AS (
    SELECT 1 AS n
    UNION ALL
    SELECT n + 1 FROM cnt WHERE n < 10
  )
SELECT n FROM cnt;
```

#### CTE Boundaries

- Recursive depth limit: 1000 (configurable)
- Circular reference detection: return error
- Current: basic recursive works, complex edge cases deferred

### VIEW Disposition

| State | Condition |
|---|---|
| PARTIAL/DEFERRED | Currently saves definition only, no expansion |
| DONE (if expanded) | View expansion implemented with correct semantics |

### MERGE Disposition

Current status: `execute()` returns `UNSUPPORTED`

Decision required:
- Option A: Implement MERGE into main execution path
- Option B: Keep UNSUPPORTED with clear documentation

## Boundaries

- README, MYSQL_COMPAT_STATUS, COMPREHENSIVE_ASSESSMENT_REPORT must agree on status
- No ambiguous PARTIAL without remediation issue linkage
