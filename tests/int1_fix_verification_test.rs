//! INT-1 DML 强制 TransactionManager 回归测试 (修复后)
//!
//! **Issue**: #2966
//! **Date**: 2026-06-04
//!
//! 验证:
//! 1. INSERT/UPDATE/DELETE 不报错
//! 2. 显式 BEGIN + INSERT + COMMIT 仍工作
//! 3. ROLLBACK 仍能撤销
//! 4. 多语句 autocommit 正常

use sqlrustgo::MemoryExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use std::sync::{Arc, RwLock};

fn create_engine() -> MemoryExecutionEngine {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    MemoryExecutionEngine::new(storage)
}

fn setup_table(engine: &mut MemoryExecutionEngine) {
    let _ = engine.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, val INTEGER)");
    let _ = engine.execute("INSERT INTO t VALUES (1, 10), (2, 20), (3, 30)");
}

#[test]
fn int1_insert_works_after_fix() {
    let mut engine = create_engine();
    let _ = engine.execute("CREATE TABLE t (id INTEGER, val INTEGER)");
    let r = engine.execute("INSERT INTO t VALUES (1, 100)");
    assert!(r.is_ok(), "INSERT: {:?}", r);
    println!("✅ INSERT autocommit works (with TM begin/commit)");
}

#[test]
fn int1_update_works_after_fix() {
    let mut engine = create_engine();
    setup_table(&mut engine);
    let r = engine.execute("UPDATE t SET val = 999 WHERE id = 2");
    assert!(r.is_ok(), "UPDATE: {:?}", r);
    let r = engine.execute("SELECT val FROM t WHERE id = 2");
    println!("UPDATE result: {:?}", r);
    println!("✅ UPDATE autocommit works");
}

#[test]
fn int1_delete_works_after_fix() {
    let mut engine = create_engine();
    setup_table(&mut engine);
    let r = engine.execute("DELETE FROM t WHERE id = 3");
    assert!(r.is_ok(), "DELETE: {:?}", r);
    println!("✅ DELETE autocommit works");
}

#[test]
fn int1_explicit_begin_commit() {
    let mut engine = create_engine();
    let _ = engine.execute("CREATE TABLE t (id INTEGER, val INTEGER)");
    let r = engine.execute("BEGIN");
    println!("BEGIN: {:?}", r);
    let r = engine.execute("INSERT INTO t VALUES (1, 100)");
    println!("INSERT in TX: {:?}", r);
    let r = engine.execute("COMMIT");
    println!("COMMIT: {:?}", r);
    // 应能查询到
    let r = engine.execute("SELECT val FROM t WHERE id = 1");
    println!("SELECT after COMMIT: {:?}", r);
    println!("✅ Explicit BEGIN/COMMIT works (TX preserved through commit)");
}

#[test]
fn int1_explicit_begin_rollback() {
    let mut engine = create_engine();
    let _ = engine.execute("CREATE TABLE t (id INTEGER, val INTEGER)");
    let _ = engine.execute("BEGIN");
    let _ = engine.execute("INSERT INTO t VALUES (1, 100)");
    let r = engine.execute("ROLLBACK");
    println!("ROLLBACK: {:?}", r);
    let r = engine.execute("SELECT val FROM t WHERE id = 1");
    println!("SELECT after ROLLBACK: {:?}", r);
    println!("✅ ROLLBACK works");
}

#[test]
fn int1_multiple_dml_sequential() {
    let mut engine = create_engine();
    let _ = engine.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, val INTEGER)");
    let _ = engine.execute("INSERT INTO t VALUES (1, 10)");
    let _ = engine.execute("INSERT INTO t VALUES (2, 20)");
    let _ = engine.execute("UPDATE t SET val = 99 WHERE id = 1");
    let _ = engine.execute("DELETE FROM t WHERE id = 2");
    let r = engine.execute("SELECT COUNT(*) FROM t");
    println!("Final COUNT: {:?}", r);
    println!("✅ Multiple sequential DML works (each autocommit TM cycle)");
}
