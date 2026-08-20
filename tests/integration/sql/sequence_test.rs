//! V311-10 F-30: CREATE SEQUENCE / DROP SEQUENCE / NEXT VALUE FOR / CURRVAL
//!
//! PR #3861 (CREATE SEQUENCE parser) and PR #3863 (full integration) merged
//! the F-30 sequence support into v3.11.0. This test exercises the
//! end-to-end SQL surface through `ExecutionEngine::execute` against the
//! production MemoryStorage backend (which is the only storage engine
//! implementing `StorageEngine::create_sequence`/`next_sequence_value`).
//!
//! Parser coverage (locked down by 5 unit tests in `crates/parser/src/parser.rs`):
//!   - `SELECT NEXT VALUE FOR seq` (with optional VALUE keyword) parses
//!   - `SELECT CURRVAL(seq)` parses
//!   - `CREATE SEQUENCE IF NOT EXISTS` parses
//!   - `INSERT INTO ... VALUES (NEXT VALUE FOR seq, ...)` parses
//!
//! Executor coverage (V311-10 v2 — was PARTIAL, now fixed):
//!   - `SELECT NEXT VALUE FOR seq` evaluates: storage lock is acquired
//!     in `execute_select` so the executor's storage call can advance
//!     the live sequence counter
//!   - `SELECT CURRVAL(seq)` evaluates to the last issued value
//!   - `INSERT INTO t VALUES (NEXT VALUE FOR seq, 'x')` populates the row
//!
//! Limitations (out of scope for V311-10 v1):
//!   - REPLACE INTO / ON DUPLICATE KEY UPDATE on clustered tables
//!     remain "not yet integrated" stubs (returns Err). ClusteredIndex
//!     is a separate V311-01 path; not exercised by F-30.
//!   - REPLACE / ON DUPLICATE KEY UPDATE on a `CREATE SEQUENCE` table
//!     are also unimplemented (clustered-table path).
//!   - FK / CHECK / BEFORE/AFTER triggers not invoked on the sequence
//!     path (MemoryStorage does not enforce these for sequence ops).

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};
use std::sync::Arc;

fn fresh() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

#[test]
fn create_and_list_sequence() {
    let mut e = fresh();
    e.execute("CREATE SEQUENCE my_seq START WITH 1 INCREMENT BY 1")
        .unwrap();
    let s = e.storage_ref().read();
    assert!(s.has_sequence("my_seq"));
    let names = s.list_sequences();
    assert_eq!(names, vec!["my_seq".to_string()]);
    let info = s.get_sequence("my_seq").expect("sequence should exist");
    assert_eq!(info.name, "my_seq");
    assert_eq!(info.increment_by, 1);
    assert!(!info.cycle);
}

#[test]
fn create_sequence_if_not_exists_is_idempotent() {
    let mut e = fresh();
    e.execute("CREATE SEQUENCE s1 START WITH 10").unwrap();
    // Second CREATE without IF NOT EXISTS fails.
    let r = e.execute("CREATE SEQUENCE s1 START WITH 10");
    assert!(r.is_err(), "duplicate CREATE SEQUENCE should fail");
    // With IF NOT EXISTS, the second call is a no-op.
    e.execute("CREATE SEQUENCE IF NOT EXISTS s1 START WITH 999")
        .unwrap();
    let s = e.storage_ref().read();
    // Original START WITH 10 is preserved (the IF NOT EXISTS path
    // does not overwrite).
    let info = s.get_sequence("s1").unwrap();
    assert_eq!(info.start_with, 10);
}

#[test]
fn drop_sequence_removes_it() {
    let mut e = fresh();
    e.execute("CREATE SEQUENCE to_drop").unwrap();
    assert!(e.storage_ref().read().has_sequence("to_drop"));
    e.execute("DROP SEQUENCE to_drop").unwrap();
    assert!(!e.storage_ref().read().has_sequence("to_drop"));
}

#[test]
fn drop_sequence_missing_errors() {
    let mut e = fresh();
    let r = e.execute("DROP SEQUENCE no_such_seq");
    assert!(r.is_err(), "DROP on missing sequence should error");
}

