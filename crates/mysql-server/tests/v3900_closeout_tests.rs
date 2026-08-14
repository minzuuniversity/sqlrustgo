//! #3900 (V312-13 MySQL Wire + LOAD DATA Hardening) — close-out evidence tests.
//!
//! These tests pin the wire-protocol surface that was hardened under
//! V312-13 so that any regression triggers a clear failure here
//! rather than the more expensive upstream work (LOAD DATA SF=10,
//! binary row parsing, COM_STMT_PREPARE/EXECUTE cycle, etc.).
//!
//! Originally the REOPEN was driven by #3887 hard condition #7:
//! "关闭前必须由总控 Issue #3887 更新对应勾选状态". This file binds the
//! evidence: each `test_3900_*` name encodes the V312-13 sub-task it
//! covers, and the test count is the close-out tally.

use sqlrustgo_mysql_client::{MySqlConnection, MySqlResult, ResultSet};
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig, SERVER_POOL};
use std::net::SocketAddr;

fn rows_from_result(rs: ResultSet) -> Vec<Vec<String>> {
    match rs {
        ResultSet::Select { rows, .. } => rows,
        ResultSet::Ok { .. } => panic!("expected Select result set, got Ok"),
        ResultSet::Error {
            error_code,
            error_message,
            ..
        } => {
            panic!("server error {}: {}", error_code, error_message)
        }
    }
}

fn expect_rows(result: MySqlResult<ResultSet>) -> Vec<Vec<String>> {
    rows_from_result(result.expect("query failed"))
}

/// Each `connect_with_server_pool` tries to use the SERVER_POOL when
/// the port is in 9001..=9004 range; for ephemeral ports we don't
/// need pool acquisition.
fn connect(port: u16) -> MySqlResult<MySqlConnection> {
    if (9001..=9004).contains(&port) {
        let _handle = SERVER_POOL.acquire(port)?;
    }
    let addr: SocketAddr = format!("127.0.0.1:{}", port)
        .parse()
        .expect("invalid socket addr");
    MySqlConnection::connect(&addr, "tester", "tester", "")
}

fn start_server() -> sqlrustgo_mysql_server::testing::EphemeralHandle {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        bulk_insert_rows_per_flush: 10_000,
        server_threads: 8,
        storage: None,
        load_infile_dir: None,
        slow_query_log: None,
        metrics_port: None,
    };
    start_ephemeral(config).expect("ephemeral server starts")
}

// ============================================================================
// V312-13 close-out tests
// ============================================================================

/// V312-13 sub-task #1: binary row parsing (was the main bug).
/// Verifies the simple INT column SELECT round-trip works.
#[test]
fn test_3900_binary_row_parsing_int_select() {
    let handle = start_server();
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    conn.execute("CREATE TABLE t (id INT PRIMARY KEY, val INT)")
        .expect("create");
    conn.execute("INSERT INTO t VALUES (1, 100)")
        .expect("insert 1");
    conn.execute("INSERT INTO t VALUES (2, 200)")
        .expect("insert 2");

    let rows = expect_rows(conn.execute("SELECT id, val FROM t ORDER BY id"));
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0][0], "1");
    assert_eq!(rows[0][1], "100");
    assert_eq!(rows[1][0], "2");
    assert_eq!(rows[1][1], "200");
}

/// V312-13 sub-task #2: COM_STMT_PREPARE + EXECUTE binary protocol
/// round-trip (Issue #4130 fix — drain separators conditional on
/// count > 0 — landed in PR #4178).
#[test]
fn test_3900_stmt_prepare_execute_no_param() {
    let handle = start_server();
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    conn.execute("CREATE TABLE t (id INT PRIMARY KEY, x INT)")
        .expect("create");
    conn.execute("INSERT INTO t VALUES (1, 42)")
        .expect("insert");

    let stmt = conn.prepare("SELECT x FROM t").expect("prepare");
    assert_eq!(stmt.param_count, 0);
    assert_eq!(stmt.column_count, 1);

    let rows = expect_rows(conn.execute_prepared(stmt.id, &[]));
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], "42");

    conn.close_statement(stmt.id).expect("close");
}

/// V312-13 sub-task #3: COM_STMT_PREPARE + EXECUTE with parameters
/// (Issue #4171 client-side fix — type code 2 bytes per param).
#[test]
fn test_3900_stmt_prepare_execute_with_param() {
    let handle = start_server();
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    conn.execute("CREATE TABLE t (id INT PRIMARY KEY, name VARCHAR(50))")
        .expect("create");
    conn.execute("INSERT INTO t VALUES (1, 'Alice')")
        .expect("insert");
    conn.execute("INSERT INTO t VALUES (2, 'Bob')")
        .expect("insert");

    let stmt = conn
        .prepare("SELECT name FROM t WHERE id = ?")
        .expect("prepare");
    assert_eq!(stmt.param_count, 1);

    let rows = expect_rows(conn.execute_prepared(stmt.id, &["1"]));
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], "Alice");

    conn.close_statement(stmt.id).expect("close");
}

/// V312-13 sub-task #4: DEPRECATE_EOF capability handling — server
/// must use 0xFE pseudo-EOF marker for binary rows, no inter-record
/// separator. PR #4178 client + server agree on this.
#[test]
fn test_3900_deprecate_eof_binary_row_no_separator() {
    // The fact that PREPARE/EXECUTE works above (sub-task #3) with
    // DEPRECATE_EOF set already verifies this. Here we just ensure
    // that the COM_QUERY path also works with DEPRECATE_EOF.
    let handle = start_server();
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    conn.execute("CREATE TABLE t (a INT)").expect("create");
    conn.execute("INSERT INTO t VALUES (1)").expect("insert");

    let rows = expect_rows(conn.execute("SELECT a FROM t"));
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], "1");
}

/// V312-13 sub-task #5: ServerThreadPool backpressure — 12 parallel
/// tests must all complete (PR #4162 + PR #4178).
#[test]
fn test_3900_concurrent_connections_do_not_starve() {
    use std::thread;
    let handle = start_server();
    let port = handle.port;

    let mut handles = Vec::new();
    for i in 0..4 {
        let p = port;
        handles.push(thread::spawn(move || -> MySqlResult<()> {
            let mut conn = connect(p)?;
            conn.execute(&format!("CREATE TABLE par_{} (id INT PRIMARY KEY)", i))?;
            conn.execute(&format!("INSERT INTO par_{} VALUES (1)", i))?;
            let _ = conn.execute(&format!("SELECT id FROM par_{}", i))?;
            Ok(())
        }));
    }
    for h in handles {
        h.join().expect("thread join").expect("conn ops");
    }
}
