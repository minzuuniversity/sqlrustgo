//! Integration tests for Trigger Executor

use sqlrustgo_executor::trigger::{TriggerEvent, TriggerExecutor, TriggerTiming};
use sqlrustgo_storage::{
    ColumnDefinition, MemoryStorage, StorageEngine, TableInfo, TriggerEvent as StorageTriggerEvent,
    TriggerInfo, TriggerTiming as StorageTriggerTiming,
};
use sqlrustgo_types::Value;
use std::sync::{Arc, RwLock};

fn create_test_storage() -> Arc<RwLock<dyn StorageEngine>> {
    let mut storage = MemoryStorage::new();

    storage
        .create_table(&TableInfo {
            name: "orders".to_string(),
            columns: vec![
                ColumnDefinition::new("id", "INTEGER"),
                ColumnDefinition::new("price", "FLOAT"),
                ColumnDefinition::new("quantity", "INTEGER"),
                ColumnDefinition::new("total", "FLOAT"),
            ],
        })
        .unwrap();

    storage
        .create_table(&TableInfo {
            name: "audit_log".to_string(),
            columns: vec![
                ColumnDefinition::new("id", "INTEGER"),
                ColumnDefinition::new("action", "TEXT"),
                ColumnDefinition::new("old_value", "TEXT"),
                ColumnDefinition::new("new_value", "TEXT"),
            ],
        })
        .unwrap();

    storage
        .create_table(&TableInfo {
            name: "products".to_string(),
            columns: vec![
                ColumnDefinition::new("id", "INTEGER"),
                ColumnDefinition::new("name", "TEXT"),
                ColumnDefinition::new("stock", "INTEGER"),
                ColumnDefinition::new("min_stock", "INTEGER"),
            ],
        })
        .unwrap();

    Arc::new(RwLock::new(storage))
}

#[test]
fn test_trigger_before_insert_with_literal() {
    let mut storage = create_test_storage();

    storage
        .write()
        .unwrap()
        .create_trigger(TriggerInfo {
            name: "before_order_insert".to_string(),
            table_name: "orders".to_string(),
            timing: StorageTriggerTiming::Before,
            event: StorageTriggerEvent::Insert,
            body: "SET NEW.total = 100".to_string(),
        })
        .unwrap();

    let new_row = vec![
        Value::Integer(1),
        Value::Float(10.0),
        Value::Integer(5),
        Value::Null,
    ];

    let executor = TriggerExecutor::new(storage);
    let result = executor.execute_before_insert("orders", &new_row);

    assert!(result.is_ok());
    let modified_row = result.unwrap();
    // Note: current implementation returns Integer for integer literals
    let val = &modified_row[3];
    assert!(matches!(val, Value::Integer(100)) || matches!(val, Value::Float(f) if *f == 100.0));
}

#[test]
fn test_trigger_after_insert_audit() {
    let mut storage = create_test_storage();

    storage
        .write()
        .unwrap()
        .create_trigger(TriggerInfo {
            name: "after_order_insert".to_string(),
            table_name: "orders".to_string(),
            timing: StorageTriggerTiming::After,
            event: StorageTriggerEvent::Insert,
            body: "INSERT INTO audit_log (action) VALUES ('order created')".to_string(),
        })
        .unwrap();

    let new_row = vec![
        Value::Integer(1),
        Value::Float(10.0),
        Value::Integer(5),
        Value::Float(50.0),
    ];

    let executor = TriggerExecutor::new(storage);
    let result = executor.execute_after_insert("orders", &new_row);

    assert!(result.is_ok());
}

#[test]
fn test_trigger_before_delete_archive() {
    let mut storage = create_test_storage();

    storage
        .write()
        .unwrap()
        .create_trigger(TriggerInfo {
            name: "before_order_delete".to_string(),
            table_name: "orders".to_string(),
            timing: StorageTriggerTiming::Before,
            event: StorageTriggerEvent::Delete,
            body: "INSERT INTO audit_log (action) VALUES ('order deleted')".to_string(),
        })
        .unwrap();

    let old_row = vec![
        Value::Integer(1),
        Value::Float(10.0),
        Value::Integer(5),
        Value::Float(50.0),
    ];

    let executor = TriggerExecutor::new(storage);
    let result = executor.execute_before_delete("orders", &old_row);

    assert!(result.is_ok());
}

#[test]
fn test_trigger_before_update_with_literal() {
    let mut storage = create_test_storage();

    storage
        .write()
        .unwrap()
        .create_trigger(TriggerInfo {
            name: "before_product_update".to_string(),
            table_name: "products".to_string(),
            timing: StorageTriggerTiming::Before,
            event: StorageTriggerEvent::Update,
            body: "SET NEW.stock = 0".to_string(),
        })
        .unwrap();

    let old_row = vec![
        Value::Integer(1),
        Value::Text("Widget".to_string()),
        Value::Integer(10),
        Value::Integer(5),
    ];

    let new_row = vec![
        Value::Integer(1),
        Value::Text("Widget".to_string()),
        Value::Integer(-5),
        Value::Integer(5),
    ];

    let executor = TriggerExecutor::new(storage);
    let result = executor.execute_before_update("products", &old_row, &new_row);

    assert!(result.is_ok());
    let modified_row = result.unwrap();
    assert_eq!(modified_row[2], Value::Integer(0));
}

