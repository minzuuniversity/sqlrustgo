//! V312-55D (Round-26 follow-up) regression tests — AFTER-trigger side-effects
//! must roll back atomically with the parent statement.
//!
//! Background (pre-V312-55D):
//!   A parent `INSERT INTO parent` inside `BEGIN ... ROLLBACK` correctly
//!   reverted the parent row, but the AFTER-trigger's side-effects (e.g.
//!   `INSERT INTO audit VALUES (NEW.id, ...)`) survived the ROLLBACK because
//!   only the parent's undo entry was captured — the trigger's audit row
//!   sat in FileStorage's `insert_buffer` with no inverse.
//!
//! Fix (V312-55D, commit message in engine_dml.rs:147-153):
//!   1. `TriggerExecutor` gains a `TriggerUndoRecorder` trait + `set_undo_recorder`
//!      hook so the engine can wire its transaction manager into trigger DML.
//!   2. `trigger_undo_sink: Arc<Mutex<Vec<UndoRecord>>>` lives on
//!      `ExecutionEngine`; trigger DML pushes typed `UndoRecord`s there.
//!   3. After every AFTER-trigger fires, `drain_trigger_undo_into_tx`
//!      moves those records into the active transaction's undo log so
//!      `ROLLBACK` (and `ROLLBACK TO SAVEPOINT`) re-plays them in reverse
//!      alongside the parent statement's undo entries.
//!
//! Scope of this test file:
//!   - AFTER INSERT/UPDATE/DELETE trigger side-effects roll back under
//!     `BEGIN ... ROLLBACK` (the canonical user-visible guarantee).
//!   - Trigger side-effects survive `BEGIN ... COMMIT` (no surprise loss
//!     of work the user committed).
//!   - Autocommit path (no `BEGIN`) is unchanged — trigger sink drops when
//!     `current_tx_id` is None.
//!
//! Out of scope (regression guards live elsewhere):
//!   - Parser-level trigger syntax — see `issue_4514_create_trigger_test.rs`.
//!   - SAVEPOINT-level undo — see `oracle_g5_sem1.rs`.
//!   - `BEFORE INSERT SET NEW.col` propagation — see V312-55C path.

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

/// V312-55D §1: AFTER INSERT trigger's audit row MUST be undone when the
/// parent INSERT is rolled back. Pre-V312-55D this left the audit row in
/// storage because only the parent's undo entry was captured.
#[test]
fn v312_55d_after_insert_trigger_rolled_back() {
    let mut e = fresh_engine();
    e.execute("CREATE TABLE orders (id int, sku text)").unwrap();
    e.execute("CREATE TABLE audit (id int, note text)").unwrap();
    e.execute(
        "CREATE TRIGGER audit_orders AFTER INSERT ON orders FOR EACH ROW \
         INSERT INTO audit VALUES (NEW.id, NEW.sku)",
    )
    .expect("CREATE TRIGGER AFTER INSERT");

    // Parent INSERT inside an explicit transaction.
    e.execute("BEGIN").expect("BEGIN");
    e.execute("INSERT INTO orders VALUES (1, 'sku-A')")
        .expect("INSERT INTO orders");
    e.execute("ROLLBACK").expect("ROLLBACK");

    // Both parent AND trigger side-effect must be gone — atomicity
    // guarantee that V312-55D added.
    let orders = select_all(&mut e, "orders");
    let audit = select_all(&mut e, "audit");
    assert!(
        orders.is_empty(),
        "ROLLBACK must remove parent INSERT row from orders, got {} rows",
        orders.len()
    );
    assert!(
        audit.is_empty(),
        "V312-55D: ROLLBACK must also remove AFTER-trigger audit row, got {} rows: {:?}",
        audit.len(),
        audit
    );
}

/// V312-55D §2: AFTER UPDATE trigger's audit row MUST be undone when the
/// parent UPDATE is rolled back. Uses OLD/NEW references inside a
/// BEGIN/END block body so the audit captures pre/post state.
#[test]
fn v312_55d_after_update_trigger_rolled_back() {
    let mut e = fresh_engine();
    e.execute("CREATE TABLE counters (id int PRIMARY KEY, val int)")
        .unwrap();
    e.execute("CREATE TABLE update_audit (id int, old_val int, new_val int)")
        .unwrap();
    // Seed one row so the UPDATE matches something.
    e.execute("INSERT INTO counters VALUES (1, 100)").unwrap();
    e.execute(
        "CREATE TRIGGER audit_updates AFTER UPDATE ON counters FOR EACH ROW \
         BEGIN \
         INSERT INTO update_audit VALUES (NEW.id, OLD.val, NEW.val); \
         END",
    )
    .expect("CREATE TRIGGER AFTER UPDATE");

    e.execute("BEGIN").expect("BEGIN");
    e.execute("UPDATE counters SET val = 999 WHERE id = 1")
        .expect("UPDATE counters");
    e.execute("ROLLBACK").expect("ROLLBACK");

    let counters = select_all(&mut e, "counters");
    let audit = select_all(&mut e, "update_audit");
    assert_eq!(
        counters.len(),
        1,
        "ROLLBACK must leave the original row in place"
    );
    assert_eq!(
        counters[0][1],
        Value::Integer(100),
        "ROLLBACK must restore val=100, got {:?}",
        counters[0][1]
    );
    assert!(
        audit.is_empty(),
        "V312-55D: ROLLBACK must also remove AFTER UPDATE audit row, got {} rows: {:?}",
        audit.len(),
        audit
    );
}

