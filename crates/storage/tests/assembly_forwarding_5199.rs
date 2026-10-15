//! #5199 item 1: assembly-path consistency gate.
//!
//! Asserts every layer of the production storage chain forwards the
//! performance-sensitive / database-scoped read methods. Production
//! assemblies (from `crates/mysql-server/src/main.rs` + `lib.rs`):
//!
//! * `--storage file` (default): `WalStorage<MvccStorage<FileStorage>,
//!   FileBackedWalManager>`
//! * `--storage parallel`: `ParallelWalStorage<MvccStorage<FileStorage>,
//!   FileBackedWalManager>`
//!
//! Oracle styles (per #5199 §4 — ratios, not absolute ms):
//! * **Ratio** (`scan_pk`, `scan_pk_range`): cost must not scale with table
//!   size. A full scan (~100x) fails; an index hit passes.
//! * **Correctness** (`scan_in_db`, `scan_with_index_in_db`): the trait
//!   default drops `db` (`let _ = db; self.scan(table)`), silently
//!   misrouting multi-DB statements (#5025 / #5057 / #5192).

use sqlrustgo_storage::engine::{ColumnDefinition, StorageEngine, TableInfo};
use sqlrustgo_storage::{
    FileBackedWalManager, FileStorage, MvccStorage, ParallelWalStorage, Value, WalStorage,
};

fn info() -> TableInfo {
    TableInfo {
        name: "t".to_string(),
        columns: vec![ColumnDefinition {
            name: "id".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: false,
            primary_key: true,
            char_max_length: None,
            collation: None,
            default_value: None,
            auto_increment: false,
        }],
        foreign_keys: vec![],
        unique_constraints: vec![],
        ..Default::default()
    }
}

fn wal_chain(dir: &std::path::Path) -> WalStorage<MvccStorage<FileStorage>, FileBackedWalManager> {
    let file = FileStorage::new_with_wal(dir.to_path_buf()).expect("FileStorage::new_with_wal");
    let mvcc = MvccStorage::new(file);
    let wal = FileBackedWalManager::new(dir.join("t.wal")).expect("wal");
    WalStorage::new(mvcc, wal).expect("WalStorage::new")
}

fn parallel_chain(
    dir: &std::path::Path,
) -> ParallelWalStorage<MvccStorage<FileStorage>, FileBackedWalManager> {
    let file = FileStorage::new_with_wal(dir.to_path_buf()).expect("FileStorage::new_with_wal");
    let mvcc = MvccStorage::new(file);
    let wal = FileBackedWalManager::new(dir.join("t.wal")).expect("wal");
    ParallelWalStorage::new(mvcc, wal)
}

fn measure_scan_pk<S: StorageEngine>(s: &S, rows: i64) -> std::time::Duration {
    let key = Value::Integer(rows);
    for _ in 0..5 {
        let _ = s.scan_pk("t", "id", &key);
    }
    let start = std::time::Instant::now();
    for _ in 0..50 {
        let _ = s.scan_pk("t", "id", &key);
    }
    start.elapsed()
}

fn measure_scan_pk_range<S: StorageEngine>(s: &S, rows: i64) -> std::time::Duration {
    let low = Value::Integer(rows);
    let high = Value::Integer(rows);
    for _ in 0..5 {
        let _ = s.scan_pk_range("t", &low, &high);
    }
    let start = std::time::Instant::now();
    for _ in 0..50 {
        let _ = s.scan_pk_range("t", &low, &high);
    }
    start.elapsed()
}

fn seed<S: StorageEngine>(s: &mut S, rows: i64) {
    s.create_table(&info()).expect("create");
    for i in 1..=rows {
        s.insert("t", vec![vec![Value::Integer(i)]])
            .expect("insert");
    }
    s.flush().expect("flush");
}

#[test]
fn wal_chain_scan_pk_is_flat_in_table_size() {
    fn measure(rows: i64) -> std::time::Duration {
        let dir = tempfile::tempdir().expect("tmpdir");
        let mut s = wal_chain(dir.path());
        seed(&mut s, rows);
        measure_scan_pk(&s, rows)
    }
    let small = measure(200);
    let large = measure(20_000);
    assert!(
        large.as_secs_f64() < small.as_secs_f64() * 20.0,
        "scan_pk through WalStorage<MvccStorage<FileStorage>> scaled with \
         table size: 200 rows = {small:?}, 20000 rows = {large:?}. A wrapper \
         is dropping the forwarding and the trait default full scan runs."
    );
}

#[test]
fn wal_chain_scan_pk_range_is_flat_in_table_size() {
    fn measure(rows: i64) -> std::time::Duration {
        let dir = tempfile::tempdir().expect("tmpdir");
        let mut s = wal_chain(dir.path());
        seed(&mut s, rows);
        measure_scan_pk_range(&s, rows)
    }
    let small = measure(200);
    let large = measure(20_000);
    assert!(
        large.as_secs_f64() < small.as_secs_f64() * 20.0,
        "scan_pk_range through WalStorage<MvccStorage<FileStorage>> scaled \
         with table size: 200 rows = {small:?}, 20000 rows = {large:?}. The \
         trait default is a full scan + filter."
    );
}

#[test]
fn parallel_chain_scan_pk_is_flat_in_table_size() {
    fn measure(rows: i64) -> std::time::Duration {
        let dir = tempfile::tempdir().expect("tmpdir");
        let mut s = parallel_chain(dir.path());
        seed(&mut s, rows);
        measure_scan_pk(&s, rows)
    }
    let small = measure(200);
    let large = measure(20_000);
    assert!(
        large.as_secs_f64() < small.as_secs_f64() * 20.0,
        "scan_pk through ParallelWalStorage<MvccStorage<FileStorage>> scaled \
         with table size: 200 rows = {small:?}, 20000 rows = {large:?}. \
         ParallelWalStorage must forward scan_pk to the inner B+Tree."
    );
}

