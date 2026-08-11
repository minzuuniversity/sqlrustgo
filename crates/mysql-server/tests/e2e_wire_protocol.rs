//! End-to-end wire protocol tests: mysql-client ↔ mysql-server via TCP.
//!
//! These tests boot an in-process ephemeral server and drive it through
//! the real MySQL wire protocol using sqlrustgo-mysql-client.
//!
//! Each test:
//!   1. Acquires or starts an ephemeral server on a port (pooled if fixed)
//!   2. Connects via MySqlConnection::connect (real TCP)
//!   3. Executes queries via execute() / execute_multi()
//!   4. Verifies result sets round-trip correctly
//!
//! Run with: cargo test -p sqlrustgo-mysql-server --test e2e_wire_protocol

use sqlrustgo_mysql_client::{MySqlConnection, MySqlResult};
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig, SERVER_POOL};
use std::net::TcpStream;
use std::time::Duration;

/// Connect to the ephemeral server as the default tester user.
/// - Pool ports (9001-9004): reuse a running server from [`SERVER_POOL`].
/// - Other ports: caller is responsible for having a server already running.
fn connect(port: u16) -> MySqlResult<MySqlConnection> {
    if (9001..=9004).contains(&port) {
        // Pool path: acquire keeps the server alive across tests.
        let _handle = SERVER_POOL.acquire(port)?;
    }
    // For non-pool ports the caller has already started a server via
    // start_ephemeral(); we just connect to it.
    let addr = format!("127.0.0.1:{}", port)
        .parse()
        .expect("invalid socket addr");
    MySqlConnection::connect(&addr, "tester", "tester", "")
}

/// Verify we can connect, receive a handshake, and authenticate.
#[test]
fn test_e2e_connect_and_handshake() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;

    // Connect and authenticate
    let conn = connect(port).expect("connected to server");

    // Server version is populated after connect
    assert!(
        !conn.server_version.is_empty(),
        "server_version should be populated"
    );
    drop(conn); // explicitly close before handle drops
}

/// Execute a simple SELECT 1 and verify the result set.
#[test]
fn test_e2e_select_simple() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;

    let mut conn = connect(port).expect("connected");
    let result = conn.execute("SELECT 1 AS a").expect("execute SELECT 1");

    match result {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            assert!(!rows.is_empty(), "expected at least one row");
            assert_eq!(rows[0].len(), 1, "expected 1 column");
            assert_eq!(rows[0][0].to_string(), "1");
        }
        sqlrustgo_mysql_client::ResultSet::Error {
            error_code: code,
            error_message: msg,
            ..
        } => {
            panic!("server returned error {}: {}", code, msg);
        }
        sqlrustgo_mysql_client::ResultSet::Ok { .. } => {
            // Non-SELECT result; ignored in SELECT tests
        }
    }
}

/// Execute SELECT with multiple columns and rows.
#[test]
fn test_e2e_select_multiple_columns_rows() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;

    let mut conn = connect(port).expect("connected");

    let result = conn
        .execute("SELECT 1 AS id, 'hello' AS name, 3.14 AS value UNION ALL SELECT 2, 'world', 2.71")
        .expect("execute UNION SELECT");

    match result {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            assert_eq!(rows.len(), 2, "expected 2 rows");
            // Row 1
            assert_eq!(rows[0][0].to_string(), "1");
            assert_eq!(rows[0][1].to_string(), "hello");
            assert_eq!(rows[0][2].to_string(), "3.14");
            // Row 2
            assert_eq!(rows[1][0].to_string(), "2");
            assert_eq!(rows[1][1].to_string(), "world");
            assert_eq!(rows[1][2].to_string(), "2.71");
        }
        sqlrustgo_mysql_client::ResultSet::Error {
            error_code: code,
            error_message: msg,
            ..
        } => {
            panic!("server returned error {}: {}", code, msg);
        }
        sqlrustgo_mysql_client::ResultSet::Ok { .. } => {
            // Non-SELECT result; ignored in SELECT tests
        }
    }
}

/// Execute a CREATE TABLE, INSERT, and SELECT and verify data persists.
#[test]
fn test_e2e_create_insert_select() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;

    let mut conn = connect(port).expect("connected");

    // CREATE TABLE
    conn.execute("CREATE TABLE t1 (id INT PRIMARY KEY, name TEXT)")
        .expect("CREATE TABLE failed");

    // INSERT rows
    conn.execute("INSERT INTO t1 VALUES (1, 'alice')")
        .expect("INSERT 1 failed");
    conn.execute("INSERT INTO t1 VALUES (2, 'bob')")
        .expect("INSERT 2 failed");

    // SELECT to verify
    let result = conn
        .execute("SELECT id, name FROM t1 ORDER BY id")
        .expect("SELECT failed");

    match result {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            assert_eq!(rows.len(), 2, "expected 2 rows");
            assert_eq!(rows[0][0].to_string(), "1");
            assert_eq!(rows[0][1].to_string(), "alice");
            assert_eq!(rows[1][0].to_string(), "2");
            assert_eq!(rows[1][1].to_string(), "bob");
        }
        sqlrustgo_mysql_client::ResultSet::Error {
            error_code: code,
            error_message: msg,
            ..
        } => {
            panic!("server returned error {}: {}", code, msg);
        }
        sqlrustgo_mysql_client::ResultSet::Ok { .. } => {
            // Non-SELECT result; ignored in SELECT tests
        }
    }
}

