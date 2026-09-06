# parser-expression-index Specification

## ADDED Requirements

### Requirement: CREATE INDEX column list accepts identifiers and expressions

The SQL parser SHALL accept a column list inside `CREATE INDEX` that
mixes simple identifiers (`name`) with arbitrary expressions
(`CASE WHEN ... END`, `lower(name)`, arithmetic, function calls,
etc.). The dispatcher records each entry as a typed
`IndexColumnSpec` value that captures either form.

#### Scenario: issue body example

Given `CREATE INDEX idx_case ON t(CASE WHEN price > 15 THEN 1 ELSE 0
END)`, the parser succeeds. The resulting
`CreateIndexStatement.columns` is a one-element vector whose
entry carries the expression (no column name).

#### Scenario: mixed column and expression

Given `CREATE INDEX idx_mixed ON t(name, CASE WHEN price > 15 THEN
1 ELSE 0 END)`, the parser succeeds. The resulting
`CreateIndexStatement.columns` is a two-element vector: the first
entry is a column-name spec for `name`, the second is the expression
spec for the `CASE WHEN ... END`.

#### Scenario: simple column index (regression)

Given `CREATE INDEX idx_simple ON t(name)`, the parser still
succeeds and the result is a one-element column-name spec.

### Requirement: storage and catalog IndexInfo carry the same typed column list

The storage `IndexInfo` and the catalog `IndexInfo` MUST change
their `columns` field from `Vec<String>` to
`Vec<IndexColumnSpec>`. The serde representation MUST remain
backwards-compatible with previously-persisted JSON: the new
`expression` field is `#[serde(default)]`, and the existing column
names live in the `name` field.

#### Scenario: old JSON still loads

Given a previously-persisted `IndexInfo` JSON with
`"columns": ["name", "price"]`, the new struct loads it as a
two-element vector of column-name specs.

#### Scenario: new JSON with expression round-trips

Given a `CreateIndexStatement` with one expression entry, the
serialised `IndexInfo` JSON contains an entry with the
`expression` field set and the `name` field absent, and reload
preserves the expression.