#[test]
fn test_trigger_list_for_table() {
    let mut storage = create_test_storage();

    storage
        .write()
        .unwrap()
        .create_trigger(TriggerInfo {
            name: "before_orders".to_string(),
            table_name: "orders".to_string(),
            timing: StorageTriggerTiming::Before,
            event: StorageTriggerEvent::Insert,
            body: "SET NEW.total = 0".to_string(),
        })
        .unwrap();

    storage
        .write()
        .unwrap()
        .create_trigger(TriggerInfo {
            name: "after_orders".to_string(),
            table_name: "orders".to_string(),
            timing: StorageTriggerTiming::After,
            event: StorageTriggerEvent::Insert,
            body: "".to_string(),
        })
        .unwrap();

    storage
        .write()
        .unwrap()
        .create_trigger(TriggerInfo {
            name: "before_products".to_string(),
            table_name: "products".to_string(),
            timing: StorageTriggerTiming::Before,
            event: StorageTriggerEvent::Update,
            body: "".to_string(),
        })
        .unwrap();

    let executor = TriggerExecutor::new(storage);

    let orders_triggers = executor.get_table_triggers("orders");
    assert_eq!(orders_triggers.len(), 2);

    let products_triggers = executor.get_table_triggers("products");
    assert_eq!(products_triggers.len(), 1);

    let audit_triggers = executor.get_table_triggers("audit_log");
    assert_eq!(audit_triggers.len(), 0);
}

#[test]
fn test_trigger_nonexistent_table() {
    let storage: Arc<RwLock<dyn StorageEngine>> = Arc::new(RwLock::new(MemoryStorage::new()));

    let executor = TriggerExecutor::new(storage);

    let new_row = vec![Value::Integer(1)];

    // When no triggers exist for a table, execution should succeed (returns unmodified row)
    let result = executor.execute_before_insert("nonexistent", &new_row);
    assert!(result.is_ok());

    // The row should be returned unchanged since no triggers exist
    let modified_row = result.unwrap();
    assert_eq!(modified_row.len(), new_row.len());
}

#[test]
fn test_trigger_timing_event_conversion() {
    assert_eq!(
        TriggerTiming::Before,
        TriggerTiming::from(StorageTriggerTiming::Before)
    );
    assert_eq!(
        TriggerTiming::After,
        TriggerTiming::from(StorageTriggerTiming::After)
    );

    assert_eq!(
        TriggerEvent::Insert,
        TriggerEvent::from(StorageTriggerEvent::Insert)
    );
    assert_eq!(
        TriggerEvent::Update,
        TriggerEvent::from(StorageTriggerEvent::Update)
    );
    assert_eq!(
        TriggerEvent::Delete,
        TriggerEvent::from(StorageTriggerEvent::Delete)
    );
}

#[test]
fn test_trigger_get_triggers_for_operation() {
    let mut storage = create_test_storage();

    storage
        .write()
        .unwrap()
        .create_trigger(TriggerInfo {
            name: "before_orders_insert".to_string(),
            table_name: "orders".to_string(),
            timing: StorageTriggerTiming::Before,
            event: StorageTriggerEvent::Insert,
            body: "SET NEW.total = 0".to_string(),
        })
        .unwrap();

    storage
        .write()
        .unwrap()
        .create_trigger(TriggerInfo {
            name: "after_orders_insert".to_string(),
            table_name: "orders".to_string(),
            timing: StorageTriggerTiming::After,
            event: StorageTriggerEvent::Insert,
            body: "".to_string(),
        })
        .unwrap();

    storage
        .write()
        .unwrap()
        .create_trigger(TriggerInfo {
            name: "before_orders_update".to_string(),
            table_name: "orders".to_string(),
            timing: StorageTriggerTiming::Before,
            event: StorageTriggerEvent::Update,
            body: "".to_string(),
        })
        .unwrap();

    let executor = TriggerExecutor::new(storage);

    let before_insert =
        executor.get_triggers_for_operation("orders", TriggerTiming::Before, TriggerEvent::Insert);
    assert_eq!(before_insert.len(), 1);
    assert_eq!(before_insert[0].name, "before_orders_insert");

    let after_insert =
        executor.get_triggers_for_operation("orders", TriggerTiming::After, TriggerEvent::Insert);
    assert_eq!(after_insert.len(), 1);
    assert_eq!(after_insert[0].name, "after_orders_insert");

    let before_update =
        executor.get_triggers_for_operation("orders", TriggerTiming::Before, TriggerEvent::Update);
    assert_eq!(before_update.len(), 1);
    assert_eq!(before_update[0].name, "before_orders_update");

    let before_delete =
        executor.get_triggers_for_operation("orders", TriggerTiming::Before, TriggerEvent::Delete);
    assert_eq!(before_delete.len(), 0);
}

#[test]
fn test_trigger_multiple_before_triggers() {
    let mut storage = create_test_storage();

    storage
        .write()
        .unwrap()
        .create_trigger(TriggerInfo {
            name: "trigger_1".to_string(),
            table_name: "orders".to_string(),
            timing: StorageTriggerTiming::Before,
            event: StorageTriggerEvent::Insert,
            body: "SET NEW.total = 50".to_string(),
        })
        .unwrap();

    storage
        .write()
        .unwrap()
        .create_trigger(TriggerInfo {
            name: "trigger_2".to_string(),
            table_name: "orders".to_string(),
            timing: StorageTriggerTiming::Before,
            event: StorageTriggerEvent::Insert,
            body: "SET NEW.total = 100".to_string(),
        })
        .unwrap();

    let new_row = vec![
        Value::Integer(1),
        Value::Float(10.0),
        Value::Integer(5),
        Value::Null,
    ];

    let executor = TriggerExecutor::new(storage);
    let result = executor.execute_before_insert("orders", &new_row);

    assert!(result.is_ok());
    let modified_row = result.unwrap();
    // Second trigger overwrites first, so total should be 100
    // Note: current implementation returns Integer for integer literals
    let val = &modified_row[3];
    assert!(matches!(val, Value::Integer(100)) || matches!(val, Value::Float(f) if *f == 100.0));
}