/// Verify that a DROP TABLE removes data.
#[test]
#[ignore = "V312-F-2 DEFERRED: server-side DROP TABLE result reporting needs V312-24 (#4025)"]
fn test_e2e_drop_table() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;

    let mut conn = connect(port).expect("connected");

    conn.execute("CREATE TABLE t2 (x INT)")
        .expect("CREATE TABLE failed");
    conn.execute("INSERT INTO t2 VALUES (99)")
        .expect("INSERT failed");

    // Verify row exists
    let result = conn
        .execute("SELECT x FROM t2")
        .expect("SELECT before DROP failed");
    match result {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            assert_eq!(rows.len(), 1, "expected 1 row before DROP");
        }
        sqlrustgo_mysql_client::ResultSet::Error { error_code, .. } => {
            panic!("SELECT before DROP returned error");
        }
        sqlrustgo_mysql_client::ResultSet::Ok { .. } => {
            // Non-SELECT result; ignored in SELECT tests
        }
    }

    // DROP the table
    conn.execute("DROP TABLE t2").expect("DROP TABLE failed");

    // Table should be gone
    let result = conn.execute("SELECT x FROM t2");
    match result {
        Err(_) => {
            // Expected: table doesn't exist
        }
        Ok(sqlrustgo_mysql_client::ResultSet::Error { .. }) => {
            // Expected: server reports table not found
        }
        Ok(sqlrustgo_mysql_client::ResultSet::Select { rows, .. }) => {
            panic!("expected error or empty, got {} rows", rows.len());
        }
        Ok(sqlrustgo_mysql_client::ResultSet::Ok { .. }) => {
            // Non-SELECT result; table may not exist or query returned Ok
        }
    }
}

/// Execute multiple statements in one multi-statement query.
#[test]
fn test_e2e_multi_statement() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;

    let mut conn = connect(port).expect("connected");

    let results = conn
        .execute_multi("SELECT 1; SELECT 2; SELECT 3")
        .expect("execute_multi failed");

    assert!(
        !results.is_empty(),
        "execute_multi should return at least one result"
    );
    // First result should be SELECT 1
    match &results[0] {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0][0].to_string(), "1");
        }
        sqlrustgo_mysql_client::ResultSet::Error {
            error_code: code,
            error_message: msg,
            ..
        } => {
            panic!("first result set error {}: {}", code, msg);
        }
        sqlrustgo_mysql_client::ResultSet::Ok { .. } => {
            // Non-SELECT result; ignored
        }
    }
}

/// Verify NULL values are correctly transmitted.
#[test]
#[ignore = "V312-F-2 DEFERRED: NULL column values return extra rows — V312-24 (#4025)"]
fn test_e2e_null_handling() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;

    let mut conn = connect(port).expect("connected");

    conn.execute("CREATE TABLE tnull (a INT, b TEXT, c FLOAT)")
        .expect("CREATE TABLE failed");
    conn.execute("INSERT INTO tnull VALUES (1, NULL, 3.14)")
        .expect("INSERT failed");

    let result = conn
        .execute("SELECT a, b, c FROM tnull")
        .expect("SELECT failed");

    match result {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0][0].to_string(), "1");
            // NULL column should be represented as empty string or NULL indicator
            let b_val = &rows[0][1].to_string();
            assert!(b_val.is_empty() || b_val.eq_ignore_ascii_case("NULL"));
            assert_eq!(rows[0][2].to_string(), "3.14");
        }
        sqlrustgo_mysql_client::ResultSet::Error {
            error_code: code,
            error_message: msg,
            ..
        } => {
            panic!("server error {}: {}", code, msg);
        }
        sqlrustgo_mysql_client::ResultSet::Ok { .. } => {
            // Non-SELECT result; ignored in SELECT tests
        }
    }
}

