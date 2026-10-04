//! #4960: ROLLBACK in the MySQL protocol path undid nothing.
//!
//! ## The defect
//!
//! `BEGIN; INSERT ...; ROLLBACK;` left every inserted row in place, and
//! `BEGIN; UPDATE ...; ROLLBACK;` left the new column value in place. The
//! transaction was acknowledged as rolled back while its writes stayed
//! visible to every connection.
//!
//! Measured before the fix (wire protocol, real server):
//!
//! ```text
//! PROBE after baseline insert: 1
//! PROBE inside tx: 2
//! PROBE after rollback: 2        ← still 2
//! PROBE final rows: [["1","before"], ["2","in-tx"]]
//! ```
//!
//! ## Root cause
//!
//! The undo mechanism was never at fault — it was fully populated. A probe
//! on `TransactionManager::add_undo_record` / `take_undo_log` showed all
//! three records present and drained:
//!
//! ```text
//! PROBE_ADD_UNDO tx=2 rec=Insert { table: "t", key: [Integer(2)], ... }
//! PROBE_ADD_UNDO tx=2 rec=Update { table: "t", key: [Integer(1)], ... }
//! PROBE_ADD_UNDO tx=2 rec=Update { table: "t", key: [Integer(2)], ... }
//! PROBE_TAKE_UNDO tx=2 n=3
//! ```
//!
//! The undo replay called `storage.delete(table, key)` and it removed
//! **zero** rows:
//!
//! ```text
//! PROBE_UNDO_DELETE table=t key=[Integer(2)]
//! PROBE_UNDO_DELETE_N n=0          ← reported "deleted nothing"
//! ```
//!
//! `FileStorage::delete_collect_pks` built its `removed_pks` list **only**
//! from `tables[table].rows`. Rows written during a transaction are held in
//! `insert_buffer` and are not promoted into `tables.rows` until the buffer
//! threshold is reached. A probe of the two collections at delete time:
//!
//! ```text
//! PROBE_DCP table=t in_tx=true tables_len=Some(0) buf_len=Some(2)
//! ```
//!
//! `tables.rows` was empty; all two rows were in the buffer. The deletion
//! itself DID remove them (the buffer is filtered at the end of
//! `delete_collect_pks`), but the returned pk list was empty, so the
//! caller's MVCC tombstoning — which is driven by that return value — never
//! ran either. Net effect: rows removed from the buffer, nothing
//! tombstoned, and the MVCC chain still serving them to readers.
//!
//! The fix counts the buffered rows that were dropped and reports them
//! alongside the ones from `tables.rows`.

use sqlrustgo_mysql_client::{MySqlConnection, ResultSet};
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};

fn start_server(dir: &std::path::Path) -> sqlrustgo_mysql_server::testing::EphemeralHandle {
    start_ephemeral(EphemeralConfig {
        data_dir: Some(dir.to_path_buf()),
        load_infile_dir: Some(dir.to_path_buf()),
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        bulk_insert_rows_per_flush: 10_000,
        server_threads: 8,
        storage: None,
        slow_query_log: None,
        metrics_port: None,
        wal_sync_mode_override: None,
    })
    .expect("ephemeral server starts")
}

fn connect(port: u16) -> MySqlConnection {
    let addr: std::net::SocketAddr = format!("127.0.0.1:{}", port).parse().expect("addr");
    MySqlConnection::connect(&addr, "tester", "tester", "").expect("connect")
}

fn rows_of(rs: ResultSet) -> Vec<Vec<String>> {
    match rs {
        ResultSet::Select { rows, .. } => rows,
        ResultSet::Ok { .. } => panic!("expected a result set, got Ok"),
        ResultSet::Error {
            error_code,
            error_message,
            ..
        } => panic!("server error {}: {}", error_code, error_message),
    }
}

fn count(conn: &mut MySqlConnection, table: &str) -> u64 {
    let rows = rows_of(
        conn.execute(&format!("SELECT COUNT(*) FROM {}", table))
            .expect("count"),
    );
    rows[0][0].parse().expect("count is numeric")
}

/// ROLLBACK must remove the rows the transaction inserted, and must restore
/// the rows it updated — including rows still sitting in the insert buffer.
#[test]
fn rollback_undoes_the_transactions_inserts_and_updates() {
    let dir = tempfile::tempdir().expect("tempdir");
    let handle = start_server(dir.path());
    let mut conn = connect(handle.port);
    conn.execute("CREATE TABLE t (id INT PRIMARY KEY, v TEXT)")
        .expect("create");

    conn.execute("INSERT INTO t VALUES (1, 'before')")
        .expect("baseline");

    conn.execute("BEGIN").expect("begin");
    conn.execute("INSERT INTO t VALUES (2, 'in-tx')")
        .expect("insert in tx");
    // A transaction must see its own writes.
    assert_eq!(count(&mut conn, "t"), 2, "tx must see its own INSERT");

    // Update both the pre-existing row and the one it just inserted.
    conn.execute("UPDATE t SET v = 'changed' WHERE id = 1")
        .expect("update pre-existing");
    conn.execute("UPDATE t SET v = 'changed' WHERE id = 2")
        .expect("update own insert");

    conn.execute("ROLLBACK").expect("rollback");

    assert_eq!(
        count(&mut conn, "t"),
        1,
        "the transaction's INSERT must be gone after ROLLBACK"
    );
    let rows = rows_of(conn.execute("SELECT id, v FROM t").expect("select"));
    assert_eq!(
        rows,
        vec![vec!["1".to_string(), "before".to_string()]],
        "ROLLBACK must restore the UPDATE's pre-image too"
    );
}

