//! Cross-Path Consistency E2E Tests (SPEC-024: F-10 mysql-server unify)
//!
//! Validates that SQL execution results are consistent across 3 paths:
//! 1. MemoryStorage (L1, in-memory, no persistence)
//! 2. WalStorage<MemoryStorage, MemoryWalManager> (L2, WAL stub)
//! 3. WalStorage<FileStorage, FileBackedWalManager> (L3, persistent, mysql-server actual)
//!
//! Per DEFERRED_PRS.md §3.4 推荐测试矩阵.
//!
//! Phase 2a migration (OpenSpec §2): only path 3 (the production
//! "mysql-server actual" stack) must be driven over the wire protocol.
//! Paths 1 and 2 are below the wire-protocol layer (they exercise
//! the same `ExecutionEngine` code paths the wire server uses
//! internally) and stay as in-process calls per the spec
//! "in-process direct-call tests are kept for unit tests below
//! the wire-protocol layer".

mod common;

use common::MySqlTestClient;
use sqlrustgo::{ExecutionEngine, MemoryExecutionEngine};
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};
use sqlrustgo_storage::{MemoryStorage, MemoryWalManager, WalStorage};
use std::sync::{Arc, RwLock};
use tempfile::TempDir;

fn create_memory_engine() -> MemoryExecutionEngine {
    ExecutionEngine::with_memory()
}

fn create_wal_memory_engine() -> ExecutionEngine<WalStorage<MemoryStorage, MemoryWalManager>> {
    let inner = MemoryStorage::new();
    let wal = MemoryWalManager::new();
    let storage = WalStorage::new(inner, wal).expect("Failed to create WAL memory storage");
    let storage_arc = Arc::new(RwLock::new(storage));
    ExecutionEngine::new(storage_arc)
}

fn create_wal_file_engine(dir: &std::path::Path) -> MySqlTestClient {
    let cfg = EphemeralConfig {
        host: "127.0.0.1".to_string(),
        bootstrap_tables: false,
        bootstrap_users: true,
        data_dir: Some(dir.to_path_buf()),
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
    };
    let handle = start_ephemeral(cfg).expect("start_ephemeral");
    MySqlTestClient::connect_handle(handle).expect("MySqlTestClient::connect_handle")
}

/// Cross-path SELECT: 3 路径应返回相同行数 + 相同内容
#[test]
fn cross_path_select_consistency() {
    let dir = TempDir::new().unwrap();
    let setup_sql = vec![
        "CREATE TABLE t (id INTEGER, name TEXT)",
        "INSERT INTO t VALUES (1, 'alice')",
        "INSERT INTO t VALUES (2, 'bob')",
        "INSERT INTO t VALUES (3, 'charlie')",
    ];

    let r1 = {
        let mut e = create_memory_engine();
        for sql in &setup_sql {
            e.execute(sql).expect("Path 1 setup failed");
        }
        e.execute("SELECT id, name FROM t ORDER BY id")
            .expect("Path 1 SELECT failed")
            .rows
    };

    let r2 = {
        let mut e = create_wal_memory_engine();
        for sql in &setup_sql {
            e.execute(sql).expect("Path 2 setup failed");
        }
        e.execute("SELECT id, name FROM t ORDER BY id")
            .expect("Path 2 SELECT failed")
            .rows
    };

    let r3 = {
        let mut c = create_wal_file_engine(dir.path());
        for sql in &setup_sql {
            c.exec(sql).expect("Path 3 setup failed");
        }
        c.query_rows("SELECT id, name FROM t ORDER BY id")
            .expect("Path 3 SELECT failed")
    };

    assert_eq!(r1.len(), 3, "Path 1 should have 3 rows");
    assert_eq!(r2.len(), 3, "Path 2 should have 3 rows");
    assert_eq!(r3.len(), 3, "Path 3 should have 3 rows");

    let r1_str: Vec<Vec<String>> = r1
        .iter()
        .map(|row| row.iter().map(value_to_compare_string).collect())
        .collect();
    let r2_str: Vec<Vec<String>> = r2
        .iter()
        .map(|row| row.iter().map(value_to_compare_string).collect())
        .collect();

    assert_eq!(r1_str, r2_str, "Memory vs WAL memory results differ");
    assert_eq!(r2_str, r3, "WAL memory vs WAL file results differ");
}