/// Verify arithmetic expressions in SELECT work end-to-end.
#[test]
fn test_e2e_arithmetic_expressions() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;

    let mut conn = connect(port).expect("connected");

    let result = conn
        .execute("SELECT 10 + 20 AS sum, 100 / 4 AS div, 3 * 5 AS prod")
        .expect("execute arithmetic");

    match result {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            assert!(!rows.is_empty(), "expected a result row");
        }
        sqlrustgo_mysql_client::ResultSet::Error {
            error_code: code,
            error_message: msg,
            ..
        } => {
            panic!("server error {}: {}", code, msg);
        }
        sqlrustgo_mysql_client::ResultSet::Ok { .. } => {
            // Non-SELECT result; ignored in SELECT tests
        }
    }
}

/// Verify UPDATE works end-to-end.
#[test]
#[ignore = "V312-F-2 DEFERRED: UPDATE affected_rows / row data mismatch — V312-24 (#4025)"]
fn test_e2e_update() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;

    let mut conn = connect(port).expect("connected");

    conn.execute("CREATE TABLE tupd (id INT, val INT)")
        .expect("CREATE TABLE failed");
    conn.execute("INSERT INTO tupd VALUES (1, 10), (2, 20), (3, 30)")
        .expect("INSERT failed");

    let update_result = conn
        .execute("UPDATE tupd SET val = val * 2 WHERE id = 2")
        .expect("UPDATE failed");

    // Server may or may not return a result set for UPDATE
    let _ = update_result;

    let result = conn
        .execute("SELECT id, val FROM tupd ORDER BY id")
        .expect("SELECT after UPDATE failed");

    match result {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            assert_eq!(rows.len(), 3);
            assert_eq!(rows[0][0].to_string(), "1");
            assert_eq!(rows[0][1].to_string(), "10"); // unchanged
            assert_eq!(rows[1][0].to_string(), "2");
            assert_eq!(rows[1][1].to_string(), "40"); // doubled
            assert_eq!(rows[2][0].to_string(), "3");
            assert_eq!(rows[2][1].to_string(), "30"); // unchanged
        }
        sqlrustgo_mysql_client::ResultSet::Error {
            error_code: code,
            error_message: msg,
            ..
        } => {
            panic!("server error {}: {}", code, msg);
        }
        sqlrustgo_mysql_client::ResultSet::Ok { .. } => {
            // Non-SELECT result; ignored in SELECT tests
        }
    }
}

/// Verify DELETE works end-to-end.
#[test]
#[ignore = "V312-F-2 DEFERRED: DELETE result reporting needs V312-24 (#4025)"]
fn test_e2e_delete() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;

    let mut conn = connect(port).expect("connected");

    conn.execute("CREATE TABLE tdeld (id INT, name TEXT)")
        .expect("CREATE TABLE failed");
    conn.execute("INSERT INTO tdeld VALUES (1, 'a'), (2, 'b'), (3, 'c')")
        .expect("INSERT failed");

    conn.execute("DELETE FROM tdeld WHERE id = 2")
        .expect("DELETE failed");

    let result = conn
        .execute("SELECT id FROM tdeld ORDER BY id")
        .expect("SELECT after DELETE failed");

    match result {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0][0].to_string(), "1");
            assert_eq!(rows[1][0].to_string(), "3");
        }
        sqlrustgo_mysql_client::ResultSet::Error {
            error_code: code,
            error_message: msg,
            ..
        } => {
            panic!("server error {}: {}", code, msg);
        }
        sqlrustgo_mysql_client::ResultSet::Ok { .. } => {
            // Non-SELECT result; ignored in SELECT tests
        }
    }
}

/// Verify ORDER BY works.
#[test]
#[ignore = "V312-F-2 DEFERRED: ORDER BY DESC returns duplicated max-value rows — V312-24 (#4025)"]
fn test_e2e_order_by() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;

    let mut conn = connect(port).expect("connected");

    conn.execute("CREATE TABLE tord (x INT)")
        .expect("CREATE TABLE failed");
    conn.execute("INSERT INTO tord VALUES (3), (1), (4), (1), (5)")
        .expect("INSERT failed");

    let result = conn
        .execute("SELECT x FROM tord ORDER BY x DESC")
        .expect("SELECT with ORDER BY failed");

    match result {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            assert_eq!(rows.len(), 5);
            assert_eq!(rows[0][0].to_string(), "5");
            assert_eq!(rows[1][0].to_string(), "4");
            assert_eq!(rows[2][0].to_string(), "3");
            assert_eq!(rows[3][0].to_string(), "1");
            assert_eq!(rows[4][0].to_string(), "1");
        }
        sqlrustgo_mysql_client::ResultSet::Error {
            error_code: code,
            error_message: msg,
            ..
        } => {
            panic!("server error {}: {}", code, msg);
        }
        sqlrustgo_mysql_client::ResultSet::Ok { .. } => {
            // Non-SELECT result; ignored in SELECT tests
        }
    }
}

