//! #5057: `MemoryStorage`'s `_in_db` methods must actually scope.
//!
//! They arrived as trait defaults that ignore the `db` argument
//! (`let _ = db; self.delete(table, filters)`), so they looked like the
//! explicit-database API while behaving like the shared-state one.
//!
//! `MemoryStorage` is not a test-only backend: `mysql-server` builds one
//! for `--memory` and the CLI defaults to it.

use sqlrustgo_storage::engine::{StorageEngine, TableInfo};
use sqlrustgo_storage::MemoryStorage;

fn table(name: &str) -> TableInfo {
    let mut info = TableInfo::default();
    info.name = name.to_string();
    info.columns = vec![sqlrustgo_storage::ColumnDefinition::new("id", "INTEGER")];
    info
}

fn seeded() -> MemoryStorage {
    let mut s = MemoryStorage::new();
    s.create_database("d1").unwrap();
    s.create_database("d2").unwrap();
    for db in ["d1", "d2"] {
        s.set_current_db(db).unwrap();
        s.create_table(&table("t")).unwrap();
    }
    s
}

#[test]
fn create_table_in_db_lands_in_the_named_database() {
    let mut s = seeded();
    // Stored state says d2 throughout.
    s.set_current_db("d2").unwrap();
    s.create_table_in_db("d1", &table("made")).unwrap();

    assert!(s.has_table_in("d1", "made"), "d1 should have the new table");
    assert!(
        !s.has_table_in("d2", "made"),
        "the table must not appear in whichever database happens to be stored"
    );
}

#[test]
fn drop_table_in_db_leaves_the_other_database_alone() {
    let mut s = seeded();
    s.drop_table_in_db("d1", "t").unwrap();

    assert!(!s.has_table_in("d1", "t"), "d1's table was dropped");
    assert!(
        s.has_table_in("d2", "t"),
        "d2 has a table of the same name and must keep it"
    );
}
