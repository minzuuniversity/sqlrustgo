# server-embedded-test-harness Specification

## Purpose
TBD - created by archiving change mysql-server-canonical-entry. Update Purpose after archive.
## Requirements
### Requirement: start_ephemeral contract

`start_ephemeral` MUST boot an in-process MySQL server and
return an `EphemeralHandle` for the test to drive. The function
SHALL bind a `TcpListener` on `config.host:0`, read the
OS-assigned port, spawn a background thread that runs
`run_server_with_listener_and_shutdown_with_bootstrap_and_tables`
with a non-blocking accept loop and a shared `Arc<AtomicBool>`
shutdown signal. When `config.bootstrap_users` is `true`, the
function MUST pre-create a `tester` user with password
`tester`; when `config.bootstrap_tables` is `true`, it MUST
pre-create the internal catalog tables `content`, `vectors`,
`documents`. The function MUST create a temporary data
directory under `std::env::temp_dir()` named
`sqlrustgo_ephemeral_{port}_{pid}` and return an
`EphemeralHandle` that holds the port, the shutdown flag, the
`JoinHandle`, and the data dir path.

- Bind a `TcpListener` on `config.host:0` and read the
  OS-assigned port.
- Spawn a background thread that runs
  `run_server_with_listener_and_shutdown_with_bootstrap_and_tables`
  with a non-blocking accept loop and a shared
  `Arc<AtomicBool>` shutdown signal.
- When `config.bootstrap_users` is `true`, pre-create a
  `tester` user with password `tester` via the bootstrap
  callback.
- When `config.bootstrap_tables` is `true`, pre-create the
  internal catalog tables `content`, `vectors`, `documents`.
- Create a temporary data directory under `std::env::temp_dir()`
  named `sqlrustgo_ephemeral_{port}_{pid}`.
- Return an `EphemeralHandle` that holds the port, the
  shutdown flag, the `JoinHandle`, and the data dir path.

#### Scenario: start_ephemeral returns a usable handle

- GIVEN a test calls `start_ephemeral(EphemeralConfig::default())`
- WHEN the function returns `Ok(handle)`
- THEN `handle.port` MUST be the OS-assigned port the server
  is listening on, and a `TcpStream::connect(("127.0.0.1",
  handle.port))` MUST succeed

### Requirement: EphemeralHandle Drop

`EphemeralHandle::drop` SHALL:

- Set the shutdown flag to `true` (memory order `SeqCst`).
- Join the server thread (bounded by the 50ms accept-loop
  poll interval).
- Remove the temporary data directory.

The drop SHALL be idempotent: a second `drop` is a no-op (the
shutdown flag, `JoinHandle`, and data dir are `Option` /
`Mutex`-wrapped).

#### Scenario: Drop joins the thread and cleans the data dir

- GIVEN a test has an `EphemeralHandle` in scope
- WHEN the test exits and the handle is dropped
- THEN the server thread SHALL exit within 50ms, the temporary
  data dir SHALL be removed, and the listener port SHALL be
  reusable for the next test

### Requirement: Detached handle for external servers

`EphemeralHandle::detached_for_external_server(port)` SHALL
return a no-op handle whose `Drop` is inert (does not touch
a shutdown flag, server thread, or data dir) — used by the L3
acceptance test that spawns `sqlrustgo-mysql-server serve` as
a subprocess and connects to it.

#### Scenario: connect_at synthesises a no-op handle

- GIVEN a test calls `MySqlTestClient::connect_at(addr, user,
  password)` against a subprocess server
- WHEN the test exits and the returned client is dropped
- THEN the drop MUST NOT attempt to clean up the subprocess
  server (it is owned by the subprocess handle in the test)

### Requirement: Public test-harness API
The `sqlrustgo-mysql-server` library MUST expose a public, stable, semver-pinned test-harness API:

```rust
pub mod testing {
    pub struct EphemeralServer { /* fields private */ }
    pub struct EphemeralHandle { pub port: u16, pub pid: u32 }
    pub fn start_ephemeral(config: EphemeralConfig) -> Result<EphemeralHandle, StartupError>;
}
```

`start_ephemeral` MUST bind an OS-assigned port (`port = 0`), fork an internal task (NOT a separate OS process), install a temporary `data_dir` (auto-cleaned on drop), and return the `EphemeralHandle` synchronously. This is the only sanctioned way for in-tree tests to obtain a working SQLRustGo server.

#### Scenario: Ephemeral handle is usable
- **WHEN** a test calls `start_ephemeral(EphemeralConfig::default())`
- **THEN** within 5 seconds the returned `EphemeralHandle.port` accepts a MySQL `COM_HANDSHAKE` and responds with the server's `SERVER_VERSION`

#### Scenario: Ephemeral data dir is isolated
- **WHEN** two tests run in parallel and each calls `start_ephemeral(...)`
- **THEN** each test sees only the data it wrote (no cross-test contamination), verified by writing `t = 1` in test A and `SELECT 1 FROM t` in test B returning zero rows

#### Scenario: Drop cleans up
- **WHEN** the `EphemeralServer` returned by `start_ephemeral` is dropped
- **THEN** the listener is closed within 1 second, the temporary data dir is removed, and no orphan processes remain

### Requirement: All in-tree tests use start_ephemeral
Every test file under `tests/` that exercises DDL, DML, transaction lifecycle, recovery, or any advanced subsystem MUST drive SQL through `start_ephemeral`. Direct `ExecutionEngine::execute(...)` calls are permitted only in `#[cfg(test)] mod tests` blocks inside `crates/*/src/**` for unit tests of the executor, planner, optimizer, and storage layers themselves.

#### Scenario: Test file inventory matches rule
- **WHEN** a CI check greps `tests/` for `ExecutionEngine::execute(` and for `start_ephemeral(`
- **THEN** every file that contains the former also contains the latter (or is in the explicit allow-list of crate-internal unit tests)

#### Scenario: Existing in-process test rewritten
- **WHEN** a test that previously called `ExecutionEngine::with_wal_file(...)` and then `engine.execute(...)` is migrated
- **THEN** the new test calls `start_ephemeral(...)`, returns a `MySqlConnection`, and sends the SQL over the wire; the test's `cargo test` invocation succeeds and produces the same logical assertions

### Requirement: Test-harness stability across feature flags
The `testing` module MUST be compiled in both the default and `advanced` feature configurations. A test that calls `start_ephemeral` MUST compile under both `cargo test -p sqlrustgo-mysql-server` and `cargo test -p sqlrustgo-mysql-server --features advanced`.

#### Scenario: Default build exports testing
- **WHEN** `cargo test -p sqlrustgo-mysql-server --no-run` is run with no features
- **THEN** the test binary links and references the `testing::start_ephemeral` symbol

#### Scenario: Advanced build also exports testing
- **WHEN** the same command is run with `--features advanced`
- **THEN** the test binary still links and references the symbol

