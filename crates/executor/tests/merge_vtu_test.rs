use sqlrustgo_executor::execution::ExecutionResult;
use sqlrustgo_executor::ExecutorResult;
use sqlrustgo_planner::{Expr, MergeClause, MergeStatement};
use sqlrustgo_storage::engine::{ColumnDefinition, StorageEngine, TableInfo};
use sqlrustgo_storage::MemoryStorage;
use sqlrustgo_types::Value;
use std::sync::{Arc, RwLock};

fn create_memory_storage() -> Arc<RwLock<MemoryStorage>> {
    Arc::new(RwLock::new(MemoryStorage::new()))
}

#[test]
fn test_merge_statement_new() {
    let on_condition = Expr::binary_expr(
        Expr::column("source.id"),
        sqlrustgo_planner::Operator::Eq,
        Expr::column("target.id"),
    );
    let merge = MergeStatement::new(
        "target".to_string(),
        "source".to_string(),
        on_condition,
        None,
        None,
    );
    assert_eq!(merge.target_table, "target");
    assert_eq!(merge.source_table, "source");
    assert!(merge.matched_clause.is_none());
    assert!(merge.not_matched_clause.is_none());
}

#[test]
fn test_merge_statement_with_clauses() {
    let on_condition = Expr::binary_expr(
        Expr::column("s.id"),
        sqlrustgo_planner::Operator::Eq,
        Expr::column("t.id"),
    );
    let matched = MergeClause {
        update_columns: vec!["val".to_string()],
        update_values: vec![Expr::column("s.val")],
        insert_columns: vec![],
        insert_values: vec![],
    };
    let not_matched = MergeClause {
        update_columns: vec!["id".to_string(), "val".to_string()],
        update_values: vec![Expr::column("s.id"), Expr::column("s.val")],
        insert_columns: vec![],
        insert_values: vec![],
    };
    let merge = MergeStatement::new(
        "target".to_string(),
        "source".to_string(),
        on_condition,
        Some(matched),
        Some(not_matched),
    );
    assert!(merge.matched_clause.is_some());
    assert!(merge.not_matched_clause.is_some());
}

#[test]
fn test_execution_result_ok() {
    let result = ExecutionResult::ok(5);
    assert_eq!(result.affected_rows, 5);
    assert!(result.last_insert_id.is_none());
    assert!(result.payload.is_none());
}

#[test]
fn test_execution_result_with_payload() {
    let payload = vec![Value::Integer(1), Value::Text("a".to_string())];
    let result = ExecutionResult::with_payload(payload.clone());
    assert_eq!(result.affected_rows, 0);
    assert_eq!(result.payload, Some(payload));
}

#[test]
fn test_execution_result_with_insert_id() {
    let result = ExecutionResult::ok(1).with_insert_id(42);
    assert_eq!(result.last_insert_id, Some(42));
    assert_eq!(result.affected_rows, 1);
}

#[test]
fn test_executor_result_new() {
    let result = ExecutorResult::new(vec![vec![Value::Integer(1)], vec![Value::Integer(2)]], 2);
    assert_eq!(result.rows.len(), 2);
    assert_eq!(result.affected_rows, 2);
}

#[test]
fn test_executor_result_empty() {
    let result = ExecutorResult::empty();
    assert!(result.rows.is_empty());
    assert_eq!(result.affected_rows, 0);
}

#[test]
fn test_storage_create_table() {
    let storage = create_memory_storage();
    let table_info = TableInfo {
        name: "test_t".to_string(),
        columns: vec![ColumnDefinition::new("id", "INTEGER")],
        foreign_keys: vec![],
        unique_constraints: vec![],
        check_constraints: vec![],
        partition_info: None,
    };
    {
        let mut s = storage.write().unwrap();
        s.create_table(&table_info).unwrap();
    }
    {
        let s = storage.read().unwrap();
        let info = s.get_table_info("test_t").unwrap();
        assert_eq!(info.name, "test_t");
        assert_eq!(info.columns.len(), 1);
    }
}

#[test]
fn test_storage_insert_and_scan() {
    let storage = create_memory_storage();
    let table_info = TableInfo {
        name: "t".to_string(),
        columns: vec![ColumnDefinition::new("id", "INTEGER")],
        foreign_keys: vec![],
        unique_constraints: vec![],
        check_constraints: vec![],
        partition_info: None,
    };
    {
        let mut s = storage.write().unwrap();
        s.create_table(&table_info).unwrap();
        s.insert("t", vec![vec![Value::Integer(1)], vec![Value::Integer(2)]])
            .unwrap();
    }
    {
        let s = storage.read().unwrap();
        let rows = s.scan("t").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0][0], Value::Integer(1));
        assert_eq!(rows[1][0], Value::Integer(2));
    }
}

#[test]
fn test_storage_delete() {
    let storage = create_memory_storage();
    let table_info = TableInfo {
        name: "t".to_string(),
        columns: vec![ColumnDefinition::new("id", "INTEGER")],
        foreign_keys: vec![],
        unique_constraints: vec![],
        check_constraints: vec![],
        partition_info: None,
    };
    {
        let mut s = storage.write().unwrap();
        s.create_table(&table_info).unwrap();
        s.insert("t", vec![vec![Value::Integer(1)], vec![Value::Integer(2)]])
            .unwrap();
    }
    {
        let mut s = storage.write().unwrap();
        // MemoryStorage::delete ignores filters and deletes ALL rows
        let deleted = s.delete("t", &[Value::Integer(1)]).unwrap();
        assert_eq!(deleted, 2);
    }
    {
        let s = storage.read().unwrap();
        let rows = s.scan("t").unwrap();
        assert_eq!(rows.len(), 0);
    }
}

#[test]
fn test_column_definition_new() {
    let col = ColumnDefinition::new("name", "TEXT");
    assert_eq!(col.name, "name");
    assert_eq!(col.data_type, "TEXT");
    assert!(!col.nullable);
    assert!(!col.primary_key);
}

#[test]
fn test_table_info_default() {
    let info = TableInfo {
        name: "default_t".to_string(),
        columns: vec![],
        foreign_keys: vec![],
        unique_constraints: vec![],
        check_constraints: vec![],
        partition_info: None,
    };
    assert_eq!(info.name, "default_t");
    assert!(info.columns.is_empty());
}

#[test]
fn test_vtu_guard_wraps_storage() {
    use sqlrustgo_storage::VtuGuard;
    let inner = MemoryStorage::new();
    let guard = VtuGuard::new(inner, "merge_test");
    // Can access the inner storage through VtuGuard
    let _inner_ref: &MemoryStorage = guard.inner();
    assert!(guard.inner().get_table_info("nonexistent").is_err());
}