fn value_to_compare_string(v: &sqlrustgo_types::Value) -> String {
    use sqlrustgo_types::Value;
    match v {
        Value::Null => String::new(),
        Value::Integer(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Text(s) => s.clone(),
        Value::Boolean(b) => b.to_string(),
        Value::Blob(b) => String::from_utf8_lossy(b).to_string(),
    }
}

/// Cross-path INSERT: 3 路径都能看到插入
#[test]
fn cross_path_insert_visibility() {
    let dir = TempDir::new().unwrap();

    let r1 = {
        let mut e = create_memory_engine();
        e.execute("CREATE TABLE t (id INTEGER)").unwrap();
        e.execute("INSERT INTO t VALUES (1)").unwrap();
        e.execute("INSERT INTO t VALUES (2)").unwrap();
        e.execute("SELECT COUNT(*) FROM t").unwrap().rows[0][0].clone()
    };

    let r2 = {
        let mut e = create_wal_memory_engine();
        e.execute("CREATE TABLE t (id INTEGER)").unwrap();
        e.execute("INSERT INTO t VALUES (1)").unwrap();
        e.execute("INSERT INTO t VALUES (2)").unwrap();
        e.execute("SELECT COUNT(*) FROM t").unwrap().rows[0][0].clone()
    };

    let r3 = {
        let mut c = create_wal_file_engine(dir.path());
        c.exec("CREATE TABLE t (id INTEGER)").unwrap();
        c.exec("INSERT INTO t VALUES (1)").unwrap();
        c.exec("INSERT INTO t VALUES (2)").unwrap();
        c.query_one_i64("SELECT COUNT(*) FROM t").unwrap()
    };

    assert_eq!(r1, sqlrustgo_types::Value::Integer(2));
    assert_eq!(r2, sqlrustgo_types::Value::Integer(2));
    assert_eq!(r3, 2);
}

/// Cross-path UPDATE: 3 路径结果一致
#[test]
fn cross_path_update_consistency() {
    let dir = TempDir::new().unwrap();

    let setup = "CREATE TABLE t (id INTEGER, value INTEGER)";
    let insert = "INSERT INTO t VALUES (1, 100)";
    let update = "UPDATE t SET value = 200 WHERE id = 1";
    let select = "SELECT value FROM t WHERE id = 1";

    let r1 = {
        let mut e = create_memory_engine();
        e.execute(setup).unwrap();
        e.execute(insert).unwrap();
        e.execute(update).unwrap();
        e.execute(select).unwrap().rows[0][0].clone()
    };

    let r2 = {
        let mut e = create_wal_memory_engine();
        e.execute(setup).unwrap();
        e.execute(insert).unwrap();
        e.execute(update).unwrap();
        e.execute(select).unwrap().rows[0][0].clone()
    };

    let r3 = {
        let mut c = create_wal_file_engine(dir.path());
        c.exec(setup).unwrap();
        c.exec(insert).unwrap();
        c.exec(update).unwrap();
        c.query_rows(select).unwrap()[0][0].clone()
    };

    let expected = "200";
    assert_eq!(format!("{r1:?}"), "Integer(200)");
    assert_eq!(format!("{r2:?}"), "Integer(200)");
    assert_eq!(r3, expected);
}

/// Cross-path UPDATE multi-column: 3 路径结果一致
#[test]
fn cross_path_update_multi_column_consistency() {
    let dir = TempDir::new().unwrap();

    let setup = "CREATE TABLE t (id INTEGER, a INTEGER, b TEXT)";
    let insert = "INSERT INTO t VALUES (1, 10, 'old')";
    let update = "UPDATE t SET a = 99, b = 'new' WHERE id = 1";
    let select = "SELECT a, b FROM t WHERE id = 1";

    let (r1_a, r1_b) = {
        let mut e = create_memory_engine();
        e.execute(setup).unwrap();
        e.execute(insert).unwrap();
        e.execute(update).unwrap();
        let row = &e.execute(select).unwrap().rows[0];
        (row[0].clone(), row[1].clone())
    };

    let (r2_a, r2_b) = {
        let mut e = create_wal_memory_engine();
        e.execute(setup).unwrap();
        e.execute(insert).unwrap();
        e.execute(update).unwrap();
        let row = &e.execute(select).unwrap().rows[0];
        (row[0].clone(), row[1].clone())
    };

    let r3 = {
        let mut c = create_wal_file_engine(dir.path());
        c.exec(setup).unwrap();
        c.exec(insert).unwrap();
        c.exec(update).unwrap();
        c.query_rows(select).unwrap()
    };

    assert_eq!(format!("{r1_a:?}"), "Integer(99)");
    assert_eq!(format!("{r2_a:?}"), "Integer(99)");
    assert_eq!(r3[0][0], "99");
    assert_eq!(format!("{r1_b:?}"), "Text(\"new\")");
    assert_eq!(format!("{r2_b:?}"), "Text(\"new\")");
    assert_eq!(r3[0][1], "new");
}

/// Cross-path UPDATE without WHERE clause
///
/// Three paths must produce the same result. Asserts that
/// `UPDATE t SET value = 0` (no WHERE) zeroes all rows.
#[ignore = "Bug discovered 2026-06-06 — UPDATE without WHERE doesn't apply (SUM stays 600). Tracked in #3236."]
#[test]
fn cross_path_update_no_where_consistency() {
    let dir = TempDir::new().unwrap();

    let setup = "CREATE TABLE t (id INTEGER, value INTEGER)";
    let inserts = [
        "INSERT INTO t VALUES (1, 100)",
        "INSERT INTO t VALUES (2, 200)",
        "INSERT INTO t VALUES (3, 300)",
    ];
    let update = "UPDATE t SET value = 0";
    let select = "SELECT SUM(value) FROM t";

    let r1 = {
        let mut e = create_memory_engine();
        e.execute(setup).unwrap();
        for ins in &inserts {
            e.execute(ins).unwrap();
        }
        e.execute(update).unwrap();
        e.execute(select).unwrap().rows[0][0].clone()
    };

    let r2 = {
        let mut e = create_wal_memory_engine();
        e.execute(setup).unwrap();
        for ins in &inserts {
            e.execute(ins).unwrap();
        }
        e.execute(update).unwrap();
        e.execute(select).unwrap().rows[0][0].clone()
    };

    let r3 = {
        let mut c = create_wal_file_engine(dir.path());
        c.exec(setup).unwrap();
        for ins in &inserts {
            c.exec(ins).unwrap();
        }
        c.exec(update).unwrap();
        c.query_one_i64(select).unwrap()
    };

    assert_eq!(format!("{r1:?}"), "Integer(0)");
    assert_eq!(format!("{r2:?}"), "Integer(0)");
    assert_eq!(r3, 0);
}

/// Cross-path UPDATE no-match: WHERE 不命中任何行，3 路径结果一致
#[test]
fn cross_path_update_no_match_consistency() {
    let dir = TempDir::new().unwrap();

    let setup = "CREATE TABLE t (id INTEGER, value INTEGER)";
    let insert = "INSERT INTO t VALUES (1, 100)";
    let update = "UPDATE t SET value = 999 WHERE id = 999";
    let select = "SELECT value FROM t WHERE id = 1";

    let r1 = {
        let mut e = create_memory_engine();
        e.execute(setup).unwrap();
        e.execute(insert).unwrap();
        e.execute(update).unwrap();
        e.execute(select).unwrap().rows[0][0].clone()
    };

    let r2 = {
        let mut e = create_wal_memory_engine();
        e.execute(setup).unwrap();
        e.execute(insert).unwrap();
        e.execute(update).unwrap();
        e.execute(select).unwrap().rows[0][0].clone()
    };

    let r3 = {
        let mut c = create_wal_file_engine(dir.path());
        c.exec(setup).unwrap();
        c.exec(insert).unwrap();
        c.exec(update).unwrap();
        c.query_rows(select).unwrap()[0][0].clone()
    };

    // WHERE 不命中，原值保留
    assert_eq!(format!("{r1:?}"), "Integer(100)");
    assert_eq!(format!("{r2:?}"), "Integer(100)");
    assert_eq!(r3, "100");
}

/// Cross-path UPDATE multi-row: WHERE 命中多行
#[test]
fn cross_path_update_multi_row_consistency() {
    let dir = TempDir::new().unwrap();

    let setup = "CREATE TABLE t (id INTEGER, group_id INTEGER, value INTEGER)";
    let inserts = [
        "INSERT INTO t VALUES (1, 1, 100)",
        "INSERT INTO t VALUES (2, 1, 200)",
        "INSERT INTO t VALUES (3, 2, 300)",
        "INSERT INTO t VALUES (4, 1, 400)",
    ];
    let update = "UPDATE t SET value = 0 WHERE group_id = 1";
    let select = "SELECT COUNT(*) FROM t WHERE value = 0";

    let r1 = {
        let mut e = create_memory_engine();
        e.execute(setup).unwrap();
        for ins in &inserts {
            e.execute(ins).unwrap();
        }
        e.execute(update).unwrap();
        e.execute(select).unwrap().rows[0][0].clone()
    };

    let r2 = {
        let mut e = create_wal_memory_engine();
        e.execute(setup).unwrap();
        for ins in &inserts {
            e.execute(ins).unwrap();
        }
        e.execute(update).unwrap();
        e.execute(select).unwrap().rows[0][0].clone()
    };

    let r3 = {
        let mut c = create_wal_file_engine(dir.path());
        c.exec(setup).unwrap();
        for ins in &inserts {
            c.exec(ins).unwrap();
        }
        c.exec(update).unwrap();
        c.query_one_i64(select).unwrap()
    };

    // group_id=1 的有 3 行（id 1, 2, 4），都应被更新为 0
    assert_eq!(format!("{r1:?}"), "Integer(3)");
    assert_eq!(format!("{r2:?}"), "Integer(3)");
    assert_eq!(r3, 3);
}

/// Cross-path UPDATE with self-reference: SET value = value + N
///
/// Three paths must produce the same result. Asserts that
/// `UPDATE t SET value = value + 100` evaluates the expression
/// per-row (each row's own value) rather than using a single
/// shared evaluation context.
#[ignore = "Bug discovered 2026-06-06 — UPDATE self-reference evaluates against first row only (3 rows all become 110, SUM=330 not 360). Tracked in #3236."]
#[test]
fn cross_path_update_self_reference_consistency() {
    let dir = TempDir::new().unwrap();

    let setup = "CREATE TABLE t (id INTEGER, value INTEGER)";
    let inserts = [
        "INSERT INTO t VALUES (1, 10)",
        "INSERT INTO t VALUES (2, 20)",
        "INSERT INTO t VALUES (3, 30)",
    ];
    let update = "UPDATE t SET value = value + 100";
    let select = "SELECT SUM(value) FROM t";

    let r1 = {
        let mut e = create_memory_engine();
        e.execute(setup).unwrap();
        for ins in &inserts {
            e.execute(ins).unwrap();
        }
        e.execute(update).unwrap();
        e.execute(select).unwrap().rows[0][0].clone()
    };

    let r2 = {
        let mut e = create_wal_memory_engine();
        e.execute(setup).unwrap();
        for ins in &inserts {
            e.execute(ins).unwrap();
        }
        e.execute(update).unwrap();
        e.execute(select).unwrap().rows[0][0].clone()
    };

    let r3 = {
        let mut c = create_wal_file_engine(dir.path());
        c.exec(setup).unwrap();
        for ins in &inserts {
            c.exec(ins).unwrap();
        }
        c.exec(update).unwrap();
        c.query_one_i64(select).unwrap()
    };

    // 10+100 + 20+100 + 30+100 = 360
    assert_eq!(format!("{r1:?}"), "Integer(360)");
    assert_eq!(format!("{r2:?}"), "Integer(360)");
    assert_eq!(r3, 360);
}

/// Cross-path DELETE: 3 路径结果一致
#[test]
fn cross_path_delete_consistency() {
    let dir = TempDir::new().unwrap();

    let setup = "CREATE TABLE t (id INTEGER)";
    let insert = "INSERT INTO t VALUES (1)";
    let delete = "DELETE FROM t WHERE id = 1";
    let select = "SELECT COUNT(*) FROM t";

    let r1 = {
        let mut e = create_memory_engine();
        e.execute(setup).unwrap();
        e.execute(insert).unwrap();
        e.execute(delete).unwrap();
        e.execute(select).unwrap().rows[0][0].clone()
    };

    let r2 = {
        let mut e = create_wal_memory_engine();
        e.execute(setup).unwrap();
        e.execute(insert).unwrap();
        e.execute(delete).unwrap();
        e.execute(select).unwrap().rows[0][0].clone()
    };

    let r3 = {
        let mut c = create_wal_file_engine(dir.path());
        c.exec(setup).unwrap();
        c.exec(insert).unwrap();
        c.exec(delete).unwrap();
        c.query_one_i64(select).unwrap()
    };

    let zero = sqlrustgo_types::Value::Integer(0);
    assert_eq!(r1, zero);
    assert_eq!(r2, zero);
    assert_eq!(r3, 0);
}

/// Cross-path error consistency: 3 路径错误 SQL 应产生错误
#[test]
fn cross_path_error_consistency() {
    let dir = TempDir::new().unwrap();
    let bad_sql = "SELECT * FROM nonexistent_table_xyz";

    let mut p1 = create_memory_engine();
    let r1 = p1.execute(bad_sql);
    assert!(r1.is_err(), "Path 1 should error on missing table");

    let mut p2 = create_wal_memory_engine();
    let r2 = p2.execute(bad_sql);
    assert!(r2.is_err(), "Path 2 should error on missing table");

    let mut p3 = create_wal_file_engine(dir.path());
    let r3 = p3.query_rows(bad_sql);
    assert!(r3.is_err(), "Path 3 should error on missing table");
}
