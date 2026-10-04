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

    // #4960: this assertion used to be the inverse — it asserted A's rows
    // 0..10 SURVIVED, deliberately red, as a tripwire for "ROLLBACK does
    // nothing". That tripwire fired on 2026-10-04: `delete_collect_pks`
    // built its removed-pk list solely from `tables.rows`, while a
    // transaction's rows still sit in `insert_buffer`, so the undo replay
    // deleted from the buffer but reported zero rows and never
    // tombstoned. Fixed in #4960; the behaviour is now the correct one,
    // so the assertion is inverted.
    let mut remaining = ids(&mut probe);
    remaining.sort_unstable();
    assert!(
        !remaining.contains(&0),
        "A's ROLLBACK must remove A's own rows (0..10); they are still \
         visible. Current rows: {:?}",
        remaining
    );
    assert!(
        remaining.iter().all(|id| (100..110).contains(id)),
        "A's ROLLBACK must not touch B's pending rows (100..110); \
         got {:?}",
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