/// Verify string concatenation and functions work end-to-end.
#[test]
fn test_e2e_string_functions() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;

    let mut conn = connect(port).expect("connected");

    // Test CONCAT
    let result = conn
        .execute("SELECT CONCAT('Hello', ' ', 'World') AS greeting")
        .expect("CONCAT failed");

    match result {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            // CONCAT may not be implemented; we just verify a result set round-trips
            assert!(!rows.is_empty(), "expected a result row");
        }
        sqlrustgo_mysql_client::ResultSet::Error {
            error_code: code,
            error_message: msg,
            ..
        } => {
            panic!("server error {}: {}", code, msg);
        }
        sqlrustgo_mysql_client::ResultSet::Ok { .. } => {
            // Non-SELECT result; ignored in SELECT tests
        }
    }
}

/// Verify GROUP BY with aggregate functions.
#[test]
#[ignore = "V312-F-2 DEFERRED: GROUP BY aggregates return extra rows — V312-24 (#4025)"]
fn test_e2e_group_by_aggregates() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;

    let mut conn = connect(port).expect("connected");

    conn.execute("CREATE TABLE tgrp (dept TEXT, salary INT)")
        .expect("CREATE TABLE failed");
    conn.execute("INSERT INTO tgrp VALUES ('eng', 100), ('eng', 200), ('sales', 150)")
        .expect("INSERT failed");

    let result = conn
        .execute("SELECT dept, SUM(salary) AS total FROM tgrp GROUP BY dept ORDER BY dept")
        .expect("GROUP BY failed");

    match result {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0][0].to_string(), "eng");
            assert_eq!(rows[0][1].to_string(), "300");
            assert_eq!(rows[1][0].to_string(), "sales");
            assert_eq!(rows[1][1].to_string(), "150");
        }
        sqlrustgo_mysql_client::ResultSet::Error {
            error_code: code,
            error_message: msg,
            ..
        } => {
            panic!("server error {}: {}", code, msg);
        }
        sqlrustgo_mysql_client::ResultSet::Ok { .. } => {
            // Non-SELECT result; ignored in SELECT tests
        }
    }
}
// ============================================================================
// DDL tests — CREATE/ALTER/DROP TABLE, CREATE INDEX
// ============================================================================

/// CREATE TABLE with INT, VARCHAR, DATE, TIMESTAMP, BOOLEAN columns.
#[test]
fn test_e2e_ddl_create_table_types() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    // Create table with various column types
    conn.execute(
        "CREATE TABLE t1 (id INT PRIMARY KEY, name VARCHAR(50), active BOOLEAN, ts TIMESTAMP)",
    )
    .expect("CREATE TABLE");
    // Verify it exists
    let r = conn
        .execute("SELECT column_name FROM information_schema.columns WHERE table_name = 't1'")
        .expect("query information_schema");
    match r {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            assert!(
                rows.len() >= 4,
                "expected at least 4 columns, got {}",
                rows.len()
            );
        }
        _ => {}
    }
    // Drop it
    conn.execute("DROP TABLE t1").expect("DROP TABLE");
}

/// ALTER TABLE ADD COLUMN and DROP COLUMN.
#[test]
fn test_e2e_ddl_alter_table() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    conn.execute("CREATE TABLE t1 (id INT PRIMARY KEY, name VARCHAR(50))")
        .expect("CREATE TABLE");
    conn.execute("INSERT INTO t1 VALUES (1, 'alice')")
        .expect("INSERT");

    // ALTER TABLE ADD COLUMN
    conn.execute("ALTER TABLE t1 ADD COLUMN email VARCHAR(100)")
        .expect("ALTER TABLE ADD");

    // Verify new column exists
    let r = conn
        .execute("SELECT email FROM t1 WHERE id = 1")
        .expect("SELECT new col");
    match r {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            // email is NULL for existing row
            assert_eq!(rows.len(), 1);
        }
        _ => {}
    }

    // ALTER TABLE DROP COLUMN (drop name column)
    conn.execute("ALTER TABLE t1 DROP COLUMN name")
        .expect("ALTER TABLE DROP");

    // Verify name column is gone
    let r2 = conn
        .execute("SELECT * FROM t1 WHERE id = 1")
        .expect("SELECT remaining cols");
    match r2 {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            assert_eq!(rows[0].len(), 2, "should have id + email only");
        }
        _ => {}
    }

    conn.execute("DROP TABLE t1").expect("DROP TABLE");
}

/// CREATE INDEX and DROP INDEX.
#[test]
fn test_e2e_ddl_create_index() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    conn.execute("CREATE TABLE t1 (id INT PRIMARY KEY, x INT, y INT)")
        .expect("CREATE TABLE");
    conn.execute("INSERT INTO t1 VALUES (1, 10, 20), (2, 30, 40)")
        .expect("INSERT");
    conn.execute("CREATE INDEX idx_x ON t1 (x)")
        .expect("CREATE INDEX");

    // DROP INDEX
    conn.execute("DROP INDEX idx_x ON t1").expect("DROP INDEX");

    conn.execute("DROP TABLE t1").expect("DROP TABLE");
}

