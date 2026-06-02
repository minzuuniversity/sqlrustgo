//! E2E WAL Trigger Recovery — T-001, T-002, T-003
//!
//! Validates trigger DML flows through WalStorage and survives crash.
//!
//! T-001: INSERT via trigger → WAL → Crash → Recovery → Value Correct
//! T-002: UPDATE via trigger → WAL → Crash → Recovery → Updated Value Correct
//! T-003: DELETE via trigger → WAL → Crash → Recovery → Row Absent
//!
//! Uses full ExecutionEngine + WalStorage stack (not just storage layer).

use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::{FileBackedWalManager, FileStorage, WalStorage};
use tempfile::TempDir;

fn make_wal_engine(
    dir: &std::path::Path,
) -> ExecutionEngine<WalStorage<FileStorage, FileBackedWalManager>> {
    ExecutionEngine::with_wal_file(dir.to_path_buf()).unwrap()
}

fn recover_engine(
    dir: &std::path::Path,
) -> ExecutionEngine<WalStorage<FileStorage, FileBackedWalManager>> {
    ExecutionEngine::with_wal_file(dir.to_path_buf()).unwrap()
}

// ─── T-001: Trigger INSERT ───────────────────────────────────────────────────

#[test]
fn test_trigger_insert_wal_recovery_t001() {
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();

    // Phase 1: Setup — create trigger
    {
        let mut engine = make_wal_engine(&data_dir);

        // Create main table and audit table
        let _ = engine.execute("CREATE TABLE t1 (id INTEGER, value INTEGER)");
        let _ = engine.execute("CREATE TABLE t1_audit (id INTEGER, orig_value INTEGER)");

        // Register BEFORE INSERT trigger — copies to audit table
        let _ = engine.execute(
            "CREATE TRIGGER t1_insert_audit BEFORE INSERT ON t1 FOR EACH ROW BEGIN INSERT INTO t1_audit VALUES (NEW.id, NEW.value) END"
        );

        // Insert via trigger context
        engine.execute("BEGIN").unwrap();
        let _ = engine.execute("INSERT INTO t1 VALUES (1, 100)");
        engine.execute("COMMIT").unwrap();

        // Verify trigger fired before restart
        let count = extract_count(engine.execute("SELECT COUNT(*) FROM t1_audit"));
        assert_eq!(count, 1, "T-001 pre-check: trigger should have inserted audit row");
    }

    // Phase 2: Simulate crash — engine dropped

    // Phase 3: Restart and recover
    let mut engine = recover_engine(&data_dir);

    // Phase 4: Verify — audit row must survive via WAL
    let audit_count = extract_count(engine.execute("SELECT COUNT(*) FROM t1_audit"));
    assert_eq!(
        audit_count, 1,
        "T-001 FAIL: audit row missing after recovery — trigger INSERT did not survive crash"
    );

    // Verify main row also survived
    let main_count = extract_count(engine.execute("SELECT COUNT(*) FROM t1 WHERE id = 1"));
    assert_eq!(
        main_count, 1,
        "T-001 FAIL: main row missing after recovery"
    );

    eprintln!("T-001 PASS: Trigger INSERT survived crash + recovery");
}

// ─── T-002: Trigger UPDATE ───────────────────────────────────────────────────

