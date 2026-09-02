## ADDED Requirements

### Requirement: CLI batch stdin must skip standalone `--` comment lines

`sqlrustgo-cli sqlite --batch --mode csv` MUST treat a line whose trimmed content starts with `--` as a no-op when reading from stdin and from `.read <file>`. The CLI MUST NOT propagate such lines to the parser; doing so currently causes `Parse error: Unexpected token: Eof` because the lexer drops the comment and the parser then sees an empty token stream.

#### Scenario: stdin input is purely a single `--` comment line followed by `SELECT 1;`

- **WHEN** the user runs `printf -- '-- only a comment\nSELECT 1;\n' | sqlrustgo-cli sqlite --batch --mode csv /tmp/db`
- **THEN** the process MUST exit with status 0 and MUST execute `SELECT 1;` (no `Parse error` printed)

#### Scenario: `.read` of a SQL file containing standalone `--` comment lines

- **WHEN** the user runs `.read teaching-seed.sql` and that file contains `CREATE TABLE ...;` interleaved with `-- comment` lines
- **THEN** each `-- comment` line MUST be silently skipped and the surrounding statements MUST execute normally

### Requirement: CLI batch stdin must accept multi-line SQL statements

`sqlrustgo-cli sqlite --batch --mode csv` MUST buffer input until it has a complete SQL statement (terminated by a top-level `;` outside any parentheses or string literal) before invoking the engine. This covers `CREATE TABLE` with each column on its own line, multi-line `INSERT INTO ... VALUES (...)`, and any other construct that spans multiple physical input lines.

#### Scenario: stdin input is a multi-line `CREATE TABLE`

- **WHEN** the user runs `printf '%s' 'CREATE TABLE t(\n  id INT,\n  x INT\n);\n' | sqlrustgo-cli sqlite --batch --mode csv /tmp/db`
- **THEN** the process MUST exit with status 0 and MUST create the table `t(id INT, x INT)`; a subsequent `INSERT INTO t VALUES (1, 2);` followed by `SELECT * FROM t;` MUST return `(1, 2)`

#### Scenario: stdin input mixes blank lines, comment lines, single-line, and multi-line statements

- **WHEN** the input is a sequence such as:
  ```
  -- header comment

  CREATE TABLE products(
    id INT PRIMARY KEY,
    stock INT
);

  -- mid comment
  INSERT INTO products VALUES (1, 100);
  SELECT * FROM products;
  ```
- **THEN** the process MUST exit with status 0 and MUST create `products`, insert the row, and return `(1, 100)` for the final SELECT

### Requirement: CLI batch stdin must surface errors from multi-line statements

When a multi-line statement fails to parse, the CLI MUST surface the parser error to `stderr` and, when `continue_on_error` is false, MUST abort the batch with a non-zero exit code — preserving the existing fail-fast contract.

#### Scenario: multi-line CREATE TABLE with a syntax error inside

- **WHEN** the input is `CREATE TABLE t(\n  id INT,\n  stock BROKEN\n);` and `continue_on_error=false`
- **THEN** the CLI MUST print a `Parse error: ...` line on stderr and exit with a non-zero status; no partial CREATE TABLE MUST be created

### Requirement: CLI batch stdin must tolerate semicolons inside strings or parens

When splitting buffered input on `;`, the CLI MUST NOT treat a `;` inside `'...'` / `"..."` or inside `(...)` as a statement terminator. This matches the semantics of `parse_statements`.

#### Scenario: INSERT VALUES containing `;` inside a string literal

- **WHEN** the input is `INSERT INTO t VALUES (1, 'a;b');` as a single logical statement
- **THEN** the CLI MUST execute the INSERT as one statement; the `;` inside the string MUST NOT cause a premature split
