//! #5266: no-namespace storage impls must refuse a foreign database.
//!
//! The trait used to answer the `_in_db` read/write methods with a default
//! body that dropped `db` (`let _ = db;`) and touched whatever was current.
//! #5250/#5277 removed that default for `create_table_in_db`,
//! `drop_table_in_db`, `has_table_in`, `get_table_info_in` and
//! `scan_in_db`; this file locks the remaining read/write methods the same
//! way for the four no-namespace storages (`AppendOnlyStorage`,
//! `BinaryTableStorage`, `BinaryTableStorageV2`, `TableLevelStorage`).
//!
//! Contract under test (see `engine.rs`):
//! * `SqlResult` methods must return `Err` for a stated database other
//!   than `default` — they must not silently operate on the flat namespace.
//! * `list_tables_in_db` (returns `Vec<String>`) returns empty for a
//!   foreign database.
//! * `default` itself is accepted and delegates to the db-less version.

use sqlrustgo_storage::append_only_storage::AppendOnlyStorage;
use sqlrustgo_storage::engine::{
    RowFilter, RowMutation, SqlError, StorageEngine, TableInfo, Value,
};
use sqlrustgo_storage::binary_storage_v2::BinaryTableStorageV2;
use sqlrustgo_storage::table_level_storage::TableLevelStorage;
use sqlrustgo_storage::vtu_guard::VtuGuard;
use sqlrustgo_storage::wal::MemoryWalManager;
use sqlrustgo_storage::BinaryTableStorage;

fn table(name: &str) -> TableInfo {
    let mut info = TableInfo::default();
    info.name = name.to_string();
    info.columns = vec![sqlrustgo_storage::ColumnDefinition::new("id", "INTEGER")];
    info
}

fn foreign_db_err(e: SqlError) -> bool {
    matches!(e, SqlError::ExecutionError(msg) if msg.contains("no database namespace"))
}

/// AppendOnlyStorage: every `_in_db` SqlResult method must refuse a foreign db.
#[test]
fn append_only_refuses_foreign_db() {
    let mut s =
        AppendOnlyStorage::new(std::env::temp_dir().join("sqrg_5266_ao")).expect("temp dir");
    s.create_table(&table("t")).unwrap();
    let rec = vec![vec![Value::Integer(1)]];
    let filter: &dyn Fn(&sqlrustgo_storage::Record) -> bool = &|_| true;

    assert!(foreign_db_err(
        s.scan_with_filter_in_db("d1", "t", filter).unwrap_err()
    ));
    assert!(foreign_db_err(
        s.scan_with_index_in_db("d1", "t", "ix", &Value::Integer(1))
            .unwrap_err()
    ));
    assert!(foreign_db_err(
        s.parallel_scan_in_db("d1", "t", 1).map(|_| ()).unwrap_err()
    ));
    assert!(foreign_db_err(
        s.insert_in_db("d1", "t", rec.clone()).unwrap_err()
    ));
    assert!(foreign_db_err(s.delete_in_db("d1", "t", &[]).unwrap_err()));
    assert!(foreign_db_err(
        s.delete_collect_pks_in_db("d1", "t", &[]).unwrap_err()
    ));
    let rf: RowFilter = Box::new(|_| true);
    assert!(foreign_db_err(
        s.delete_if_in_db("d1", "t", &rf).unwrap_err()
    ));
    assert!(foreign_db_err(
        s.update_in_db("d1", "t", &[], &[(0, Value::Integer(2))])
            .unwrap_err()
    ));
    let mu = RowMutation::new(vec![(0usize, Value::Integer(2))], 0);
    assert!(foreign_db_err(
        s.update_if_in_db("d1", "t", &rf, &mu).unwrap_err()
    ));

    // default is accepted.
    s.insert_in_db("default", "t", rec).unwrap();
    assert!(!s.list_tables_in_db("d1").contains(&"t".to_string()));
}

/// BinaryTableStorage: every `_in_db` SqlResult method must refuse a foreign db.
#[test]
fn binary_table_refuses_foreign_db() {
    let mut s = BinaryTableStorage::new(std::env::temp_dir().join("sqrg_5266_bt")).unwrap();
    s.create_table(&table("t")).unwrap();
    let rec = vec![vec![Value::Integer(1)]];
    let filter: &dyn Fn(&sqlrustgo_storage::Record) -> bool = &|_| true;

    assert!(foreign_db_err(
        s.scan_with_filter_in_db("d1", "t", filter).unwrap_err()
    ));
    assert!(foreign_db_err(
        s.scan_with_index_in_db("d1", "t", "ix", &Value::Integer(1))
            .unwrap_err()
    ));
    assert!(foreign_db_err(
        s.parallel_scan_in_db("d1", "t", 1).map(|_| ()).unwrap_err()
    ));
    assert!(foreign_db_err(
        s.insert_in_db("d1", "t", rec.clone()).unwrap_err()
    ));
    assert!(foreign_db_err(s.delete_in_db("d1", "t", &[]).unwrap_err()));
    assert!(foreign_db_err(
        s.delete_collect_pks_in_db("d1", "t", &[]).unwrap_err()
    ));
    let rf: RowFilter = Box::new(|_| true);
    assert!(foreign_db_err(
        s.delete_if_in_db("d1", "t", &rf).unwrap_err()
    ));
    assert!(foreign_db_err(
        s.update_in_db("d1", "t", &[], &[(0, Value::Integer(2))])
            .unwrap_err()
    ));
    let mu = RowMutation::new(vec![(0usize, Value::Integer(2))], 0);
    assert!(foreign_db_err(
        s.update_if_in_db("d1", "t", &rf, &mu).unwrap_err()
    ));
}

