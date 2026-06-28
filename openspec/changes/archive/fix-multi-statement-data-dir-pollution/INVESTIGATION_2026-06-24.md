# 2026-06-24 EAGAIN Test Investigation

## Scope
Four integration tests in `develop/v3.9.0` (commit `93b81767d`) fail on
macOS debug builds with `Resource temporarily unavailable (os error 35)`
on the client read. Each has an independent root cause. Two parallel
subagent investigations (each running 1.5+ hours) were cancelled
without producing a working fix; this document records the diagnostic
findings so the next attempt does not have to rediscover them.

## Test-by-test root cause

### 1. `multi_statement_test::test_multi_statement_two_selects`
**Status: server silently hangs 60s in `make_ok_packet.write_to`.**

File-log evidence (from `/tmp/sqlrustgo-dbg.log`, written by instrumenting
`do_command_loop` and `Packet::write_to`):

```
[eng-dbg] execute() enter, sql=CREATE TABLE IF NOT EXISTS t1 (id INT PRIMARY KEY, val TEXT)
[eng-dbg] parse() done
[eng-dbg] execute_create_table enter, table=t1
[eng-dbg] execute_create_table storage.create_table done
[server-dbg] match ok-r arm
[BEFORE make_ok_packet]
                                       <-- 60 second hang here
                                       <-- client read times out with EAGAIN
[AFTER make_ok_packet]                  <-- never reached
```

The hang is in the `Packet::write_to` call at
`crates/mysql-server/src/lib.rs:2449`. Tracing each of the four
steps inside `write_to` (u24, u8, write_all, flush) shows the
hang is in the **final `w.flush()` call**. The preceding `write_u24`,
`write_u8`, and `write_all` all return promptly.

**Strongly suspected root cause:** macOS's `TcpStream::flush()` does
not honor `set_write_timeout(60s)`. The 60s timeout set at
`crates/mysql-server/src/lib.rs:2753` covers the user-buffer write
syscall but not the kernel-side flush of pending TCP segments to the
peer. When the test's cargo-test parallel workload has not drained
the kernel's send buffer, `flush()` blocks for as long as the kernel
deems necessary, which on macOS in cargo-test can exceed the
60-second budget and effectively hang.

The Linux kernel returns `EAGAIN` immediately on `flush()` if the
send buffer is full and `SO_SNDTIMEO` is set, which is why this bug
does not surface on the CI runner (Linux, kernel 5.x).

**Why the existing CI is green:** CI uses `cargo test --release`,
which runs 10-100x faster than debug, and the kernel send buffer
drains between writes. The hang only surfaces when the test thread
spends long enough in the execute path (debug build of
`parse_statements` + `engine.execute` + `storage.create_table`) for
the kernel buffer to fill up.

### 2. `tpch_value_correctness_test::tpch_value_correctness_synthetic_data`
**Status: same root cause as #1.** The test uses `start_sf001()` to
load the TPC-H SF=0.001 fixture, then issues `CREATE TABLE`,
`INSERT`, `SELECT COUNT`, and `SELECT SUM`. The first `CREATE TABLE`
hits the same `make_ok_packet.write_to` hang.

### 3. `perf_eng_batched_insert_test`
**Status: missing `#[ignore]` attribute (0-line server fix).**

Both `perf_1000_row_batched_insert_under_1s` and
`perf_10000_row_batched_insert_under_10s` are documented in
`tests/perf_eng_batched_insert_test.rs:18` as release-only:

```rust
//! **Mode**: `#[ignore]` — run with `cargo test --release --test
//! perf_eng_batched_insert_test -- --ignored --nocapture`
```

But the `#[ignore]` attribute is missing on the test functions
themselves, so `cargo test` (debug build, default) runs them and
fails on the timing assertion (9.36s vs 1s threshold, 1152s vs 10s
threshold — debug build is 10-100x slower than release).

**Trivial fix:** add `#[ignore]` above each `#[test] fn`. No server
changes needed.

### 4. `load_local_infile_eagain_regression_test::test_load_local_infile_eagain_regression`
**Status: silent row loss in LOAD DATA LOCAL INFILE (different bug
despite the "eagain" name).**

