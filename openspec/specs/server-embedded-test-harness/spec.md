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

