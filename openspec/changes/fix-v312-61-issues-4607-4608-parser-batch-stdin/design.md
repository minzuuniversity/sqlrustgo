## Context

`sqlrustgo-cli` exposes a `sqlite --batch --mode csv` mode that reads SQL from stdin and feeds it line-by-line into `SqliteMode::execute_sql` → `engine.execute(sql)`. Two reproducible failures on HEAD `b14ad8df0` (develop/v3.12.0):

1. **#4607 standalone `--` comment line.** `run_batch_stdin_with_input` (crates/sqlrustgo-cli/src/sqlite_mode.rs:339) only checks `trimmed.is_empty()`. A line like `-- only a comment` is non-empty, so it is passed verbatim to `execute_sql`. The lexer (crates/parser/src/lexer.rs:53 `skip_whitespace`) consumes the whole `-- ...` sequence to EOF, leaving a single `Token::Eof`. The single-statement parser (crates/parser/src/parser.rs:11164 `parse`) then bails with `Parse error: Unexpected token: Eof`. The `.read` dot-command path (line 274) already filters `trimmed.starts_with("--")`; the batch-stdin path forgot to.

2. **#4608 multi-line `CREATE TABLE`.** Batch stdin reads `stdin().lock().lines()` (line 335) → a `Vec<String>` of physical lines. Each line is fed to `execute_sql` as an independent statement. A statement like
   ```sql
   CREATE TABLE t(
     id INT,
     x INT
   );
   ```
   becomes three `execute_sql` calls with payloads `CREATE TABLE t(`, `id INT,`, `x INT`, `);`. The first one is a partial CREATE TABLE — parser reports `Parse error: Unexpected token: Identifier("id")` (or whichever token the partial produces). The engine is otherwise capable of handling the full multi-line statement; the lexer treats `\n` as whitespace (crates/parser/src/lexer.rs:77 `!ch.is_whitespace()`), so a single multi-line `CREATE TABLE` parsed via `parse_statements` works.

Both root causes live entirely in the CLI layer. No engine, parser, lexer, storage, transaction, or executor change is required — the layer above the engine has to group lines into complete statements before invoking it.

The project already exposes `sqlrustgo_parser::split_sql_statements` (crates/parser/src/parser.rs:11263) which is the exact primitive we need: it splits a SQL string on `;` while respecting parentheses, square brackets, single/double quotes, line comments (`-- ...`), and block comments (`/* ... */`). Each fragment is trimmed and empty fragments are dropped. Pure-whitespace/comment input returns an empty `Vec`.

The new design becomes: join all input lines into a single buffer with `\n` separators, call `split_sql_statements(&buffer)`, and execute each fragment. A single fragment that is empty after trim (defensive guard against future `split_sql_statements` regressions) is skipped. This collapses both bugs to one place and reuses the parser's well-tested splitter.

## Goals / Non-Goals

**Goals:**
- Make `run_batch_stdin_with_input` accept standalone `--` comment lines without erroring.
- Make `run_batch_stdin_with_input` accept multi-line statements (CREATE TABLE columns on separate lines, multi-line INSERT VALUES, etc.).
- Make `.read <file>` consistent with batch stdin (same fix, same code path).
- Preserve error semantics: bad SQL still surfaces via `eprintln!` and aborts when `continue_on_error=false`.
- No new public API on the engine or parser.
- Stay under 60 lines of new code, all in `crates/sqlrustgo-cli/src/sqlite_mode.rs`.

