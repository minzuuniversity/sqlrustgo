//! #5168: `WalStorage` / `ParallelWalStorage` never forward `scan_pk`, so the
//! server — whose storage is `ParallelWalStorage -> MvccStorage ->
//! FileStorage` — silently runs the trait default (full table scan) for
//! every primary-key point lookup.
//!
//! The oracle must distinguish "indexed" from "scanned". Both return the
//! same row, so comparing results proves nothing. Two things do:
//!
//! 1. A key that does NOT exist must return `None` either way — useless.
//! 2. But `scan_pk` on a wrapper can be distinguished from the default by
//!    what the inner engine is asked to do. Instead of measuring latency
//!    (flaky), assert the forwarding contract directly: the wrapper must
//!    reach the inner engine's indexed path.

use sqlrustgo_storage::engine::{ColumnDefinition, StorageEngine, TableInfo};
use sqlrustgo_storage::file_storage::FileStorage;
use sqlrustgo_storage::Value;

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

/// Baseline: the inner `FileStorage` index really is populated. If this
/// fails, the wrappers are not the problem — `FileStorage` is.
#[test]
fn file_storage_pk_index_is_populated() {
    let dir = tempfile::tempdir().expect("tmpdir");
    let mut s = FileStorage::new(dir.path().to_path_buf()).expect("open");
    s.create_table(&info()).expect("create");
    for i in 1..=100i64 {
        s.insert("t", vec![vec![Value::Integer(i)]])
            .expect("insert");
    }
    s.flush_all_buffers().expect("flush");

    assert!(
        s.scan_with_index("t", "id", &Value::Integer(50))
            .expect("index lookup")
            .len()
            == 1,
        "FileStorage's PK B+Tree must contain inserted rows; if not, the \
         bug is in rebuild_pk_indexes, not in wrapper forwarding"
    );
}

/// A missing key must be cheap on an indexed lookup. On the default
/// full-scan implementation it costs the same as a present key, because the
/// scan runs to completion either way. Comparing the two therefore measures
/// "was the index consulted", which is exactly the property at issue —
/// and it is a ratio, so absolute machine speed does not matter.
#[test]
fn pk_lookup_cost_must_not_scale_with_table_size() {
    fn measure(rows: i64) -> std::time::Duration {
        let dir = tempfile::tempdir().expect("tmpdir");
        let mut s = FileStorage::new(dir.path().to_path_buf()).expect("open");
        s.create_table(&info()).expect("create");
        for i in 1..=rows {
            s.insert("t", vec![vec![Value::Integer(i)]])
                .expect("insert");
        }
        s.flush_all_buffers().expect("flush");

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

    let small = measure(200);
    let large = measure(20_000);

    // An indexed lookup is O(log N): 100x the rows must not cost anywhere
    // near 100x the time. Allow generous headroom for noise while still
    // failing a linear scan (which would be ~100x).
    assert!(
        large.as_secs_f64() < small.as_secs_f64() * 20.0,
        "scan_pk cost scaled with table size: 200 rows = {small:?}, \
         20000 rows = {large:?}. A full scan is the cause."
    );
}

/// The server's actual storage chain is
/// `WalStorage<MvccStorage<FileStorage>, _>`. If the wrappers forwarded
/// `scan_pk`, this must behave like the bare `FileStorage` case above.
/// It does not, because neither wrapper overrides `scan_pk`.
#[test]
fn wal_wrapped_pk_lookup_must_not_scan() {
    use sqlrustgo_storage::engine::TableInfo;
    use sqlrustgo_storage::{FileBackedWalManager, FileStorage, MvccStorage, WalStorage};

    fn measure(rows: i64) -> std::time::Duration {
        let dir = tempfile::tempdir().expect("tmpdir");
        let file = FileStorage::new(dir.path().to_path_buf()).expect("FileStorage::new");
        let mvcc = MvccStorage::new(file);
        let wal = FileBackedWalManager::new(dir.path().join("t.wal")).expect("wal");
        let mut s = WalStorage::new(mvcc, wal).expect("WalStorage::new");

        s.create_table(&TableInfo {
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
        })
        .expect("create");

        for i in 1..=rows {
            s.insert("t", vec![vec![Value::Integer(i)]])
                .expect("insert");
        }
        s.flush().expect("flush");

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

    let small = measure(200);
    let large = measure(20_000);

    // This is the failing case: WalStorage does not forward scan_pk, so the
    // trait default full scan runs and cost tracks table size.
    assert!(
        large.as_secs_f64() < small.as_secs_f64() * 20.0,
        "scan_pk through WalStorage scaled with table size: 200 rows = \
         {small:?}, 20000 rows = {large:?}. WalStorage does not override \
         scan_pk, so the trait default full scan runs even though the inner \
         FileStorage has a populated PK index."
    );
}
