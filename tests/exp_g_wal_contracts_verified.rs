//!
//! Experiment G: Verify WAL contracts with correct result extraction
//!
//! KEY FINDING from exp_f: The query engine returns ALL columns regardless
//! of SELECT clause. So we must index correctly:
//!   SELECT balance FROM accounts WHERE id = 1
//!   Result rows[0] = [id=1, balance=900]
//!   balance is at rows[0][1], not rows[0][0]
//!

use sqlrustgo::ExecutionEngine;
use tempfile::TempDir;

fn extract_count(result: sqlrustgo_types::SqlResult<sqlrustgo::ExecutorResult>) -> i64 {
    let rows = result.unwrap().rows;
    match rows.first().and_then(|r| r.first()) {
        Some(sqlrustgo_types::Value::Integer(n)) => *n,
        _ => -1,
    }
}

// Balance is at index 1 for "SELECT balance FROM ..."
fn extract_balance(result: sqlrustgo_types::SqlResult<sqlrustgo::ExecutorResult>) -> sqlrustgo_types::Value {
    let rows = result.unwrap().rows;
    // rows[0] = full row [id, balance], balance is at index 1
    rows.get(0).and_then(|r| r.get(1)).cloned().unwrap_or(sqlrustgo_types::Value::Null)
}

#[test]
fn test_wal_004_update_survives() {
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();

    {
        let mut engine = ExecutionEngine::with_wal_file(data_dir.clone()).unwrap();

        let _ = engine.execute("CREATE TABLE accounts (id INTEGER, balance INTEGER)");
        engine.execute("BEGIN").unwrap();
        let _ = engine.execute("INSERT INTO accounts VALUES (1, 100)");
        engine.execute("COMMIT").unwrap();

        engine.execute("BEGIN").unwrap();
        let _ = engine.execute("UPDATE accounts SET balance = 900 WHERE id = 1");
        engine.execute("COMMIT").unwrap();

        let balance_pre = extract_balance(engine.execute("SELECT balance FROM accounts WHERE id = 1"));
        eprintln!("EXP-G: pre-restart balance={:?}", balance_pre);
        assert_eq!(balance_pre, sqlrustgo_types::Value::Integer(900),
            "Pre-restart: balance should be 900");
    }

    {
        let mut engine = ExecutionEngine::with_wal_file(data_dir.clone()).unwrap();

        let count = extract_count(engine.execute("SELECT COUNT(*) FROM accounts WHERE id = 1"));
        assert_eq!(count, 1, "Row must survive");

        let balance = extract_balance(engine.execute("SELECT balance FROM accounts WHERE id = 1"));
        eprintln!("EXP-G: post-restart balance={:?}", balance);

        assert_eq!(balance, sqlrustgo_types::Value::Integer(900),
            "WAL-004 violated: balance is {:?}, expected 900", balance);
    }
}

#[test]
fn test_wal_001_insert_commit_survives() {
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();

    {
        let mut engine = ExecutionEngine::with_wal_file(data_dir.clone()).unwrap();
        let _ = engine.execute("CREATE TABLE users (id INTEGER, name TEXT)");
        engine.execute("BEGIN").unwrap();
        let _ = engine.execute("INSERT INTO users VALUES (1, 'Alice')");
        engine.execute("COMMIT").unwrap();
    }

    {
        let mut engine = ExecutionEngine::with_wal_file(data_dir.clone()).unwrap();
        let count = extract_count(engine.execute("SELECT COUNT(*) FROM users WHERE id = 1"));
        assert_eq!(count, 1, "WAL-001: committed data must survive restart");
    }
}

#[test]
fn test_wal_002_uncommitted_no_survive() {
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();

    {
        let mut engine = ExecutionEngine::with_wal_file(data_dir.clone()).unwrap();
        let _ = engine.execute("CREATE TABLE users (id INTEGER, name TEXT)");
        engine.execute("BEGIN").unwrap();
        let _ = engine.execute("INSERT INTO users VALUES (1, 'Bob')");
        // NO COMMIT - crash simulation
    }

    {
        let mut engine = ExecutionEngine::with_wal_file(data_dir.clone()).unwrap();
        let count = extract_count(engine.execute("SELECT COUNT(*) FROM users WHERE id = 1"));
        assert_eq!(count, 0, "WAL-002: uncommitted data must NOT survive");
    }
}

#[test]
fn test_wal_005_delete_survives() {
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();

    {
        let mut engine = ExecutionEngine::with_wal_file(data_dir.clone()).unwrap();
        let _ = engine.execute("CREATE TABLE data (id INTEGER, name TEXT)");
        engine.execute("BEGIN").unwrap();
        let _ = engine.execute("INSERT INTO data VALUES (1, 'to_delete')");
        engine.execute("COMMIT").unwrap();
        engine.execute("BEGIN").unwrap();
        let _ = engine.execute("DELETE FROM data WHERE id = 1");
        engine.execute("COMMIT").unwrap();
    }

    {
        let mut engine = ExecutionEngine::with_wal_file(data_dir.clone()).unwrap();
        let count = extract_count(engine.execute("SELECT COUNT(*) FROM data"));
        assert_eq!(count, 0, "WAL-005: deleted row must NOT survive");
    }
}

#[test]
fn test_wal_003_multi_tx_ordering() {
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();

    {
        let mut engine = ExecutionEngine::with_wal_file(data_dir.clone()).unwrap();
        let _ = engine.execute("CREATE TABLE accounts (id INTEGER, balance INTEGER)");
        engine.execute("BEGIN").unwrap();
        let _ = engine.execute("INSERT INTO accounts VALUES (1, 100)");
        engine.execute("COMMIT").unwrap();
        engine.execute("BEGIN").unwrap();
        let _ = engine.execute("INSERT INTO accounts VALUES (2, 200)");
        engine.execute("COMMIT").unwrap();
    }

    {
        let mut engine = ExecutionEngine::with_wal_file(data_dir.clone()).unwrap();
        let cnt = extract_count(engine.execute("SELECT COUNT(*) FROM accounts"));
        assert_eq!(cnt, 2, "WAL-003: both committed rows must survive");

        // Get all rows - verify id=1 has balance 100, id=2 has balance 200
        let rows = engine.execute("SELECT * FROM accounts ORDER BY id").unwrap().rows;
        eprintln!("EXP-G-MULTI: rows={:?}", rows);

        let r1_balance: i64 = match rows.get(0).and_then(|r| r.get(1)) {
            Some(sqlrustgo_types::Value::Integer(n)) => *n,
            _ => -1,
        };
        let r2_balance: i64 = match rows.get(1).and_then(|r| r.get(1)) {
            Some(sqlrustgo_types::Value::Integer(n)) => *n,
            _ => -1,
        };

        assert_eq!(r1_balance, 100, "Row 1 balance");
        assert_eq!(r2_balance, 200, "Row 2 balance");
    }
}