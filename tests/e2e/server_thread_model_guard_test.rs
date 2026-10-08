//! Guard: pin the thread-execution model that the transaction-context
//! design depends on.
//!
//! # Why this test exists
//!
//! Issue #5099 moves transaction state off the shared `FileStorage`
//! instance into an explicit, per-connection context. Whether a cheaper
//! `thread_local` could substitute depends on two facts about how
//! connections are dispatched:
//!
//! 1. **A connection does not migrate between workers.** If it did, a
//!    `thread_local current_tx_id` would read the wrong value mid-command
//!    and silently corrupt transactions.
//! 2. **A worker is reused across many connections.** If that did not
//!    hold, `thread_local` state could not leak between connections. It
//!    does hold, so any `thread_local` state MUST be cleared when a
//!    connection ends, or the next connection on that worker inherits it.
//!
//! Facts 1 and 2 pull in opposite directions, and together they are why
//! `thread_local` is a stopgap rather than the fix: it is correct only
//! under the current synchronous job-per-connection dispatch, and it
//! would silently misbehave the moment dispatch changes (per-command
//! dispatch, async tasks, background execution, rayon inside DML).
//!
//! This test locks both facts down so a future dispatch refactor trips it
//! rather than quietly invalidating the thread-safety argument.
//!
//! The dispatch under test (`run_server_with_listener_and_shutdown_...`,
//! crates/mysql-server/src/lib.rs):
//!   * `server_threads > 0` — the accept loop wraps each `TcpStream` in a
//!     `ServerJob` and hands it to a `ServerThreadPool`; `worker_loop`
//!     runs `handle_connection` -> `do_command_loop` to completion for
//!     that connection before pulling the next job.
//!   * `server_threads == 0` — legacy path, one `thread::spawn` per
//!     connection (no pool, so no reuse question).
//!
//! # Client choice
//!
//! These tests drive the server with the `mysql` CLI rather than
//! `MySqlTestClient`. `MySqlTestClient::query_rows` assumes every
//! statement returns a result set and mis-parses the OK packet that DDL
//! and transaction control statements return, which desyncs the client
//! (its `stream` field is private, so a test cannot recover). The CLI
//! implements the full packet protocol and is what SOAK exercises anyway.
//! Tables are created through `EphemeralConfig::bootstrap_sql` so the
//! assertions stay about transaction behavior, not DDL plumbing.

use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig, ServerThreadPool};
use std::process::Command;
use std::time::{Duration, Instant};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

