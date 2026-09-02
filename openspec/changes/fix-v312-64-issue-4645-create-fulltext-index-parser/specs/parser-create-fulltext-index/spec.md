## ADDED Requirements

### Requirement: Parser must accept `CREATE FULLTEXT INDEX` syntax

The parser MUST accept the MySQL-compatible `CREATE FULLTEXT INDEX <name> [IF NOT EXISTS] ON <table>(<col_list>)` syntax and emit a `Statement::CreateFulltextIndex` AST node with `{ name, table, columns, if_not_exists }` fields populated. The lexer already promotes `FULLTEXT` to `Token::Fulltext`; this requirement closes the gap where the parser's `parse_create` dispatcher (crates/parser/src/parser.rs:2616) returns an `Err` on `Some(Token::Fulltext)`.

#### Scenario: basic single-column FULLTEXT INDEX

- **WHEN** the parser is given `CREATE FULLTEXT INDEX ft_idx ON t(body)`
- **THEN** `parse(...)` MUST return `Ok(Statement::CreateFulltextIndex(CreateFulltextIndexStatement { name: "ft_idx", table: "t", columns: ["body"], if_not_exists: false }))`

#### Scenario: multi-column FULLTEXT INDEX

- **WHEN** the parser is given `CREATE FULLTEXT INDEX ft_idx ON articles(title, body, tags)`
- **THEN** the resulting AST MUST carry `columns: ["title", "body", "tags"]`

#### Scenario: FULLTEXT INDEX with IF NOT EXISTS

- **WHEN** the parser is given `CREATE FULLTEXT INDEX IF NOT EXISTS ft_idx ON t(body)`
- **THEN** the resulting AST MUST carry `if_not_exists: true`

### Requirement: Executor must reject FULLTEXT INDEX with a clear runtime error

The execution engine MUST dispatch `Statement::CreateFulltextIndex` to a runtime error path that returns `SqlError::ExecutionError` with a message identifying the limitation and pointing to the SQLite-FTS5 alternative. The error MUST NOT be a parse error and MUST NOT silently succeed.

#### Scenario: FULLTEXT INDEX executed via CLI

- **WHEN** a user pipes `CREATE TABLE t(body TEXT); CREATE FULLTEXT INDEX ft_idx ON t(body);` to `sqlrustgo-cli sqlite --batch --mode csv /tmp/db`
- **THEN** the parser MUST accept both statements (no `Parse error`), the executor MUST print a single stderr line containing the substring `FULLTEXT INDEX is not yet implemented`, and the process exit code MUST be `1`

### Requirement: Parser must produce a clear error on malformed FULLTEXT INDEX

The parser MUST still reject malformed FULLTEXT INDEX statements with a clear error message. The new AST node MUST NOT be a backdoor that accepts garbage.

#### Scenario: FULLTEXT INDEX missing name

- **WHEN** the parser is given `CREATE FULLTEXT INDEX ON t(body)` (no index name)
- **THEN** `parse(...)` MUST return `Err` containing the substring `"Expected fulltext index name"`

#### Scenario: FULLTEXT INDEX missing column list

- **WHEN** the parser is given `CREATE FULLTEXT INDEX ft_idx ON t` (no parenthesised columns)
- **THEN** `parse(...)` MUST return `Err` containing the substring `"("` (expected open paren for column list)
