## Context

Issue #4645 reports that `sqlrustgo-cli sqlite` rejects `CREATE FULLTEXT INDEX ft_idx ON t(body)` with a parser error. Root cause is localised to `parse_create` in `crates/parser/src/parser.rs:2616`, which dispatches on ten `Token` variants and returns an error on anything else (including `Token::Fulltext`). The lexer already promotes `FULLTEXT` to `Token::Fulltext` (lexer.rs:544), so the keyword is recognised as a token but not handled in the grammar.

The full FTS5/FTS3 storage implementation is a multi-month effort (tokenizer, segment storage, ranking, MATCH operator). For #4645 the minimum viable fix per the issue body is to "接受为不可识别 (但不应解析错)" — parse the syntax into a valid AST node, then surface a clear "not implemented" runtime error at the executor layer. This matches the convention used by other not-yet-implemented DDL paths in this codebase.

## Goals / Non-Goals

**Goals:**
- Parser accepts `CREATE FULLTEXT INDEX <name> ON <table>(<col1>, <col2>, ...)` and produces a valid AST node.
- Executor returns a clear `ExecutionError` with the issue number, rather than a parse crash.
- One new AST node (`CreateFulltextIndexStatement`), one new `Statement` variant, one new `parse_create_fulltext_index` method, one new executor dispatch arm.
- Regression tests covering: parser accepts the syntax, executor returns the runtime error, missing table name / column list yields a clear parser error.

**Non-Goals:**
- FTS5/FTS3 storage engine implementation (tokenizer, segments, MATCH operator).
- MySQL-style `CREATE FULLTEXT INDEX ... WITH PARSER ngram` options — FTS3 only, low value, out of scope.
- Backwards compatibility for any pre-existing grammar that may have accepted `FULLTEXT` as an identifier in some context — keyword reservation is a deliberate cost (verified by searching existing tests; no test uses `fulltext` as a column or table name).
- `DROP FULLTEXT INDEX` — orthogonal; can be added if requested.

## Decisions

### D1. Parse-only fix, executor runtime error

Keep the executor at zero-implementation for FULLTEXT. The parser emits `Statement::CreateFulltextIndex` with structured fields; the executor's `match` arm returns `SqlError::ExecutionError("FULLTEXT INDEX is not yet implemented (issue #4645); consider CREATE VIRTUAL TABLE ... USING fts5")`. This:
- Unblocks teaching-seed schema loading (the original ask).
- Gives users a clear error pointing to the alternative SQLite-FTS5 form.
- Keeps the diff small and reviewable.
- Leaves the AST ready for future FTS5 implementation.

**Alternatives considered:**
- Reject at parse time with a clearer error message: violates the issue's "不应解析错" requirement.
- Implement a minimal FTS5 shim using LIKE: silently produces wrong results for `MATCH 'foo bar'` queries; not acceptable.
- Forward `CREATE FULLTEXT INDEX` to a virtual table under the hood: a clever rewrite, but breaks observability (the user thinks they got an index when they got a virtual table) and contradicts the issue's preference for clear errors.

### D2. New AST struct mirrors `CreateIndexStatement`

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct CreateFulltextIndexStatement {
    pub name: String,
    pub table: String,
    pub columns: Vec<String>,
    pub if_not_exists: bool,
}
```

Field shape deliberately mirrors `CreateIndexStatement` (parser.rs:240) so future FTS implementation can share most of the helper code. The `if_not_exists` flag is included for forward compatibility — MySQL allows `CREATE FULLTEXT INDEX IF NOT EXISTS`, and the parser recognises `IF NOT EXISTS` as a generic modifier in other DDL.

### D3. Parser methods are private (`fn`, not `pub fn`)

`parse_create_fulltext_index` is an internal helper on `impl Parser`. No new public surface on the Parser type. The only new public items are the AST struct + enum variant + re-export.

### D4. Reuse existing `parse_identifier` and `parse_qualified_name` helpers

The new method follows the exact same shape as `parse_create_index` (parser.rs:2859): consume identifier → optional `IF NOT EXISTS` → identifier (table) → `( col1, col2 )`. No new parsing primitives needed.

### D5. Wire executor dispatch in `src/execution_engine.rs:729`

Add `Statement::CreateFulltextIndex(_) => Err(SqlError::ExecutionError(...))` to the existing `match` block. The error message references the issue number so future readers can find this change's commit.

## Risks / Trade-offs

- **Keyword reservation.** Treating `FULLTEXT` as a reserved keyword means user schemas that used it as an identifier would break. Mitigation: a project-wide grep confirmed no test or sample uses `fulltext` as an identifier; the keyword is already reserved at the lexer level (lexer.rs:544) so it's not a new reservation.
- **AST growth.** Adding one variant to `Statement` is a structural change that touches every `match` on `Statement`. Mitigation: `cargo build --all-features` will catch any missed match arm; no semantic change to existing variants.
- **FTS3 vs FTS5 confusion.** Users familiar with SQLite FTS5 will be surprised by the MySQL-style syntax error. Mitigation: the runtime error message explicitly suggests `CREATE VIRTUAL TABLE ... USING fts5` as the FTS5-compatible alternative.
- **No `DROP FULLTEXT INDEX` counterpart.** Users can parse `CREATE` but `DROP FULLTEXT INDEX` will still error. Out of scope per the issue; the existing `DROP INDEX` path is sufficient for cleanup if needed (with a caveat about FTS5 virtual-table cleanup being different).

## Verification Plan

- Unit tests in `crates/parser/src/parser.rs` covering:
  - `CREATE FULLTEXT INDEX ft_idx ON t(body)` → `Statement::CreateFulltextIndex { name: "ft_idx", table: "t", columns: ["body"] }`.
  - Multi-column: `CREATE FULLTEXT INDEX ft_idx ON t(a, b, c)` → 3 columns.
  - `CREATE FULLTEXT INDEX IF NOT EXISTS ft_idx ON t(body)` → `if_not_exists: true`.
  - Negative: `CREATE FULLTEXT INDEX` (missing everything) → clear parser error.
- Integration test in `tests/integration/sql/v312_64_create_fulltext_index_test.rs`:
  - Run via the CLI binary; expect parse-then-runtime-error shape: exit code 0, error message on stderr containing "FULLTEXT INDEX is not yet implemented" and "4645".
- `cargo build --all-features` clean.
- `cargo test -p sqlrustgo-parser --all-features` green.
- `cargo test -p sqlrustgo-cli --all-features --lib` green.
- `cargo clippy --all-features -- -D warnings` green.
