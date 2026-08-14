// Stored Procedure Catalog Integration Tests
//
// These tests verify the stored procedure and trigger catalog integration
// that was added for Issue #1636 (存储过程与触发器 Catalog 集成)

use parking_lot::RwLock;
use sqlrustgo::Value;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use sqlrustgo_catalog::Catalog;
use std::sync::Arc;

#[test]
fn test_create_and_call_procedure_with_catalog() {
    let catalog = Arc::new(RwLock::new(Catalog::new("test")));
    let mut engine = ExecutionEngine::with_memory_and_catalog(catalog.clone());

    engine
        .execute("CREATE TABLE users (id INTEGER, name TEXT)")
        .unwrap();
    engine
        .execute("INSERT INTO users VALUES (1, 'Alice')")
        .unwrap();
    engine
        .execute("INSERT INTO users VALUES (2, 'Bob')")
        .unwrap();

    let create_result =
        engine.execute("CREATE PROCEDURE get_count() BEGIN SELECT COUNT(*) FROM users; END");
    assert!(
        create_result.is_ok(),
        "CREATE PROCEDURE should succeed with catalog"
    );

    let call_result = engine.execute("CALL get_count()");
    assert!(
        call_result.is_ok(),
        "CALL should succeed after procedure creation"
    );
}

#[test]
fn test_call_requires_catalog() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);

    let result = engine.execute("CALL my_proc(1, 2)");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("catalog"),
        "Error should mention catalog requirement"
    );
}

#[test]
fn test_create_procedure_requires_catalog() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);

    let result = engine.execute("CREATE PROCEDURE test_proc() BEGIN END");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("catalog"),
        "Error should mention catalog requirement"
    );
}

#[test]
fn test_procedure_not_found() {
    let catalog = Arc::new(RwLock::new(Catalog::new("test")));
    let mut engine = ExecutionEngine::with_memory_and_catalog(catalog);

    let result = engine.execute("CALL nonexistent_proc()");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("not found"),
        "Error should indicate procedure not found"
    );
}

#[test]
fn test_duplicate_procedure_error() {
    let catalog = Arc::new(RwLock::new(Catalog::new("test")));
    let mut engine = ExecutionEngine::with_memory_and_catalog(catalog.clone());

    let create1 = engine.execute("CREATE PROCEDURE test_dup() BEGIN SELECT 1; END");
    assert!(create1.is_ok(), "expected ok, got: {:?}", create1);

    let create2 = engine.execute("CREATE PROCEDURE test_dup() BEGIN SELECT 2; END");
    assert!(create2.is_err(), "Duplicate procedure should return error");
}

#[test]
fn test_create_trigger_with_catalog() {
    let catalog = Arc::new(RwLock::new(Catalog::new("test")));
    let mut engine = ExecutionEngine::with_memory_and_catalog(catalog.clone());

    engine
        .execute("CREATE TABLE users (id INTEGER, name TEXT, created_ts TEXT)")
        .unwrap();

    let create_trigger = engine.execute(
        "CREATE TRIGGER before_insert_ts BEFORE INSERT ON users FOR EACH ROW BEGIN SET NEW.created_ts = 'triggered'; END"
    );
    assert!(create_trigger.is_ok(), "CREATE TRIGGER should succeed");

    let insert_result = engine.execute("INSERT INTO users VALUES (1, 'Alice')");
    assert!(
        insert_result.is_ok(),
        "expected ok, got: {:?}",
        insert_result
    );
}

#[test]
fn test_before_insert_trigger_sets_column() {
    let catalog = Arc::new(RwLock::new(Catalog::new("test")));
    let mut engine = ExecutionEngine::with_memory_and_catalog(catalog.clone());

    engine
        .execute("CREATE TABLE orders (id INTEGER, price FLOAT, quantity INTEGER, total FLOAT)")
        .unwrap();

    let create_trigger = engine.execute(
        "CREATE TRIGGER calc_total BEFORE INSERT ON orders FOR EACH ROW BEGIN SET NEW.total = 50.0; END"
    );
    assert!(create_trigger.is_ok(), "CREATE TRIGGER should succeed");

    let result = engine.execute("INSERT INTO orders VALUES (1, 10.0, 5, 0.0)");
    assert!(
        result.is_ok(),
        "INSERT should succeed even if trigger modifies value"
    );

    let rows = engine.execute("SELECT * FROM orders").unwrap();
    assert_eq!(rows.rows.len(), 1);
}

