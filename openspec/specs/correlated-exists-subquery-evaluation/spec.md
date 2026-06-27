# correlated-exists-subquery-evaluation Specification

## Purpose
TBD - created by archiving change v390-sprint6-q4-exists-overcount. Update Purpose after archive.
## Requirements
### Requirement: Correlated EXISTS subquery must evaluate against the outer row

The executor MUST evaluate `EXISTS(correlated_subquery)` by executing
the inner SELECT with the outer row's column values substituted, and
returning `true` iff the inner SELECT returns at least one row. The
current `Expression::Exists(_) => true` conservative fallback
(`src/engine_utils.rs:215`) MUST be replaced with a real evaluation
that respects the subquery's WHERE clause and any correlated column
references.

#### Scenario: EXISTS with a matching inner row returns true

- **WHEN** the executor evaluates `EXISTS(SELECT 1 FROM inner WHERE inner.x = outer.x)` for an outer row where some inner row has `inner.x = outer.x`
- **THEN** the EXISTS returns `true`

#### Scenario: EXISTS with no matching inner row returns false

- **WHEN** the executor evaluates `EXISTS(SELECT 1 FROM inner WHERE inner.x = outer.x)` for an outer row where no inner row has `inner.x = outer.x`
- **THEN** the EXISTS returns `false`

#### Scenario: TPC-H Q4 returns per-priority counts matching PostgreSQL

- **WHEN** the executor runs the Q4 simplified form
  `SELECT o_orderpriority, count(*) FROM orders WHERE o_orderdate >= '1993-07-01' AND o_orderdate < '1993-10-01' AND EXISTS(SELECT * FROM lineitem WHERE l_orderkey = o_orderkey AND l_commitdate < l_receiptdate) GROUP BY o_orderpriority ORDER BY o_orderpriority`
  against the SF=0.001 fixture
- **THEN** the result has 5 rows with `count(*)` values matching
  PostgreSQL: `[27, 23, 29, 23, 16]` (sum=118), NOT the current
  over-included `[107, 104, 98, 102, 84]` (sum=495)

#### Scenario: Q13 and Q16 still pass after the change

- **WHEN** the executor runs the Q13 form
  `SELECT c_count, count(*) FROM (SELECT c_custkey, count(o_orderkey) c_count FROM customer LEFT JOIN orders ON c_custkey = o_custkey AND o_comment NOT LIKE '%special%requests%' GROUP BY c_custkey) GROUP BY c_count ORDER BY c_count DESC`
  or the Q16 form
  `SELECT p_brand, p_type, p_size, count(DISTINCT ps_suppkey) FROM partsupp, part WHERE p_partkey = ps_partkey AND p_brand <> 'Brand#45' AND p_type NOT LIKE 'MEDIUM POLISHED%' AND p_size IN (...) GROUP BY ...`
- **THEN** the existing Q13/Q16 cell-diff results are preserved (no
  regression on correlated subqueries that don't use `count(*)` in
  combination with the conservative fallback's "all rows pass" semantic)

