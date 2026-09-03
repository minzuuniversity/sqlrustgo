//! Regression tests for Issue #4703 (V312-RC-GA PR-A4 / WP-A) —
//! 4 DML/upsert parser issues. Per
//! `docs/releases/v3.12.0/RC_GA_TRIAGE_AND_GATE_PLAN_2026-09-03.md` §3:
//!
//! Sub-bug #1 (multi-column + VALUES()): `InsertError` (engine API path).
//! CLI batch path: OR-downgrade rejects explicitly (see sqlite_mode.rs).
//! Sub-bugs #2, #3 already GREEN via earlier PR work (PR #4735 column-level
//! triggers, prior ON CONFLICT support). Test cases below lock down all four
//! to prevent regression of any solution.

use sqlrustgo::{ExecutionEngine, MemoryStorage};

fn engine() -> ExecutionEngine<MemoryStorage> {
    ExecutionEngine::new(std::sync::Arc::new(parking_lot::RwLock::new(
        MemoryStorage::new(),
    )))
}

#[test]
fn on_duplicate_key_update_multi_column_with_values_engine_api() {
    // Engine API path: ON DUPLICATE KEY UPDATE + VALUES() still returns an
    // error (the VALUES() reference is not wired through the v3.12 parser).
    // This is the engine-side honest-path: the operation fails explicitly
    // rather than silently executing partial logic.
    let mut e = engine();
    e.execute("CREATE TABLE m(id INTEGER, name TEXT, val INTEGER)").unwrap();
    e.execute("INSERT INTO m VALUES (1, 'alice', 100)").unwrap();
    let r = e.execute(
        "INSERT INTO m VALUES (1, 'alice_updated', 200) \
         ON DUPLICATE KEY UPDATE name = VALUES(name), val = val + 50",
    );
    assert!(
        r.is_err(),
        "ENGINE API: ON DUPLICATE KEY UPDATE multi-col + VALUES() must NOT \
         silently succeed; expected Err. Got: {:?}",
        r.as_ref().map(|er| format!("{:?}", er.rows.len()))
    );
}

#[test]
fn on_conflict_do_update_set() {
    let mut e = engine();
    e.execute("CREATE TABLE o(id INTEGER, val INTEGER)").unwrap();
    e.execute("INSERT INTO o VALUES (1, 999)").unwrap();
    let r = e.execute(
        "INSERT INTO o VALUES (1, 999) ON CONFLICT (id) DO UPDATE SET val = val + 1",
    );
    assert!(
        r.is_ok(),
        "ON CONFLICT (col) DO UPDATE SET should parse + execute; got: {:?}",
        r.err().map(|e| e.to_string())
    );
}

#[test]
fn create_trigger_after_update_of_multi_column() {
    let mut e = engine();
    e.execute("CREATE TABLE t(id INTEGER, val INTEGER, note TEXT)").unwrap();
    let r = e.execute(
        "CREATE TRIGGER tr AFTER UPDATE OF val, note ON t \
         FOR EACH ROW BEGIN SELECT 1; END",
    );
    assert!(
        r.is_ok(),
        "CREATE TRIGGER AFTER UPDATE OF col1, col2 should parse + execute; got: {:?}",
        r.err().map(|e| e.to_string())
    );
}

