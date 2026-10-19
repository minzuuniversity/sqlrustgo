//! #5057: `FileStorage`'s `parallel_scan_in_db` / `scan_with_filter_in_db`
//! must resolve against the database they are given, not the engine-wide
//! "current database".
//!
//! Both used to fall through to the `StorageEngine` trait defaults, which
//! discard the database argument and delegate to the `current_db` variants.
//! `d1` and `d2` here hold tables of *different shapes* (`d1`'s row has two
//! values, `d2`'s has three), so a scan that resolves the wrong database
//! cannot pass by coincidence.
//!
//! #5375: `current_db` / `set_current_db` were removed from the engine
//! (#5057 step 3). The database-less variants now resolve against
//! `DEFAULT_DATABASE`, so `default` is seeded with a `t` of its own —
//! distinguishable from both `d1` and `d2` — and the database-less
//! assertions below pin that.

use sqlrustgo_storage::engine::StorageEngine;
use sqlrustgo_storage::{FileStorage, Record, Value};

/// `t(id, v)` with `id` as the primary key.
fn table_info() -> sqlrustgo_storage::TableInfo {
    sqlrustgo_storage::TableInfo {
        name: "t".into(),
        columns: vec![column("id", true), column("v", false), column("w", false)],
        ..Default::default()
    }
}

fn column(name: &str, primary_key: bool) -> sqlrustgo_storage::ColumnDefinition {
    sqlrustgo_storage::ColumnDefinition {
        name: name.into(),
        data_type: "INTEGER".into(),
        nullable: !primary_key,
        primary_key,
        ..Default::default()
    }
}

fn row(id: i64, v: i64, w: i64) -> Record {
    vec![Value::Integer(id), Value::Integer(v), Value::Integer(w)]
}

/// A `FileStorage` holding `t` in `d1`, `d2` and `default`, each with a
/// row of its own.
fn seeded(dir: &std::path::Path) -> FileStorage {
    let mut s = FileStorage::new(dir.to_path_buf()).expect("open");
    s.create_database("d1").expect("create d1");
    s.create_database("d2").expect("create d2");

    s.create_table_in_db("d1", &table_info()).expect("t in d1");
    s.insert_in_db("d1", "t", vec![row(1, 10, 100)])
        .expect("insert into d1");

    s.create_table_in_db("d2", &table_info()).expect("t in d2");
    s.insert_in_db("d2", "t", vec![row(2, 20, 200)])
        .expect("insert into d2");

    s.create_table_in_db("default", &table_info())
        .expect("t in default");
    s.insert_in_db("default", "t", vec![row(3, 30, 300)])
        .expect("insert into default");

    s
}

/// The database named in the call wins over the current one.
#[test]
fn scan_with_filter_in_db_reads_the_named_database() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let s = seeded(tmp.path());

    let rows = s
        .scan_with_filter_in_db("d1", "t", &|_| true)
        .expect("scan d1");

    assert_eq!(
        rows,
        vec![row(1, 10, 100)],
        "d1's single row; d2's row must not appear"
    );
}

/// The filter is applied to the named database's rows, not the current one.
/// `v = 20` exists only in `d2`.
#[test]
fn scan_with_filter_in_db_filters_the_named_database() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let s = seeded(tmp.path());

    let rows = s
        .scan_with_filter_in_db("d1", "t", &|r| r[1] == Value::Integer(20))
        .expect("scan d1");

    assert!(
        rows.is_empty(),
        "no d1 row has v = 20; a hit means the scan read d2 — got {rows:?}"
    );
}

/// `parallel_scan_in_db` partitions the named database's rows. Partitioned
/// output arrives in unspecified order and count, so flatten before asserting.
#[test]
fn parallel_scan_in_db_partitions_the_named_database() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let s = seeded(tmp.path());

    let mut rows: Vec<Record> = s
        .parallel_scan_in_db("d1", "t", 4)
        .expect("scan d1")
        .into_iter()
        .flat_map(|p| p.collect::<Vec<Record>>())
        .collect();
    rows.sort_by_key(|r| format!("{r:?}"));

    assert_eq!(
        rows,
        vec![row(1, 10, 100)],
        "d1's single row; d2's row must not be mixed in"
    );
}

/// The two methods are not aliases for one another — one filters, one
/// partitions. Pointed at the same database they must return the same rows.
#[test]
fn the_two_forms_agree_within_one_database() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let s = seeded(tmp.path());

    let mut scanned = s
        .scan_with_filter_in_db("d1", "t", &|_| true)
        .expect("scan d1");
    let mut partitioned: Vec<Record> = s
        .parallel_scan_in_db("d1", "t", 4)
        .expect("scan d1")
        .into_iter()
        .flat_map(|p| p.collect::<Vec<Record>>())
        .collect();

    scanned.sort_by_key(|r| format!("{r:?}"));
    partitioned.sort_by_key(|r| format!("{r:?}"));
    assert_eq!(
        scanned, partitioned,
        "both forms must agree when scoped to the same database"
    );
}

/// A `filter` that rejects everything must yield nothing — pinning that the
/// filter is threaded through to the named database rather than ignored.
#[test]
fn scan_with_filter_in_db_honours_a_rejecting_filter() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let s = seeded(tmp.path());

    assert!(s
        .scan_with_filter_in_db("d1", "t", &|_| false)
        .expect("scan d1")
        .is_empty());
    assert!(s
        .scan_with_filter_in_db("d2", "t", &|_| false)
        .expect("scan d2")
        .is_empty());
}

/// The database-less variants still work — the fix moved their bodies into
/// `_in` variants and must not have changed what they return.
///
/// #5375: they resolve against `DEFAULT_DATABASE` now, so they answer
/// with `default`'s row — not `d1`'s and not `d2`'s.
#[test]
fn the_database_less_variants_resolve_against_the_default_database() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let s = seeded(tmp.path());

    assert_eq!(
        s.scan_with_filter("t", &|_| true).expect("scan"),
        vec![row(3, 30, 300)],
        "the no-db form resolves against `default`"
    );
    let mut partitioned: Vec<Record> = s
        .parallel_scan("t", 4)
        .expect("scan")
        .into_iter()
        .flat_map(|p| p.collect::<Vec<Record>>())
        .collect();
    partitioned.sort_by_key(|r| format!("{r:?}"));
    assert_eq!(partitioned, vec![row(3, 30, 300)]);
}
