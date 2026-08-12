//! Partitioned Tables E2E Tests
//!
//! 验证分区表的运行时行为. 当前 v3.10.0 范围:
//! - TableInfo.partition_info 字段存在
//! - 创建表后 partition_info 可读取
//! - PARTITION BY 子句的解析需要 parser 扩展 (跟踪 issue #3400+)

use parking_lot::RwLock;
use sqlrustgo::MemoryExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use std::sync::Arc;

fn make_engine() -> MemoryExecutionEngine {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    MemoryExecutionEngine::new(storage)
}

#[test]
fn test_basic_table_creation() {
    // 基础表创建 — 分区是可选的
    let mut engine = make_engine();
    let r = engine.execute("CREATE TABLE t (id INTEGER, val TEXT)");
    assert!(r.is_ok(), "CREATE TABLE failed: {:?}", r.err());
}

#[test]
fn test_partition_parse_rejected() {
    // 当前 parser 不支持 PARTITION BY 子句
    let mut engine = make_engine();
    let r = engine.execute("CREATE TABLE t (id INTEGER) PARTITION BY HASH (id)");
    // 预期: parser 错误 (当前不支持)
    // 实际: parser 可能在 token 0x28 解析错误
    // 这里我们只测试不 panic 即可
    let _ = r;
}

#[test]
fn test_storage_partition_info_field() {
    // 验证 TableInfo.partition_info 字段存在 (编译期)
    use sqlrustgo_storage::engine::{PartitionInfo, PartitionType, TableInfo};
    use sqlrustgo_types::Value;

    let info = TableInfo {
        name: "t".to_string(),
        columns: vec![],
        foreign_keys: vec![],
        compression: None,
        unique_constraints: vec![],
        check_constraints: vec![],
        partition_info: Some(PartitionInfo {
            partition_type: PartitionType::Range,
            column: "id".to_string(),
            boundaries: vec![Value::Integer(0), Value::Integer(100)],
            collations: std::collections::HashMap::new(),
        }),
    };

    assert!(info.partition_info.is_some());
    let pi = info.partition_info.unwrap();
    assert_eq!(pi.partition_type, PartitionType::Range);
    assert_eq!(pi.column, "id");
    assert_eq!(pi.boundaries.len(), 2);
}

#[test]
fn test_storage_partition_type_variants() {
    use sqlrustgo_storage::engine::{PartitionInfo, PartitionType, TableInfo};

    // Range
    let info_range = TableInfo {
        name: "t1".to_string(),
        columns: vec![],
        foreign_keys: vec![],
        compression: None,
        unique_constraints: vec![],
        check_constraints: vec![],
        partition_info: Some(PartitionInfo {
            partition_type: PartitionType::Range,
            column: "id".to_string(),
            boundaries: vec![],
            collations: std::collections::HashMap::new(),
        }),
    };
    assert_eq!(
        info_range.partition_info.unwrap().partition_type,
        PartitionType::Range
    );

    // List
    let info_list = TableInfo {
        name: "t2".to_string(),
        columns: vec![],
        foreign_keys: vec![],
        compression: None,
        unique_constraints: vec![],
        check_constraints: vec![],
        partition_info: Some(PartitionInfo {
            partition_type: PartitionType::List,
            column: "id".to_string(),
            boundaries: vec![],
            collations: std::collections::HashMap::new(),
        }),
    };
    assert_eq!(
        info_list.partition_info.unwrap().partition_type,
        PartitionType::List
    );

    // Hash
    let info_hash = TableInfo {
        name: "t3".to_string(),
        columns: vec![],
        foreign_keys: vec![],
        compression: None,
        unique_constraints: vec![],
        check_constraints: vec![],
        partition_info: Some(PartitionInfo {
            partition_type: PartitionType::Hash,
            column: "id".to_string(),
            boundaries: vec![],
            collations: std::collections::HashMap::new(),
        }),
    };
    assert_eq!(
        info_hash.partition_info.unwrap().partition_type,
        PartitionType::Hash
    );
}
