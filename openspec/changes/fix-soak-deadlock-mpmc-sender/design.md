## Context

The MySQL server uses a thread pool (`ServerThreadPool`, 16 workers by default) with a bounded `sync_channel(N*2)` to hand accepted TCP connections to worker threads. The accept loop in `run_server_with_listener_and_shutdown_with_bootstrap_tables_and_sql` (crates/mysql-server/src/lib.rs:3330-3368) calls `p.send(job)` on every accept. `SyncSender::send` blocks the caller (the accept loop) when the channel is full, which happens whenever all workers are busy AND the buffer of N*2 jobs is also queued.

Under SOAK load (TPC-H Q1-Q22 + CRUD, 80/20 ratio, 4-32 dynamic client threads, 30+ minutes wall-clock), every worker eventually holds a long-running query (Q1/Q3 aggregate over millions of lineitem rows + concurrent writes holding WAL/storage locks). The 32-job buffer fills, the accept loop blocks in `mpmc::Sender::send`, and:

1. The accept loop is no longer polling the `shutdown` flag → graceful shutdown is unreachable
2. The listener is still `LISTEN`-ing on its socket, but the kernel accept queue fills up too (default `kern.ipc.somaxconn=128` on macOS) → new client SYNs time out with `ETIMEDOUT (60)` instead of being rejected fast
3. The only escape is killing the process via SIGKILL/SIGTERM

This is a **classic producer-consumer deadlock**: producer (accept loop) blocks on a channel that only the consumers (workers) can drain, but the consumers are stuck on a shared resource (WAL/storage lock) that no one is releasing.

A separate but related issue: `src/execution_engine.rs:2611` has an `eprintln!("[IR VALIDATION] Predicate mismatch: ...")`. This is a per-UPDATE debug validation that fires every time legacy vs IR plan predicate evaluation disagrees. Under SOAK it spams stderr at hundreds of lines per second, drowning the actual error log.

## Goals / Non-Goals

**Goals:**
- Accept loop stays responsive to the `shutdown` flag under any backpressure
- Channel buffer sized to absorb typical query bursts (default N=16, buffer ≥ 64)
- Server logs stay readable under load (no debug spam)
- A regression test catches this deadlock class before it ships

**Non-Goals:**
- Changing the worker model (still N workers + MPSC; not switching to async/Tokio)
- Fixing Q1/Q3 query performance (separate concern, may need an index)
- Implementing connection admission control or rate limiting (out of scope)
- Changing the `EphemeralConfig::server_threads` default value

## Decisions

### 1. Bounded accept send with timeout, not unbounded

**Decision**: Replace `p.send(job)` with `p.send_timeout(job, Duration::from_millis(200))`. If the timeout fires, the accept loop sleeps for 50ms (existing `WouldBlock` path) before retrying.

**Rationale**: 
- A bounded timeout (200 ms) means the accept loop wakes at least every 200 ms regardless of queue depth, so `shutdown` is always polled
- 200 ms is short enough to look "responsive" to a TCP client (kernel retransmit timers are 200-1000 ms) but long enough that a brief burst (e.g., 32-query reconnect wave) doesn't immediately reject
- The kernel will queue incoming SYNs up to `somaxconn=128`; once that's full, clients see `ETIMEDOUT` either way. Rejecting from our side is no worse for clients but much better for the operator (the server still responds to `kill` and `--shutdown`)

**Alternatives considered**:
- Unbounded channel (`mpsc::channel()`) — rejected: hides backpressure, allows unbounded memory growth under sustained overload
- `try_send` + drop connection immediately — rejected: too aggressive (rejects under any temporary spike)
- Async accept (Tokio) — rejected: too large a refactor for this targeted fix

### 2. Channel multiplier 2 → 4

**Decision**: Change `CHANNEL_BUFFER_MULTIPLIER` from `2` to `4` in `ServerThreadPool`.

**Rationale**: 
- Default `N=16` → buffer was 32, now 64
- Each `ServerJob` is ~250 bytes (stream + Arc<RwLock<...>> + Arc<rustls::ServerConfig> + UserStore), so 64 slots ≈ 16 KB heap — negligible
- 64 slots = 4 worker-rounds of buffering; sufficient for TPC-H burst patterns where one query takes 4× as long as another
- Combined with `send_timeout(200ms)`, this means: queue absorbs 4 worker-rounds of work, and any longer spike is backpressured but never deadlocked

### 3. Demote IR VALIDATION warning to `tracing::debug!`

**Decision**: Replace `eprintln!("[IR VALIDATION] Predicate mismatch: legacy={}, ir={}", ...)` with `tracing::debug!("IR predicate mismatch: legacy={}, ir={}", ...)`.

**Rationale**: This validation runs on every UPDATE to catch regressions in the new IR-based planner. It is not a production error; it's an invariant check for development. `tracing::debug!` keeps it discoverable via `RUST_LOG=debug` without polluting production logs.

**Note**: This is a separate but co-located change. We could leave it for a follow-up PR, but the SOAK reports it as noise, and the fix is one line.

### 4. Regression test placement

**Decision**: Add `tests/mixed_workload_deadlock_regression.rs` that:
1. Spawns an ephemeral server with `EphemeralConfig::server_threads=4` (small for fast test)
2. Loads TPC-H SF=0.001 fixture
3. Spawns 16 client threads: 12 query threads (Q1, Q3, Q4, Q6, Q10, Q12, Q14, Q15, Q17, Q19, Q20, Q22 round-robin) + 4 CRUD threads (INSERT/UPDATE/DELETE on lineitem)
4. Runs for 30 seconds
5. Asserts: at least one `SELECT 1+1` succeeds **after** the 30-second soak, proving the accept loop is still serving

**Rationale**: 
- 30 seconds is enough to reproduce the deadlock class without making CI slow (current SOAK is 30 min, too long for unit tests)
- 12+4 threads > server_threads=4 forces backpressure on the channel, which is the precondition for the deadlock
- The post-soak `SELECT 1+1` is a binary assertion (passes = accept loop alive, fails = deadlock)

## Risks / Trade-offs

| Risk | Mitigation |
|------|-----------|
| 200 ms timeout too aggressive — legitimate burst connections rejected | Tunable via env var `SQLRUSTGO_ACCEPT_SEND_TIMEOUT_MS` |
| 200 ms timeout too lenient — accept loop unresponsive in slow environments | Default 200 ms; CI runs the regression test to catch regressions |
| Channel buffer 64 still insufficient under pathological load | `send_timeout` ensures the accept loop never blocks indefinitely regardless of buffer size |
| Demoting IR VALIDATION hides a real planner regression | `tracing::debug!` keeps it discoverable; CI runs with `RUST_LOG=debug` for planner tests |
| Regression test flaky if CI is slow | Use relative timing (30s soak + 5s post-soak probe) and retry on transient failures |

## Migration Plan

1. Land the fix as a single PR (commit 1: lib.rs changes; commit 2: execution_engine.rs demotion; commit 3: regression test)
2. Run the SOAK test on Z6G4 again (1h) to verify the deadlock no longer reproduces
3. Run the new regression test in CI (fast, deterministic)
4. Run the full G7 gate (`scripts/gate/check_p13_soak_test.sh`) — must still pass 11/11
5. No data migration or rolling restart concerns (internal change)

## Open Questions

None. All decisions resolved during analysis.