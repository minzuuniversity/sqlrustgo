//! #4957 — verification result: the issue is real but misattributed.
//!
//! #4957 points at `FileStorage::rollback_transaction`
//! (`file_storage.rs:3765-3769`), which drains the insert buffer for every
//! table after the per-row undo pass. The code is exactly as described.
//!
//! **But that is not the path the MySQL wire takes.** Verified:
//!
//! * `FileStorage::rollback_transaction` is called only from
//!   `crates/server/src/openclaw_endpoints.rs` (3 call sites) — a
//!   different service.
//! * The protocol path is
//!   `ExecutionEngine::rollback_transaction` (`src/execution_engine_methods.rs:1754`)
//!   → `storage.rollback_transaction_lockfree()` (`:1777`).
//! * `WalStorage::rollback_transaction_lockfree` (`wal_storage.rs:1082`)
//!   writes a WAL `Rollback` entry and then calls
//!   `discard_all_buffers_shared()` — **no per-row undo at all**.
//!
//! So the belt-and-suspenders buffer wipe #4957 flags is real but
//! off-path, and the on-path behaviour was worse in a different way: at
//! audit time a connection's own ROLLBACK did not undo its own buffered
//! rows (abandoned transactions stayed visible). That was reported here
//! rather than folded into #4957, and was subsequently fixed (#4960);
//! the isolation repairs (#4974/#4983) also made uncommitted rows
//! invisible to other connections.
//!
//! The assertions pin the *correct* post-fix behaviour. This file was
//! originally written as a tripwire encoding the pre-fix observations
//! ("go red when the behaviour is fixed"), and was updated when #4960
//! and the isolation repairs landed — a stale tripwire would keep
//! asserting the defect as if it were the contract.

use sqlrustgo_mysql_client::{MySqlConnection, ResultSet};
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};

fn start() -> sqlrustgo_mysql_server::testing::EphemeralHandle {
    start_ephemeral(EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        bulk_insert_rows_per_flush: 10_000,
        load_infile_dir: None,
        server_threads: 8,
        storage: None,
        slow_query_log: None,
        metrics_port: None,
        wal_sync_mode_override: None,
    })
    .expect("ephemeral server starts")
}

fn connect(port: u16) -> MySqlConnection {
    let addr = format!("127.0.0.1:{}", port).parse().expect("addr");
    MySqlConnection::connect(&addr, "tester", "tester", "").expect("connect")
}

fn count(conn: &mut MySqlConnection) -> i64 {
    match conn.execute("SELECT COUNT(*) FROM t").expect("count") {
        ResultSet::Select { rows, .. } => rows
            .first()
            .and_then(|r| r.first())
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(-1),
        other => panic!("expected select, got {:?}", other),
    }
}

fn ids(conn: &mut MySqlConnection) -> Vec<i64> {
    match conn.execute("SELECT id FROM t").expect("select ids") {
        ResultSet::Select { rows, .. } => rows
            .iter()
            .filter_map(|r| r.first().and_then(|s| s.parse::<i64>().ok()))
            .collect(),
        other => panic!("expected select, got {:?}", other),
    }
}

/// The core question. Two connections, two open transactions:
/// A inserts 10 rows, B inserts 10 rows, then A rolls back.
///
/// A's 10 rows should be gone. B's 10 rows belong to a still-open
/// transaction on a different connection — they are not A's to discard.
#[test]
fn one_connections_rollback_must_not_discard_anothers_buffered_rows() {
    let handle = start();
    let port = handle.port;
    connect(port)
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)")
        .expect("create table");

    let mut a = connect(port);
    let mut b = connect(port);

    a.execute("BEGIN").expect("A begin");
    for i in 0..10 {
        a.execute(&format!("INSERT INTO t VALUES ({}, 'a')", i))
            .expect("A insert");
    }

    b.execute("BEGIN").expect("B begin");
    for i in 100..110 {
        b.execute(&format!("INSERT INTO t VALUES ({}, 'b')", i))
            .expect("B insert");
    }

    // Isolation (#4974/#4983): uncommitted rows must not leak to other
    // connections. The original assertion here pinned the pre-fix
    // observation (both transactions' 20 rows visible to a third
    // connection); the tripwire contract flips the assertion when the
    // behaviour is repaired — which it was.
    let mut probe = connect(port);
    assert_eq!(
        count(&mut probe),
        0,
        "uncommitted rows must stay invisible to other connections"
    );

    a.execute("ROLLBACK").expect("A rollback");

    // A's own rows are gone after its ROLLBACK (#4960). B's pending rows
    // must survive a rollback on another connection — the property this
    // test is named for — so ask B, whose own transaction still sees its
    // own writes.
    assert_eq!(
        count(&mut a),
        0,
        "A's ROLLBACK must remove A's own rows (0..10)"
    );
    // Regression pin (Fix 6): A's ROLLBACK must not leak B's rows to
    // other connections either. Pre-fix both connections held tx id 1
    // (per-connection TransactionManager counters), so A's rollback
    // discarded B's pending versions too while B's buffered rows
    // survived — a fresh third connection then merged all 10 of them.
    let mut probe2 = connect(port);
    assert_eq!(
        count(&mut probe2),
        0,
        "third connection must not see B's uncommitted rows after A's ROLLBACK"
    );
    assert_eq!(
        count(&mut b),
        10,
        "A's ROLLBACK must not discard B's pending rows (100..110)"
    );

    b.execute("COMMIT").expect("B commit");
    let mut remaining = ids(&mut probe);
    remaining.sort_unstable();
    assert_eq!(
        remaining,
        (100..110).collect::<Vec<_>>(),
        "after B's COMMIT exactly B's rows must be visible; A's \
         rolled-back rows stay gone"
    );
}

/// Two connections rolling back independently: the second rollback must
/// not resurrect or drop the first connection's committed rows.
#[test]
fn rollback_after_another_connection_committed_keeps_that_work() {
    let handle = start();
    let port = handle.port;
    connect(port)
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)")
        .expect("create table");

    let mut a = connect(port);
    let mut b = connect(port);

    // B commits first, so its rows are already durable.
    b.execute("BEGIN").expect("B begin");
    for i in 200..205 {
        b.execute(&format!("INSERT INTO t VALUES ({}, 'b')", i))
            .expect("B insert");
    }
    b.execute("COMMIT").expect("B commit");

    a.execute("BEGIN").expect("A begin");
    a.execute("INSERT INTO t VALUES (1, 'a')")
        .expect("A insert");
    a.execute("ROLLBACK").expect("A rollback");

    let mut probe = connect(port);
    // #4960: the `contains(&1)` assertion used to be the inverse (a
    // deliberate tripwire). ROLLBACK now correctly removes A's abandoned
    // row while B's committed rows are untouched.
    let mut remaining = ids(&mut probe);
    remaining.sort_unstable();
    assert!(
        remaining.contains(&200) && remaining.contains(&204),
        "B's committed rows must survive any other connection's rollback; \
         got {:?}",
        remaining
    );
    assert!(
        !remaining.contains(&1),
        "A's ROLLBACK must remove A's abandoned row (id 1); it is still \
         visible. Current rows: {:?}",
        remaining
    );
    assert_eq!(
        remaining.len(),
        5,
        "exactly B's 5 committed rows must remain, got {:?}",
        remaining
    );
}
