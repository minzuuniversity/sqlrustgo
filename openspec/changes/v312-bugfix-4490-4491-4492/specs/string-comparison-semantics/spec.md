## ADDED Requirements

### Requirement: blank-padded equality comparison for strings
The `=` comparison operator in `eval_binary_op` (`crates/executor/src/expr/mod.rs`) MUST treat two strings as equal when they differ only by trailing whitespace. This implements SQL standard semantics for `CHAR(n)` vs short string comparison (MySQL blank-padded semantics; SQLite ignores trailing whitespace).

The blank-padded rule applies only to the comparison operators `=`, `<>`, `<`, `>`, `<=`, `>=` when both operands are `Value::Text`. It does NOT modify `Value::PartialEq` (which preserves strict equality for Hash/sort/aggregate use cases).

Implementation strategy: trim trailing whitespace from both operands before comparison. This is equivalent to right-padding the shorter string to the length of the longer one and then comparing (when the longer string's trailing characters are all whitespace).

#### Scenario: char(n) vs short string equality
- **WHEN** table `t` has column `sex CHAR(2)` storing the value `'F '` (one trailing space)
- **AND** the query is `SELECT COUNT(*) FROM t WHERE sex = 'F'`
- **THEN** the result is `1` (matches MySQL/SQLite behavior; previously returned `0`)

#### Scenario: trailing-space strings are equal
- **WHEN** evaluating `'abc' = 'abc '`
- **THEN** result is `true`

#### Scenario: leading-space difference preserved
- **WHEN** evaluating `' abc' = 'abc'`
- **THEN** result is `false` (leading space is significant)

#### Scenario: inequality with different content
- **WHEN** evaluating `'abc' = 'def'`
- **THEN** result is `false`

#### Scenario: ordering operators also use trimmed comparison
- **WHEN** evaluating `'abc' < 'abd'`
- **THEN** result is `true` (unchanged behavior; both trimmed sides `abc` < `abd`)

### Requirement: Value equality semantics unchanged for internal operations
`Value::PartialEq` MUST continue to perform strict equality (no trimming). This preserves invariants in HashMap keys, sort comparators, and aggregate grouping.

#### Scenario: HashMap grouping uses strict equality
- **WHEN** two distinct `Value::Text` values `'F'` and `'F '` are used as keys in a HashMap-based GROUP BY
- **THEN** they are NOT grouped together (strict equality; the blank-padded rule applies only to `eval_binary_op`)