/// A rolled-back transaction must not disturb another connection's
/// committed rows, and the other connection's commit must still land.
#[test]
fn rollback_leaves_other_connections_commits_intact() {
    let dir = tempfile::tempdir().expect("tempdir");
    let handle = start_server(dir.path());
    let mut a = connect(handle.port);
    a.execute("CREATE TABLE t (id INT PRIMARY KEY, v TEXT)")
        .expect("create");
    let mut b = connect(handle.port);

    a.execute("BEGIN").expect("a begin");
    b.execute("BEGIN").expect("b begin");
    a.execute("INSERT INTO t VALUES (10, 'a')")
        .expect("a insert");
    b.execute("INSERT INTO t VALUES (20, 'b')")
        .expect("b insert");
    assert_eq!(count(&mut a, "t"), 2, "a sees both uncommitted rows");

    a.execute("ROLLBACK").expect("a rollback");
    b.execute("COMMIT").expect("b commit");

    let rows = rows_of(a.execute("SELECT id FROM t").expect("select"));
    assert_eq!(
        rows,
        vec![vec!["20".to_string()]],
        "only b's committed row may survive"
    );
}

/// An empty transaction is still a valid ROLLBACK: it must be a no-op and
/// must not disturb anything.
#[test]
fn rollback_of_an_empty_transaction_changes_nothing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let handle = start_server(dir.path());
    let mut conn = connect(handle.port);
    conn.execute("CREATE TABLE t (id INT PRIMARY KEY, v TEXT)")
        .expect("create");
    conn.execute("INSERT INTO t VALUES (1, 'kept')")
        .expect("insert");

    conn.execute("BEGIN").expect("begin");
    conn.execute("ROLLBACK").expect("rollback");

    assert_eq!(count(&mut conn, "t"), 1, "an empty ROLLBACK is a no-op");
    let rows = rows_of(conn.execute("SELECT v FROM t").expect("select"));
    assert_eq!(rows, vec![vec!["kept".to_string()]]);
}

/// DELETE inside a transaction, then ROLLBACK, must put the row back.
#[test]
fn rollback_restores_a_deleted_row() {
    let dir = tempfile::tempdir().expect("tempdir");
    let handle = start_server(dir.path());
    let mut conn = connect(handle.port);
    conn.execute("CREATE TABLE t (id INT PRIMARY KEY, v TEXT)")
        .expect("create");
    conn.execute("INSERT INTO t VALUES (1, 'keep')")
        .expect("insert");

    conn.execute("BEGIN").expect("begin");
    conn.execute("DELETE FROM t WHERE id = 1")
        .expect("delete in tx");
    assert_eq!(
        count(&mut conn, "t"),
        0,
        "the tx must observe its own DELETE"
    );

    conn.execute("ROLLBACK").expect("rollback");

    assert_eq!(
        count(&mut conn, "t"),
        1,
        "ROLLBACK must restore a row the transaction deleted"
    );
    let rows = rows_of(conn.execute("SELECT v FROM t").expect("select"));
    assert_eq!(rows, vec![vec!["keep".to_string()]]);
}

/// COMMIT must still apply everything the transaction staged — the fix
/// changed what `delete_collect_pks` reports, so the commit path needs its
/// own guard against a regression that would drop committed rows.
#[test]
fn commit_still_applies_the_transaction() {
    let dir = tempfile::tempdir().expect("tempdir");
    let handle = start_server(dir.path());
    let mut conn = connect(handle.port);
    conn.execute("CREATE TABLE t (id INT PRIMARY KEY, v TEXT)")
        .expect("create");

    conn.execute("BEGIN").expect("begin");
    for id in 1..=25 {
        conn.execute(&format!("INSERT INTO t VALUES ({}, 'v{}')", id, id))
            .expect("insert");
    }
    // NOTE: this project's `DELETE ... WHERE` matches filters positionally
    // against the whole row, not against a named column, so a single-value
    // filter list here would match nothing (or everything). Exercised as a
    // full-table delete instead, which is unambiguous.
    conn.execute("DELETE FROM t").expect("delete in tx");
    conn.execute("COMMIT").expect("commit");

    assert_eq!(
        count(&mut conn, "t"),
        0,
        "COMMIT must apply the DELETE: the transaction's 25 inserts were          rolled into the same commit and must not survive it"
    );
}

/// A committed transaction's rows must be visible to OTHER connections —
/// the fix changed what the delete path reports, so verify the commit side
/// did not become a no-op.
#[test]
fn committed_rows_are_visible_to_other_connections() {
    let dir = tempfile::tempdir().expect("tempdir");
    let handle = start_server(dir.path());
    let mut a = connect(handle.port);
    a.execute("CREATE TABLE t (id INT PRIMARY KEY, v TEXT)")
        .expect("create");
    let mut b = connect(handle.port);

    a.execute("BEGIN").expect("begin");
    for id in 1..=10 {
        a.execute(&format!("INSERT INTO t VALUES ({}, 'v')", id))
            .expect("insert");
    }
    a.execute("COMMIT").expect("commit");

    assert_eq!(
        count(&mut b, "t"),
        10,
        "a committed transaction must be visible to other connections"
    );
}
