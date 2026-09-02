## 1. Refactor batch dispatch path

- [ ] 1.1 Rewrite `SqliteMode::run_batch_stdin_with_input` to join all input lines with `\n` into one buffer, call `sqlrustgo_parser::split_sql_statements` (crates/parser/src/parser.rs:11263) on the buffer, and dispatch each non-empty trimmed fragment via `execute_sql`.
- [ ] 1.2 Rewrite `SqliteMode::execute_dotcmd` `DotCmd::Read` to read the whole file content (already done) and apply the same `split_sql_statements` dispatch instead of per-line `execute_sql`.
- [ ] 1.3 Add a `// Uses sqlrustgo_parser::split_sql_statements — see parser.rs:11263` comment above the dispatch loop so future divergence from the parser is visible in code review.

## 2. Tests

- [ ] 2.1 Add `run_batch_stdin_with_only_comment_line_succeeds` to `crates/sqlrustgo-cli/src/sqlite_mode.rs` `#[cfg(test)] mod tests` covering: input = `["-- only a comment", "SELECT 1"]`, expect exit=0 and a row produced.
- [ ] 2.2 Add `run_batch_stdin_with_multiline_create_table_succeeds` covering: multi-line `CREATE TABLE t(\n  id INT PRIMARY KEY,\n  stock INT\n);` plus an INSERT and a SELECT, all via the new accumulator.
- [ ] 2.3 Add `run_batch_stdin_with_mixed_comments_and_multiline_succeeds` covering the full teaching-seed pattern: blank lines, `-- comment` lines, single-line statements, multi-line statements interleaved.
- [ ] 2.4 Add `run_batch_stdin_multiline_syntax_error_aborts` covering a multi-line statement with a parser error inside; expect exit=1 when `continue_on_error=false` and the partial table must not exist.
- [ ] 2.5 Add `run_batch_stdin_semicolon_inside_string_not_split` covering `INSERT INTO t VALUES (1, 'a;b');` followed by `SELECT * FROM t;` — verify the `;` inside the literal does not split the input prematurely.
- [ ] 2.6 Add `read_dotcmd_with_multiline_create_table_succeeds` covering `.read`-style multi-line CREATE TABLE (the same root cause applies to the `.read` path).

## 3. Documentation and verification

- [ ] 3.1 Add a one-paragraph note to `CURRENT_VERSION.md` documenting the CLI batch-stdin fix and referencing issues #4607 and #4608.
- [ ] 3.2 Run `cargo build --all-features` and confirm clean build.
- [ ] 3.3 Run `cargo test --package sqlrustgo-cli --all-features` and confirm all new + existing tests pass.
- [ ] 3.4 Run `cargo clippy --all-features -- -D warnings` on the affected package.
- [ ] 3.5 Reproduce both issue scenarios via `printf ... | sqlrustgo-cli sqlite --batch --mode csv /tmp/db` and confirm exit 0.