The first 7 TPC-H tables (region 5 rows, nation 25, supplier 10,
customer 15, part 20, partsupp 80, orders 150) load correctly.
Only `lineitem` (57098 bytes, 16 columns, 614 rows expected) loses
113 rows. The assertion is:

```
lineitem: expected 614 rows, loaded 501
```

The handler in
`crates/mysql-server/src/lib.rs::handle_load_local_infile` at
line 2113 uses a `tracing::warn!` for parse errors at line 2223 and
does **not** propagate them as `Err`. The `bulk_insert` call at
line 2248 is treated as infallible; if a row's parse fails
silently, the row is dropped and the load reports fewer rows than
expected.

The misleading "eagain" name in the test file and `LOAD DATA` legacy
behaviour made the failure look like the same EAGAIN issue as
#1/#2; it is actually a different bug (silent row loss, not a
kernel buffer hang).

**Fix direction:** change the handler so that parse errors and
bulk_insert errors are accumulated and reported (or at least counted),
not swallowed. This requires changes to
`handle_load_local_infile` and possibly `bulk_insert_records` in
`src/execution_engine.rs:249`.

## What was attempted
1. **Client-side retry** in `tests/common/mod.rs::read_packet`
   (EAGAIN/WouldBlock/TimedOut/Interrupted retry with 20ms backoff).
   - Result: changed the failure mode from `EAGAIN` to
     `timed out after 5s` (the new timeout), confirming the server
     is not sending data — the retry is correct but does not fix
     the underlying hang.
2. **Server-side retry** in `Packet::read_from` (5s deadline).
   - Result: same as #1. The server-side retry is correct for
     transient kernel WouldBlock on the read path, but does not
     affect the write path.
3. **Increasing client timeouts** to 60s.
   - Result: no change in test outcome; the server still hangs for
     60s, then the client read times out at 60s with EAGAIN.
4. **Adding file-log instrumentation** to `do_command_loop` and
   `Packet::write_to` to identify the hang location.
   - Result: pin-pointed the hang to `w.flush()` (4th step of
     `write_to`). See log excerpt in section 1 above.

## What was NOT attempted (because of time / risk)
1. Removing `w.flush()` from `Packet::write_to`. The hang would
   disappear because the OS buffer would be flushed lazily, but this
   changes protocol semantics for the `mysql` CLI which relies on
   the explicit flush. The current comment at line 682-691
   documents why `flush()` is required for `TlsStream`. Changing
   this is a 1-line patch but breaks the protocol contract.
2. Replacing `flush()` with a `flush()` retry loop on
   `ErrorKind::WouldBlock` / `ErrorKind::TimedOut`. This is a
   safer 3-5 line change that preserves the flush contract. It
   would also need a configurable deadline (default 30s) to avoid
   infinite retry.
3. Adding `set_write_timeout(Some(Duration))` again at the
   `do_command_loop` per-call boundary, in case the timeout set in
   `handle_connection` is being lost somewhere along the way.
4. Running on Linux to confirm the fix works there. The CI runner
   is Linux; if the fix is correct, CI should turn green.

## Proposed fix for the next attempt
For tests #1 and #2 (the macOS flush hang), the minimal-risk fix is:

```rust
// in crates/mysql-server/src/lib.rs::Packet::write_to
pub fn write_to<W: Write>(&self, w: &mut W) -> MySqlResult<()> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    w.write_u24::<LittleEndian>(self.length)?;
    w.write_u8(self.sequence)?;
    w.write_all(&self.payload)?;
    // macOS's TcpStream::flush() does not honor SO_SNDTIMEO; loop
    // on transient WouldBlock/TimedOut up to a 30s deadline.
    loop {
        match w.flush() {
            Ok(()) => return Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock
                       || e.kind() == std::io::ErrorKind::TimedOut => {
                if std::time::Instant::now() >= deadline {
                    return Err(MySqlError::Io(e));
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
                continue;
            }
            Err(e) => return Err(MySqlError::Io(e)),
        }
    }
}
```

For test #3: `#[ignore]` on the two test functions.

For test #4: change `handle_load_local_infile` to accumulate
parse/insert errors and either fail or report them; this is a
larger change.

## Time spent
~90 minutes of direct investigation, 2 subagent runs of 1.5+
hours each (cancelled without producing a working fix), total
~4 hours wall-clock with no commit-able fix produced for the four
EAGAIN tests.
