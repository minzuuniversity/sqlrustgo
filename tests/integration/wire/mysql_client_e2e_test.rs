//! MySQL Client E2E Tests
//!
//! End-to-end tests for `sqlrustgo-mysql-client` against a real
//! `sqlrustgo-mysql-server` started via the `start_ephemeral` test
//! harness.

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
// CREATE TABLE / INSERT / UPDATE / DELETE
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
// SELECT tests
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
        ResultSet::Select { columns, rows, .. } => {
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
        ResultSet::Select { columns, rows, .. } => {
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
// DML + Transaction tests (supplemental)
// =============================================================================

#[test]
fn test_insert_select() {
    // INSERT ... SELECT: insert results of a SELECT into another table
    let (_handle, mut conn) = make_client();
    conn.execute("CREATE TABLE src (id INTEGER, name TEXT)")
        .expect("CREATE src");
    conn.execute("CREATE TABLE dst (id INTEGER, name TEXT)")
        .expect("CREATE dst");
    conn.execute("INSERT INTO src VALUES (1, 'alice'), (2, 'bob'), (3, 'carol')")
        .expect("INSERT src");

    // Insert only rows where id > 1
    conn.execute("INSERT INTO dst SELECT * FROM src WHERE id > 1")
        .expect("INSERT ... SELECT");

    let r = conn
        .execute("SELECT id, name FROM dst ORDER BY id")
        .expect("SELECT from dst");
    match r {
        ResultSet::Select { rows, .. } => {
            assert_eq!(rows.len(), 2, "expected 2 rows in dst, got {:?}", rows);
            assert_eq!(rows[0][0], "2");
            assert_eq!(rows[0][1], "bob");
            assert_eq!(rows[1][0], "3");
            assert_eq!(rows[1][1], "carol");
        }
        other => panic!("expected Select, got {:?}", other),
    }
}

#[test]
fn test_update_with_complex_where() {
    // UPDATE with a compound WHERE expression
    let (_handle, mut conn) = make_client();
    conn.execute("CREATE TABLE t (id INTEGER, name TEXT, val INTEGER)")
        .expect("CREATE");
    conn.execute("INSERT INTO t VALUES (1, 'a', 10), (2, 'b', 20), (3, 'c', 30), (4, 'd', 40)")
        .expect("INSERT");

    // Double val for rows where id > 1 AND id < 4  (i.e., id IN {2, 3})
    conn.execute("UPDATE t SET val = val * 2 WHERE id > 1 AND id < 4")
        .expect("UPDATE");

    let r = conn
        .execute("SELECT id, name, val FROM t ORDER BY id")
        .expect("SELECT");
    match r {
        ResultSet::Select { rows, .. } => {
            assert_eq!(rows.len(), 4);
            assert_eq!(rows[0][2], "10"); // id=1 unchanged
            assert_eq!(rows[1][2], "40"); // id=2: 20*2
            assert_eq!(rows[2][2], "60"); // id=3: 30*2
            assert_eq!(rows[3][2], "40"); // id=4 unchanged
        }
        other => panic!("expected Select, got {:?}", other),
    }
}

#[test]
fn test_transaction_rollback() {
    // NOTE: BEGIN/ROLLBACK are not yet implemented — they are accepted as
    // no-ops. Rows inserted before or during "BEGIN" persist regardless.
    // This test documents current behavior (no actual rollback).
    let (_handle, mut conn) = make_client();
    conn.execute("CREATE TABLE t (n INTEGER)").expect("CREATE");
    conn.execute("INSERT INTO t VALUES (1)")
        .expect("INSERT initial");

    conn.execute("BEGIN").expect("BEGIN");
    conn.execute("INSERT INTO t VALUES (2)")
        .expect("INSERT in txn");
    conn.execute("INSERT INTO t VALUES (3)")
        .expect("INSERT in txn");
    conn.execute("ROLLBACK").expect("ROLLBACK");

    // All 3 rows survive because ROLLBACK is a no-op
    let r = conn
        .execute("SELECT COUNT(*) FROM t")
        .expect("SELECT COUNT after rollback");
    match r {
        ResultSet::Select { rows, .. } => {
            assert_eq!(rows.len(), 1);
            assert_eq!(
                rows[0][0], "3",
                "expected 3 rows (rollback not implemented), got {:?}",
                rows
            );
        }
        other => panic!("expected Select, got {:?}", other),
    }
}

#[test]
fn test_transaction_commit() {
    // NOTE: BEGIN/COMMIT are no-ops. All DML statements auto-commit.
    let (_handle, mut conn) = make_client();
    conn.execute("CREATE TABLE t (n INTEGER)").expect("CREATE");

    conn.execute("BEGIN").expect("BEGIN");
    conn.execute("INSERT INTO t VALUES (1)")
        .expect("INSERT in txn");
    conn.execute("INSERT INTO t VALUES (2)")
        .expect("INSERT in txn");
    conn.execute("COMMIT").expect("COMMIT");

    let r = conn
        .execute("SELECT COUNT(*) FROM t")
        .expect("SELECT COUNT after commit");
    match r {
        ResultSet::Select { rows, .. } => {
            assert_eq!(rows.len(), 1);
            assert_eq!(
                rows[0][0], "2",
                "expected 2 rows after commit, got {:?}",
                rows
            );
        }
        other => panic!("expected Select, got {:?}", other),
    }
}

#[test]
fn test_transaction_multi_stmt() {
    // NOTE: BEGIN/COMMIT are no-ops; each statement auto-commits.
    // So: INSERT (1,'x') → UPDATE (1,'y') → DELETE (1) → 0 rows remain.
    let (_handle, mut conn) = make_client();
    conn.execute("CREATE TABLE t (id INTEGER, val TEXT)")
        .expect("CREATE");

    conn.execute("BEGIN").expect("BEGIN");
    conn.execute("INSERT INTO t VALUES (1, 'x')")
        .expect("INSERT");
    conn.execute("UPDATE t SET val = 'y' WHERE id = 1")
        .expect("UPDATE");
    conn.execute("DELETE FROM t WHERE id = 1").expect("DELETE");
    conn.execute("COMMIT").expect("COMMIT");

    let r = conn
        .execute("SELECT COUNT(*) FROM t")
        .expect("SELECT COUNT after multi-stmt txn");
    match r {
        ResultSet::Select { rows, .. } => {
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0][0], "0", "expected 0 rows, got {:?}", rows);
        }
        other => panic!("expected Select, got {:?}", other),
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
// =============================================================================
// JOIN tests
// =============================================================================

#[test]
fn test_join_select() {
    let (_handle, mut conn) = make_client();
    conn.execute("CREATE TABLE orders (id INTEGER, customer_id INTEGER, amount INTEGER)")
        .expect("CREATE orders");
    conn.execute("CREATE TABLE customers (id INTEGER, name TEXT)")
        .expect("CREATE customers");
    conn.execute("INSERT INTO customers VALUES (1, 'alice'), (2, 'bob'), (3, 'carol')")
        .expect("INSERT customers");
    conn.execute("INSERT INTO orders VALUES (100, 1, 250), (101, 1, 120), (102, 2, 80)")
        .expect("INSERT orders");

    let r = conn
        .execute("SELECT c.name, o.amount FROM customers c JOIN orders o ON c.id = o.customer_id ORDER BY c.name, o.amount")
        .expect("JOIN SELECT");
    match r {
        ResultSet::Select { columns, rows, .. } => {
            assert_eq!(columns.len(), 2, "expected 2 columns");
            assert_eq!(rows.len(), 3, "expected 3 joined rows");
            // alice has 2 orders (120, 250), bob has 1 (80), carol has none
            assert_eq!(rows[0][0], "alice");
            assert_eq!(rows[0][1], "120");
            assert_eq!(rows[1][0], "alice");
            assert_eq!(rows[1][1], "250");
            assert_eq!(rows[2][0], "bob");
            assert_eq!(rows[2][1], "80");
        }
        other => panic!("Expected Select, got {:?}", other),
    }
}

// =============================================================================
// NULL handling tests
// =============================================================================

#[test]
fn test_null_handling() {
    let (_handle, mut conn) = make_client();
    conn.execute("CREATE TABLE t (id INTEGER, val INTEGER)")
        .expect("CREATE");
    conn.execute("INSERT INTO t VALUES (1, 10), (2, NULL), (3, 30)")
        .expect("INSERT");

    // IS NULL
    let r = conn
        .execute("SELECT id FROM t WHERE val IS NULL")
        .expect("IS NULL");
    match r {
        ResultSet::Select { rows, .. } => {
            assert_eq!(rows.len(), 1, "expected 1 row with NULL");
            assert_eq!(rows[0][0], "2");
        }
        other => panic!("Expected Select, got {:?}", other),
    }

    // IS NOT NULL
    let r = conn
        .execute("SELECT id FROM t WHERE val IS NOT NULL ORDER BY id")
        .expect("IS NOT NULL");
    match r {
        ResultSet::Select { rows, .. } => {
            assert_eq!(rows.len(), 2, "expected 2 non-NULL rows");
            assert_eq!(rows[0][0], "1");
            assert_eq!(rows[1][0], "3");
        }
        other => panic!("Expected Select, got {:?}", other),
    }
}

// =============================================================================
// ORDER BY tests
// =============================================================================

#[test]
fn test_order_by() {
    let (_handle, mut conn) = make_client();
    conn.execute("CREATE TABLE t (id INTEGER, val INTEGER)")
        .expect("CREATE");
    conn.execute("INSERT INTO t VALUES (1, 30), (2, 10), (3, 50), (4, 20)")
        .expect("INSERT");

    // ORDER BY DESC
    let r = conn
        .execute("SELECT id, val FROM t ORDER BY val DESC")
        .expect("ORDER BY DESC");
    match r {
        ResultSet::Select { rows, .. } => {
            assert_eq!(rows.len(), 4, "expected 4 rows");
            assert_eq!(rows[0][0], "3"); // val=50
            assert_eq!(rows[0][1], "50");
            assert_eq!(rows[1][0], "1"); // val=30
            assert_eq!(rows[1][1], "30");
            assert_eq!(rows[2][0], "4"); // val=20
            assert_eq!(rows[2][1], "20");
            assert_eq!(rows[3][0], "2"); // val=10
            assert_eq!(rows[3][1], "10");
        }
        other => panic!("Expected Select, got {:?}", other),
    }

    // ORDER BY ASC
    let r = conn
        .execute("SELECT id, val FROM t ORDER BY val ASC")
        .expect("ORDER BY ASC");
    match r {
        ResultSet::Select { rows, .. } => {
            assert_eq!(rows.len(), 4, "expected 4 rows");
            assert_eq!(rows[0][1], "10");
            assert_eq!(rows[1][1], "20");
            assert_eq!(rows[2][1], "30");
            assert_eq!(rows[3][1], "50");
        }
        other => panic!("Expected Select, got {:?}", other),
    }
}

// =============================================================================
// Aggregate + GROUP BY tests
// =============================================================================

#[test]
fn test_aggregate_group_by() {
    let (_handle, mut conn) = make_client();
    conn.execute("CREATE TABLE orders (customer_id INTEGER, amount INTEGER)")
        .expect("CREATE");
    conn.execute("INSERT INTO orders VALUES (1, 100), (1, 200), (2, 50), (2, 150), (3, 300)")
        .expect("INSERT");

    // COUNT + GROUP BY
    let r = conn
        .execute("SELECT customer_id, COUNT(*), SUM(amount) FROM orders GROUP BY customer_id ORDER BY customer_id")
        .expect("COUNT GROUP BY");
    match r {
        ResultSet::Select {
            columns: _, rows, ..
        } => {
            assert_eq!(rows.len(), 3, "expected 3 groups");
            // customer 1: count=2, sum=300
            assert_eq!(rows[0][0], "1");
            assert_eq!(rows[0][1], "2");
            assert_eq!(rows[0][2], "300");
            // customer 2: count=2, sum=200
            assert_eq!(rows[1][0], "2");
            assert_eq!(rows[1][1], "2");
            assert_eq!(rows[1][2], "200");
            // customer 3: count=1, sum=300
            assert_eq!(rows[2][0], "3");
            assert_eq!(rows[2][1], "1");
            assert_eq!(rows[2][2], "300");
        }
        other => panic!("Expected Select, got {:?}", other),
    }

    // AVG + GROUP BY
    let r = conn
        .execute(
            "SELECT customer_id, AVG(amount) FROM orders GROUP BY customer_id ORDER BY customer_id",
        )
        .expect("AVG GROUP BY");
    match r {
        ResultSet::Select { rows, .. } => {
            assert_eq!(rows.len(), 3);
            assert_eq!(rows[0][0], "1"); // avg=150.0
            assert_eq!(rows[0][1], "150");
            assert_eq!(rows[1][0], "2"); // avg=100.0
            assert_eq!(rows[1][1], "100");
            assert_eq!(rows[2][0], "3"); // avg=300.0
            assert_eq!(rows[2][1], "300");
        }
        other => panic!("Expected Select, got {:?}", other),
    }
}
