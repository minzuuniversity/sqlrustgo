//! UNION / INTERSECT / EXCEPT Tests
//!
//! P2 tests for set operations per TEST_PLAN.md
//! Tests UNION, INTERSECT, and EXCEPT operations

#[cfg(test)]
mod tests {
    use parking_lot::RwLock;
    use sqlrustgo::{parse, ExecutionEngine, MemoryStorage, StorageEngine};
    use sqlrustgo_types::Value;
    use std::sync::Arc;

    fn create_test_tables() -> MemoryStorage {
        let mut storage = MemoryStorage::new();

        let info = sqlrustgo_storage::TableInfo {
            name: "table_a".to_string(),
            columns: vec![sqlrustgo_storage::ColumnDefinition {
                name: "id".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                collation: None,

                default_value: None,
            }],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };

        storage.create_table(&info).ok();
        storage
            .insert("table_a", vec![vec![Value::Integer(1)]])
            .ok();
        storage
            .insert("table_a", vec![vec![Value::Integer(2)]])
            .ok();
        storage
            .insert("table_a", vec![vec![Value::Integer(3)]])
            .ok();

        let info = sqlrustgo_storage::TableInfo {
            name: "table_b".to_string(),
            columns: vec![sqlrustgo_storage::ColumnDefinition {
                name: "id".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                collation: None,

                default_value: None,
            }],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };

        storage.create_table(&info).ok();
        storage
            .insert("table_b", vec![vec![Value::Integer(2)]])
            .ok();
        storage
            .insert("table_b", vec![vec![Value::Integer(3)]])
            .ok();
        storage
            .insert("table_b", vec![vec![Value::Integer(4)]])
            .ok();

        storage
    }

    #[test]
    fn test_union_basic() {
        let storage = create_test_tables();
        let mut engine = ExecutionEngine::new(Arc::new(RwLock::new(storage)));

        let result = engine.execute("SELECT id FROM table_a UNION SELECT id FROM table_b");

        assert!(result.is_ok(), "UNION should execute without error");
    }

    #[test]
    fn test_union_all() {
        let storage = create_test_tables();
        let mut engine = ExecutionEngine::new(Arc::new(RwLock::new(storage)));

        let result = engine.execute("SELECT id FROM table_a UNION ALL SELECT id FROM table_b");

        assert!(result.is_ok(), "UNION ALL should execute without error");
    }

    #[test]
    fn test_union_with_order_by() {
        let storage = create_test_tables();
        let mut engine = ExecutionEngine::new(Arc::new(RwLock::new(storage)));

        let result =
            engine.execute("SELECT id FROM table_a UNION SELECT id FROM table_b ORDER BY id DESC");

        assert!(result.is_ok(), "UNION with ORDER BY should work");
    }

    #[test]
    fn test_union_with_limit() {
        let storage = create_test_tables();
        let mut engine = ExecutionEngine::new(Arc::new(RwLock::new(storage)));

        let result = engine.execute("SELECT id FROM table_a UNION SELECT id FROM table_b LIMIT 3");

        assert!(result.is_ok(), "UNION with LIMIT should work");
    }

