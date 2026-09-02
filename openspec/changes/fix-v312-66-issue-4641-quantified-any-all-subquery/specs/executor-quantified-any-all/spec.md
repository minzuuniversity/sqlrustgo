## ADDED Requirements

### Requirement: Executor MUST evaluate `val > ANY (subq)` against outer rows

The executor MUST, for each outer row, evaluate the LHS value `val` against the first column of the subquery result using the `>` operator. The row passes if at least one subquery value satisfies the comparison. An empty subquery result MUST return no outer rows (SQL standard).

#### Scenario: `> ANY` with a non-correlated subquery (issue #4641 example)

- **WHEN** the executor runs:
  1. `CREATE TABLE a(val INT)` and rows `(10),(20),(30)`
  2. `CREATE TABLE b(val INT)` and rows `(10),(25),(35)`
  3. `SELECT * FROM a WHERE val > ANY (SELECT val FROM b)`
- **THEN** the result MUST contain rows `(20)` and `(30)` (val > 10 for at least one row in b)

### Requirement: Executor MUST evaluate `val > ALL (subq)` against outer rows

The executor MUST, for each outer row, evaluate the LHS value `val` against every value in the subquery result using the `>` operator. The row passes if every subquery value satisfies the comparison. An empty subquery result MUST include every outer row (vacuous truth).
The executor MUST, for each outer row, evaluate the LHS value `val` against the first column of the subquery result using the `>` operator. The row passes if at least one subquery value satisfies the comparison. An empty subquery result MUST return no outer rows (SQL standard).
#### Scenario: `> ALL` with non-empty subquery — only matches above MAX

- **WHEN** the executor runs `SELECT * FROM a WHERE val > ALL (SELECT val FROM b)` against `a` rows `(10),(20),(30)` and `b` rows `(10),(25),(35)`
- **THEN** the result MUST contain no rows (no value in `a` exceeds `35`, the max of `b`)

#### Scenario: `> ALL` with empty subquery — all rows match

- **WHEN** the executor runs `SELECT * FROM a WHERE val > ALL (SELECT val FROM b WHERE 1=0)` against `a` rows `(10),(20),(30)`
- **THEN** the result MUST contain all three rows `(10)`, `(20)`, `(30)` (vacuous truth)

### Requirement: Executor MUST evaluate `= ANY (subq)` and `= ALL (subq)`

The executor MUST support both: `= ANY` passes the outer row if at least one subquery value equals the outer value; `= ALL` passes if every subquery value equals the outer value (or the subquery is empty).

#### Scenario: `= ANY` matches against any subquery value

- **WHEN** the executor runs `SELECT * FROM a WHERE val = ANY (SELECT val FROM b)` against `a` rows `(10),(20),(30)` and `b` rows `(10),(25),(35)`
- **THEN** the result MUST contain only `(10)` (only value present in both `a` and `b`)

### Requirement: Executor MUST support `<`, `<=`, `>=`, `!=` operators

The six SQL comparison operators MUST all work with ANY / ALL quantifiers.

#### Scenario: `< ANY`

- **WHEN** the executor runs `SELECT * FROM a WHERE val < ANY (SELECT val FROM b)` against `a=(10,20,30)` and `b=(10,25,35)`
- **THEN** the result MUST contain `(10)` (10 < 25 for at least one `b` row)

### Requirement: Correlated `QuantifiedOp` MUST NOT crash

When a `QuantifiedOp` subquery references outer columns (correlated), the executor MUST either evaluate correctly or fall back to a conservative `true` (matching the existing IN/EXISTS pattern at `eval_predicate`). Crashing or returning all-false is unacceptable.

#### Scenario: Correlated `> ANY` subquery does not panic

- **WHEN** the executor runs `SELECT * FROM outer_t WHERE val > ANY (SELECT inner_col FROM inner_t WHERE inner_col < outer_t.val)` against two populated tables
- **THEN** the executor MUST NOT panic; the result MAY be over-inclusive (conservative `true` fallback) or correctly evaluated
