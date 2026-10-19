//! #5057: `MemoryStorage`'s `_in_db` methods must resolve against the
//! database they are given.
//!
//! Three of them were either missing or outright broken:
//!
//! * `parallel_scan` looked its table up with `table.to_lowercase()`, which
//!   stopped matching once #5025 made keys database-scoped — a parallel scan
//!   of a table outside `default` failed with "Table not found".
//! * `scan_with_filter` / `delete_if` had no `_in_db` override at all, so
//!   they resolved through the trait default (`let _ = db;`).
//!
//! #5375: `current_db` / `set_current_db` were removed from the engine
//! (#5057 step 3), so every database-less call now resolves against
//! `DEFAULT_DATABASE` instead of a per-engine "active database" slot.
//! These tests are migrated to that contract: the database is now named
//! at the call site, and the unscoped calls are asserted to stay in
//! `default` rather than following a switch.

use sqlrustgo_storage::engine::{Record, RowFilter, StorageEngine, TableInfo};
use sqlrustgo_storage::MemoryStorage;
use sqlrustgo_types::Value;

fn table(name: &str) -> TableInfo {
    let mut info = TableInfo::default();
    info.name = name.to_string();
    info.columns = vec![sqlrustgo_storage::ColumnDefinition::new("id", "INTEGER")];
    info
}

/// Two databases, each holding a table called `t` with distinguishable rows,
/// plus `default` holding a `t` of its own.
fn seeded() -> MemoryStorage {
    let mut s = MemoryStorage::new();
    s.create_database("d1").unwrap();
    s.create_database("d2").unwrap();
    for (db, id) in [("d1", 1i64), ("d2", 2)] {
        s.create_table_in_db(db, &table("t")).unwrap();
        s.insert_in_db(db, "t", vec![vec![Value::Integer(id)]])
            .unwrap();
    }
    // `default` gets a table of the same name so the database-less calls
    // below have something to resolve to — and so a call that answered
    // with `default`'s row can be told apart from one that answered with
    // d1's or d2's.
    s.create_table_in_db("default", &table("t")).unwrap();
    s.insert_in_db("default", "t", vec![vec![Value::Integer(3)]])
        .unwrap();
    s
}

#[test]
fn parallel_scan_in_db_finds_a_table_outside_the_default_database() {
    let mut s = MemoryStorage::new();
    s.create_database("d1").unwrap();
    s.create_table_in_db("d1", &table("t")).unwrap();
    s.insert_in_db("d1", "t", vec![vec![Value::Integer(7)]])
        .unwrap();

    // Before the fix this was `Table not found: t` — the lookup used the
    // bare lowercased name while the cache is keyed `d1\x01t`.
    let parts = s
        .parallel_scan_in_db("d1", "t", 1)
        .expect("parallel_scan_in_db must find d1.t");
    let rows: Vec<Record> = parts
        .into_iter()
        .flat_map(|p| p.collect::<Vec<Record>>())
        .collect();
    assert_eq!(rows, vec![vec![Value::Integer(7)]]);
}

#[test]
fn unscoped_parallel_scan_resolves_against_the_default_database() {
    let s = seeded();
    let parts = s
        .parallel_scan("t", 1)
        .expect("parallel_scan must not fail");
    let rows: Vec<Record> = parts
        .into_iter()
        .flat_map(|p| p.collect::<Vec<Record>>())
        .collect();
    assert_eq!(
        rows,
        vec![vec![Value::Integer(3)]],
        "the unscoped scan resolves against `default`, so it must answer \
         with default's row — not d1's (1) and not d2's (2)"
    );
}

#[test]
fn parallel_scan_in_db_reads_the_named_database() {
    let s = seeded();
    let parts = s
        .parallel_scan_in_db("d1", "t", 1)
        .expect("explicit database must win");
    let rows: Vec<Record> = parts
        .into_iter()
        .flat_map(|p| p.collect::<Vec<Record>>())
        .collect();
    assert_eq!(rows, vec![vec![Value::Integer(1)]], "asked for d1");
}

#[test]
fn scan_with_filter_in_db_reads_the_named_database() {
    let s = seeded();
    let d1 = s.scan_with_filter_in_db("d1", "t", &|_| true).unwrap();
    let d2 = s.scan_with_filter_in_db("d2", "t", &|_| true).unwrap();
    assert_eq!(d1, vec![vec![Value::Integer(1)]]);
    assert_eq!(d2, vec![vec![Value::Integer(2)]]);
}

#[test]
fn delete_if_in_db_spares_the_other_database() {
    let mut s = seeded();
    let filter: RowFilter = Box::new(|row: &Record| row[0] == Value::Integer(1));
    let n = s.delete_if_in_db("d1", "t", &filter).unwrap();
    assert_eq!(n, 1);

    assert!(
        s.scan_in_db("d1", "t").unwrap().is_empty(),
        "d1's row was deleted"
    );
    assert_eq!(
        s.scan_in_db("d2", "t").unwrap(),
        vec![vec![Value::Integer(2)]],
        "d2's row of the same name must survive"
    );
}
