# Design: mysql-server-canonical-entry

## Decisions

### D1. Subcommand shape: `serve` (default) + 5 named subcommands

`clap` `Subcommand` enum with `serve` as the default (when no
subcommand is given). Reasons:

- Backward-compatible — `sqlrustgo-mysql-server --host … --port …`
  still starts the wire-protocol server.
- Easy to extend — adding `bench` / `gmp` / `diag` later is just
  another enum variant, no CLI-parser rewrite.
- Discoverable — `sqlrustgo-mysql-server --help` lists every
  entry point.

### D2. Raw-protocol test client, not the `mysql` crate

`mysql` crate 25.x's default transport is partially TLS-aware
(it tries SSL when the server advertises the SSL capability) and
its URL parameter surface shifts between major versions. A raw
client (HandshakeResponse41 + COM_QUERY + result-set decoder)
gives us full control, no SSL upgrade path, and no test-binary
dependency on the `mysql` crate. The math for
`mysql_native_password` is implemented inline with the `sha1`
crate (~30 lines).

### D3. Test harness is a public `testing` module, not a separate crate

`start_ephemeral`, `EphemeralConfig`, `EphemeralHandle` live under
`sqlrustgo_mysql_server::testing` so integration tests can use
them via the existing `[dev-dependencies] sqlrustgo-mysql-server`
edge without adding a new crate. The module is gated behind a
`pub mod testing` so a downstream consumer can disable it by
patching if they want.

### D4. Non-blocking accept loop with `Arc<AtomicBool>` shutdown

The test harness needs to bring the server down on `Drop` without
a client having to connect. A blocking `accept()` would hang;
`libc::SO_RCVTIMEO` adds a per-syscall cost on every connection.
`listener.set_nonblocking(true)` + a 50ms `WouldBlock` sleep is
the simplest solution; the latency is invisible to production
clients because every accept immediately tries again when the
syscall returns.

A latent bug was uncovered during this work: on macOS, an accepted
stream from a non-blocking listener **inherits** the non-blocking
flag. `handle_connection` now calls `stream.set_nonblocking(false)`
explicitly at the top so `set_read_timeout` actually governs the
wait. Without this fix, every client that sends data gets
`EAGAIN` from the server's read.

### D5. Bootstrap split into two flags

`EphemeralConfig` exposes `bootstrap_tables: bool` and
`bootstrap_users: bool` (both default `true`). Tests that need
a clean catalog set `bootstrap_tables: false` and keep
`bootstrap_users: true` so authentication still works.
Originally a single `bootstrap: bool` flag was tried; it could
not satisfy both "clean catalog" and "auth works" at once, so
the flag was split.

### D6. Legacy binaries are deprecation stubs, not deletions

The five legacy binaries' `main()` is replaced with a stub that
prints a clear migration message and exits 0. The surrounding
crate code (e.g. `crates/sql-cli/src/commands.rs`) remains so
the crate still compiles, but it is no longer reachable from the
binary. This is the smallest disruptive change that satisfies
"consolidate" — full deletion can come after the canonical
binary grows full feature parity.

### D7. `MySqlTestClient::connect_at` for out-of-process servers

The L3 acceptance test spawns the compiled binary as a subprocess
and connects to it. `connect_at(addr, user, password)` reuses the
handshake + auth + COM_QUERY helpers and synthesises a
no-op `EphemeralHandle` (via
`EphemeralHandle::detached_for_external_server`) so the client's
`Drop` does not try to clean up state it does not own.

## Risks

- **R1**: The deprecation stubs leave ~700 lines of dead code in
  the legacy crate src trees. The follow-up is to delete that
  code once the canonical binary's `bench` / `gmp` / `diag`
  subcommands reach feature parity.
- **R2**: The raw-protocol client has only the surface we test
  (DDL/DML, COM_QUERY result sets, no prepared statements). A
  test that needs `caching_sha2_password` or prepared statements
  will need a feature extension.
- **R3**: macOS-inherited non-blocking on accepted streams is a
  pre-existing bug surfaced by this work; other production code
  paths that bypass `start_ephemeral` (e.g. `run_server(host,
  port)`) do not hit the bug because they use the original
  `run_server` code path that does not call `set_nonblocking`.

## Migration Plan

1. Land `start_ephemeral` keystone (Phase 1, merged).
2. Land raw MySQL test client + first batch of migrated tests
   (Phase 2a, merged).
3. Land subcommand CLI on the canonical binary (Phase 3, merged).
4. Deprecate legacy binaries (Phase 5, merged).
5. Add `CHANGELOG` entry (Phase 6, merged).
6. L3 acceptance via out-of-process spawn (Phase 7, merged).
7. Archive this change (Phase 8).
