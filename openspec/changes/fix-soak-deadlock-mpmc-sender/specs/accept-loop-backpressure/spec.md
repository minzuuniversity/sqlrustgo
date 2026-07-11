## ADDED Requirements

### Requirement: Accept loop remains responsive to graceful shutdown under backpressure

The MySQL server accept loop SHALL wake at least every `accept_send_timeout_ms` (default 200 ms) when handing work to the worker pool, so the `shutdown` flag is polled even when the worker channel is full.

#### Scenario: Accept loop wakes under full worker channel
- **WHEN** the worker pool's bounded channel is full and `tx.send_timeout(job, 200ms)` returns `Err(Timeout)`
- **THEN** the accept loop sleeps for 50 ms (existing WouldBlock path) and re-tries, ensuring it polls the `shutdown` flag within at most 250 ms total

#### Scenario: Graceful shutdown unblocks within timeout
- **WHEN** `shutdown.load()` returns `true` while the accept loop is blocked in `send_timeout`
- **THEN** the next accept iteration (≤ 200 ms later) exits the accept loop and the server process can terminate cleanly

### Requirement: Worker channel buffer absorbs typical query bursts

The `ServerThreadPool` SHALL size its bounded channel to `server_threads * CHANNEL_BUFFER_MULTIPLIER` slots where `CHANNEL_BUFFER_MULTIPLIER = 4`.

#### Scenario: Default channel capacity is 64
- **WHEN** `ServerThreadPool::start(16)` is called (the default)
- **THEN** the underlying `sync_channel` has capacity `16 * 4 = 64`

### Requirement: Regression test detects accept-loop deadlock

A `tests/mixed_workload_deadlock_regression.rs` test SHALL run a 30-second mixed TPC-H + CRUD workload against an ephemeral server with `server_threads=4` and assert that the server still accepts new connections after the soak.

#### Scenario: Post-soak connection succeeds
- **WHEN** the soak finishes (30 seconds elapsed)
- **THEN** a new client `SELECT 1+1` probe succeeds within 5 seconds, proving the accept loop is alive

#### Scenario: Soak generates backpressure
- **WHEN** the test runs with 12 query threads + 4 CRUD threads against `server_threads=4`
- **THEN** at least one `send_timeout` timeout fires during the 30-second soak (assertable via a counter exposed on `ServerThreadPool` or a test-only metric)

### Requirement: IR VALIDATION warning is silent in production logs

The IR vs legacy predicate mismatch check in `src/execution_engine.rs` SHALL log via `tracing::debug!` rather than `eprintln!`, so production logs are not flooded under SOAK load.

#### Scenario: Predicate mismatch goes to debug log
- **WHEN** the legacy and IR predicates disagree on an UPDATE
- **THEN** a `tracing::debug!` event is emitted; no `stderr` output occurs at default log level

#### Scenario: Debug log discoverable when needed
- **WHEN** `RUST_LOG=sqlrustgo::execution_engine=debug` is set
- **THEN** the predicate mismatch is visible in stderr