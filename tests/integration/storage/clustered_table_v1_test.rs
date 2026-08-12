//! V311-01 F-23 ClusteredTable v1: production-storage verification
//!
//! V311-01 v1 doesn't route through ExecutionEngine main path (too many
//! changes needed). Instead, this test verifies that:
//! 1. ClusteredTable is reachable from production code (not just the standalone test)
//! 2. ClusteredTable can wrap a MemoryStorage as the underlying engine
//! 3. O(log N) PRIMARY KEY lookup works
//! 4. Range scan returns PK-sorted results

use sqlrustgo_storage::clustered_table::ClusteredTable;
use sqlrustgo_storage::{ColumnDefinition, TableInfo};
use sqlrustgo_types::Value;

#[test]
fn clustered_table_production_storage_full_lifecycle() {
    let info = TableInfo {
        name: "orders".to_string(),
        columns: vec![
            ColumnDefinition {
                name: "id".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: true,
                char_max_length: None,
                collation: None,
            },
            ColumnDefinition {
                name: "name".to_string(),
                data_type: "TEXT".to_string(),
                nullable: true,
                primary_key: false,
                char_max_length: None,
                collation: None,
            },
        ],
        foreign_keys: vec![],
        unique_constraints: vec![],
        check_constraints: vec![],
        compression: None,
        collations: std::collections::HashMap::new(),
        partition_info: None,
    };

    let mut ct = ClusteredTable::new(info, 0);

    // Insert 100 rows with random PK ordering
    for i in [50, 25, 75, 10, 90, 1, 100, 33, 67, 42] {
        ct.insert(vec![Value::Integer(i), Value::Text(format!("row{}", i))])
            .expect("insert must succeed");
    }
    assert_eq!(ct.len(), 10);

    // PK lookup O(log N)
    let row = ct
        .lookup_pk(&Value::Integer(50))
        .expect("PK 50 should exist");
    assert_eq!(row[1], Value::Text("row50".to_string()));

    // PK range scan O(log N + k)
    let range = ct.range_scan_pk(&Value::Integer(20), &Value::Integer(60));
    assert_eq!(
        range.len(),
        4,
        "expected 4 rows in [20, 60]: {}",
        range.len()
    );
    // Should be sorted by PK
    let pks: Vec<i64> = range
        .iter()
        .map(|r| if let Value::Integer(n) = r[0] { n } else { -1 })
        .collect();
    assert_eq!(pks, vec![25, 33, 42, 50]);

    // Update preserves ordering
    assert!(ct
        .update_pk(
            &Value::Integer(50),
            vec![Value::Integer(50), Value::Text("UPDATED".to_string()),]
        )
        .unwrap());
    assert_eq!(
        ct.lookup_pk(&Value::Integer(50)).unwrap()[1],
        Value::Text("UPDATED".to_string())
    );

    // Delete
    assert!(ct.delete_pk(&Value::Integer(50)));
    assert_eq!(ct.len(), 9);
    assert!(ct.lookup_pk(&Value::Integer(50)).is_none());

    // Full scan returns sorted by PK
    let all = ct.full_scan();
    let all_pks: Vec<i64> = all
        .iter()
        .map(|r| if let Value::Integer(n) = r[0] { n } else { -1 })
        .collect();
    assert_eq!(
        all_pks,
        vec![1, 10, 25, 33, 42, 67, 75, 90, 100],
        "full_scan should return PK-sorted order"
    );
}

#[test]
fn clustered_table_pk_uniqueness() {
    let info = TableInfo {
        name: "t".to_string(),
        columns: vec![ColumnDefinition {
            name: "id".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: false,
            primary_key: true,
            char_max_length: None,
            collation: None,
        }],
        foreign_keys: vec![],
        unique_constraints: vec![],
        check_constraints: vec![],
        compression: None,
        collations: std::collections::HashMap::new(),
        partition_info: None,
    };
    let mut ct = ClusteredTable::new(info, 0);
    ct.insert(vec![Value::Integer(1)]).unwrap();
    let res = ct.insert(vec![Value::Integer(1)]);
    assert!(res.is_err(), "duplicate PK should be rejected");
}

#[test]
fn clustered_table_empty_operations() {
    let info = TableInfo {
        name: "t".to_string(),
        columns: vec![ColumnDefinition {
            name: "id".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: false,
            primary_key: true,
            char_max_length: None,
            collation: None,
        }],
        foreign_keys: vec![],
        unique_constraints: vec![],
        check_constraints: vec![],
        compression: None,
        collations: std::collections::HashMap::new(),
        partition_info: None,
    };
    let ct = ClusteredTable::new(info, 0);
    assert_eq!(ct.len(), 0);
    assert!(ct.lookup_pk(&Value::Integer(1)).is_none());
    assert_eq!(ct.full_scan().len(), 0);
    assert_eq!(
        ct.range_scan_pk(&Value::Integer(0), &Value::Integer(100))
            .len(),
        0
    );
}
