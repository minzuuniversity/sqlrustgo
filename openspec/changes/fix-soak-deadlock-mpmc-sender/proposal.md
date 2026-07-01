## Why

SOAK testing of `sqlrustgo-mysql-server` under mixed TPC-H workload (Q1-Q22 + CRUD on 5 tables, 80/20 ratio, 4-32 dynamic threads, ~30 minutes wall-clock) reproduces a **complete server deadlock**: the accept loop blocks indefinitely in `std::sync::mpmc::Sender::send` because all worker threads are busy executing `handle_connection`, and the bounded `sync_channel` (capacity = N*2 = 32 for default 16 workers) is full. Meanwhile 28+ client connections accumulate as `SYN_SENT`, the server stops accepting new TCP connections (accept loop is blocked), and the only escape (graceful shutdown via `shutdown` flag) is unreachable because the main thread is parked in `Sender::send`.

Sample evidence (PID 5915, port 3397, 30 min after start):
```
sqlrustgo_mysql_server::run_server_with_listener_and_shutdown_with_bootstrap_tables_and_sql
  → std::sync::mpmc::Sender::send          (100% time)
  → mpmc::array::Channel::send
  → std::thread::Thread::park
  → _dispatch_semaphore_wait_slow
  → semaphore_wait_trap
```

This is **a real production bug** that any long-running deployment will eventually hit — the SOAK test surfaced it. The current `server_threads=16` with `CHANNEL_BUFFER_MULTIPLIER=2` is insufficient for the workload's burst patterns.

## What Changes

- **Add bounded accept backpressure** in the accept loop: when the worker pool is full and `tx.send` would block for > N ms, **temporarily stop accepting new TCP connections** (yield the accept loop with the existing 50ms sleep) instead of parking indefinitely. This keeps the main thread responsive to the `shutdown` flag and to client retries.
- **Add accept-loop liveness timeout**: the accept loop must wake at least every `server_threads * 4` ms (default 64 ms) to check `shutdown`. Currently the only wake-up is `set_nonblocking(true)` + `WouldBlock` 50ms sleep, which works only when no client connects; under heavy backpressure the loop blocks in `send()` and never polls shutdown.
- **Increase channel capacity under default**: change `CHANNEL_BUFFER_MULTIPLIER` from `2` to `4` so the buffer = `N * 4 = 64` for default 16 workers, accommodating bursty TPC-H queries that take > 2 worker-rounds to complete.
- **Add `IR VALIDATION` warning demotion**: the `eprintln!` in `src/execution_engine.rs:2611` (legacy vs IR predicate mismatch) is a debug validation that fires per-update; under SOAK it floods stderr. Demote to `tracing::debug!` so production logs stay readable.
- **Add a SOAK regression test**: a `tests/` integration test that runs mixed Q1-Q22 + CRUD for 60 seconds against an ephemeral server and asserts the server is still accepting new connections at the end (no deadlock).

## Capabilities

### New Capabilities

- `accept-loop-backpressure`: The MySQL server's accept loop SHALL remain responsive to graceful shutdown and shall not park indefinitely waiting on a full worker queue.

### Modified Capabilities

_None — no spec-level behavioral changes elsewhere._

## Impact

**Files modified:**
- `crates/mysql-server/src/lib.rs`
  - `run_server_with_listener_and_shutdown_with_bootstrap_tables_and_sql` (line 3167): add send timeout, increase channel multiplier
  - `ServerThreadPool::start` (line 4310): increase `CHANNEL_BUFFER_MULTIPLIER`
- `src/execution_engine.rs` (line 2611): demote `eprintln!` → `tracing::debug!`

**Files added:**
- `tests/mixed_workload_deadlock_regression.rs` — 60s SOAK regression that exercises Q1-Q22 + CRUD and asserts server liveness

**Performance impact:**
- Backpressure timeout adds a single `Instant::now()` check per accepted connection (negligible)
- Channel capacity 32 → 64 uses ~32 * sizeof(ServerJob) = ~16 KB more heap per server instance

**Compatibility:**
- Existing `EphemeralConfig::server_threads` default unchanged (16)
- No public API break

**Risk:**
- Adding accept timeout means under sustained overload, new connections get **rejected at the TCP layer** (kernel still completes the SYN/ACK then drops) instead of being queued forever. This is the correct production behavior — overloaded servers should fail fast, not appear alive.