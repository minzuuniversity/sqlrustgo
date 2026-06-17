//! G5 SEM-1 Savepoint MVCC Oracle (V4 fix)
//!
//! 验证 SAVEPOINT + ROLLBACK TO SAVEPOINT + RELEASE SAVEPOINT 行为符合 SQL 规范.
//! Oracle: 任何 tx 状态改变, COUNT(*) 必须与 ground truth 一致.

mod common;

use sqlrustgo::{ExecutionEngine, MemoryStorage};
use sqlrustgo_types::Value;
use std::sync::{Arc, RwLock};

fn make_engine() -> ExecutionEngine<MemoryStorage> {
    ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())))
}

fn setup_table(engine: &mut ExecutionEngine<MemoryStorage>) {
    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY, val INTEGER)")
        .unwrap();
    engine
        .execute("INSERT INTO t VALUES (1, 100)")
        .unwrap();
    engine
        .execute("INSERT INTO t VALUES (2, 200)")
        .unwrap();
}

fn count_rows(engine: &mut ExecutionEngine<MemoryStorage>) -> i64 {
    let r = engine.execute("SELECT COUNT(*) FROM t").unwrap();
    match &r.rows[0][0] {
        Value::Integer(n) => *n,
        _ => panic!("expected Int"),
    }
}

#[test]
fn g5_sem1_savepoint_rollback_restores_state_oracle() {
    let mut engine = make_engine();
    setup_table(&mut engine);

    engine.execute("BEGIN").unwrap();
    engine.execute("SAVEPOINT sp1").unwrap();
    engine.execute("INSERT INTO t VALUES (3, 300)").unwrap();
    assert_eq!(count_rows(&mut engine), 3, "After INSERT in savepoint: 3 rows");

    engine
        .execute("ROLLBACK TO SAVEPOINT sp1")
        .expect("rollback to savepoint should succeed");
    assert_eq!(
        count_rows(&mut engine),
        2,
        "Oracle: ROLLBACK TO sp1 restores to 2 rows (before INSERT)"
    );

    engine.execute("COMMIT").unwrap();
    assert_eq!(count_rows(&mut engine), 2, "Final state: 2 rows");
}

#[test]
fn g5_sem1_release_savepoint_persists_oracle() {
    let mut engine = make_engine();
    setup_table(&mut engine);

    engine.execute("BEGIN").unwrap();
    engine.execute("SAVEPOINT sp1").unwrap();
    engine.execute("INSERT INTO t VALUES (3, 300)").unwrap();
    engine.execute("RELEASE SAVEPOINT sp1").unwrap();
    engine.execute("COMMIT").unwrap();

    assert_eq!(
        count_rows(&mut engine),
        3,
        "Oracle: RELEASE SAVEPOINT persists changes, 3 rows after COMMIT"
    );
}

#[test]
fn g5_sem1_savepoint_nested_oracle() {
    let mut engine = make_engine();
    setup_table(&mut engine);

    engine.execute("BEGIN").unwrap();
    engine.execute("SAVEPOINT outer").unwrap();
    engine.execute("INSERT INTO t VALUES (3, 300)").unwrap();
    engine.execute("SAVEPOINT inner").unwrap();
    engine
        .execute("INSERT INTO t VALUES (4, 400)")
        .unwrap();
    assert_eq!(count_rows(&mut engine), 4, "After both inserts: 4 rows");

    engine
        .execute("ROLLBACK TO SAVEPOINT inner")
        .unwrap();
    assert_eq!(
        count_rows(&mut engine),
        3,
        "Oracle: rollback to inner → 3 rows (4 deleted, 3 kept)"
    );

    engine.execute("COMMIT").unwrap();
    assert_eq!(count_rows(&mut engine), 3, "Final state: 3 rows");
}
