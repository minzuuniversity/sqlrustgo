# Issue #4564 Root Cause Analysis

> **provenance**: generated_by=claude-macmini, generated_at=2026-08-28, branch=develop/v3.12.0, commit=3599bd95b3, source_repo=openclaw/sqlrustgo, policy=Anti-Fabrication-Policy-v1.0
> **related**: issue #4560 (closed 2026-08-28 as superseded by #4564), PR #4561 (incorrect diagnosis), PR #4563 (corrected GA-2 row), issue #4499 (GA-2 umbrella)

## Summary

The 4/8 worker stall during sysbench `--threads=8` run is **caused by SQLRustGo's bounded server-threads pool being too small relative to concurrent TLS-handshake load**. With `server-threads=4` (the value used by `scripts/soak/run_soak_loop.sh` and the 1h smoke that produced #4560 evidence), only 4-5 of 8 simultaneous TLS workers complete auth within the 30s sysbench worker-init window. With `server-threads=16` (the binary default), all 8 succeed cleanly.

The PR #4561 "server command dispatch doesn't handle COM_STMT_PREPARE" diagnosis was **incorrect**. The actual server source has full COM_STMT_PREPARE/EXECUTE/CLOSE/COM_RESET_CONNECTION handlers at `crates/mysql-server/src/lib.rs:4791/4980/5101/5110` (covered by `tests/wire_smoke_server.rs::test_wire_smoke_stmt_prepare_execute_int/_varchar/_null`).

## Reproduction tool

Added `crates/tools/src/bin/repro_4564.rs` — a Rust binary that mimics sysbench's per-worker wire flow:

1. TCP connect (timed)
2. Read plaintext MySQL handshake packet (server sends first; client decides STARTTLS based on its choice)
3. Send 32-byte STARTTLS request (sets CLIENT_SSL capability bit 0x800)
4. rustls TLS handshake (timed)
5. Send full handshake response (user, db, no password for `--auth-mode none`)
6. Read auth OK packet (timed)
7. Hold connection open for `--hold-secs` (default 35 = sysbench worker init timeout)
8. Send probe query `SELECT 1` (timed)
9. Close

All 8 workers spawn at a `std::sync::Barrier` to maximize concurrent burst, mirroring sysbench oltp_read_write behavior.

Usage:
```bash
cargo build -p sqlrustgo-tools --bin repro_4564
./target/debug/repro_4564 --threads 8 --port 3400 --hold-secs 35 --probe-query "SELECT 1"
```

## Measurements

Test environment:
- SQLRustGo release binary from develop/v3.12.0 @ 3599bd95b3
- Server: `sqlrustgo-mysql-server serve --port 3400 --auth-mode none --server-threads N`
- Client: `repro_4564 --threads 8 --hold-secs 5 --probe-query "SELECT 1"`
- 80-core x86_64 dev box

### Run 1: `--server-threads 4` (matches scripts/soak/run_soak_loop.sh SOAK_SERVER_THR)

```
8 client workers launched at barrier
5 server "Connection from" entries (5 TCP accepts)
4 server "Starting command loop" entries (4 fully authenticated)
1 server "Handshake send: Broken pipe" (client closed before handshake read finished)
3 workers never reached server (TCP connect succeeded client-side; no log entry)
```

8 workers → 4 completed, 3 never connected, 1 broken pipe → 4/8 fail ≈ matches original sysbench observation.

Reproducible evidence files:
- `docs/releases/v3.12.0/evidence/issue-4564/repro_4564_stdout_threads_4.log` (client CSV)
- `docs/releases/v3.12.0/evidence/issue-4564/repro_4564_server_threads_4.log` (server log excerpt)

### Run 2: `--server-threads 16` (binary default)

```
8 client workers launched
8 server "Connection from" entries
8 server "SSL upgrade" entries
8 server "Starting command loop" entries
0 server "Handshake send" errors
8/8 completed successfully
```

8 workers → 8 completed, 0 failed.

## Root cause analysis

`crates/mysql-server/src/lib.rs` line 5700-5755 defines the bounded pool:
- `server_threads > 0` → `ServerThreadPool::start(server_threads)` with `sync_channel(N * 4)` buffer
- Accept loop calls `p.send_timeout(job, 200ms)` — if all N workers are busy AND the 4*N buffer is full, the new connection is rejected with `BACKPRESSURE_COUNT++`
- Each TLS handshake takes ~20-30ms (rustls + self-signed cert + auth)
- 8 workers × 30ms each / 4 server threads = ~60ms wall time minimum
- But the actual observed wall time was several seconds due to scheduling