#[test]
fn test_after_insert_trigger_executes() {
    let catalog = Arc::new(RwLock::new(Catalog::new("test")));
    let mut engine = ExecutionEngine::with_memory_and_catalog(catalog.clone());

    engine
        .execute("CREATE TABLE source (id INTEGER, val TEXT)")
        .unwrap();

    engine
        .execute("CREATE TRIGGER log_insert AFTER INSERT ON source FOR EACH ROW BEGIN END")
        .unwrap();

    let result = engine.execute("INSERT INTO source VALUES (1, 'test')");
    assert!(
        result.is_ok(),
        "AFTER INSERT trigger should execute without error"
    );
}

#[test]
fn test_before_update_trigger_sets_column() {
    let catalog = Arc::new(RwLock::new(Catalog::new("test")));
    let mut engine = ExecutionEngine::with_memory_and_catalog(catalog.clone());

    engine
        .execute("CREATE TABLE products (id INTEGER, price FLOAT, discounted_price FLOAT)")
        .unwrap();

    engine
        .execute("INSERT INTO products VALUES (1, 100.0, 0.0)")
        .unwrap();

    let create_trigger = engine.execute(
        "CREATE TRIGGER apply_discount BEFORE UPDATE ON products FOR EACH ROW BEGIN SET NEW.discounted_price = 90.0; END"
    );
    assert!(create_trigger.is_ok(), "CREATE TRIGGER should succeed");

    let result = engine.execute("UPDATE products SET price = 100.0 WHERE id = 1");
    assert!(
        result.is_ok(),
        "UPDATE should succeed even if trigger modifies value"
    );
}

#[test]
fn test_after_update_trigger_executes() {
    let catalog = Arc::new(RwLock::new(Catalog::new("test")));
    let mut engine = ExecutionEngine::with_memory_and_catalog(catalog.clone());

    engine
        .execute("CREATE TABLE items (id INTEGER, status TEXT)")
        .unwrap();

    engine
        .execute("INSERT INTO items VALUES (1, 'pending')")
        .unwrap();

    engine
        .execute("CREATE TRIGGER log_status AFTER UPDATE ON items FOR EACH ROW BEGIN END")
        .unwrap();

    let result = engine.execute("UPDATE items SET status = 'approved' WHERE id = 1");
    assert!(
        result.is_ok(),
        "AFTER UPDATE trigger should execute without error"
    );
}

#[test]
fn test_before_delete_trigger_executes() {
    let catalog = Arc::new(RwLock::new(Catalog::new("test")));
    let mut engine = ExecutionEngine::with_memory_and_catalog(catalog.clone());

    engine
        .execute("CREATE TABLE items (id INTEGER, name TEXT)")
        .unwrap();

    engine
        .execute("INSERT INTO items VALUES (1, 'to_delete')")
        .unwrap();

    engine
        .execute("CREATE TRIGGER archive_delete BEFORE DELETE ON items FOR EACH ROW BEGIN END")
        .unwrap();

    let result = engine.execute("DELETE FROM items WHERE id = 1");
    assert!(
        result.is_ok(),
        "BEFORE DELETE trigger should execute without error"
    );

    let remaining = engine.execute("SELECT * FROM items").unwrap();
    assert_eq!(remaining.rows.len(), 0, "Row should be deleted");
}

#[test]
fn test_after_delete_trigger_executes() {
    let catalog = Arc::new(RwLock::new(Catalog::new("test")));
    let mut engine = ExecutionEngine::with_memory_and_catalog(catalog.clone());

    engine
        .execute("CREATE TABLE orders (id INTEGER, amount FLOAT)")
        .unwrap();

    engine
        .execute("INSERT INTO orders VALUES (1, 100.0)")
        .unwrap();

    engine
        .execute("CREATE TRIGGER track_deletion AFTER DELETE ON orders FOR EACH ROW BEGIN END")
        .unwrap();

    let result = engine.execute("DELETE FROM orders WHERE id = 1");
    assert!(
        result.is_ok(),
        "AFTER DELETE trigger should execute without error"
    );
}

#[test]
fn test_multiple_triggers_on_same_table() {
    let catalog = Arc::new(RwLock::new(Catalog::new("test")));
    let mut engine = ExecutionEngine::with_memory_and_catalog(catalog.clone());

    engine
        .execute("CREATE TABLE events (id INTEGER, data TEXT, ts TEXT)")
        .unwrap();

    engine
        .execute("CREATE TRIGGER set_ts1 BEFORE INSERT ON events FOR EACH ROW BEGIN SET NEW.ts = 'first'; END")
        .unwrap();

    engine
        .execute("CREATE TRIGGER set_ts2 BEFORE INSERT ON events FOR EACH ROW BEGIN SET NEW.ts = 'second'; END")
        .unwrap();

    let result = engine.execute("INSERT INTO events VALUES (1, 'test', '')");
    assert!(result.is_ok());

    let rows = engine.execute("SELECT ts FROM events").unwrap();
    assert_eq!(rows.rows.len(), 1);
}

