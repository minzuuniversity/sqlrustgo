# Spec: server-embedded-test-harness

## ADDED Requirements

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