#[test]
fn parallel_chain_scan_pk_range_is_flat_in_table_size() {
    fn measure(rows: i64) -> std::time::Duration {
        let dir = tempfile::tempdir().expect("tmpdir");
        let mut s = parallel_chain(dir.path());
        seed(&mut s, rows);
        measure_scan_pk_range(&s, rows)
    }
    let small = measure(200);
    let large = measure(20_000);
    assert!(
        large.as_secs_f64() < small.as_secs_f64() * 20.0,
        "scan_pk_range through ParallelWalStorage<MvccStorage<FileStorage>> \
         scaled with table size: 200 rows = {small:?}, 20000 rows = {large:?}"
    );
}

#[test]
fn wal_chain_scan_in_db_forwards_db() {
    let dir = tempfile::tempdir().expect("tmpdir");
    let mut s = wal_chain(dir.path());
    s.create_database("aud").expect("create_database");
    s.create_table_in_db(
        "aud",
        &TableInfo {
            name: "t".to_string(),
            ..info()
        },
    )
    .expect("create_table_in_db");
    s.insert_in_db("aud", "t", vec![vec![Value::Integer(1)]])
        .expect("insert_in_db");
    s.create_table(&info()).expect("create default t");
    s.insert("t", vec![vec![Value::Integer(999)]])
        .expect("insert default");
    s.flush().expect("flush");

    let rows = s.scan_in_db("aud", "t").expect("scan_in_db");
    assert_eq!(
        rows.len(),
        1,
        "scan_in_db(\"aud\",\"t\") returned {} rows; expected 1. If you see \
         2, the wrapper dropped `db` and merged both databases.",
        rows.len()
    );
    assert_eq!(
        rows[0][0],
        Value::Integer(1),
        "scan_in_db returned the wrong row — db was dropped"
    );
}

#[test]
fn parallel_chain_scan_in_db_forwards_db() {
    let dir = tempfile::tempdir().expect("tmpdir");
    let mut s = parallel_chain(dir.path());
    s.create_database("aud").expect("create_database");
    s.create_table_in_db(
        "aud",
        &TableInfo {
            name: "t".to_string(),
            ..info()
        },
    )
    .expect("create_table_in_db");
    s.insert_in_db("aud", "t", vec![vec![Value::Integer(1)]])
        .expect("insert_in_db");
    s.create_table(&info()).expect("create default t");
    s.insert("t", vec![vec![Value::Integer(999)]])
        .expect("insert default");
    s.flush().expect("flush");

    let rows = s.scan_in_db("aud", "t").expect("scan_in_db");
    assert_eq!(
        rows.len(),
        1,
        "scan_in_db(\"aud\",\"t\") through ParallelWalStorage returned {} \
         rows; expected 1. ParallelWalStorage does not override scan_in_db, \
         so the trait default drops `db` and scans the current database.",
        rows.len()
    );
}

#[test]
fn wal_chain_scan_with_index_in_db_forwards_db() {
    let dir = tempfile::tempdir().expect("tmpdir");
    let mut s = wal_chain(dir.path());
    s.create_database("aud").expect("create_database");
    s.create_table_in_db(
        "aud",
        &TableInfo {
            name: "t".to_string(),
            ..info()
        },
    )
    .expect("create_table_in_db");
    for i in 1..=10i64 {
        s.insert_in_db("aud", "t", vec![vec![Value::Integer(i)]])
            .expect("insert_in_db");
    }
    s.flush().expect("flush");

    let rows = s
        .scan_with_index_in_db("aud", "t", "id", &Value::Integer(5))
        .expect("scan_with_index_in_db");
    assert_eq!(
        rows.len(),
        1,
        "scan_with_index_in_db(\"aud\",\"t\",\"id\",5) returned {} rows; \
         expected exactly 1 via the PK B+Tree.",
        rows.len()
    );
    assert_eq!(rows[0][0], Value::Integer(5));
}

#[test]
fn parallel_chain_scan_with_index_in_db_forwards_db() {
    let dir = tempfile::tempdir().expect("tmpdir");
    let mut s = parallel_chain(dir.path());
    s.create_database("aud").expect("create_database");
    s.create_table_in_db(
        "aud",
        &TableInfo {
            name: "t".to_string(),
            ..info()
        },
    )
    .expect("create_table_in_db");
    for i in 1..=10i64 {
        s.insert_in_db("aud", "t", vec![vec![Value::Integer(i)]])
            .expect("insert_in_db");
    }
    s.flush().expect("flush");

    let rows = s
        .scan_with_index_in_db("aud", "t", "id", &Value::Integer(5))
        .expect("scan_with_index_in_db");
    assert_eq!(
        rows.len(),
        1,
        "scan_with_index_in_db through ParallelWalStorage returned {} rows; \
         expected 1. The wrapper does not override scan_with_index_in_db, \
         so the trait default drops `db`.",
        rows.len()
    );
}

#[test]
fn wal_chain_pk_index_is_populated() {
    let dir = tempfile::tempdir().expect("tmpdir");
    let mut s = wal_chain(dir.path());
    seed(&mut s, 100);

    let hit = s
        .scan_with_index_in_db("default", "t", "id", &Value::Integer(50))
        .expect("index lookup");
    assert_eq!(
        hit.len(),
        1,
        "PK B+Tree on the production chain must contain inserted rows"
    );
}
