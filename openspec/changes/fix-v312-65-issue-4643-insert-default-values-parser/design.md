## Context

Issue #4643: `INSERT INTO t DEFAULT VALUES` is a standard SQL form that inserts exactly one row populated entirely from the table's column DEFAULT expressions (or NULL when no default is defined). The current parser rejects it with `Parse error: Expected VALUES or SELECT` (parser.rs:6877), even though the lexer already promotes `DEFAULT` to `Token::Default` (crates/parser/src/token.rs) and the executor already handles the per-value `DEFAULT` sentinel via `materialise_default_tokens` (src/engine_helpers.rs:32).

The fix is a three-line grammar change plus a small executor branch. The minimum viable behaviour, per the issue body, is to parse `INSERT INTO t DEFAULT VALUES` and produce the expected output row — `(NULL, 100, 'hello')` for the issue's example table.

## Goals / Non-Goals

**Goals:**
- Parser accepts `INSERT INTO <table> DEFAULT VALUES` and emits `InsertStatement { default_values: true, .. }`.
- Executor inserts one row where each column is its declared DEFAULT (or NULL when no default).
- One new AST field (`default_values: bool`), one new parser branch, one new executor branch.
- Regression tests covering: parser accepts, executor inserts one row with correct defaults, parser still rejects `INSERT INTO t` (no values/select/default_values).

**Non-Goals:**
- Multi-row `DEFAULT VALUES` syntax (SQLite/MySQL only allow one row; the SQL standard form is single-row).
- Default-value expressions referencing other columns or sequences (the existing `materialise_default_tokens` path uses pre-parsed literal defaults; complex DEFAULT expressions are out of scope here).
- `INSERT INTO t (col1) DEFAULT VALUES` (partial-column `DEFAULT VALUES`) — not a standard form; if requested it can be added later.

## Decisions

### D1. Add `default_values: bool` field (not Option type) to InsertStatement

A plain `bool` with `false` default is the smallest API change. Every existing construction site of `InsertStatement` adds `default_values: false`. The Rust compiler will surface any missed construction site at build time.

**Alternatives considered:**
- Use a sentinel `Vec<Vec<Expression>>` shape (e.g. empty `values` plus a `default_values: bool`): same as D1, just renamed.
- Add a new `Statement::InsertDefaultValues` variant: requires new match arms in every site that matches on `Statement::Insert`; much higher blast radius for negligible benefit.
- Reuse `select` field with a synthetic SelectStatement: awkward and confusing.

### D2. Parser order: `VALUES` first, then `SELECT`, then `DEFAULT VALUES`

The current dispatch at parser.rs:6814 / 6870 is `VALUES` then `SELECT`; the issue asks for `DEFAULT VALUES` as a third alternative. The new dispatch adds the `DEFAULT VALUES` branch after the `SELECT` branch:

```rust
} else if matches!(self.current(), Some(Token::Default)) {
    self.next(); // consume DEFAULT
    self.expect(Token::Values)?;
    (Vec::new(), None)  // empty values, no select — executor infers from default_values flag
} else if matches!(self.current(), Some(Token::Select)) {
    ...
}
```

But — the parser must remember the `default_values` flag on the `InsertStatement`. So we set a `bool` before the dispatch and read it when constructing the `InsertStatement`.

### D3. Executor: build a synthetic all-DEFAULT row, reuse `materialise_default_tokens`

When `default_values` is `true`, build:

```rust
let sentinel_row = vec![Value::Text("DEFAULT".to_string()); table_info.columns.len()];
let records = vec![sentinel_row];
let all_records = materialise_default_tokens(records, &[], &table_info.columns);
```

The `columns` argument is `&[]` (no explicit column list) so `materialise_default_tokens` takes the no-column-list branch (engine_helpers.rs:44-60) which substitutes DEFAULT for every column position.

### D4. Validation: parser must reject `INSERT INTO t DEFAULT` (missing VALUES)

The parser MUST still produce a clear error if the user writes `INSERT INTO t DEFAULT` without the trailing `VALUES`. The existing `expect(Token::Values)` will surface `"Expected 'VALUES'"` which is acceptable.

### D5. Re-export: InsertStatement already re-exported, no change needed

`InsertStatement` is already exported from `crates/parser/src/lib.rs:20`. The new field is part of the existing struct — no additional re-export work.

## Risks / Trade-offs

- **AST field proliferation.** Adding a field to `InsertStatement` touches every construction site. Mitigation: `cargo build --all-features` catches any missed site; existing construction sites are all in `parser.rs` and tests.
- **Keyword reservation.** `DEFAULT` is already a reserved keyword in this parser (it's used for column DEFAULT expressions, parser.rs:6845). No new reservation.
- **`materialise_default_tokens` is the single point of failure.** If that helper has a bug for the no-column-list path, `DEFAULT VALUES` inherits it. Mitigation: the helper is already exercised by `INSERT VALUES (DEFAULT)` and `INSERT VALUES (col1, DEFAULT)` tests; no new failure mode is introduced.
- **No `ON CONFLICT` interaction.** `INSERT ... DEFAULT VALUES` could legitimately be combined with `ON CONFLICT DO NOTHING` / `ON CONFLICT DO UPDATE`. The new parser branch sets `default_values = true` before the `ON CONFLICT` clause is parsed, so the clause flows through unchanged. This is an opportunistic improvement (not a goal), but is consistent with the AST shape.

## Verification Plan

- Unit tests in `crates/parser/src/parser.rs` (in the existing `set_op_tests` mod near the `v312_63_parser_issues_test`-style tests):
  - `test_parse_insert_default_values_basic` — `INSERT INTO t DEFAULT VALUES` produces `InsertStatement { default_values: true, .. }`.
  - `test_parse_insert_default_values_with_explicit_columns_fails_or_parses` — `INSERT INTO t (a) DEFAULT VALUES` either parses with explicit column list or errors with a clear message. (Document chosen behaviour.)
  - `test_parse_insert_default_lowercase` — `insert into t default values` parses (keyword case-insensitivity is handled by the lexer).
- Integration test in `tests/integration/sql/v312_65_insert_default_values_test.rs`:
  - Run via `ExecutionEngine::execute` against `MemoryStorage`:
    - `CREATE TABLE t(id INTEGER, val INTEGER DEFAULT 100, name TEXT DEFAULT 'hello')`
    - `INSERT INTO t DEFAULT VALUES`
    - `SELECT * FROM t` → exactly one row of `(NULL, 100, 'hello')`.
- `cargo build --all-features` clean.
- `cargo test -p sqlrustgo-parser --all-features` green.
- `cargo test -p sqlrustgo-cli --all-features --lib` green (no regression).
- `cargo test --test v312_65_insert_default_values_test` green.
- `cargo clippy --all-features` clean.
- Manual CLI repro: same `printf` as the issue, exit 0, expected row.