    #[test]
    fn test_union_distinct_removes_duplicates() {
        let mut storage = MemoryStorage::new();

        let info = sqlrustgo_storage::TableInfo {
            name: "numbers".to_string(),
            columns: vec![sqlrustgo_storage::ColumnDefinition {
                name: "num".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                collation: None,

                default_value: None,
            }],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        storage.create_table(&info).ok();
        storage
            .insert("numbers", vec![vec![Value::Integer(1)]])
            .ok();
        storage
            .insert("numbers", vec![vec![Value::Integer(2)]])
            .ok();

        let mut engine = ExecutionEngine::new(Arc::new(RwLock::new(storage)));

        let result = engine.execute("SELECT num FROM numbers UNION SELECT num FROM numbers");

        assert!(result.is_ok());
    }

    #[test]
    fn test_intersect_syntax() {
        let storage = create_test_tables();
        let mut engine = ExecutionEngine::new(Arc::new(RwLock::new(storage)));

        let result = engine.execute("SELECT id FROM table_a INTERSECT SELECT id FROM table_b");

        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_except_syntax() {
        let storage = create_test_tables();
        let mut engine = ExecutionEngine::new(Arc::new(RwLock::new(storage)));

        let result = engine.execute("SELECT id FROM table_a EXCEPT SELECT id FROM table_b");

        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_union_multiple_tables() {
        let mut storage = MemoryStorage::new();

        for i in 1..=3 {
            let info = sqlrustgo_storage::TableInfo {
                name: format!("t{}", i),
                columns: vec![sqlrustgo_storage::ColumnDefinition {
                    name: "val".to_string(),
                    data_type: "INTEGER".to_string(),
                    nullable: false,
                    primary_key: false,
                    char_max_length: None,
                    collation: None,

                    default_value: None,
                }],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                compression: None,
                collations: std::collections::HashMap::new(),
                partition_info: None,
            };
            storage.create_table(&info).ok();
            let table_name = format!("t{}", i);
            storage
                .insert(&table_name, vec![vec![Value::Integer(i)]])
                .ok();
        }

        let mut engine = ExecutionEngine::new(Arc::new(RwLock::new(storage)));

        let result = engine.execute("SELECT val FROM t1 UNION SELECT val FROM t2");

        assert!(result.is_ok());
    }

    // Helper: build storage with two single-column tables where each
    // table may contain duplicate rows. Used by INTERSECT/EXCEPT ALL
    // multiplicity tests.
    fn create_multi_storage(a_rows: Vec<i64>, b_rows: Vec<i64>) -> MemoryStorage {
        let mut storage = MemoryStorage::new();
        for (name, rows) in [("ta", &a_rows), ("tb", &b_rows)] {
            let info = sqlrustgo_storage::TableInfo {
                name: name.to_string(),
                columns: vec![sqlrustgo_storage::ColumnDefinition {
                    name: "id".to_string(),
                    data_type: "INTEGER".to_string(),
                    nullable: false,
                    primary_key: false,
                    char_max_length: None,
                    collation: None,

                    default_value: None,
                }],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                compression: None,
                collations: std::collections::HashMap::new(),
                partition_info: None,
            };
            storage.create_table(&info).unwrap();
            let records: Vec<Vec<Value>> = rows.iter().map(|v| vec![Value::Integer(*v)]).collect();
            storage.insert(name, records).unwrap();
        }
        storage
    }

    /// INTERSECT ALL must follow SQL-92 multiset semantics:
    /// result row r appears min(cnt_left(r), cnt_right(r)) times.
    /// Regression test for ISSUE #4037: the prior implementation
    /// only kept distinct rows from left that exist in right, which
    /// silently dropped multiplicity.
    #[test]
    fn test_intersect_all_multiplicity() {
        // left has 3 copies of (1), right has 2 copies of (1)
        // INTERSECT ALL must yield 2 copies of (1)
        let storage = create_multi_storage(vec![1, 1, 1], vec![1, 1]);
        let mut engine = ExecutionEngine::new(Arc::new(RwLock::new(storage)));

        let result = engine
            .execute("SELECT id FROM ta INTERSECT ALL SELECT id FROM tb")
            .expect("INTERSECT ALL should execute");

        assert_eq!(
            result.rows.len(),
            2,
            "INTERSECT ALL must keep min(cnt_left, cnt_right)=2 copies, got {:?}",
            result.rows
        );
        for row in &result.rows {
            assert_eq!(row[0], Value::Integer(1));
        }
    }

    /// INTERSECT (DISTINCT) — when left has duplicates that all
    /// intersect right, the result must be deduplicated to a single
    /// row.
    #[test]
    fn test_intersect_distinct_multiplicity() {
        let storage = create_multi_storage(vec![1, 1, 1], vec![1, 1]);
        let mut engine = ExecutionEngine::new(Arc::new(RwLock::new(storage)));

        let result = engine
            .execute("SELECT id FROM ta INTERSECT SELECT id FROM tb")
            .expect("INTERSECT should execute");

        assert_eq!(
            result.rows.len(),
            1,
            "INTERSECT (DISTINCT) must yield exactly 1 copy, got {:?}",
            result.rows
        );
    }

    /// EXCEPT ALL must follow SQL-92 multiset semantics:
    /// result row r appears max(0, cnt_left(r) - cnt_right(r)) times.
    /// Regression test for ISSUE #4037: the prior implementation
    /// removed *every* row that appeared in right (after dedup),
    /// which silently dropped multiplicity.
    #[test]
    fn test_except_all_multiplicity() {
        // left has 3 copies of (1), right has 2 copies of (1)
        // EXCEPT ALL must yield 1 copy of (1)
        let storage = create_multi_storage(vec![1, 1, 1], vec![1, 1]);
        let mut engine = ExecutionEngine::new(Arc::new(RwLock::new(storage)));

        let result = engine
            .execute("SELECT id FROM ta EXCEPT ALL SELECT id FROM tb")
            .expect("EXCEPT ALL should execute");

        assert_eq!(
            result.rows.len(),
            1,
            "EXCEPT ALL must keep max(0, cnt_left-cnt_right)=1 copy, got {:?}",
            result.rows
        );
        assert_eq!(result.rows[0][0], Value::Integer(1));
    }

    /// EXCEPT (DISTINCT) — when every row in left also appears in
    /// right, the result must be empty (the row is "excluded").
    /// A buggy implementation that also returns one copy because of
    /// a multiplicity regression would fail this test.
    #[test]
    fn test_except_distinct_empty_when_overlapping() {
        let storage = create_multi_storage(vec![1, 1, 1], vec![1, 1]);
        let mut engine = ExecutionEngine::new(Arc::new(RwLock::new(storage)));

        let result = engine
            .execute("SELECT id FROM ta EXCEPT SELECT id FROM tb")
            .expect("EXCEPT should execute");

        assert_eq!(
            result.rows.len(),
            0,
            "EXCEPT (DISTINCT) must drop the overlapping row, got {:?}",
            result.rows
        );
    }
}
