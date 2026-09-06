//! V312-95 v3 / P3-HINT-001 follow-up regression test:
//! `FileStorage::list_all_indexes()` must return every index that
//! was created via `CREATE INDEX`, so the executor's
//! `INDEXED BY <name>` validator (src/engine_select.rs) can confirm
//! the index exists. Before this fix, FileStorage did not override
//! the default trait method (which returns `Vec::new()`), so the
//! CLI batch-mode `INDEXED BY` validator reported "index does not
//! exist" for every index, even immediately after a successful
//! `CREATE INDEX`. The in-process `MemoryStorage` worked because
//! it has its own `list_all_indexes` override.

use sqlrustgo_storage::engine::{ColumnDefinition, IndexInfo, StorageEngine, TableInfo};
use sqlrustgo_storage::FileStorage;
use sqlrustgo_parser::IndexColumnSpec;

// V312-95 v3: FileStorage has an inherent 3-arg `create_index(table, column, idx)`
// method that shadows the StorageEngine trait's 1-arg `create_index(IndexInfo)`.
// We use the fully-qualified trait method so the IndexInfo record is registered
// in the metadata catalog. The 3-arg helper bypasses the catalog.
use std::fs;

fn fresh_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "v312_95_v3_list_idx_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn make_table(name: &str) -> TableInfo {
    TableInfo {
        name: name.to_string(),
        columns: vec![ColumnDefinition {
            name: "name".to_string(),
            data_type: "TEXT".to_string(),
            nullable: true,
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn make_index(table: &str, index_name: &str, column: &str) -> IndexInfo {
    IndexInfo {
        name: index_name.to_string(),
        table: table.to_string(),
        columns: vec![IndexColumnSpec {
            name: Some(column.to_string()),
            expression: None,
        }],
        is_unique: false,
        original_sql: format!("CREATE INDEX {} ON {}({})", index_name, table, column),
    }
}

#[test]
fn list_all_indexes_after_single_create() {
    let dir = fresh_dir();
    let mut storage = FileStorage::new(dir.clone()).unwrap();
    storage.create_table(&make_table("t")).unwrap();
    StorageEngine::create_index(&mut storage, make_index("t", "idx_t_name", "name")).unwrap();
    let indexes = storage.list_all_indexes();
    assert_eq!(indexes.len(), 1, "list_all_indexes must return the just-created index");
    let idx = &indexes[0];
    assert_eq!(idx.name, "idx_t_name");
    assert_eq!(idx.table, "t");
    assert_eq!(idx.columns.len(), 1);
    assert_eq!(idx.columns[0].name.as_deref(), Some("name"));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn list_all_indexes_empty_initially() {
    let dir = fresh_dir();
    let storage = FileStorage::new(dir.clone()).unwrap();
    let indexes = storage.list_all_indexes();
    assert!(indexes.is_empty(), "no CREATE INDEX => empty catalog");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn list_all_indexes_after_drop() {
    let dir = fresh_dir();
    let mut storage = FileStorage::new(dir.clone()).unwrap();
    storage.create_table(&make_table("t")).unwrap();
    StorageEngine::create_index(&mut storage, make_index("t", "idx_t_name", "name")).unwrap();
    assert_eq!(storage.list_all_indexes().len(), 1);
    // Drop via the trait method (user-supplied index name).
    StorageEngine::drop_index(&mut storage, "t", "idx_t_name").unwrap();
    assert_eq!(
        storage.list_all_indexes().len(),
        0,
        "drop_index must also remove the metadata entry"
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn list_all_indexes_multiple_indexes_same_table() {
    let dir = fresh_dir();
    let mut storage = FileStorage::new(dir.clone()).unwrap();
    let mut t = make_table("t");
    t.columns.push(ColumnDefinition {
        name: "id".to_string(),
        data_type: "INT".to_string(),
        nullable: true,
        primary_key: true,
        ..Default::default()
    });
    storage.create_table(&t).unwrap();
    StorageEngine::create_index(&mut storage, make_index("t", "idx_t_id", "id")).unwrap();
    StorageEngine::create_index(&mut storage, make_index("t", "idx_t_name", "name")).unwrap();
    let indexes = storage.list_all_indexes();
    assert_eq!(indexes.len(), 2, "both indexes must be in the catalog");
    let mut names: Vec<&str> = indexes.iter().map(|i| i.name.as_str()).collect();
    names.sort();
    assert_eq!(names, vec!["idx_t_id", "idx_t_name"]);
    fs::remove_dir_all(&dir).ok();
}