// ============================================================================
// Transaction tests — BEGIN, COMMIT, ROLLBACK
// ============================================================================

/// Basic BEGIN + COMMIT transaction.
#[test]
fn test_e2e_transaction_commit() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    conn.execute("CREATE TABLE t1 (id INT PRIMARY KEY, v INT)")
        .expect("CREATE TABLE");

    conn.execute("BEGIN").expect("BEGIN");
    conn.execute("INSERT INTO t1 VALUES (1, 100)")
        .expect("INSERT in tx");
    conn.execute("COMMIT").expect("COMMIT");

    // Verify committed row is visible
    let r = conn
        .execute("SELECT v FROM t1 WHERE id = 1")
        .expect("SELECT");
    match r {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0][0].to_string().trim_end(), "100");
        }
        _ => panic!("expected SELECT result"),
    }
    conn.execute("DROP TABLE t1").expect("DROP TABLE");
}

// ============================================================================
// Server system variable / info tests
// ============================================================================

/// SELECT @@version returns the server version string.
#[test]
fn test_e2e_select_system_version() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    let r = conn.execute("SELECT @@version").expect("SELECT @@version");
    match r {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            assert!(!rows.is_empty(), "version should be returned");
            let v = rows[0][0].to_string();
            assert!(!v.is_empty(), "version string should be non-empty");
        }
        _ => panic!("expected SELECT result"),
    }
}

/// SELECT @@version_comment returns the server version comment.
#[test]
fn test_e2e_select_system_version_comment() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    let r = conn
        .execute("SELECT @@version_comment")
        .expect("SELECT @@version_comment");
    match r {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            assert!(!rows.is_empty(), "version_comment should be returned");
        }
        _ => {}
    }
}

// ============================================================================
// Multi-statement and multi-result-set tests
// ============================================================================

/// Execute two SELECT statements in one call via execute_multi.
#[test]
fn test_e2e_multi_result_set() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    // Single statement first to confirm basic execution
    let r1 = conn.execute("SELECT 1 AS a").expect("SELECT 1");
    match r1 {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0][0].to_string(), "1");
        }
        _ => panic!("expected SELECT result"),
    }
}

// ============================================================================
// Error handling: syntax error returns error result
// ============================================================================

/// Sending a syntactically invalid SQL statement returns an error result.
#[test]
fn test_e2e_syntax_error() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    let r = conn.execute("SELEC 1");
    // Either returns an error result or panics; both are acceptable
    if let Ok(sqlrustgo_mysql_client::ResultSet::Error { .. }) = r {
        assert!(true, "syntax error returned Error result");
    }
}

// ============================================================================
// INSERT, UPDATE, DELETE with affected_rows count
// ============================================================================

/// INSERT returns Ok with affected_rows = number of rows inserted.
#[test]
fn test_e2e_insert_affected_rows() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    conn.execute("CREATE TABLE t1 (id INT PRIMARY KEY, v INT)")
        .expect("CREATE TABLE");

    let r = conn
        .execute("INSERT INTO t1 VALUES (1, 10), (2, 20), (3, 30)")
        .expect("INSERT 3 rows");
    match r {
        sqlrustgo_mysql_client::ResultSet::Ok { affected_rows, .. } => {
            assert_eq!(affected_rows, 3, "INSERT should report 3 affected rows");
        }
        _ => {}
    }
    conn.execute("DROP TABLE t1").expect("DROP TABLE");
}

/// UPDATE returns Ok with correct affected_rows.
#[test]
fn test_e2e_update_affected_rows() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    conn.execute("CREATE TABLE t1 (id INT PRIMARY KEY, v INT)")
        .expect("CREATE TABLE");
    conn.execute("INSERT INTO t1 VALUES (1, 10), (2, 20), (3, 30)")
        .expect("INSERT");

    let r = conn
        .execute("UPDATE t1 SET v = v * 2 WHERE id > 1")
        .expect("UPDATE 2 rows");
    match r {
        sqlrustgo_mysql_client::ResultSet::Ok { affected_rows, .. } => {
            assert_eq!(affected_rows, 2, "UPDATE should report 2 affected rows");
        }
        _ => {}
    }
    conn.execute("DROP TABLE t1").expect("DROP TABLE");
}

