//! MySQL Client E2E Tests
//!
//! End-to-end tests for `sqlrustgo-mysql-client` against a real
//! `sqlrustgo-mysql-server` started via the `start_ephemeral` test
//! harness.
//!
//! 已知问题: server 端的 column_def 包格式不规范 (缺少 org_name 和
//! length_of_fixed_fields 字段), 暂时阻断 SELECT 行解析. INSERT/UPDATE/
//! DELETE 通过 OK 包路径正常. 等 server 修复后再启用 SELECT 测试.

use sqlrustgo_mysql_client::{MySqlConnection, ResultSet};
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};
use std::net::SocketAddr;

/// Wait for the server to be reachable on the ephemeral port.
fn wait_for_server(port: u16) -> SocketAddr {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        if let Ok(stream) = std::net::TcpStream::connect(("127.0.0.1", port)) {
            drop(stream);
            return SocketAddr::from(([127, 0, 0, 1], port));
        }
        if std::time::Instant::now() >= deadline {
            panic!("Server at 127.0.0.1:{} not reachable", port);
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

/// Start an ephemeral server and return a connected client.
fn make_client() -> (
    sqlrustgo_mysql_server::testing::EphemeralHandle,
    MySqlConnection,
) {
    let handle = start_ephemeral(EphemeralConfig::default()).expect("start_ephemeral");
    let port = handle.port;
    let addr = wait_for_server(port);
    let conn = MySqlConnection::connect(&addr, "tester", "tester", "").expect("connect");
    (handle, conn)
}

// =============================================================================
// Handshake + Authentication
// =============================================================================

#[test]
fn test_handshake_and_connect() {
    let (_handle, conn) = make_client();
    assert!(!conn.server_version.is_empty(), "server_version is empty");
    println!("Connected to server version: {}", conn.server_version);
}

#[test]
fn test_ping() {
    let (_handle, mut conn) = make_client();
    conn.ping().expect("ping should succeed");
}

// =============================================================================
// CREATE TABLE / INSERT / UPDATE / DELETE (工作正常)
// =============================================================================

#[test]
fn test_create_table() {
    let (_handle, mut conn) = make_client();
    let r = conn.execute("CREATE TABLE t1 (id INTEGER PRIMARY KEY, name TEXT, val INTEGER)");
    match r {
        Ok(ResultSet::Ok { .. }) => {}
        other => panic!("CREATE TABLE expected Ok, got {:?}", other),
    }
}

#[test]
fn test_insert_returns_ok() {
    let (_handle, mut conn) = make_client();
    conn.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT)")
        .expect("CREATE TABLE");
    let r = conn
        .execute("INSERT INTO t VALUES (1, 'alice'), (2, 'bob'), (3, 'carol')")
        .expect("INSERT");
    match r {
        ResultSet::Ok { affected_rows, .. } => {
            assert!(affected_rows >= 1, "affected_rows = {}", affected_rows);
        }
        other => panic!("INSERT expected Ok, got {:?}", other),
    }
}

#[test]
fn test_update_returns_ok() {
    let (_handle, mut conn) = make_client();
    conn.execute("CREATE TABLE t (id INTEGER, val TEXT)")
        .expect("CREATE");
    conn.execute("INSERT INTO t VALUES (1, 'old'), (2, 'old')")
        .expect("INSERT");
    let r = conn
        .execute("UPDATE t SET val = 'new' WHERE id = 1")
        .expect("UPDATE");
    match r {
        ResultSet::Ok { affected_rows, .. } => {
            assert!(affected_rows >= 1, "affected_rows = {}", affected_rows);
        }
        other => panic!("UPDATE expected Ok, got {:?}", other),
    }
}

#[test]
fn test_delete_returns_ok() {
    let (_handle, mut conn) = make_client();
    conn.execute("CREATE TABLE t (id INTEGER, name TEXT)")
        .expect("CREATE");
    conn.execute("INSERT INTO t VALUES (1, 'a'), (2, 'b'), (3, 'c')")
        .expect("INSERT");
    let r = conn.execute("DELETE FROM t WHERE id = 2").expect("DELETE");
    match r {
        ResultSet::Ok { affected_rows, .. } => {
            assert!(affected_rows >= 1, "affected_rows = {}", affected_rows);
        }
        other => panic!("DELETE expected Ok, got {:?}", other),
    }
}

// =============================================================================
// SELECT — currently disabled (server column_def bug)
// Run with `cargo test -- --ignored` once server is fixed.
// =============================================================================

#[test]
fn test_select_returns_rows() {
    let (_handle, mut conn) = make_client();
    conn.execute("CREATE TABLE t (id INTEGER, name TEXT)")
        .expect("CREATE");
    conn.execute("INSERT INTO t VALUES (1, 'alice'), (2, 'bob')")
        .expect("INSERT");

    let r = conn
        .execute("SELECT id, name FROM t ORDER BY id")
        .expect("SELECT");
    match r {
        ResultSet::Select { columns, rows } => {
            assert_eq!(columns.len(), 2, "expected 2 columns");
            assert!(columns[0].name.eq_ignore_ascii_case("id"));
            assert!(columns[1].name.eq_ignore_ascii_case("name"));
            assert_eq!(rows.len(), 2, "expected 2 rows");
            assert_eq!(rows[0][0], "1");
            assert_eq!(rows[0][1], "alice");
        }
        other => panic!("SELECT expected Select result, got {:?}", other),
    }
}

#[test]
fn test_select_with_where() {
    let (_handle, mut conn) = make_client();
    conn.execute("CREATE TABLE t (id INTEGER, name TEXT)")
        .expect("CREATE");
    conn.execute("INSERT INTO t VALUES (1, 'a'), (2, 'b'), (3, 'c')")
        .expect("INSERT");
    let r = conn
        .execute("SELECT name FROM t WHERE id = 2")
        .expect("SELECT");
    match r {
        ResultSet::Select { columns, rows } => {
            assert_eq!(columns.len(), 1);
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0][0], "b");
        }
        other => panic!("Expected Select, got {:?}", other),
    }
}

