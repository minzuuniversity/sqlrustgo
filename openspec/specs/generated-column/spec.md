# generated-column Specification

## Purpose
TBD - created by archiving change fix-v313-95-4697-generated-columns. Update Purpose after archive.
## Requirements
### Requirement: GENERATED ALWAYS AS clause on CREATE TABLE columns

The SQL parser SHALL accept a `GENERATED ALWAYS AS (expr) [STORED|VIRTUAL]`
clause immediately after a column's data type in `CREATE TABLE`. The
clause is recorded on the column's AST node so downstream consumers can
inspect it.

`GENERATED`, `ALWAYS`, and `AS` are required keywords. The parenthesised
expression follows standard expression grammar. `STORED` and `VIRTUAL` are
optional; if absent, the parser defaults to `VIRTUAL` (SQL standard).

#### Scenario: GENERATED ALWAYS AS STORED parses

Given `CREATE TABLE t(id INT, total INT GENERATED ALWAYS AS (a + b) STORED)`,
when the parser processes the column list, then the column `total` carries
`generated = Some(GeneratedColumn { expression: "(a + b)", stored: true })`.

#### Scenario: GENERATED ALWAYS AS VIRTUAL parses

Given `CREATE TABLE t(id INT, bonus INT GENERATED ALWAYS AS (a * 0.1)
VIRTUAL)`, then the column `bonus` carries
`generated = Some(GeneratedColumn { expression: "(a * 0.1)", stored:
false })`.

#### Scenario: No GENERATED clause leaves field None

Given `CREATE TABLE t(id INT, name TEXT)`, then both columns carry
`generated = None`.

#### Scenario: Default mode is VIRTUAL when STORED/VIRTUAL omitted

Given `CREATE TABLE t(id INT, full_name TEXT GENERATED ALWAYS AS
(first_name || last_name))`, then `full_name.generated` is `Some(...)` with
`stored == false`.

### Requirement: GeneratedColumn AST and serde round-trip

A `ColumnDefinition` that carries a generated-column clause SHALL have a
`generated: Option<GeneratedColumn>` field populated to `Some(...)`.
`GeneratedColumn` SHALL contain a `expression: String` (Display text of the
parsed expression) and a `stored: bool` flag.

`ColumnDefinition` round-tripped through JSON/serde SHALL preserve the
`generated` field as `Some(...)` or `None`. Documents persisted before
this fix (where `generated` is absent) SHALL still deserialize because the
field uses `#[serde(default)]`.

#### Scenario: GeneratedColumn struct fields present

Given any `ColumnDefinition` carrying a GENERATED clause, the
`GeneratedColumn` is constructed with `expression` as the Display text of
the parsed `Expression` and `stored` set according to the keyword used
(`STORED = true`, `VIRTUAL = false`, default = `false`).

#### Scenario: Backward-compatible deserialization

Given a JSON document with a `ColumnDefinition` whose `generated` field is
absent, deserialization succeeds with `generated = None`. Given a JSON
document with `generated = null`, deserialization succeeds with
`generated = None`. Given a JSON document with
`generated = { "expression": "x+1", "stored": true }`, deserialization
succeeds with `generated = Some(GeneratedColumn { expression: "x+1",
stored: true })`.

