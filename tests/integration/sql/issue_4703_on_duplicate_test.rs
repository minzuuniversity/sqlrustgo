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
    // #5350: this used to assert that the engine *rejects* multi-column
    // `ON DUPLICATE KEY UPDATE ... VALUES(col)`. That expectation described a
    // parser gap, and the fixture had no PRIMARY KEY, so the statement could
    // never have engaged ODKU anyway — it degenerated to a plain INSERT.
    // `VALUES(col)` is now wired through, so lock down the real upsert
    // semantics instead: the duplicate row is updated in place (no second
    // row), `name` takes the incoming value, and `val = val + 50` applies to
    // the *stored* value.
    let mut e = engine();
    e.execute("CREATE TABLE m(id INTEGER PRIMARY KEY, name TEXT, val INTEGER)")
        .unwrap();
    e.execute("INSERT INTO m VALUES (1, 'alice', 100)").unwrap();
    let r = e.execute(
        "INSERT INTO m VALUES (1, 'alice_updated', 200) \
         ON DUPLICATE KEY UPDATE name = VALUES(name), val = val + 50",
    );
    assert!(
        r.is_ok(),
        "ENGINE API: multi-col ON DUPLICATE KEY UPDATE + VALUES() must execute; got: {:?}",
        r.as_ref().err().map(|e| e.to_string())
    );

    let rows = e
        .execute("SELECT id, name, val FROM m")
        .expect("SELECT after upsert")
        .rows;
    assert_eq!(
        rows.len(),
        1,
        "upsert must update in place, not append: {rows:?}"
    );
    assert_eq!(
        rows[0][1],
        sqlrustgo::Value::Text("alice_updated".to_string())
    );
    assert_eq!(rows[0][2], sqlrustgo::Value::Integer(150));
}

#[test]
fn on_conflict_do_update_set() {
    let mut e = engine();
    e.execute("CREATE TABLE o(id INTEGER, val INTEGER)")
        .unwrap();
    e.execute("INSERT INTO o VALUES (1, 999)").unwrap();
    let r = e.execute("INSERT INTO o VALUES (1, 999) ON CONFLICT (id) DO UPDATE SET val = val + 1");
    assert!(
        r.is_ok(),
        "ON CONFLICT (col) DO UPDATE SET should parse + execute; got: {:?}",
        r.err().map(|e| e.to_string())
    );
}

#[test]
fn create_trigger_after_update_of_multi_column() {
    let mut e = engine();
    e.execute("CREATE TABLE t(id INTEGER, val INTEGER, note TEXT)")
        .unwrap();
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