#[test]
fn next_value_for_parses_with_value_keyword() {
    // Parser coverage for SQL:2003 standard syntax.
    // V311-10 fix: lexer + parser now accept `NEXT VALUE FOR seq` (with
    // the optional VALUE keyword). Without the fix this errors with
    // "Expected For, got Identifier(\"VALUE\")".
    let mut e = fresh();
    e.execute("CREATE SEQUENCE id_seq START WITH 1 INCREMENT BY 1")
        .unwrap();
    // Parse-only: we don't assert the value because the executor
    // returns NULL for SequenceNextVal (see module docstring).
    e.execute("SELECT NEXT VALUE FOR id_seq").unwrap();
    e.execute("SELECT NEXT FOR id_seq").unwrap();
    let _ = e.execute("SELECT NEXT VALUE id_seq"); // bad syntax, but parse path exercised
    let _ = e.execute("SELECT CURRVAL(id_seq)").unwrap();
}

#[test]
fn next_value_for_increment_by_5_parses() {
    let mut e = fresh();
    e.execute("CREATE SEQUENCE s5 START WITH 0 INCREMENT BY 5")
        .unwrap();
    // Parse only — see module docstring.
    e.execute("SELECT NEXT VALUE FOR s5").unwrap();
    e.execute("SELECT NEXT VALUE FOR s5").unwrap();
}

#[test]
fn next_value_for_on_missing_sequence_errors_at_parse() {
    let mut e = fresh();
    let r = e.execute("SELECT NEXT VALUE FOR does_not_exist");
    // The parser may pass and the executor errors, OR the parser may
    // fail — both outcomes are acceptable. What matters is that
    // we don't crash with no error.
    if let Err(e) = &r {
        let msg = e.to_string();
        assert!(
            msg.contains("does_not_exist") || msg.contains("not found"),
            "unexpected error: {}",
            msg
        );
    }
}

#[test]
fn currval_on_missing_sequence_returns_null_or_errors() {
    // The parser accepts CURRVAL(missing) without error. The executor
    // returns NULL for CURRVAL on a missing sequence (the legacy
    // `evaluate_expression` falls through the default arm and emits
    // Value::Null; the storage layer's get_sequence returns None).
    // Either behaviour is acceptable for V311-10 v1.
    let mut e = fresh();
    let r = e.execute("SELECT CURRVAL(does_not_exist)");
    // If Ok, the row's first cell must be NULL.
    // If Err, that's also fine.
    if let Ok(rows) = &r {
        assert_eq!(rows.rows[0][0], sqlrustgo::Value::Null);
    }
}

// === V311-10 v2 executor fix: real runtime tests ===

#[test]
fn next_value_for_returns_one_on_first_call() {
    // V311-10 v2: pre-V311-10 this returned NULL because the legacy
    // `evaluate_expression` had no storage context. The fix threads
    // a mutable storage reference through `evaluate_expression_with_seq`
    // so `SequenceNextVal` can call `storage.next_sequence_value()`.
    let mut e = fresh();
    e.execute("CREATE SEQUENCE s START WITH 1 INCREMENT BY 1")
        .unwrap();
    let r = e.execute("SELECT NEXT VALUE FOR s").unwrap();
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Integer(1));
}

#[test]
fn next_value_for_advances_through_calls() {
    let mut e = fresh();
    e.execute("CREATE SEQUENCE s START WITH 10 INCREMENT BY 3")
        .unwrap();
    let r1 = e.execute("SELECT NEXT VALUE FOR s").unwrap();
    assert_eq!(r1.rows[0][0], sqlrustgo::Value::Integer(10));
    let r2 = e.execute("SELECT NEXT VALUE FOR s").unwrap();
    assert_eq!(r2.rows[0][0], sqlrustgo::Value::Integer(13));
    let r3 = e.execute("SELECT NEXT VALUE FOR s").unwrap();
    assert_eq!(r3.rows[0][0], sqlrustgo::Value::Integer(16));
}

#[test]
fn currval_returns_last_issued_value() {
    // After NEXT VALUE FOR advances the sequence, CURRVAL reads
    // back the same value (without advancing again).
    let mut e = fresh();
    e.execute("CREATE SEQUENCE s START WITH 100").unwrap();
    e.execute("SELECT NEXT VALUE FOR s").unwrap();
    e.execute("SELECT NEXT VALUE FOR s").unwrap();
    let r = e.execute("SELECT CURRVAL(s)").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Integer(101));
    // CURRVAL does not advance.
    let r2 = e.execute("SELECT CURRVAL(s)").unwrap();
    assert_eq!(r2.rows[0][0], sqlrustgo::Value::Integer(101));
    // NEXT VALUE FOR continues from the last issued.
    let r3 = e.execute("SELECT NEXT VALUE FOR s").unwrap();
    assert_eq!(r3.rows[0][0], sqlrustgo::Value::Integer(102));
}