#[test]
fn test_trigger_executes_insert() {
    let catalog = Arc::new(RwLock::new(Catalog::new("test")));
    let mut engine = ExecutionEngine::with_memory_and_catalog(catalog.clone());

    engine
        .execute("CREATE TABLE orders (id INTEGER, product_id INTEGER, quantity INTEGER)")
        .unwrap();
    engine
        .execute("CREATE TABLE inventory (product_id INTEGER, stock INTEGER)")
        .unwrap();

    engine
        .execute("INSERT INTO inventory VALUES (1, 100)")
        .unwrap();

    let result = engine.execute(
        "CREATE TRIGGER decrement_stock AFTER INSERT ON orders FOR EACH ROW BEGIN UPDATE inventory SET stock = stock - NEW.quantity WHERE product_id = NEW.product_id; END"
    );
    assert!(result.is_ok(), "CREATE TRIGGER should succeed");

    let insert_result = engine.execute("INSERT INTO orders VALUES (1, 1, 10)");
    assert!(insert_result.is_ok(), "INSERT should trigger trigger");

    let inventory = engine
        .execute("SELECT stock FROM inventory WHERE product_id = 1")
        .unwrap();
    assert_eq!(inventory.rows.len(), 1);
}

#[test]
fn test_trigger_executes_update() {
    let catalog = Arc::new(RwLock::new(Catalog::new("test")));
    let mut engine = ExecutionEngine::with_memory_and_catalog(catalog.clone());

    engine
        .execute("CREATE TABLE products (id INTEGER, name TEXT, price INTEGER)")
        .unwrap();
    engine
        .execute(
            "CREATE TABLE price_history (product_id INTEGER, old_price INTEGER, new_price INTEGER)",
        )
        .unwrap();

    engine
        .execute("INSERT INTO products VALUES (1, 'Widget', 100)")
        .unwrap();

    let result = engine.execute(
        "CREATE TRIGGER log_price_change AFTER UPDATE ON products FOR EACH ROW BEGIN INSERT INTO price_history VALUES (OLD.id, OLD.price, NEW.price); END"
    );
    assert!(result.is_ok(), "CREATE TRIGGER should succeed");

    let update_result = engine.execute("UPDATE products SET price = 150 WHERE id = 1");
    assert!(update_result.is_ok(), "UPDATE should trigger trigger");

    let history = engine.execute("SELECT * FROM price_history").unwrap();
    assert_eq!(history.rows.len(), 1);
}
#[test]
fn test_trigger_executes_delete() {
    let catalog = Arc::new(RwLock::new(Catalog::new("test")));
    let mut engine = ExecutionEngine::with_memory_and_catalog(catalog.clone());

    engine
        .execute("CREATE TABLE orders (id INTEGER, status TEXT)")
        .unwrap();
    engine
        .execute("CREATE TABLE cancelled_orders (id INTEGER, status TEXT)")
        .unwrap();

    engine
        .execute("INSERT INTO orders VALUES (1, 'cancelled')")
        .unwrap();

    let result = engine.execute(
        "CREATE TRIGGER move_cancelled BEFORE DELETE ON orders FOR EACH ROW BEGIN INSERT INTO cancelled_orders VALUES (OLD.id, OLD.status); END"
    );
    assert!(result.is_ok(), "CREATE TRIGGER should succeed");

    let delete_result = engine.execute("DELETE FROM orders WHERE id = 1");
    assert!(delete_result.is_ok(), "DELETE should trigger trigger");

    let remaining = engine.execute("SELECT * FROM orders").unwrap();
    assert_eq!(remaining.rows.len(), 0);

    let cancelled = engine.execute("SELECT * FROM cancelled_orders").unwrap();
    assert_eq!(cancelled.rows.len(), 1);
}

