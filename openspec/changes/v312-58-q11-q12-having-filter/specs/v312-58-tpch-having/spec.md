# Spec — v312-58-tpch-having

> **Capability**: Execute HAVING clause on grouped SELECT queries, including HAVING predicates that reference aggregate functions with arithmetic expressions (e.g., `SUM(a*b) > threshold`).
> **Status**: Proposed (Issues #4377 + #4378, V312-58 Q11/Q12)
> **Default state**: HAVING parsed into AST (`Select.having: Option<Expression>`) but silently dropped before execute → GROUP BY returns all groups without HAVING filter.

## ADDED Requirements

### Requirement: HAVING filter execution

The system SHALL execute the `Select.having` predicate against each grouped row, filtering groups whose HAVING expression evaluates to `false` (or `unknown`, per SQL three-valued logic).

#### Scenario: simple HAVING with aggregate
- **WHEN** executing `SELECT cat, COUNT(*) FROM t GROUP BY cat HAVING COUNT(*) > 5`
- **THEN** groups with `COUNT(*) <= 5` are excluded from output
- **AND** groups with `COUNT(*) > 5` are included

#### Scenario: HAVING with arithmetic expression
- **WHEN** executing `SELECT ps_partkey, SUM(ps_supplycost * ps_availqty) AS v FROM partsupp GROUP BY ps_partkey HAVING SUM(ps_supplycost * ps_availqty) > 10000`
- **THEN** the arithmetic expression `SUM(ps_supplycost * ps_availqty)` is computed once for HAVING evaluation
- **AND** only groups where this value exceeds 10000 are included

#### Scenario: HAVING with AND of aggregates
- **WHEN** executing `... HAVING SUM(x) > 100 AND COUNT(*) < 50`
- **THEN** both subpredicates are evaluated
- **AND** a group is included only if BOTH are true

#### Scenario: HAVING with OR
- **WHEN** executing `... HAVING SUM(x) > 100 OR COUNT(*) = 0`
- **THEN** a group is included if EITHER subpredicate is true
- **AND** three-valued logic: if either is unknown and the other is false, the row is excluded

### Requirement: aggregate evaluation in HAVING scope

The system SHALL evaluate aggregate function calls (`SUM`, `COUNT`, `AVG`, `MIN`, `MAX`) inside the HAVING predicate using the same aggregate state as the SELECT projection (no recomputation).

#### Scenario: HAVING reuses SELECT aggregate
- **WHEN** executing `SELECT col, SUM(x) AS total FROM t GROUP BY col HAVING SUM(x) > 0`
- **THEN** the aggregate in HAVING is the SAME computation as the SELECT projection
- **AND** the planner may recognize the redundancy (optimization, not required)

#### Scenario: HAVING references aggregate not in SELECT
- **WHEN** executing `SELECT col FROM t GROUP BY col HAVING SUM(x) > 0`
- **THEN** `SUM(x)` is computed only for HAVING (not projected)
- **AND** the column is hidden from output

### Requirement: Q11 stock-level filter (#4377)

The system SHALL execute Q11 correctly: filter groups by `HAVING SUM(ps_supplycost * ps_availqty) > 10000`.

#### Scenario: Q11 with tpch-tiny
- **WHEN** executing `SELECT ps_partkey, SUM(ps_supplycost * ps_availqty) AS part_value FROM partsupp, supplier, nation WHERE ps_suppkey = s_suppkey AND s_nationkey = n_nationkey AND n_name = 'GERMANY' GROUP BY ps_partkey HAVING SUM(ps_supplycost * ps_availqty) > 10000 ORDER BY part_value DESC`
- **THEN** output contains only groups where `SUM(ps_supplycost * ps_availqty) > 10000`
- **AND** row count is strictly less than the row count without HAVING

### Requirement: Q12 shipping-mode predicate (#4378)

The system SHALL execute Q12 correctly with the standard CASE expression form: `SUM(CASE WHEN o_orderpriority = '1-URGENT' OR o_orderpriority = '2-HIGH' THEN 1 ELSE 0 END)` and the full WHERE chain (5 predicates).

#### Scenario: Q12 standard form
- **WHEN** executing the standard TPC-H Q12 with all 5 WHERE predicates + CASE expressions in SUM
- **THEN** output has exactly 2 rows (MAIL + SHIP) for SF=1 with date range `1994-01-01 ≤ l_receiptdate < 1995-01-01`
- **AND** no other shipmode values appear (IN list filter applies)

#### Scenario: Q12 simplified form (already works in test)
- **WHEN** executing the simplified `tpch_sf01_inprocess_test` Q12 variant (`COUNT(*) AS high_line_count, 0 AS low_line_count`)
- **THEN** output is unchanged from current behavior (regression-safe)

### Requirement: integration with GROUP BY + ORDER BY

The system SHALL integrate HAVING between GROUP BY and ORDER BY in the SELECT execution pipeline:
`FROM → WHERE → GROUP BY → HAVING → SELECT → DISTINCT → ORDER BY → LIMIT`

#### Scenario: pipeline order
- **WHEN** executing any SELECT with WHERE + GROUP BY + HAVING + ORDER BY
- **THEN** WHERE is applied first (filters rows)
- **AND** GROUP BY aggregates the filtered rows
- **AND** HAVING filters groups (NOT individual rows)
- **AND** ORDER BY sorts the surviving groups

### Requirement: HAVING false positives must not leak

The system MUST NOT return a group whose HAVING expression evaluates to `false` or `unknown` (per SQL semantics).

#### Scenario: HAVING returns false
- **WHEN** a group's HAVING expression evaluates to `false`
- **THEN** the group is excluded from the final output

#### Scenario: HAVING returns unknown (NULL)
- **WHEN** a group's HAVING expression evaluates to `unknown` (e.g., comparison with NULL)
- **THEN** the group is excluded (per SQL 3-valued logic, UNKNOWN = FALSE in WHERE/HAVING)