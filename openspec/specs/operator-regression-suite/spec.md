# operator-regression-suite Specification

## Purpose
TBD - created by archiving change v390-sprint3-operator-suite-merge. Update Purpose after archive.
## Requirements
### Requirement: Aggregate operator regression suite
The system SHALL pass 9 aggregate operator-level regression tests covering integer/real SUM, empty-set semantics, AVG type preservation, COUNT(*) vs COUNT(col), and mixed-type columns.

#### Scenario: sum_integer_column_returns_integer
- **WHEN** INSERTing values (10, 20, 30) into INTEGER column
- **AND** executing `SELECT SUM(q) FROM t`
- **THEN** result SHALL be "60" (Integer)

#### Scenario: sum_real_column_returns_real
- **WHEN** INSERTing REAL values into REAL column
- **AND** executing `SELECT SUM(q) FROM t`
- **THEN** result type SHALL be `Value::Float` (not Integer, not Text)

#### Scenario: sum_empty_table_returns_null_per_sql_standard
- **WHEN** executing `SELECT SUM(q) FROM t` on empty table
- **THEN** result SHALL be NULL (per SQL standard + PG + SQLite + MySQL)

#### Scenario: count_star_vs_count_column
- **WHEN** table has 10 rows, 3 with NULL in column `c`
- **THEN** `SELECT COUNT(*)` returns 10 and `SELECT COUNT(c)` returns 7

### Requirement: Join operator regression suite
The system SHALL pass 20 join operator-level regression tests covering 2/3/4-table inner joins, LEFT OUTER, NULL join keys, duplicate keys, cartesian, and aggregate-over-join (TPC-H Q1/Q3 shape).

#### Scenario: 2-table inner join with WHERE
- **WHEN** two tables joined on `customers.id = orders.cust_id` with a WHERE filter
- **THEN** returned rows SHALL match the expected count and column ordering

#### Scenario: 3-table chained join (Issue #3277 core)
- **WHEN** 3 tables joined: `a.id = b.a_id AND b.id = c.b_id`
- **THEN** returned rows SHALL be exactly the rows where the chain matches (not 0 rows from misresolved key)

#### Scenario: 4-table chain
- **WHEN** 4 tables joined with chained ON clauses
- **THEN** result SHALL be correct (no column-resolution ambiguity)

#### Scenario: LEFT OUTER JOIN with NULL key
- **WHEN** left row has no matching right row
- **THEN** left columns SHALL be returned with right columns as NULL

#### Scenario: Duplicate join keys (cartesian within key)
- **WHEN** multiple rows share the same join key
- **THEN** all combinations SHALL be returned (cartesian semantics within key)

### Requirement: EXISTS operator regression suite (Sprint 4 protection)
The system SHALL pass 4 EXISTS operator-level regression tests, providing pre-protection for Sprint 4 (EXISTS correlated subquery work).

#### Scenario: exists_returns_true_for_matching_row
- **WHEN** subquery returns at least one row matching outer row
- **THEN** EXISTS SHALL return TRUE

#### Scenario: exists_returns_false_for_no_match
- **WHEN** subquery returns no rows matching outer row
- **THEN** EXISTS SHALL return FALSE

#### Scenario: exists_with_correlated_subquery (Sprint 4 prep)
- **WHEN** subquery references outer row column
- **THEN** EXISTS SHALL evaluate per outer row correctly

#### Scenario: exists_with_null_comparison
- **WHEN** join key is NULL on either side
- **THEN** EXISTS SHALL return FALSE (NULL is not equal to anything)