/// C-3c.4: Trigger modifications are rolled back when the outer transaction rolls back.
#[test]
fn test_trigger_rollback_undoes_trigger_modifications() {
    let catalog = Arc::new(RwLock::new(Catalog::new("test")));
    let mut engine = ExecutionEngine::with_memory_and_catalog(catalog.clone());

    engine
        .execute("CREATE TABLE orders (id INTEGER, product_id INTEGER, quantity INTEGER)")
        .unwrap();
    engine
        .execute("CREATE TABLE inventory (product_id INTEGER, stock INTEGER)")
        .unwrap();
    engine
        .execute("INSERT INTO inventory VALUES (1, 100)")
        .unwrap();

    engine
        .execute(
            "CREATE TRIGGER decrement_stock AFTER INSERT ON orders FOR EACH ROW BEGIN \
             UPDATE inventory SET stock = stock - NEW.quantity WHERE product_id = NEW.product_id; \
             END",
        )
        .unwrap();

    // Outer transaction: insert an order (fires trigger that decrements stock), then ROLLBACK.
    engine.execute("BEGIN").unwrap();
    engine
        .execute("INSERT INTO orders VALUES (1, 1, 10)")
        .unwrap();

    // Mid-tx: the trigger has already fired; stock is decremented to 90 in the active tx log.
    let mid_tx_stock = engine
        .execute("SELECT stock FROM inventory WHERE product_id = 1")
        .unwrap();
    assert_eq!(mid_tx_stock.rows[0][0], Value::Integer(90));

    engine.execute("ROLLBACK").unwrap();

    // After ROLLBACK: the trigger's UPDATE on inventory must be undone. Stock stays at 100.
    let stock_after_rollback = engine
        .execute("SELECT stock FROM inventory WHERE product_id = 1")
        .unwrap();
    assert_eq!(
        stock_after_rollback.rows[0][0],
        Value::Integer(100),
        "Trigger modifications must be rolled back when outer tx rolls back"
    );

    // The orders insert itself must also be undone.
    let orders = engine.execute("SELECT * FROM orders").unwrap();
    assert_eq!(
        orders.rows.len(),
        0,
        "Outer-tx INSERT must be rolled back alongside the trigger side-effect"
    );
}

/// C-3c.5: Trigger modifications persist after COMMIT.
#[test]
fn test_trigger_commit_persists_trigger_modifications() {
    let catalog = Arc::new(RwLock::new(Catalog::new("test")));
    let mut engine = ExecutionEngine::with_memory_and_catalog(catalog.clone());

    engine
        .execute("CREATE TABLE orders (id INTEGER, product_id INTEGER, quantity INTEGER)")
        .unwrap();
    engine
        .execute("CREATE TABLE inventory (product_id INTEGER, stock INTEGER)")
        .unwrap();
    engine
        .execute("INSERT INTO inventory VALUES (1, 100)")
        .unwrap();

    engine
        .execute(
            "CREATE TRIGGER decrement_stock AFTER INSERT ON orders FOR EACH ROW BEGIN \
             UPDATE inventory SET stock = stock - NEW.quantity WHERE product_id = NEW.product_id; \
             END",
        )
        .unwrap();

    engine.execute("BEGIN").unwrap();
    engine
        .execute("INSERT INTO orders VALUES (1, 1, 10)")
        .unwrap();
    engine.execute("COMMIT").unwrap();

    // After COMMIT: trigger modification must be persisted.
    let stock = engine
        .execute("SELECT stock FROM inventory WHERE product_id = 1")
        .unwrap();
    assert_eq!(
        stock.rows[0][0],
        Value::Integer(90),
        "Trigger modifications must persist after outer tx commits"
    );

    let orders = engine.execute("SELECT * FROM orders").unwrap();
    assert_eq!(orders.rows.len(), 1, "Order row must persist after COMMIT");
}

// =============================================================================
// V312-55A / Issue #4238: Procedure DDL lifecycle integration tests
//
// Covers the full CREATE → DROP → SHOW PROCEDURE STATUS cycle plus
// IF EXISTS, OR REPLACE, and case-insensitive name lookup. The test
// name `procedure_ddl_*` is the suffix matched by
// `cargo test --test stored_proc_catalog_test procedure_ddl` in
// scripts/gate/check_v312_procedure_trigger_gate.sh
// (`V55A-Procedure-DDL` check, second arm).
// =============================================================================