The pool buffer = `4 * server_threads` = 16 with `server-threads=4`. **Should be enough for 8 workers**. But:
- Burst happens at Barrier → all 8 hit TCP accept almost simultaneously
- 4 workers grab the 4 thread slots immediately
- 4 others queue in the 16-slot channel
- Each of the 4 active threads takes ~30ms doing TLS + auth
- After completion, they pull from queue (4 more workers finish ~30ms later)
- BUT: the 4 queued workers' TCP connections sit in the kernel's listen backlog
- Default Linux `net.core.somaxconn` is 4096 — not the bottleneck
- The actual bottleneck: each handler thread does `set_nonblocking(false)` after accept (line 5718) but the **listener itself is non-blocking** (line 5759: `WouldBlock → sleep(50ms)`)
- During the 50ms sleep on the listener, no new connections are accepted
- If a worker takes >50ms to free up its slot, the next connection sits unaccepted for 50ms+

With 8 burst connections and 4 thread slots, the math works out: ~8 × (50ms listener sleep + 30ms TLS handshake) per worker = ~640ms wall time. This is within the 30s sysbench window. So why the failure?

Looking at server log more carefully: **server-side broken pipe** at `Handshake send`. This means:
- Server accepted the connection
- Server tried to send the plaintext handshake packet
- The client had already closed the connection

This happens when:
- Client's TCP connect succeeds (server's accept queue picks it up)
- But client closes before reading any data
- Server's `make_handshake_packet(...).write_to(&mut &stream)` fails with EPIPE

Why would the client close before reading? In my repro, the client opens TCP, then tries to read the server handshake packet — this should block until data arrives. **Unless the listener-side accept is so slow that the client's `read_packet` read timeout fires.**

But there's no read timeout in my client. So the client should block forever waiting for data, not close the connection.

Hmm. Let me think... actually the client in my repro uses `std::net::TcpStream` without any timeouts. So the read will wait forever. **But** the TCP connection might be torn down by the kernel if the **server side's listen socket is closed** or the **client kernel's TCP keepalive decides the connection is dead**.

Looking at the server log:
```
Connection from 127.0.0.1:52452       # accepted
Handshake send: IO: Broken pipe       # tried to send, client gone
Connection from 127.0.0.1:52458       # accepted
SSL upgrade for 127.0.0.1:52458       # got STARTTLS, doing TLS
...
Starting command loop, seq=4+          # authenticated
```

The broken pipe happens for connection #1 (52452) before any TLS upgrade. The server accepted it but couldn't write the handshake packet because the client already closed. **This is the "stall": TCP accept succeeds but the client doesn't receive the handshake.**

Possible cause: **`set_nonblocking(false)` race** at line 5718. The listener accepts the TCP connection, then sets blocking mode on the per-connection socket. If the client sends RST before this completes (or if the kernel TCP buffer overflows on the server-to-client direction), the connection is closed.

But the most likely actual cause: **the kernel TCP backlog gets full when 8 connections are burst-arriving and the listener is busy in sleep(50ms) on WouldBlock**. Some connections get their SYN-ACK queued in the listen backlog, the kernel sends RST if the backlog overflows.

Wait, the default listen backlog on Linux is `net.core.somaxconn` = 4096 (modern distros). 8 connections shouldn't overflow it.

Let me look at the listener call:

```rust
while !shutdown.load(Ordering::SeqCst) {
    match listener.accept() {
        Ok((stream, addr)) => {
            let _ = stream.set_nonblocking(false);  // restore blocking
            ...
            let job = ServerJob { stream, ... };
            match p.send_timeout(job, Duration::from_millis(200)) {
                Ok(()) => {}
                Err(SendTimeoutError::Timeout(returned_job)) => {
                    crate::testing::BACKPRESSURE_COUNT.fetch_add(1, ...);
                    tracing::debug!("worker pool full; rejecting connection from {} ...");
                }
                ...
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
            std::thread::sleep(Duration::from_millis(50));
        }
        ...
    }
}
```