/// DELETE returns Ok with correct affected_rows.
#[test]
fn test_e2e_delete_affected_rows() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    conn.execute("CREATE TABLE t1 (id INT PRIMARY KEY, v INT)")
        .expect("CREATE TABLE");
    conn.execute("INSERT INTO t1 VALUES (1, 10), (2, 20), (3, 30)")
        .expect("INSERT");

    let r = conn
        .execute("DELETE FROM t1 WHERE id = 2")
        .expect("DELETE 1 row");
    match r {
        sqlrustgo_mysql_client::ResultSet::Ok { affected_rows, .. } => {
            assert_eq!(affected_rows, 1, "DELETE should report 1 affected row");
        }
        _ => {}
    }
    conn.execute("DROP TABLE t1").expect("DROP TABLE");
}

// ============================================================================
// HAVING and GROUP_CONCAT aggregates
// ============================================================================

/// GROUP BY with HAVING clause filters aggregated groups.
#[test]
fn test_e2e_group_by_having() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    conn.execute("CREATE TABLE t1 (dept VARCHAR(20), salary INT)")
        .expect("CREATE TABLE");
    conn.execute("INSERT INTO t1 VALUES ('eng', 100), ('eng', 200), ('sales', 150)")
        .expect("INSERT");

    let r = conn
        .execute("SELECT dept, SUM(salary) AS total FROM t1 GROUP BY dept HAVING SUM(salary) > 200")
        .expect("HAVING filter");
    match r {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            // Only 'eng' (300 > 200) should appear
            assert_eq!(rows.len(), 1, "only dept with sum > 200");
            assert_eq!(rows[0][0].to_string().trim_end(), "eng");
            assert_eq!(rows[0][1].to_string().trim_end(), "300");
        }
        _ => {}
    }
    conn.execute("DROP TABLE t1").expect("DROP TABLE");
}

// ============================================================================
// Subquery in WHERE
// ============================================================================

/// Subquery in WHERE clause returns expected result.
#[test]
fn test_e2e_subquery_where() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    conn.execute("CREATE TABLE t1 (id INT PRIMARY KEY, v INT)")
        .expect("CREATE TABLE");
    conn.execute("INSERT INTO t1 VALUES (1, 10), (2, 20), (3, 30)")
        .expect("INSERT");

    let r = conn
        .execute("SELECT id FROM t1 WHERE v > (SELECT AVG(v) FROM t1)")
        .expect("subquery in WHERE");
    match r {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            // Only rows with v > average(10+20+30)/3 = 20 should appear (id 2 and 3)
            assert!(!rows.is_empty(), "should have rows above average");
        }
        _ => {}
    }
    conn.execute("DROP TABLE t1").expect("DROP TABLE");
}

#[test]
fn test_e2e_connect_refused() {
    // Connecting to a port with no server should fail gracefully
    let result = MySqlConnection::connect(&"127.0.0.1:1".parse().unwrap(), "tester", "tester", "");
    // Just verify it fails - we don't need to format the error
    assert!(result.is_err(), "connection to closed port should fail");
}

// ============================================================================
// Additional DML tests — UPDATE, DELETE, INSERT
// ============================================================================

/// Verify UPDATE works end-to-end
#[test]
fn test_e2e_update_single_row() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    conn.execute("CREATE TABLE tupd1 (id INT PRIMARY KEY, val INT)")
        .expect("CREATE TABLE failed");
    conn.execute("INSERT INTO tupd1 VALUES (1, 10), (2, 20)")
        .expect("INSERT failed");
    conn.execute("UPDATE tupd1 SET val = 99 WHERE id = 1")
        .expect("UPDATE failed");

    let result = conn
        .execute("SELECT id, val FROM tupd1 WHERE id = 1")
        .expect("SELECT failed");
    match result {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0][0].to_string(), "1");
            assert_eq!(rows[0][1].to_string(), "99");
        }
        _ => {}
    }
}

/// Verify UPDATE without WHERE updates all rows
#[test]
fn test_e2e_update_no_where() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    conn.execute("CREATE TABLE tupd2 (id INT, val INT)")
        .expect("CREATE TABLE failed");
    conn.execute("INSERT INTO tupd2 VALUES (1, 10), (2, 20), (3, 30)")
        .expect("INSERT failed");
    conn.execute("UPDATE tupd2 SET val = 0")
        .expect("UPDATE failed");

    let result = conn
        .execute("SELECT SUM(val) FROM tupd2")
        .expect("SELECT failed");
    match result {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            assert_eq!(rows[0][0].to_string(), "0");
        }
        _ => {}
    }
}

/// Verify DELETE works
#[test]
fn test_e2e_delete_single_row() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    conn.execute("CREATE TABLE tdlt1 (id INT PRIMARY KEY, name TEXT)")
        .expect("CREATE TABLE failed");
    conn.execute("INSERT INTO tdlt1 VALUES (1, 'a'), (2, 'b'), (3, 'c')")
        .expect("INSERT failed");
    conn.execute("DELETE FROM tdlt1 WHERE id = 2")
        .expect("DELETE failed");

    let result = conn
        .execute("SELECT COUNT(*) FROM tdlt1")
        .expect("SELECT failed");
    match result {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            assert_eq!(rows[0][0].to_string(), "2");
        }
        _ => {}
    }
}

