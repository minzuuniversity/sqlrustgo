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
//! off-path, and the on-path behaviour is worse in a different way: the
//! tests below show a connection's own ROLLBACK does not undo its own
//! buffered rows. That is a separate, more serious defect (abandoned
//! transactions stay visible), so it is reported here rather than
//! folded into #4957.
//!
//! The assertions encode the *observed* behaviour, so they are written to
//! pass today and will go red if either behaviour is fixed. That makes
//! this file a tripwire for the follow-up, not a regression gate for a
//! fix that has not been written yet.

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

    // Sanity: both transactions have written into the shared buffer.
    let mut probe = connect(port);
    assert_eq!(
        count(&mut probe),
        20,
        "both transactions' rows should be pending"
    );

    a.execute("ROLLBACK").expect("A rollback");

    // OBSERVED 2026-10-04: A's rows (0..10) are STILL PRESENT after
    // ROLLBACK. The lockfree rollback path performs no per-row undo, so
    // an abandoned transaction stays visible to every other connection.
    //
    // This is the opposite of what #4957 predicted (it expected A's rows
    // to vanish and B's to survive). Both connections' rows survive,
    // which means the failure mode is "rollback does nothing", not
    // "rollback over-reaches".
    //
    // If a future fix makes ROLLBACK actually undo its own transaction,
    // this assertion goes red — that is intentional. It is a tripwire,
    // not a pass condition.
    let mut remaining = ids(&mut probe);
    remaining.sort_unstable();
    assert!(
        remaining.contains(&0),
        "EXPECTED-FIX-BEHAVIOUR: A's ROLLBACK should have removed id 0, \
         but the abandoned transaction's rows are still visible. If this \
         message appears, the rollback path has been fixed and this file \
         needs updating. Current rows: {:?}",
        remaining
    );

    b.execute("COMMIT").expect("B commit");
    let after = ids(&mut probe);
    println!(
        "OBSERVED: after A.ROLLBACK + B.COMMIT, {} rows remain: {:?}",
        after.len(),
        after
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
    // B's committed rows survive (200..204) — that part is correct.
    // A's abandoned row (id 1) also survives, which should not happen.
    let mut remaining = ids(&mut probe);
    remaining.sort_unstable();
    assert!(
        remaining.contains(&200) && remaining.contains(&204),
        "B's committed rows must survive any other connection's rollback; \
         got {:?}",
        remaining
    );
    assert!(
        remaining.contains(&1),
        "EXPECTED-FIX-BEHAVIOUR: A's ROLLBACK should have removed id 1, \
         but the abandoned row is still visible. If this message \
         appears, the rollback path has been fixed and this file needs \
         updating. Current rows: {:?}",
        remaining
    );
}
