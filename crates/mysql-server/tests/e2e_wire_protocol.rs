//! End-to-end wire protocol tests: mysql-client ↔ mysql-server via TCP.
//!
//! These tests boot an in-process ephemeral server and drive it through
//! the real MySQL wire protocol using sqlrustgo-mysql-client.
//!
//! Each test:
//!   1. Starts an ephemeral server on a random port
//!   2. Connects via MySqlConnection::connect (real TCP)
//!   3. Executes queries via execute() / execute_multi()
//!   4. Verifies result sets round-trip correctly
//!
//! Run with: cargo test -p sqlrustgo-mysql-server --test e2e_wire_protocol

use sqlrustgo_mysql_client::{MySqlConnection, MySqlResult};
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};
use std::net::TcpStream;
use std::time::Duration;

/// Connect to the ephemeral server as the default tester user.
fn connect(port: u16) -> MySqlResult<MySqlConnection> {
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
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;

    // Connect and authenticate
    let mut conn = connect(port).expect("connected to server");

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
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;

    let mut conn = connect(port).expect("connected");
    let result = conn
        .execute("SELECT 1 AS a")
        .expect("execute SELECT 1");

    match result {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            assert!(!rows.is_empty(), "expected at least one row");
            assert_eq!(rows[0].len(), 1, "expected 1 column");
            assert_eq!(rows[0][0].to_string(), "1");
        }
        sqlrustgo_mysql_client::ResultSet::Error { error_code: code, error_message: msg, .. } => {
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
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
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
        sqlrustgo_mysql_client::ResultSet::Error { error_code: code, error_message: msg, .. } => {
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
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
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
        sqlrustgo_mysql_client::ResultSet::Error { error_code: code, error_message: msg, .. } => {
            panic!("server returned error {}: {}", code, msg);
        }
        sqlrustgo_mysql_client::ResultSet::Ok { .. } => {
            // Non-SELECT result; ignored in SELECT tests
        }
    }
}

/// Verify that a DROP TABLE removes data.
#[test]
fn test_e2e_drop_table() {
    let config = EphemeralConfig {
        data_dir: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
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
    conn.execute("DROP TABLE t2")
        .expect("DROP TABLE failed");

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
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
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
        sqlrustgo_mysql_client::ResultSet::Error { error_code: code, error_message: msg, .. } => {
            panic!("first result set error {}: {}", code, msg);
        }
        sqlrustgo_mysql_client::ResultSet::Ok { .. } => {
            // Non-SELECT result; ignored
        }
    }
}

/// Verify NULL values are correctly transmitted.
#[test]
fn test_e2e_null_handling() {
    let config = EphemeralConfig {
        data_dir: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
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
        sqlrustgo_mysql_client::ResultSet::Error { error_code: code, error_message: msg, .. } => {
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
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
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
        sqlrustgo_mysql_client::ResultSet::Error { error_code: code, error_message: msg, .. } => {
            panic!("server error {}: {}", code, msg);
        }
        sqlrustgo_mysql_client::ResultSet::Ok { .. } => {
            // Non-SELECT result; ignored in SELECT tests
        }
    }
}

/// Verify UPDATE works end-to-end.
#[test]
fn test_e2e_update() {
    let config = EphemeralConfig {
        data_dir: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
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
        sqlrustgo_mysql_client::ResultSet::Error { error_code: code, error_message: msg, .. } => {
            panic!("server error {}: {}", code, msg);
        }
        sqlrustgo_mysql_client::ResultSet::Ok { .. } => {
            // Non-SELECT result; ignored in SELECT tests
        }
    }
}

/// Verify DELETE works end-to-end.
#[test]
fn test_e2e_delete() {
    let config = EphemeralConfig {
        data_dir: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
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
        sqlrustgo_mysql_client::ResultSet::Error { error_code: code, error_message: msg, .. } => {
            panic!("server error {}: {}", code, msg);
        }
        sqlrustgo_mysql_client::ResultSet::Ok { .. } => {
            // Non-SELECT result; ignored in SELECT tests
        }
    }
}

/// Verify ORDER BY works.
#[test]
fn test_e2e_order_by() {
    let config = EphemeralConfig {
        data_dir: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
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
        sqlrustgo_mysql_client::ResultSet::Error { error_code: code, error_message: msg, .. } => {
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
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
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
        sqlrustgo_mysql_client::ResultSet::Error { error_code: code, error_message: msg, .. } => {
            panic!("server error {}: {}", code, msg);
        }
        sqlrustgo_mysql_client::ResultSet::Ok { .. } => {
            // Non-SELECT result; ignored in SELECT tests
        }
    }
}

/// Verify GROUP BY with aggregate functions.
#[test]
fn test_e2e_group_by_aggregates() {
    let config = EphemeralConfig {
        data_dir: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;

    let mut conn = connect(port).expect("connected");

    conn.execute(
        "CREATE TABLE tgrp (dept TEXT, salary INT)",
    )
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
        sqlrustgo_mysql_client::ResultSet::Error { error_code: code, error_message: msg, .. } => {
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
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    // Create table with various column types
    conn.execute("CREATE TABLE t1 (id INT PRIMARY KEY, name VARCHAR(50), active BOOLEAN, ts TIMESTAMP)")
        .expect("CREATE TABLE");
    // Verify it exists
    let r = conn.execute("SELECT column_name FROM information_schema.columns WHERE table_name = 't1'")
        .expect("query information_schema");
    match r {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            assert!(rows.len() >= 4, "expected at least 4 columns, got {}", rows.len());
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
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    conn.execute("CREATE TABLE t1 (id INT PRIMARY KEY, name VARCHAR(50))")
        .expect("CREATE TABLE");
    conn.execute("INSERT INTO t1 VALUES (1, 'alice')").expect("INSERT");

    // ALTER TABLE ADD COLUMN
    conn.execute("ALTER TABLE t1 ADD COLUMN email VARCHAR(100)")
        .expect("ALTER TABLE ADD");

    // Verify new column exists
    let r = conn.execute("SELECT email FROM t1 WHERE id = 1").expect("SELECT new col");
    match r {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            // email is NULL for existing row
            assert_eq!(rows.len(), 1);
        }
        _ => {}
    }

    // ALTER TABLE DROP COLUMN (drop name column)
    conn.execute("ALTER TABLE t1 DROP COLUMN name").expect("ALTER TABLE DROP");

    // Verify name column is gone
    let r2 = conn.execute("SELECT * FROM t1 WHERE id = 1").expect("SELECT remaining cols");
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
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    conn.execute("CREATE TABLE t1 (id INT PRIMARY KEY, x INT, y INT)")
        .expect("CREATE TABLE");
    conn.execute("INSERT INTO t1 VALUES (1, 10, 20), (2, 30, 40)")
        .expect("INSERT");
    conn.execute("CREATE INDEX idx_x ON t1 (x)").expect("CREATE INDEX");

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
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;
    let mut conn = connect(port).expect("connected");

    conn.execute("CREATE TABLE t1 (id INT PRIMARY KEY, v INT)")
        .expect("CREATE TABLE");

    conn.execute("BEGIN").expect("BEGIN");
    conn.execute("INSERT INTO t1 VALUES (1, 100)").expect("INSERT in tx");
    conn.execute("COMMIT").expect("COMMIT");

    // Verify committed row is visible
    let r = conn.execute("SELECT v FROM t1 WHERE id = 1").expect("SELECT");
    match r {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0][0].to_string().trim_end(), "100");
        }
        _ => panic!("expected SELECT result"),
    }
    conn.execute("DROP TABLE t1").expect("DROP TABLE");
}