**Non-Goals:**
- REPL multi-line continuation prompt behaviour (`.read` and `run_repl_with_input` get fixed for the line-iteration bug; REPL's own continuation contract stays as-is).
- Rewriting the lexer to skip `--` differently — it already skips correctly; the fix is above the engine.
- Adding a new top-level CLI subcommand.
- Any executor, planner, or storage changes.

## Decisions

### D1. Reuse `sqlrustgo_parser::split_sql_statements`

```
let joined = lines.iter().map(String::as_str).collect::<Vec<_>>().join("\n");
for stmt in sqlrustgo_parser::split_sql_statements(&joined):
    let trimmed = stmt.trim();
    if trimmed.is_empty(): continue
    dispatch(trimmed)
```

**Rationale:** `split_sql_statements` is the parser's official string-level splitter (crates/parser/src/parser.rs:11263). It handles every edge case the lexer recognises — parens, brackets, single/double quotes, line comments (`-- ...`), block comments (`/* ... */`), and backslash escapes. Empty fragments are dropped automatically. Reusing it gives us correct `;`-handling inside string literals and parenthesised lists for free, with zero new state-machine code in the CLI. The function is already used by the wire-protocol COM_QUERY handler (parser.rs:11253-11257), so its semantics are battle-tested.

**Alternatives considered:**
- Accumulate lines and split at first top-level `;` per dispatch with a hand-rolled state machine in the CLI: duplicates `split_sql_statements`, risk of drift if `parse_statements` evolves.
- Pre-rewrite each input by collapsing `-- ...\n` then `parse_statements` on the whole stdin: simpler code but loses per-statement error isolation (one bad statement should not abort the entire run when `continue_on_error=true`).
- `engine.execute(sql)` per logical statement group with the engine calling `parse_statements` internally: requires plumbing a new engine method; out of scope.

### D2. Trim fragments defensively

`split_sql_statements` already trims each fragment and drops empties, but we still skip `trimmed.is_empty()` defensively. This makes the loop robust against future changes to `split_sql_statements` and against any empty fragment a teaching-seed SQL could conceivably produce.

### D3. Extract a private helper `execute_batch_buffer(&mut self, sql: &str) -> Result<(), CliError>`

Wraps `execute_sql` so the batch-stdin logic and the `.read` logic share one place. Keeps `extract_columns` + `engine.execute` + post-DML flush semantics consistent across both paths. Avoids duplicating the trim/comment filter.

### D4. Keep REPL per-line behaviour

`run_repl_with_input` is left unchanged in its core dispatch (lines, not statements). REPL has its own continuation contract — this issue is explicitly about batch stdin (and `.read` for parity). We do add a single comment line explaining the intentional difference so future readers don't "fix" it.

### D5. No engine API change

The fix stays in `crates/sqlrustgo-cli/src/sqlite_mode.rs`. We do not add a new public method to `ExecutionEngine`. The CLI is a thin wrapper that is allowed to know about line-vs-statement boundaries.

## Risks / Trade-offs

- **Single dependency: `split_sql_statements`.** The fix relies on this parser function; if its semantics change, our batch behaviour changes with it. Mitigation: pin a `// Uses sqlrustgo_parser::split_sql_statements — see parser.rs:11263` comment so any future divergence is visible in code review.
- **`.read` behavioural change.** Users with hand-crafted `.sql` files that relied on per-line dispatch (e.g., they embedded a `-- comment` line followed by a partial statement on the next line) get the new behaviour. This is overwhelmingly the desired behaviour (matches mysql/sqlite), but the change is observable. Mitigation: call it out in `CURRENT_VERSION.md`.
- **Performance.** Buffering before dispatch costs one extra `String` allocation + one full-string trim + one linear pass through `split_sql_statements`. For typical teaching seeds (~hundreds of lines) this is negligible (sub-millisecond).
- **Error position reporting.** A parse error inside a multi-line statement now reports a position relative to the start of the accumulated buffer, not the original line. Users see `Parse error at offset 42: ...` instead of a file:line. Mitigation: out of scope for this fix; can be added later by annotating the error with the line range.
- **Empty stream guard.** A batch stdin input that is purely whitespace / comment lines returns an empty `Vec` from `split_sql_statements` and exits `EXIT_OK` (0), preserving the "no work to do" semantic.

## Verification Plan

- Unit tests in `crates/sqlrustgo-cli/src/sqlite_mode.rs` covering: standalone `--` comment line, blank-only input, multi-line `CREATE TABLE`, multi-line `INSERT VALUES`, mixed input, and a parse-error propagation case.
- Manual repro from the issues:
  ```bash
  printf -- '-- comment\nSELECT 1;\n' | sqlrustgo-cli sqlite --batch --mode csv /tmp/db   # exits 0
  printf '%s' 'CREATE TABLE t(\n  id INT,\n  x INT\n);\n' | sqlrustgo-cli sqlite --batch --mode csv /tmp/db   # exits 0
  ```
- `cargo build --all-features` green.
- `cargo test --package sqlrustgo-cli --all-features` green.
- `cargo clippy --all-features -- -D warnings` green.