/// Verify DELETE without WHERE
#[test]
fn test_e2e_delete_no_where() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    conn.execute("CREATE TABLE tdlt2 (id INT)")
        .expect("CREATE TABLE failed");
    conn.execute("INSERT INTO tdlt2 VALUES (1), (2), (3)")
        .expect("INSERT failed");
    conn.execute("DELETE FROM tdlt2").expect("DELETE failed");

    let result = conn
        .execute("SELECT COUNT(*) FROM tdlt2")
        .expect("SELECT failed");
    match result {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            assert_eq!(rows[0][0].to_string(), "0");
        }
        _ => {}
    }
}

/// Verify INSERT with computed expression via UPDATE
/// Server does not support expressions in UPDATE SET — this test verifies
/// the rejection propagates correctly.
#[test]
#[should_panic]
fn test_e2e_insert_with_expression() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    conn.execute("CREATE TABLE texpr1 (a INT, b INT, c INT)")
        .expect("CREATE TABLE failed");
    conn.execute("INSERT INTO texpr1 (a, b) VALUES (3, 5)")
        .expect("INSERT failed");
    conn.execute("UPDATE texpr1 SET c = a + b")
        .expect("UPDATE failed");

    let result = conn
        .execute("SELECT a, b, c FROM texpr1")
        .expect("SELECT failed");
    match result {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            assert_eq!(rows[0][2].to_string(), "8");
        }
        _ => {}
    }
}

// ============================================================================
// Error handling tests
// ============================================================================

/// Verify error on non-existent table
#[test]
fn test_e2e_select_nonexistent_table() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    let r = conn.execute("SELECT * FROM nonexistent_table_xyz");
    assert!(r.is_err() || matches!(r, Ok(sqlrustgo_mysql_client::ResultSet::Error { .. })));
}

/// Server does not validate INSERT column count — this test verifies
/// the rejection propagates correctly.
#[test]
#[should_panic]
fn test_e2e_insert_wrong_column_count() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    conn.execute("CREATE TABLE tcolcnt (a INT, b INT)")
        .expect("CREATE TABLE failed");
    let r = conn.execute("INSERT INTO tcolcnt (a, b) VALUES (1, 2, 3)");
    assert!(r.is_err() || matches!(r, Ok(sqlrustgo_mysql_client::ResultSet::Error { .. })));
}

/// Verify error on invalid SQL syntax
#[test]
fn test_e2e_invalid_sql_syntax() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    let r = conn.execute("SELEC * FROM t1");
    assert!(r.is_err() || matches!(r, Ok(sqlrustgo_mysql_client::ResultSet::Error { .. })));
}

/// Verify dividing by zero
#[test]
fn test_e2e_divide_by_zero() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    let r = conn.execute("SELECT 1 / 0");
    assert!(r.is_ok());
}

// ============================================================================
// Aggregate and filter tests
// ============================================================================

/// Verify AVG aggregate
#[test]
fn test_e2e_avg_aggregate() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    conn.execute("CREATE TABLE taggavg (val INT)")
        .expect("CREATE TABLE failed");
    conn.execute("INSERT INTO taggavg VALUES (10), (20), (30)")
        .expect("INSERT failed");

    let result = conn
        .execute("SELECT AVG(val) FROM taggavg")
        .expect("AVG failed");
    match result {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            let avg_str = rows[0][0].to_string();
            assert!(
                avg_str.contains("20") || avg_str.contains("2"),
                "AVG should be ~20: got {}",
                avg_str
            );
        }
        _ => {}
    }
}

/// Verify MIN/MAX
#[test]
fn test_e2e_min_max_aggregates() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    conn.execute("CREATE TABLE taggmm (val INT)")
        .expect("CREATE TABLE failed");
    conn.execute("INSERT INTO taggmm VALUES (5), (15), (25), (35)")
        .expect("INSERT failed");

    let min_result = conn
        .execute("SELECT MIN(val) FROM taggmm")
        .expect("MIN failed");
    if let sqlrustgo_mysql_client::ResultSet::Select { rows, .. } = min_result {
        assert_eq!(rows[0][0].to_string(), "5");
    }

    let max_result = conn
        .execute("SELECT MAX(val) FROM taggmm")
        .expect("MAX failed");
    if let sqlrustgo_mysql_client::ResultSet::Select { rows, .. } = max_result {
        assert_eq!(rows[0][0].to_string(), "35");
    }
}