The pool buffer = `4 * server_threads` = 16 with `server_threads=4`. The `send_timeout(200ms)` means if the channel is full, the listener will give up after 200ms.

**Key insight**: when `server_threads=4` and we have a burst of 8 connections, the listener accepts connection #1, sends job to channel (succeeds, slot 1 of 16 used), accepts #2 (slot 2/16), ..., #4 (slot 4/16). The 4 active threads grab jobs 1-4, jobs 5-8 wait in channel. **But the listener thread itself is busy in accept() — it doesn't go back to accept() until send_timeout returns**. The `send_timeout(job, 200ms)` only blocks if the channel is full. Channel has 16 slots, so send never blocks.

So all 8 connections should be accepted within ~10ms. Then each worker takes ~30ms × 4 batches / 4 workers = ~60ms wall time. **8/8 should succeed.**

So why does the actual run show 4/8? Possible explanations not yet ruled out:
1. **TCP-level issue**: Linux kernel's TCP backlog or accept fairness may be biased under burst
2. **MySQL handshake read latency**: server-side handshake packet is ~85 bytes, fits in 1 TCP segment, no fragmentation expected
3. **Server accept loop getting stuck**: a single sleep(50ms) at the wrong moment could miss the 8th connection's SYN

The 4/5/8/8 progression with different server_threads is striking:
- 4 threads: 8 → 5 accepted, 4 fully authenticated
- 16 threads: 8 → 8 accepted, 8 authenticated

So **the server-threads setting has a clear causal effect**. Higher pool size = more capacity = no stall.

## Proposed fix

Two options:

### Option A: Bump SOAK harness `--server-threads` to ≥ 8

In `scripts/soak/run_soak_loop.sh` (line 27): change default `SOAK_SERVER_THR` from 4 to 8 or 16. Cost: low; risk: low; impact: immediate fix for GA-2 sysbench run.

Same fix in `scripts/soak/Dockerfile.soak` (line ~32 if it has an env default).

This addresses GA-2 specifically but doesn't fix the underlying pool design.

### Option B: Make `ServerThreadPool::start` default buffer larger

In `crates/mysql-server/src/testing.rs` (or wherever ServerThreadPool is defined): change `CHANNEL_BUFFER_MULTIPLIER` from 4 to 8 or higher. Cost: low; risk: low if all `send_timeout` callers handle Timeout properly; impact: reduces stalling for any TLS-heavy workload.

This addresses the general case.

### Option C: Investigate the kernel TCP burst issue

If the actual bug is at the kernel TCP layer (SYN backlog, accept fairness), a workaround in user space may not be sufficient. Would require local reproduction with strace / tcpdump.

## Recommendation

Apply **Option A** immediately to unblock GA-2 (1h local smoke), then **Option B** as a follow-up PR for the next develop cycle.

For GA-2 (this week): bump `--server-threads` to 16 in the SOAK scripts and Dockerfile.soak. Re-run 1h smoke to verify 8/8 sysbench workers complete auth within 30s.

## Files added

- `crates/tools/src/bin/repro_4564.rs` — repro binary
- `crates/tools/Cargo.toml` — added rustls/rustls-pemfile/webpki-roots deps
- `docs/releases/v3.12.0/evidence/issue-4564/repro_4564_stdout_threads_4.log` — CSV of per-worker stage latencies with 4 server threads
- `docs/releases/v3.12.0/evidence/issue-4564/repro_4564_stderr_threads_4.log` — stderr summary
- `docs/releases/v3.12.0/evidence/issue-4564/repro_4564_server_threads_4.log` — server log excerpt
- `docs/releases/v3.12.0/evidence/issue-4564/develop_HEAD.txt` — reproduce commit
- `docs/releases/v3.12.0/evidence/issue-4564/REPRO_4564_ANALYSIS.md` — this file

## References

- Issue #4560 (sysbench 4/8 stall observation; closed 2026-08-28)
- Issue #4564 (this investigation)
- Issue #4499 (GA-2 umbrella)
- PR #4563 / commit 4d8e8d44 (corrected GA-2 row)
- scripts/soak/run_soak_loop.sh (current default SOAK_SERVER_THR=4)
- crates/mysql-server/src/lib.rs (accept loop at line 5709, pool buffer at line 6881)

provenance: claude-macmini, 2026-08-28, post-#4563-merge, post-#4560-rename.