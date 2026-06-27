# Proposal: mysql-server-canonical-entry

## Why

v3.8.0 has five overlapping execution entry points (`sqlrustgo` root
REPL stub, `sqlrustgo-sql-cli`, `sqlrustgo-bench`, `sqlrustgo-bench-cli`,
`sqlrustgo-tools`) plus the `sqlrustgo-mysql-server` wire-protocol
binary. The legacy binaries duplicate state, drift in behaviour, and
force humans and CI to know which to invoke. This change makes
`sqlrustgo-mysql-server` the single canonical entry point, with the
other binaries reduced to deprecation stubs that point to the
canonical subcommand.

## What Changes

- `sqlrustgo-mysql-server` gains six subcommands:
  - `serve` (default) — start the MySQL wire-protocol server
  - `exec "<sql>"` — execute a single SQL statement and print rows
  - `repl` — interactive REPL over stdin
  - `bench` — benchmark runner (full feature parity migrates in a
    follow-up; today it prints a helpful message)
  - `gmp` — GMP (AI Native) workflow (placeholder; follows up)
  - `diag` — diagnostics / catalog dump (placeholder; follows up)
- The five legacy binaries are reduced to deprecation stubs that
  print a clear migration message and exit 0.
- New `sqlrustgo_mysql_server::testing::start_ephemeral` keystone
  lets every integration test boot an in-process MySQL server on an
  OS-assigned port and tear it down on `Drop`. This is the
  foundation of the canonical test surface.
- New `tests/common/mod.rs::MySqlTestClient` raw-protocol MySQL
  client (no `mysql` crate dependency) gives tests full control over
  HandshakeResponse41 and avoids the `mysql` crate's SSL-negotiation
  surface.
- The first batch of integration tests (`show_tables_test`,
  `wire_protocol_smoke`, `embedded_harness_smoke`,
  `embedded_harness_isolation`, `l3_canonical_binary`) drive SQL
  through the wire protocol; a future PR migrates the remaining 27
  engine-internal tests in batches.

## Capabilities

### New Capabilities

- `mysql-server-canonical-entry`: One binary, six subcommands,
  retired legacy binaries. The canonical entry point for
  v3.8.0+ execution.
- `wire-protocol-execution`: Every e2e / perf test drives SQL
  through the MySQL wire protocol. The test harness provides a
  non-blocking in-process server with optional bootstrap
  (tables, users).
- `server-embedded-test-harness`: `start_ephemeral` is the
  canonical entry point for the integration-test surface; it
  returns an `EphemeralHandle` whose `Drop` cleans up the
  background thread and the temporary data directory.

### Modified Capabilities

(none — this change adds three new capabilities; no existing
spec is modified)

## Impact

- **Users**: a single binary to install, a single set of flags to
  learn. Existing scripts that invoke the legacy binaries keep
  working; the migration message points them at the canonical
  entry.
- **CI**: a single `cargo run -p sqlrustgo-mysql-server -- serve`
  replaces the patchwork of binary-specific CI invocations.
- **Tests**: the raw `MySqlTestClient` removes the `mysql` crate
  test dependency and makes the wire surface explicit. New
  `start_ephemeral` keystone gives tests a server they can boot
  and tear down per-test.
- **Risks**:
  - The legacy binaries still build but their crates' main.rs is
    a stub. The surrounding crate code (e.g. `crates/sql-cli/src/commands.rs`)
    is now unreferenced from the binary; cleaning that up is
    future work tracked outside this change.
  - Tests that use `MemoryExecutionEngine` directly are NOT
    migrated in this change; they are engine-internal unit tests
    that do not exercise the server surface. Migrating them adds
    no real coverage.
