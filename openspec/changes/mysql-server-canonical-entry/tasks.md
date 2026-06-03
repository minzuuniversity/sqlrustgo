# Tasks: mysql-server-canonical-entry

## Phase 1 — Keystone (start_ephemeral) [merged in PR #2864]

- [x] Add `pub mod testing` in `crates/mysql-server/src/lib.rs`
- [x] `EphemeralConfig` (host) + `EphemeralHandle` (port, shutdown, join, data_dir)
- [x] `start_ephemeral(config) -> Result<EphemeralHandle, io::Error>`
- [x] `run_server_with_listener(listener)` extracted
- [x] `run_server_with_listener_and_shutdown(listener, shutdown)` with non-blocking accept
- [x] `tests/embedded_harness_smoke.rs` GREEN (HandshakeV10 returns "8.0.33-SQLRustGo")
- [x] `tests/embedded_harness_isolation.rs` GREEN (two parallel ephemerals get distinct ports)

## Phase 2a — Raw protocol test client + first batch [merged in PR #2903]

- [x] `tests/common/mod.rs::MySqlTestClient` raw protocol
- [x] HandshakeV10 parser with scramble extraction
- [x] `mysql_native_password` auth response computation
- [x] HandshakeResponse41 builder (SECURE_CONNECTION path)
- [x] COM_QUERY send + result set parse (col count, column defs, EOF, rows, terminator)
- [x] `tests/wire_protocol_smoke.rs` RED→GREEN
- [x] Migrate `tests/show_tables_test.rs` (4/4 GREEN)
- [x] Fix `handle_connection` non-blocking-inheritance bug
- [x] Split `EphemeralConfig.bootstrap` into `bootstrap_tables` / `bootstrap_users`

## Phase 3 — Canonical binary subcommands [merged in PR #2904]

- [x] clap `Subcommand` enum with `serve` / `exec` / `repl` / `bench` / `gmp` / `diag`
- [x] `serve` (default) wires to `run_server`
- [x] `exec "<sql>"` uses in-process `ExecutionEngine` with `MemoryStorage`
- [x] `repl` is a basic stdin/stdout loop (rustyline parity in follow-up)
- [x] `bench` / `gmp` / `diag` print helpful "follow-up" message and exit 2

## Phase 5 — Deprecate legacy binaries [merged in PR #2905]

- [x] `sqlrustgo` (root) — deprecation stub
- [x] `sqlrustgo-sql-cli` — deprecation stub
- [x] `sqlrustgo-bench` — deprecation stub
- [x] `sqlrustgo-bench-cli` — deprecation stub
- [x] `sqlrustgo-tools` — deprecation stub
- [x] Tighten `run_server_with_listener_and_shutdown_with_bootstrap(_and_tables)` to `pub(crate)`

## Phase 6 — Changelog entry [merged in PR #2906]

- [x] Add three entries to v3.8.0 / 核心功能 in `CHANGELOG.md`
- [x] `bash scripts/gate/check_docs_links.sh` PASS
- [x] `bash scripts/gate/check_docs_consistency.sh` PASS

## Phase 7 — L3 acceptance [merged in PR #2907]

- [x] `tests/l3_canonical_binary.rs` spawns the compiled binary as a subprocess
- [x] Poll TCP listener until ready
- [x] `MySqlTestClient::connect_at(addr, user, password)`
- [x] `EphemeralHandle::detached_for_external_server(port)` no-op handle
- [x] Auth as `root` (built-in user, no password)
- [x] DDL + DML + DQL round-trip GREEN

## Phase 8 — Archive (this PR)

- [x] Add `proposal.md` / `design.md` / `tasks.md`
- [x] Add three capability spec files with `## ADDED Requirements` deltas
- [x] `openspec validate --strict --type change mysql-server-canonical-entry` PASS
- [x] Final PR (this one)
- [x] `openspec archive mysql-server-canonical-entry` after merge

## Future work (tracked outside this change)

- [ ] Migrate remaining ~27 engine-internal integration tests
      (`tests/boundary_test.rs`, `tests/cbo_integration_test.rs`,
      `tests/change_buffer_test.rs`, etc.) to the wire surface —
      OR keep them in-process if they test engine internals
      unrelated to the server.
- [ ] rustyline + history + meta-commands in `sqlrustgo-mysql-server repl`
- [ ] Full `bench` feature parity in the canonical binary
- [ ] `gmp` subcommand wiring
- [ ] `diag` subcommand wiring
- [ ] Delete the dead `src/commands.rs` and `src/main.rs` trees
      in `crates/sql-cli`, `crates/bench`, `crates/bench-cli`,
      `crates/tools` once the canonical binary has full feature
      parity