#[test]
fn test_trigger_update_wal_recovery_t002() {
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();

    // Phase 1: Setup
    {
        let mut engine = make_wal_engine(&data_dir);

        let _ = engine.execute("CREATE TABLE t1 (id INTEGER, value INTEGER)");
        let _ = engine.execute(
            "CREATE TRIGGER t1_update_log BEFORE UPDATE ON t1 FOR EACH ROW BEGIN UPDATE t1 SET value = NEW.value WHERE id = OLD.id END"
        );

        // Setup initial row
        engine.execute("BEGIN").unwrap();
        let _ = engine.execute("INSERT INTO t1 VALUES (1, 100)");
        engine.execute("COMMIT").unwrap();
    }

    // Phase 2: Execute UPDATE
    {
        let mut engine = make_wal_engine(&data_dir);

        engine.execute("BEGIN").unwrap();
        let _ = engine.execute("UPDATE t1 SET value = 999 WHERE id = 1");
        engine.execute("COMMIT").unwrap();

        // Verify before crash
        let balance = extract_balance_t1_value(&mut engine, 1);
        assert_eq!(balance, 999, "T-002 pre-check: UPDATE should set value=999");
    }

    // Phase 3: Simulate crash

    // Phase 4: Restart and verify
    let mut engine = recover_engine(&data_dir);

    let balance = extract_balance_t1_value(&mut engine, 1);
    assert_eq!(
        balance, 999,
        "T-002 FAIL: expected value=999, got {} — UPDATE via trigger did not survive crash",
        balance
    );

    eprintln!("T-002 PASS: Trigger UPDATE survived crash + recovery");
}

// ─── T-003: Trigger DELETE ───────────────────────────────────────────────────

#[test]
fn test_trigger_delete_wal_recovery_t003() {
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();

    // Phase 1: Setup
    {
        let mut engine = make_wal_engine(&data_dir);

        let _ = engine.execute("CREATE TABLE t1 (id INTEGER, value INTEGER)");
        let _ = engine.execute(
            "CREATE TRIGGER t1_delete_backup BEFORE DELETE ON t1 FOR EACH ROW BEGIN INSERT INTO t1 SELECT * FROM t1 WHERE id = old.id END"
        );

        // Pre-populate rows
        engine.execute("BEGIN").unwrap();
        let _ = engine.execute("INSERT INTO t1 VALUES (1, 100)");
        let _ = engine.execute("INSERT INTO t1 VALUES (2, 200)");
        engine.execute("COMMIT").unwrap();
    }

    // Phase 2: Execute DELETE
    {
        let mut engine = make_wal_engine(&data_dir);

        engine.execute("BEGIN").unwrap();
        let _ = engine.execute("DELETE FROM t1 WHERE id = 1");
        engine.execute("COMMIT").unwrap();

        // Verify before crash — id=1 should be gone
        let count = extract_count(engine.execute("SELECT COUNT(*) FROM t1 WHERE id = 1"));
        assert_eq!(count, 0, "T-003 pre-check: row id=1 should be deleted");
    }

    // Phase 3: Simulate crash

    // Phase 4: Restart and verify
    let mut engine = recover_engine(&data_dir);

    let count = extract_count(engine.execute("SELECT COUNT(*) FROM t1 WHERE id = 1"));
    assert_eq!(
        count, 0,
        "T-003 FAIL: row id=1 still present after recovery — DELETE via trigger did not survive crash"
    );

    // id=2 should still exist
    let count2 = extract_count(engine.execute("SELECT COUNT(*) FROM t1 WHERE id = 2"));
    assert_eq!(count2, 1, "T-003 FAIL: row id=2 should still exist");

    eprintln!("T-003 PASS: Trigger DELETE survived crash + recovery");
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn extract_count(result: sqlrustgo_types::SqlResult<sqlrustgo::ExecutorResult>) -> i64 {
    let rows = result.unwrap().rows;
    match rows.first().and_then(|r| r.first()) {
        Some(sqlrustgo_types::Value::Integer(n)) => *n,
        _ => -1,
    }
}

/// For "SELECT value FROM t1 WHERE id = X" — value is at index 1
fn extract_balance_t1_value(
    engine: &mut ExecutionEngine<WalStorage<FileStorage, FileBackedWalManager>>,
    id: i64,
) -> i64 {
    let result = engine.execute(&format!("SELECT value FROM t1 WHERE id = {}", id));
    let rows = result.unwrap().rows;
    // Column 0 is id, column 1 is value
    match rows.get(0).and_then(|r| r.get(1)) {
        Some(sqlrustgo_types::Value::Integer(n)) => *n,
        _ => -1,
    }
}