/// BinaryTableStorageV2: every `_in_db` SqlResult method must refuse a foreign db.
#[test]
fn binary_table_v2_refuses_foreign_db() {
    let mut s = BinaryTableStorageV2::new(std::env::temp_dir().join("sqrg_5266_bv")).unwrap();
    StorageEngine::create_table(&mut s, &table("t")).unwrap();
    let rec = vec![vec![Value::Integer(1)]];
    let filter: &dyn Fn(&sqlrustgo_storage::Record) -> bool = &|_| true;

    assert!(foreign_db_err(
        s.scan_with_filter_in_db("d1", "t", filter).unwrap_err()
    ));
    assert!(foreign_db_err(
        s.scan_with_index_in_db("d1", "t", "ix", &Value::Integer(1))
            .unwrap_err()
    ));
    assert!(foreign_db_err(
        s.parallel_scan_in_db("d1", "t", 1).map(|_| ()).unwrap_err()
    ));
    assert!(foreign_db_err(
        s.insert_in_db("d1", "t", rec.clone()).unwrap_err()
    ));
    assert!(foreign_db_err(s.delete_in_db("d1", "t", &[]).unwrap_err()));
    assert!(foreign_db_err(
        s.delete_collect_pks_in_db("d1", "t", &[]).unwrap_err()
    ));
    let rf: RowFilter = Box::new(|_| true);
    assert!(foreign_db_err(
        s.delete_if_in_db("d1", "t", &rf).unwrap_err()
    ));
    assert!(foreign_db_err(
        s.update_in_db("d1", "t", &[], &[(0, Value::Integer(2))])
            .unwrap_err()
    ));
    let mu = RowMutation::new(vec![(0usize, Value::Integer(2))], 0);
    assert!(foreign_db_err(
        s.update_if_in_db("d1", "t", &rf, &mu).unwrap_err()
    ));
}

/// TableLevelStorage: every `_in_db` SqlResult method must refuse a foreign db.
#[test]
fn table_level_refuses_foreign_db() {
    // TableLevelStorage::create_table returns "not yet integrated"
    // (pre-existing, unrelated to #5266); the foreign-db check fires
    // before any table access, so no setup is needed.
    let mut s = TableLevelStorage::new(Box::new(MemoryWalManager::new()));
    let rec = vec![vec![Value::Integer(1)]];
    let filter: &dyn Fn(&sqlrustgo_storage::Record) -> bool = &|_| true;

    assert!(foreign_db_err(
        s.scan_with_filter_in_db("d1", "t", filter).unwrap_err()
    ));
    assert!(foreign_db_err(
        s.scan_with_index_in_db("d1", "t", "ix", &Value::Integer(1))
            .unwrap_err()
    ));
    assert!(foreign_db_err(
        s.parallel_scan_in_db("d1", "t", 1).map(|_| ()).unwrap_err()
    ));
    assert!(foreign_db_err(
        s.insert_in_db("d1", "t", rec.clone()).unwrap_err()
    ));
    assert!(foreign_db_err(s.delete_in_db("d1", "t", &[]).unwrap_err()));
    assert!(foreign_db_err(
        s.delete_collect_pks_in_db("d1", "t", &[]).unwrap_err()
    ));
    let rf: RowFilter = Box::new(|_| true);
    assert!(foreign_db_err(
        s.delete_if_in_db("d1", "t", &rf).unwrap_err()
    ));
    assert!(foreign_db_err(
        s.update_in_db("d1", "t", &[], &[(0, Value::Integer(2))])
            .unwrap_err()
    ));
    let mu = RowMutation::new(vec![(0usize, Value::Integer(2))], 0);
    assert!(foreign_db_err(
        s.update_if_in_db("d1", "t", &rf, &mu).unwrap_err()
    ));
}

/// Wrappers must forward, not drop: VtuGuard delegates to its inner engine.
#[test]
fn vtu_guard_forwards_foreign_db_to_inner() {
    // VtuGuard wraps a no-namespace storage; the inner refusal must surface.
    let inner =
        AppendOnlyStorage::new(std::env::temp_dir().join("sqrg_5266_vg")).expect("temp dir");
    let mut s = VtuGuard::new(inner, "test");
    let rec = vec![vec![Value::Integer(1)]];
    let e = s.insert_in_db("d1", "t", rec).unwrap_err();
    assert!(foreign_db_err(e), "VtuGuard must forward the inner refusal");
}