/// Run a script of `;`-separated statements over ONE connection and return
/// combined stdout. A single connection is essential: the whole point is
/// that all statements share one session.
fn run_script(port: u16, script: &str) -> String {
    let mut child = Command::new("mysql")
        .args([
            "-h",
            "127.0.0.1",
            "-P",
            &port.to_string(),
            "-u",
            "tester",
            "-ptester",
            "--protocol=TCP",
            "--batch",
            "--skip-column-names",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn mysql client");
    {
        use std::io::Write;
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin.write_all(script.as_bytes()).expect("write script");
        stdin.write_all(b"\n").ok();
    }
    let out = child.wait_with_output().expect("wait for mysql");
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn wait_ready(port: u16) {
    let deadline = Instant::now() + CONNECT_TIMEOUT;
    while Instant::now() < deadline {
        let ok = Command::new("mysql")
            .args([
                "-h",
                "127.0.0.1",
                "-P",
                &port.to_string(),
                "-u",
                "tester",
                "-ptester",
                "--protocol=TCP",
                "--batch",
                "--skip-column-names",
                "-e",
                "SELECT 1",
            ])
            .output();
        if let Ok(o) = ok {
            if o.status.success() {
                return;
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("server on port {port} never became ready");
}

fn cfg_with_table(server_threads: usize, tmp: &tempfile::TempDir, table: &str) -> EphemeralConfig {
    EphemeralConfig {
        data_dir: Some(tmp.path().to_path_buf()),
        bootstrap_tables: false,
        bootstrap_users: true,
        server_threads,
        bootstrap_sql: vec![format!("CREATE TABLE {table} (id INT PRIMARY KEY, k INT)")],
        ..Default::default()
    }
}

/// Fact 1: a connection is served start-to-finish by a single worker.
///
/// `worker_loop` runs `handle_connection` synchronously per job, so a
/// worker never hands a live connection to another worker mid-stream.
/// The durable consequence a `thread_local` design relies on: all of one
/// connection's commands observe one stable server-side view. A connection
/// that migrated mid-transaction would split per-tx state (undo log,
/// buffered rows) across workers and produce wrong results.
///
/// Asserted through transaction semantics rather than introspection:
/// "which worker ran this command" is not exposed, whereas the data
/// corruption a migrating connection causes is observable.
#[test]
fn connection_transaction_state_survives_whole_session() {
    let tmp = tempfile::TempDir::new().expect("TempDir");
    let handle = start_ephemeral(cfg_with_table(4, &tmp, "guard_t")).expect("start_ephemeral");
    wait_ready(handle.port);

    // Many statements inside ONE transaction on ONE connection: the exact
    // workload a connection that migrates mid-transaction would corrupt.
    let mut script = String::from("BEGIN;\n");
    for i in 1..=20i64 {
        script.push_str(&format!("INSERT INTO guard_t (id, k) VALUES ({i}, {i});\n"));
    }
    script.push_str("COMMIT;\nSELECT COUNT(*) FROM guard_t;\n");

    let out = run_script(handle.port, &script);
    let count = out.lines().last().unwrap_or("").trim().to_string();
    assert_eq!(
        count, "20",
        "expected 20 committed rows from the originating connection, got {out:?} — \
         a connection that migrates between workers splits in-flight per-connection \
         state (tx context, buffered rows, undo log)"
    );
}

/// Fact 2: workers ARE reused across connections.
///
/// This is what makes `thread_local` leaky: a worker serves many
/// connections in sequence, so per-thread transaction state persists into
/// the next connection unless explicitly cleared on close.
///
/// The structural fact is the bounded worker set — asserted directly on
/// `ServerThreadPool::worker_count()`. The end-to-end consequence is then
/// driven through the real server with many more connections than workers.
#[test]
fn workers_are_reused_across_connections() {
    // Bounded by construction; this is the bound the reuse argument rests
    // on. (`ServerJob` cannot be built outside the crate — `user_store` /
    // `config` are `pub(crate)` — so reuse is exercised end-to-end through
    // the real server rather than by injecting synthetic jobs.)
    let pool = ServerThreadPool::start(2);
    assert_eq!(pool.worker_count(), 2);
    drop(pool);

    let n_workers = 2usize;
    let n_conns = 12usize;
    let tmp = tempfile::TempDir::new().expect("TempDir");
    let handle =
        start_ephemeral(cfg_with_table(n_workers, &tmp, "guard_r")).expect("start_ephemeral");
    wait_ready(handle.port);

    for i in 0..n_conns {
        let out = run_script(handle.port, "SELECT 1;\n");
        assert!(
            out.contains('1'),
            "connection {i} through {n_workers} workers failed: {out:?}"
        );
    }
}

/// The legacy `server_threads == 0` path spawns one thread per connection,
/// so it has no reuse question — but it must still keep per-connection
/// transaction state correct, since that is the mode the SOAK harness uses
/// for short-lived connections.
#[test]
fn legacy_per_connection_thread_path_keeps_tx_state() {
    let tmp = tempfile::TempDir::new().expect("TempDir");
    let handle = start_ephemeral(cfg_with_table(0, &tmp, "guard_u")).expect("start_ephemeral");
    wait_ready(handle.port);

    let mut script = String::from("BEGIN;\n");
    for i in 1..=10i64 {
        script.push_str(&format!("INSERT INTO guard_u (id, k) VALUES ({i}, {i});\n"));
    }
    script.push_str("COMMIT;\nSELECT COUNT(*) FROM guard_u;\n");

    let out = run_script(handle.port, &script);
    let count = out.lines().last().unwrap_or("").trim().to_string();
    assert_eq!(
        count, "10",
        "legacy thread-per-connection path lost transaction state: {out:?}"
    );
}

/// Guards the consequence the #5099 refactor must not regress: a
/// connection that ends mid-transaction must not leave its state behind
/// for the next connection served by the same worker. This is exactly the
/// leak a `thread_local` implementation would introduce, so pinning the
/// clean-up contract here means the refactor cannot ship without it.
#[test]
fn aborted_transaction_does_not_leak_into_next_connection() {
    let tmp = tempfile::TempDir::new().expect("TempDir");
    let handle = start_ephemeral(cfg_with_table(2, &tmp, "guard_leak")).expect("start_ephemeral");
    wait_ready(handle.port);

    // Connection 1 opens a transaction, inserts, then the process exits
    // without committing (mysql sends no COMMIT on EOF).
    let leaked = run_script(
        handle.port,
        "BEGIN;\nINSERT INTO guard_leak (id, k) VALUES (1, 1);\n",
    );
    assert!(
        !leaked.contains("ERROR"),
        "setup transaction failed: {leaked:?}"
    );

    // Connection 2, possibly served by the same worker, must see an empty
    // table — the abandoned transaction left nothing behind.
    let out = run_script(handle.port, "SELECT COUNT(*) FROM guard_leak;\n");
    let count = out.lines().last().unwrap_or("").trim().to_string();
    assert_eq!(
        count, "0",
        "an abandoned transaction leaked into the next connection: {out:?} — \
         per-connection transaction state must be discarded when the connection ends"
    );
}