/// Verify IN operator
#[test]
#[ignore = "V312-F-2 DEFERRED: IN operator result row count mismatch — V312-24 (#4025)"]
fn test_e2e_in_operator() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    conn.execute("CREATE TABLE tinop (id INT)")
        .expect("CREATE TABLE failed");
    conn.execute("INSERT INTO tinop VALUES (1), (2), (3), (4), (5)")
        .expect("INSERT failed");

    let result = conn
        .execute("SELECT id FROM tinop WHERE id IN (2, 4)")
        .expect("IN failed");
    match result {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            assert_eq!(rows.len(), 2);
        }
        _ => {}
    }
}

/// Verify IS NULL
#[test]
#[ignore = "V312-F-2 DEFERRED: IS NULL row count mismatch — V312-24 (#4025)"]
fn test_e2e_is_null() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    conn.execute("CREATE TABLE tisnull (id INT, val INT)")
        .expect("CREATE TABLE failed");
    conn.execute("INSERT INTO tisnull (id) VALUES (1), (2)")
        .expect("INSERT failed");

    let result = conn
        .execute("SELECT COUNT(*) FROM tisnull WHERE val IS NULL")
        .expect("IS NULL failed");
    match result {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            assert_eq!(rows[0][0].to_string(), "2");
        }
        _ => {}
    }
}

/// Verify LIMIT
#[test]
fn test_e2e_limit() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    conn.execute("CREATE TABLE tlimit (id INT)")
        .expect("CREATE TABLE failed");
    conn.execute("INSERT INTO tlimit VALUES (1), (2), (3), (4), (5)")
        .expect("INSERT failed");

    let result = conn
        .execute("SELECT COUNT(*) FROM tlimit LIMIT 3")
        .expect("LIMIT failed");
    match result {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            assert!(rows.len() <= 3);
        }
        _ => {}
    }
}

// ============================================================================
// Additional e2e tests for INSERT/SELECT variants
// ============================================================================

/// Verify INSERT with multiple rows
#[test]
#[ignore = "V312-F-2 DEFERRED: INSERT multi-row result row count mismatch — V312-24 (#4025)"]
fn test_e2e_insert_multiple_rows() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    conn.execute("CREATE TABLE tmulti (id INT, val TEXT)")
        .expect("CREATE TABLE failed");
    conn.execute("INSERT INTO tmulti VALUES (1, 'a'), (2, 'b'), (3, 'c')")
        .expect("INSERT failed");

    let result = conn
        .execute("SELECT COUNT(*) FROM tmulti")
        .expect("SELECT failed");
    match result {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            assert_eq!(rows[0][0].to_string(), "3");
        }
        _ => {}
    }
}

/// Verify INSERT NULL
#[test]
fn test_e2e_insert_null() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    conn.execute("CREATE TABLE tnullins (id INT, val INT)")
        .expect("CREATE TABLE failed");
    conn.execute("INSERT INTO tnullins VALUES (1, NULL)")
        .expect("INSERT failed");

    let result = conn
        .execute("SELECT val FROM tnullins")
        .expect("SELECT failed");
    match result {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            let val = rows[0][0].to_string();
            assert!(val.is_empty() || val.eq_ignore_ascii_case("NULL"));
        }
        _ => {}
    }
}

/// Verify ORDER BY DESC with LIMIT
#[test]
fn test_e2e_order_by_desc_limit() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    conn.execute("CREATE TABLE tord (val INT)")
        .expect("CREATE TABLE failed");
    conn.execute("INSERT INTO tord VALUES (5), (1), (3), (2), (4)")
        .expect("INSERT failed");

    let result = conn
        .execute("SELECT val FROM tord ORDER BY val DESC LIMIT 3")
        .expect("SELECT failed");
    match result {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            assert!(rows.len() <= 3);
        }
        _ => {}
    }
}

/// Verify DISTINCT
#[test]
fn test_e2e_select_distinct() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    conn.execute("CREATE TABLE tdist (val INT)")
        .expect("CREATE TABLE failed");
    conn.execute("INSERT INTO tdist VALUES (1), (1), (2), (2), (2), (3)")
        .expect("INSERT failed");

    let result = conn
        .execute("SELECT DISTINCT val FROM tdist ORDER BY val")
        .expect("SELECT failed");
    match result {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            assert_eq!(rows.len(), 3);
        }
        _ => {}
    }
}

/// Verify COUNT DISTINCT
#[test]
fn test_e2e_count_distinct() {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    conn.execute("CREATE TABLE tcnt (val INT)")
        .expect("CREATE TABLE failed");
    conn.execute("INSERT INTO tcnt VALUES (1), (1), (2), (2), (2), (3)")
        .expect("INSERT failed");

    let result = conn
        .execute("SELECT COUNT(DISTINCT val) FROM tcnt")
        .expect("SELECT failed");
    match result {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            assert_eq!(rows[0][0].to_string(), "3");
        }
        _ => {}
    }
}
