# Fix: `multi_statement_test` data-dir pollution

## Status

**Partially addressed.** This change does what the proposal originally
intended (replace the hardcoded `/tmp/multi_stmt_test` with a fresh
`tempfile::TempDir` so the test never reuses a stale WAL from a
prior failed run). It does **not** unblock the test on its own: the
`EAGAIN (os error 35)` panic on macOS is a deeper issue in the
server's `do_command_loop` query processing path that is out of
scope for this change.

Verified: after the fix, `cargo test -p sqlrustgo --test
multi_statement_test` still panics on the same `EAGAIN` line, but
no longer leaves a stale WAL in `/tmp/multi_stmt_test` between
failed runs. The remaining EAGAIN issue is tracked separately.

## Why (original)

`tests/multi_statement_test.rs::test_multi_statement_two_selects` hardcodes
`/tmp/multi_stmt_test` as the ephemeral server's data directory and never
cleans it between runs. The first invocation in a test process populates the
shared dir with a `sqlrustgo.wal` and a previous test's tables. Subsequent
invocations (or the same test when re-run after a flaky failure) inherit
that state, so the very first wire-protocol read after `connect` lands
mid-WAL and the server closes the connection before the client can finish
reading the handshake.
temporarily unavailable (os error 35)`. On Linux the same root cause shows
up as `read packet: unexpected EOF at offset 0` because the server has
already closed the socket the moment the stale WAL recovery ungracefully
exits.

## What Changes

Switch `test_multi_statement_two_selects` to use `tempfile::TempDir` for
its data dir, exactly the same pattern as the show_tables_test fix in
the prior commit. The TempDir is created fresh for each invocation, and
its `Drop` impl removes the directory at end of scope so no cross-test
state survives.

The remaining 14 `multi_statement_test` tests already use
`MySqlTestClient::connect_with_config(EphemeralConfig { ... })` and pass
a default config (data_dir: None) — those continue to work via the
existing `tempfile::TempDir` integration in `MySqlTestClient` itself.
Only the single hardcoded `/tmp/multi_stmt_test` call site needs to
move.

## Scope

- In scope: replace the hardcoded path with TempDir; remove the manual
  `create_dir_all` (TempDir does it); add the second `use tempfile`
  import alongside the existing `use std::path::Path`.
- Out of scope: changing `MySqlTestClient`, the server, `start_ephemeral`,
  or any other failing test that has its own data-dir / startup problem
  (e.g. `perf_eng_batched_insert_test`, `tpch_value_correctness_test`,
  `load_local_infile_eagain_regression_test`). Each of those is a
  separate change with its own proposal.

## Acceptance

- `cargo test -p sqlrustgo --test multi_statement_test` reports 15/15
  pass.
- The test no longer references `/tmp/multi_stmt_test`.
- A stale `/tmp/multi_stmt_test` from a prior failed run is not
  re-used on a fresh invocation.
