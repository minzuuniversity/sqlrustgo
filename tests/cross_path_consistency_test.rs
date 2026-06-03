//! Cross-Path Consistency E2E Tests (SPEC-024: F-10 mysql-server unify)
//!
//! Validates that SQL execution results are consistent across 3 paths:
//! 1. MemoryStorage (L1, in-memory, no persistence)
//! 2. WalStorage<MemoryStorage, MemoryWalManager> (L2, WAL stub)
//! 3. WalStorage<FileStorage, FileBackedWalManager> (L3, persistent, mysql-server actual)
//!
//! Per DEFERRED_PRS.md §3.4 推荐测试矩阵.

use sqlrustgo::{ExecutionEngine, MemoryExecutionEngine};
use sqlrustgo_storage::{
    FileBackedWalManager, FileStorage, MemoryStorage, MemoryWalManager, StorageEngine, WalStorage,
};
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

fn create_wal_file_engine(
    dir: &std::path::Path,
) -> ExecutionEngine<WalStorage<FileStorage, FileBackedWalManager>> {
    ExecutionEngine::with_wal_file(dir.to_path_buf()).expect("Failed to create WAL file engine")
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

    // Path 1: MemoryStorage
    let mut p1 = create_memory_engine();
    for sql in &setup_sql {
        p1.execute(sql).expect("Path 1 setup failed");
    }
    let r1 = p1
        .execute("SELECT id, name FROM t ORDER BY id")
        .expect("Path 1 SELECT failed");

    // Path 2: WAL memory
    let mut p2 = create_wal_memory_engine();
    for sql in &setup_sql {
        p2.execute(sql).expect("Path 2 setup failed");
    }
    let r2 = p2
        .execute("SELECT id, name FROM t ORDER BY id")
        .expect("Path 2 SELECT failed");

    // Path 3: WAL file (mysql-server actual)
    let mut p3 = create_wal_file_engine(dir.path());
    for sql in &setup_sql {
        p3.execute(sql).expect("Path 3 setup failed");
    }
    let r3 = p3
        .execute("SELECT id, name FROM t ORDER BY id")
        .expect("Path 3 SELECT failed");

    assert_eq!(r1.rows.len(), 3, "Path 1 should have 3 rows");
    assert_eq!(r2.rows.len(), 3, "Path 2 should have 3 rows");
    assert_eq!(r3.rows.len(), 3, "Path 3 should have 3 rows");

    // 3 路径结果应完全一致
    assert_eq!(r1.rows, r2.rows, "Memory vs WAL memory results differ");
    assert_eq!(r2.rows, r3.rows, "WAL memory vs WAL file results differ");
}

/// Cross-path INSERT: 3 路径都能看到插入
#[test]
fn cross_path_insert_visibility() {
    let dir = TempDir::new().unwrap();

    // Direct test: 3 paths separately
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
        let mut e = create_wal_file_engine(dir.path());
        e.execute("CREATE TABLE t (id INTEGER)").unwrap();
        e.execute("INSERT INTO t VALUES (1)").unwrap();
        e.execute("INSERT INTO t VALUES (2)").unwrap();
        e.execute("SELECT COUNT(*) FROM t").unwrap().rows[0][0].clone()
    };

    assert_eq!(r1, sqlrustgo_types::Value::Integer(2));
    assert_eq!(r2, sqlrustgo_types::Value::Integer(2));
    assert_eq!(r3, sqlrustgo_types::Value::Integer(2));
    assert_eq!(r1, r2);
    assert_eq!(r2, r3);
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
        let mut e = create_wal_file_engine(dir.path());
        e.execute(setup).unwrap();
        e.execute(insert).unwrap();
        e.execute(update).unwrap();
        e.execute(select).unwrap().rows[0][0].clone()
    };

    let expected = sqlrustgo_types::Value::Integer(200);
    assert_eq!(r1, expected);
    assert_eq!(r2, expected);
    assert_eq!(r3, expected);
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
        let mut e = create_wal_file_engine(dir.path());
        e.execute(setup).unwrap();
        e.execute(insert).unwrap();
        e.execute(delete).unwrap();
        e.execute(select).unwrap().rows[0][0].clone()
    };

    let zero = sqlrustgo_types::Value::Integer(0);
    assert_eq!(r1, zero);
    assert_eq!(r2, zero);
    assert_eq!(r3, zero);
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
    let r3 = p3.execute(bad_sql);
    assert!(r3.is_err(), "Path 3 should error on missing table");
}