/// V312-55D §3: AFTER DELETE trigger's audit row MUST be undone when the
/// parent DELETE is rolled back. Audit captures the deleted row's OLD.id.
#[test]
fn v312_55d_after_delete_trigger_rolled_back() {
    let mut e = fresh_engine();
    e.execute("CREATE TABLE customers (id int PRIMARY KEY, name text)")
        .unwrap();
    e.execute("CREATE TABLE delete_audit (id int, name text)")
        .unwrap();
    e.execute("INSERT INTO customers VALUES (7, 'Alice')")
        .unwrap();
    e.execute(
        "CREATE TRIGGER audit_deletes AFTER DELETE ON customers FOR EACH ROW \
         BEGIN \
         INSERT INTO delete_audit VALUES (OLD.id, OLD.name); \
         END",
    )
    .expect("CREATE TRIGGER AFTER DELETE");

    e.execute("BEGIN").expect("BEGIN");
    e.execute("DELETE FROM customers WHERE id = 7")
        .expect("DELETE FROM customers");
    e.execute("ROLLBACK").expect("ROLLBACK");

    let customers = select_all(&mut e, "customers");
    let audit = select_all(&mut e, "delete_audit");
    assert_eq!(
        customers.len(),
        1,
        "ROLLBACK must restore the deleted row, got {} rows",
        customers.len()
    );
    assert_eq!(
        customers[0][1],
        Value::Text("Alice".to_string()),
        "ROLLBACK must restore name='Alice', got {:?}",
        customers[0][1]
    );
    assert!(
        audit.is_empty(),
        "V312-55D: ROLLBACK must also remove AFTER DELETE audit row, got {} rows: {:?}",
        audit.len(),
        audit
    );
}

/// V312-55D §4: COMMIT must NOT discard trigger side-effects — the user
/// explicitly committed, so the audit row stays. Regression guard so
/// future refactors don't accidentally leak the undo drain on COMMIT.
#[test]
fn v312_55d_after_insert_trigger_survives_commit() {
    let mut e = fresh_engine();
    e.execute("CREATE TABLE orders (id int, sku text)").unwrap();
    e.execute("CREATE TABLE audit (id int, note text)").unwrap();
    e.execute(
        "CREATE TRIGGER audit_orders AFTER INSERT ON orders FOR EACH ROW \
         INSERT INTO audit VALUES (NEW.id, NEW.sku)",
    )
    .expect("CREATE TRIGGER");

    e.execute("BEGIN").expect("BEGIN");
    e.execute("INSERT INTO orders VALUES (1, 'sku-A')")
        .expect("INSERT");
    e.execute("COMMIT").expect("COMMIT");

    let orders = select_all(&mut e, "orders");
    let audit = select_all(&mut e, "audit");
    assert_eq!(orders.len(), 1, "COMMIT must keep parent INSERT");
    assert_eq!(
        audit.len(),
        1,
        "V312-55D: COMMIT must keep AFTER-trigger audit row, got {} rows: {:?}",
        audit.len(),
        audit
    );
    assert_eq!(audit[0][0], Value::Integer(1));
    assert_eq!(audit[0][1], Value::Text("sku-A".to_string()));
}

/// V312-55D §5: Autocommit (no `BEGIN`) is unchanged. The trigger sink
/// drops when `current_tx_id` is None, so trigger side-effects persist
/// (the statement commits immediately). Regression guard so a future
/// refactor doesn't try to require a transaction on autocommit.
#[test]
fn v312_55d_after_insert_trigger_autocommit_persists() {
    let mut e = fresh_engine();
    e.execute("CREATE TABLE orders (id int, sku text)").unwrap();
    e.execute("CREATE TABLE audit (id int, note text)").unwrap();
    e.execute(
        "CREATE TRIGGER audit_orders AFTER INSERT ON orders FOR EACH ROW \
         INSERT INTO audit VALUES (NEW.id, NEW.sku)",
    )
    .expect("CREATE TRIGGER");

    // No BEGIN — autocommit path.
    e.execute("INSERT INTO orders VALUES (1, 'sku-A')")
        .expect("INSERT");

    let orders = select_all(&mut e, "orders");
    let audit = select_all(&mut e, "audit");
    assert_eq!(orders.len(), 1);
    assert_eq!(
        audit.len(),
        1,
        "Autocommit: trigger side-effect must persist, got {} rows: {:?}",
        audit.len(),
        audit
    );
}

/// V312-55D §6: Multiple parent inserts + multiple trigger fires inside
/// one transaction — every audit row must roll back alongside its parent.
#[test]
fn v312_55d_multiple_inserts_with_trigger_rollback() {
    let mut e = fresh_engine();
    e.execute("CREATE TABLE orders (id int, sku text)").unwrap();
    e.execute("CREATE TABLE audit (id int, note text)").unwrap();
    e.execute(
        "CREATE TRIGGER audit_orders AFTER INSERT ON orders FOR EACH ROW \
         INSERT INTO audit VALUES (NEW.id, NEW.sku)",
    )
    .expect("CREATE TRIGGER");

    e.execute("BEGIN").expect("BEGIN");
    e.execute("INSERT INTO orders VALUES (1, 'A')").unwrap();
    e.execute("INSERT INTO orders VALUES (2, 'B')").unwrap();
    e.execute("INSERT INTO orders VALUES (3, 'C')").unwrap();
    e.execute("ROLLBACK").expect("ROLLBACK");

    let orders = select_all(&mut e, "orders");
    let audit = select_all(&mut e, "audit");
    assert!(
        orders.is_empty(),
        "ROLLBACK must remove all 3 parent INSERTs, got {} rows",
        orders.len()
    );
    assert!(
        audit.is_empty(),
        "V312-55D: ROLLBACK must remove all 3 trigger audit rows, got {}: {:?}",
        audit.len(),
        audit
    );
}