#[test]
fn procedure_ddl_create_drop_show_lifecycle() {
    let catalog = Arc::new(RwLock::new(Catalog::new("test_ddl")));
    let mut engine = ExecutionEngine::with_memory_and_catalog(catalog.clone());

    // 1) CREATE — original casing preserved
    engine
        .execute("CREATE PROCEDURE MyProc() BEGIN SELECT 1; END")
        .expect("CREATE PROCEDURE should succeed");

    // 2) SHOW PROCEDURE STATUS lists the procedure by original name
    let show = engine
        .execute("SHOW PROCEDURE STATUS")
        .expect("SHOW PROCEDURE STATUS should succeed");
    assert_eq!(
        show.rows.len(),
        1,
        "exactly one procedure row expected; got {:?}",
        show.rows
    );
    assert_eq!(show.rows[0][0], Value::Text("MyProc".to_string()));
    assert_eq!(show.rows[0][1], Value::Integer(0), "no params");
    assert_eq!(show.rows[0][2], Value::Integer(1), "one body statement");

    // 3) Duplicate name → error
    let dup = engine.execute(
        "CREATE PROCEDURE myproc() BEGIN SELECT 2; END", /* different casing */
    );
    assert!(
        dup.is_err(),
        "duplicate (case-insensitive) procedure must fail; got {:?}",
        dup
    );

    // 4) OR REPLACE succeeds
    engine
        .execute("CREATE OR REPLACE PROCEDURE MyProc() BEGIN SELECT 99; END")
        .expect("CREATE OR REPLACE should succeed");
    let show2 = engine.execute("SHOW PROCEDURE STATUS").unwrap();
    assert_eq!(show2.rows.len(), 1);

    // 5) DROP PROCEDURE removes it
    engine
        .execute("DROP PROCEDURE MyProc")
        .expect("DROP PROCEDURE should succeed");
    let show3 = engine.execute("SHOW PROCEDURE STATUS").unwrap();
    assert_eq!(
        show3.rows.len(),
        0,
        "procedure must be gone after DROP"
    );
}

#[test]
fn procedure_ddl_drop_if_exists_semantics() {
    let catalog = Arc::new(RwLock::new(Catalog::new("test_if_exists")));
    let mut engine = ExecutionEngine::with_memory_and_catalog(catalog.clone());

    // IF EXISTS on a non-existent procedure must be a no-op (success).
    engine
        .execute("DROP PROCEDURE IF EXISTS ghost_proc")
        .expect("DROP IF EXISTS on missing procedure should succeed");

    // Create then drop with IF EXISTS — also success.
    engine
        .execute("CREATE PROCEDURE ghost_proc() BEGIN SELECT 1; END")
        .unwrap();
    engine
        .execute("DROP PROCEDURE IF EXISTS ghost_proc")
        .unwrap();

    // Without IF EXISTS, dropping an unknown procedure is an error.
    let bad = engine.execute("DROP PROCEDURE ghost_proc");
    assert!(
        bad.is_err(),
        "DROP PROCEDURE on missing proc without IF EXISTS must fail"
    );
}

#[test]
fn procedure_ddl_or_replace_creates_when_absent() {
    let catalog = Arc::new(RwLock::new(Catalog::new("test_or_replace")));
    let mut engine = ExecutionEngine::with_memory_and_catalog(catalog.clone());

    // OR REPLACE on a fresh procedure is equivalent to CREATE.
    engine
        .execute("CREATE OR REPLACE PROCEDURE fresh_proc() BEGIN SELECT 1; END")
        .unwrap();
    let show = engine.execute("SHOW PROCEDURE STATUS").unwrap();
    assert_eq!(show.rows.len(), 1);
    assert_eq!(show.rows[0][0], Value::Text("fresh_proc".to_string()));
}

#[test]
fn procedure_ddl_show_status_like_filter() {
    let catalog = Arc::new(RwLock::new(Catalog::new("test_like")));
    let mut engine = ExecutionEngine::with_memory_and_catalog(catalog.clone());

    engine
        .execute("CREATE PROCEDURE alpha_one() BEGIN SELECT 1; END")
        .unwrap();
    engine
        .execute("CREATE PROCEDURE alpha_two() BEGIN SELECT 2; END")
        .unwrap();
    engine
        .execute("CREATE PROCEDURE beta_one() BEGIN SELECT 3; END")
        .unwrap();

    let all = engine.execute("SHOW PROCEDURE STATUS").unwrap();
    assert_eq!(all.rows.len(), 3);

    let alpha = engine
        .execute("SHOW PROCEDURE STATUS LIKE 'alpha%'")
        .unwrap();
    assert_eq!(
        alpha.rows.len(),
        2,
        "LIKE alpha% must match alpha_one + alpha_two; got {:?}",
        alpha.rows
    );

    let only_one = engine
        .execute("SHOW PROCEDURE STATUS LIKE '%_one'")
        .unwrap();
    assert_eq!(
        only_one.rows.len(),
        2,
        "LIKE %_one must match alpha_one + beta_one; got {:?}",
        only_one.rows
    );

    let none = engine
        .execute("SHOW PROCEDURE STATUS LIKE 'nope%'")
        .unwrap();
    assert_eq!(none.rows.len(), 0);
}
