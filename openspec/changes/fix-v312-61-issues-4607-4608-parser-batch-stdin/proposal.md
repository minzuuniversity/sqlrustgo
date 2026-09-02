## Why

`sqlrustgo-cli sqlite --batch --mode csv` (HEAD `b14ad8df0`, develop/v3.12.0) fails on two realistic BustubX-EDU `teaching-seed.sql` patterns when fed line-by-line through batch stdin:

1. **#4607** — Standalone `--` comment lines produce `Parse error: Unexpected token: Eof`. The line is not empty, so the CLI hands it to `engine.execute`; the lexer drops the whole line, leaving only `Token::Eof`, and the single-statement parser chokes.
2. **#4608** — Multi-line `CREATE TABLE` definitions (each column on its own line) produce `Parse error: Unexpected token: Identifier("id")`. The CLI splits on `\n` and feeds each line to `engine.execute` as a separate statement; the parser then sees a partial `CREATE TABLE`.

These block the BustubX-EDU verification flow (`tools/sqlrustgo-verification` Part A) and force users to manually rewrite their SQL before loading. Both are CLI-only path bugs; the lexer, single-statement parser, and `engine.execute` itself already handle the inputs correctly when given as one full string.

## What Changes

- **CLI batch stdin path** (`SqliteMode::run_batch_stdin_with_input`): change the per-line handler to skip lines that are empty *or* start with `--` (issue #4607), and to accumulate consecutive lines until the buffer contains a complete SQL statement (terminated by `;` outside string literals and parens, including detection of empty buffer from comment-only input) before invoking `engine.execute` (issue #4608). The accumulator must respect single/double-quoted strings and balanced parentheses, so multi-line `CREATE TABLE t(\n  id INT,\n  x INT\n);` and other multi-line constructs (`INSERT INTO ... VALUES (1, 'x');`, BEGIN/COMMIT blocks, `CASE WHEN ... END`) parse as one statement.
- **CLI `.read` path** (`SqliteMode::execute_dotcmd` `DotCmd::Read`): apply the same fix so `.read teaching-seed.sql` works (currently still splits per-line). This is required for completeness — `.read` shares the same root cause and is invoked by the same verification flow.
- **CLI REPL per-line path** (`SqliteMode::run_repl_with_input`): leave the line-by-line dispatch alone for now — REPL behaviour is intentional (REPL traditionally executes each line as it sees Enter, and multi-line input is delivered via the SQLite-style continuation prompt, which is out of scope for this issue). Document this in code comments.
- **New helper** `SqliteMode::execute_batch_buffer(buffer: &str) -> Result<(), CliError>` that handles the case where `engine.execute` rejects an empty/empty-after-lex token stream by returning `Ok(())` instead of bubbling `Unexpected token: Eof`. This covers edge cases like `-- comment` lines and whitespace-only inputs that survive the buffer accumulator.
- **Tests** in `crates/sqlrustgo-cli/src/sqlite_mode.rs` covering:
  - Standalone `--` comment line at start, middle, end of input.
  - `--` line followed by `SELECT 1;` (must execute successfully).
  - Multi-line `CREATE TABLE` with each column on its own line.
  - Multi-line `INSERT INTO ... VALUES (...)` with newlines.
  - Mix of comment-only, blank, single-line, and multi-line statements in one batch.
  - Error path: bad SQL inside multi-line statement still surfaces to the user and aborts (unless `continue_on_error`).
- **Update** `CURRENT_VERSION.md` GA blocker section noting both issues resolved at the CLI batch stdin boundary.

No engine, parser, or lexer changes. No new public API. No storage or WAL changes. No breaking changes.

## Capabilities

### New Capabilities

- `cli-batch-stdin-multiline-and-comments`: CLI batch stdin must accept (a) standalone `--` comment lines and (b) multi-line SQL statements (CREATE TABLE with each column on its own line, etc.) by accumulating input until a top-level `;` terminator before invoking the engine. This is a CLI-internal contract; downstream consumers of the parser API are unaffected.

### Modified Capabilities

- None. Existing parser/lexer/executor specs continue to describe their pre-existing behaviour unchanged. The CLI fixes are an isolated layer above them.