#[test]
fn test_multiple_sequential_queries() {
    let (_handle, mut conn) = make_client();
    conn.execute("CREATE TABLE seq_test (n INTEGER)")
        .expect("CREATE");
    for i in 1..=10 {
        let sql = format!("INSERT INTO seq_test VALUES ({})", i);
        conn.execute(&sql).expect("INSERT");
    }
    let r = conn.execute("SELECT SUM(n) FROM seq_test").expect("SELECT");
    match r {
        ResultSet::Select { rows, .. } => {
            assert_eq!(rows[0][0], "55", "sum of 1..=10 should be 55");
        }
        other => panic!("Expected Select, got {:?}", other),
    }
}

// =============================================================================
// Error handling
// =============================================================================

#[test]
fn test_invalid_sql_returns_error() {
    let (_handle, mut conn) = make_client();
    let r = conn.execute("SELECT FROM nonexistent_table");
    match r {
        Ok(ResultSet::Error {
            error_code,
            error_message,
            ..
        }) => {
            assert!(error_code > 0, "error_code should be non-zero");
            assert!(!error_message.is_empty(), "error_message is empty");
        }
        Ok(ResultSet::Ok { .. }) => panic!("Expected error, got Ok"),
        Ok(ResultSet::Select { .. }) => panic!("Expected error, got Select"),
        Err(e) => {
            println!("Got protocol error: {}", e);
        }
    }
}

#[test]
fn test_close_after_queries() {
    let (_handle, conn) = make_client();
    let mut conn = conn;
    // Use INSERT/DELETE to avoid the SELECT column_def parser issue
    conn.execute("CREATE TABLE close_test (n INTEGER)")
        .expect("CREATE");
    conn.execute("INSERT INTO close_test VALUES (1)")
        .expect("INSERT 1");
    conn.execute("INSERT INTO close_test VALUES (2)")
        .expect("INSERT 2");
    conn.execute("DELETE FROM close_test WHERE n = 2")
        .expect("DELETE");
    conn.close().expect("close");
}
