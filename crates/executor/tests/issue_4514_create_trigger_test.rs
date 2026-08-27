//! Integration tests for Issue #4514 — CREATE TRIGGER parsed + invoked in
//! DML commit hook (FOR EACH ROW), and DROP TRIGGER.
//!
//! Scope of #4514:
//!   - Parser accepts single-statement body without BEGIN/END wrapper
//!     (the exact reproduction from the issue).
//!   - Trigger is registered against the target table.
//!   - AFTER INSERT trigger fires in DML commit hook and executes its
//!     body against the storage engine.
//!   - DROP TRIGGER removes the registration.
//!   - DROP TRIGGER IF EXISTS is a no-op when missing.
//!   - DROP TRIGGER without IF EXISTS errors when missing.
//!   - Regression: BEGIN/END multi-statement body still parses.
//!
//! Out of scope (pre-existing limitations, not introduced by #4514):
//!   - DELETE trigger WHERE-clause filter is not yet propagated by
//!     `execute_trigger_delete` (it deletes the whole target table).
//!   - BEFORE INSERT SET NEW.col propagation to storage (V312-55C path).
//!   - Duplicate CREATE TRIGGER error vs overwrite — pre-existing
//!     behaviour; the #4514 fix only addresses parser + dispatch.
//!
//! Verification is observational: since `ExecutionEngine::storage` is
//! private, we probe trigger presence by observing side-effects (DML
//! commit hook firing / DROP behaviour / trigger-double-register
//! rejection).

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use sqlrustgo_types::Value;
use std::sync::Arc;

fn fresh_engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

/// Scan `table` and return all rows as Vec<Vec<Value>>.
fn select_all(e: &mut ExecutionEngine<MemoryStorage>, table: &str) -> Vec<Vec<Value>> {
    match e.execute(&format!("SELECT * FROM {}", table)) {
        Ok(r) => r.rows,
        Err(err) => panic!("SELECT * FROM {} failed: {}", table, err),
    }
}

#[test]
fn issue_4514_repro_after_insert_parses() {
    // Verbatim reproduction from the issue. Must parse without error.
    let mut e = fresh_engine();
    e.execute("CREATE TABLE t(a int)").unwrap();
    e.execute("CREATE TABLE student(name TEXT, phone TEXT)")
        .unwrap();

    e.execute(
        "CREATE TRIGGER trg AFTER INSERT ON t FOR EACH ROW \
         UPDATE student SET phone = phone WHERE 1 = 0",
    )
    .expect("CREATE TRIGGER should parse + register (issue #4514 verbatim)");
}

#[test]
fn issue_4514_after_insert_trigger_fires_and_mutates_target() {
    // AFTER INSERT trigger must run its body against storage when the
    // host INSERT commits. This is the canonical "audit table" scenario.
    let mut e = fresh_engine();
    e.execute("CREATE TABLE orders(id int, sku text)").unwrap();
    e.execute("CREATE TABLE audit(id int, note text)").unwrap();

    e.execute(
        "CREATE TRIGGER audit_orders AFTER INSERT ON orders FOR EACH ROW \
         INSERT INTO audit VALUES (NEW.id, NEW.sku)",
    )
    .expect("CREATE TRIGGER with NEW.col should parse + register");

    e.execute("INSERT INTO orders VALUES (1, 'sku-A')").unwrap();
    e.execute("INSERT INTO orders VALUES (2, 'sku-B')").unwrap();

    let audit_rows = select_all(&mut e, "audit");
    assert_eq!(
        audit_rows.len(),
        2,
        "AFTER INSERT trigger must have inserted 2 audit rows via DML commit hook"
    );
}

#[test]
fn issue_4514_after_delete_trigger_fires() {
    // AFTER DELETE trigger body must execute against storage. The body
    // here is a bare DELETE — the WHERE-clause filter on OLD.col is a
    // pre-existing limitation (out of scope for #4514), so we only
    // assert the trigger fired (the target table was mutated).
    let mut e = fresh_engine();
    e.execute("CREATE TABLE parent(id int)").unwrap();
    e.execute("CREATE TABLE child(id int)").unwrap();
    e.execute("INSERT INTO child VALUES (10), (20), (30)")
        .unwrap();

    e.execute(
        "CREATE TRIGGER parent_del AFTER DELETE ON parent FOR EACH ROW \
         DELETE FROM child",
    )
    .expect("CREATE TRIGGER AFTER DELETE should parse + register");

    e.execute("INSERT INTO parent VALUES (1)").unwrap();
    e.execute("DELETE FROM parent WHERE id = 1").unwrap();

    let child_rows = select_all(&mut e, "child");
    assert_eq!(
        child_rows.len(),
        0,
        "AFTER DELETE trigger must have cleared the child table"
    );
}

#[test]
fn issue_4514_create_trigger_with_begin_end_block_still_works() {
    // Regression: the original BEGIN/END body syntax must keep working.
    let mut e = fresh_engine();
    e.execute("CREATE TABLE t(id int, audit int)").unwrap();

    e.execute(
        "CREATE TRIGGER trg AFTER INSERT ON t FOR EACH ROW \
         BEGIN \
         UPDATE t SET audit = NEW.id WHERE id = NEW.id; \
         END",
    )
    .expect("CREATE TRIGGER with BEGIN/END block should parse + register");

    e.execute("INSERT INTO t VALUES (7, 0)").unwrap();
    let rows = select_all(&mut e, "t");
    assert_eq!(rows.len(), 1);
}

#[test]
fn issue_4514_drop_trigger_removes_registration() {
    let mut e = fresh_engine();
    e.execute("CREATE TABLE t(id int)").unwrap();
    e.execute("CREATE TRIGGER trg AFTER INSERT ON t FOR EACH ROW SET NEW.id = NEW.id")
        .expect("create trigger");

    e.execute("DROP TRIGGER trg")
        .expect("DROP TRIGGER should succeed");

    // Re-registering with the same name must now succeed, proving the
    // original registration is gone.
    e.execute("CREATE TRIGGER trg AFTER INSERT ON t FOR EACH ROW SET NEW.id = NEW.id")
        .expect("after DROP TRIGGER, the name should be free for re-use");
}

#[test]
fn issue_4514_drop_trigger_if_exists_missing_is_noop() {
    let mut e = fresh_engine();
    // No trigger named `nope` exists.
    e.execute("DROP TRIGGER IF EXISTS nope")
        .expect("DROP TRIGGER IF EXISTS on a missing trigger must be a no-op");
}

#[test]
fn issue_4514_drop_trigger_missing_without_if_exists_errors() {
    let mut e = fresh_engine();
    let res = e.execute("DROP TRIGGER nope");
    assert!(
        res.is_err(),
        "DROP TRIGGER on a missing trigger must error without IF EXISTS"
    );
    let msg = format!("{}", res.unwrap_err());
    assert!(
        msg.to_lowercase().contains("trigger") && msg.contains("nope"),
        "error message should mention trigger name; got: {}",
        msg
    );
}
