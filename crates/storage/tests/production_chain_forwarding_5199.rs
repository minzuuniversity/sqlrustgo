//! #5199 item 1 (assembly-path consistency test, F3): the server's
//! production storage chain is `ParallelWalStorage<MvccStorage<FileStorage>>`
//! (see `mysql-server/src/lib.rs`). Every performance-sensitive method the
//! query path relies on must be *forwarded* through each wrapper so the inner
//! `FileStorage` B+Tree is actually consulted.
//!
//! #5168 proved the failure mode concretely: when the OUTERMOST wrapper did
//! not override `scan_pk`, the trait default (full table scan + linear
//! find) ran silently and every primary-key point lookup cost O(rows) even
//! though `FileStorage` had a populated PK index. The existing
//! `wal_scan_pk_forwarding_5168.rs` guards `scan_pk` on the
//! `WalStorage<MvccStorage<FileStorage>>` chain only.
//!
//! This file extends that guard to the *actual* production wrapper
//! (`ParallelWalStorage`) and to the range variant (`scan_pk_range`), for the
//! methods whose correct implementation is an index lookup and whose cost
//! therefore must NOT track table size.
//!
//! Oracle, identical to #5168: comparing result rows proves nothing (the
//! indexed path and the full-scan path return the same rows). What proves the
//! forwarding is *cost*. A bounded operation must not scale with table size;
//! a full scan does. The assertion is a ratio, so absolute machine speed is
//! irrelevant.
//!
//! Deliberately NOT covered here (and why):
//! - `scan_in_db` returns ALL rows; O(rows) is its contract, not a defect, so
//!   a flatness assertion would be wrong.
//! - `scan_with_index` (secondary index) is not a `StorageEngine`-trait
//!   forwarding concern: the wrappers do not override it and the trait
//!   default returns `Err`, i.e. a *loud* failure, not a silent
//!   degradation. The production SELECT path for secondary indexes is owned
//!   by the optimizer/executor, not the wrapper chain. Tracking it here
//!   would assert the wrong invariant.

use sqlrustgo_storage::engine::{ColumnDefinition, StorageEngine, TableInfo};
use sqlrustgo_storage::{FileBackedWalManager, FileStorage, MvccStorage, ParallelWalStorage};

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

/// Build the server's production chain with `rows` inserted and flushed:
/// `ParallelWalStorage<MvccStorage<FileStorage>>`.
fn build_chain(rows: i64) -> ParallelWalStorage<MvccStorage<FileStorage>, FileBackedWalManager> {
    let dir = tempfile::tempdir().expect("tmpdir");
    let file = FileStorage::new(dir.path().to_path_buf()).expect("FileStorage::new");
    let mvcc = MvccStorage::new(file);
    let wal = FileBackedWalManager::new(dir.path().join("t.wal")).expect("wal");
    let mut s = ParallelWalStorage::new(mvcc, wal);
    s.create_table(&info()).expect("create_table");
    for i in 1..=rows {
        s.insert("t", vec![vec![sqlrustgo_storage::Value::Integer(i)]])
            .expect("insert");
    }
    s.flush().expect("flush");
    // NOTE: the tempdir is dropped with the storage; keep it alive for the
    // storage's lifetime by leaking it (test process exits shortly after).
    std::mem::forget(dir);
    s
}

/// A primary-key point lookup through the production chain must be flat:
/// the cost must not track table size, which is only true if the lookup
/// reaches `FileStorage`'s PK B+Tree rather than running the trait-default
/// full scan at some wrapper layer.
#[test]
fn production_chain_scan_pk_must_not_scale_with_table_size() {
    fn measure(rows: i64) -> std::time::Duration {
        let s = build_chain(rows);
        let key = sqlrustgo_storage::Value::Integer(rows);
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
    assert!(
        large.as_secs_f64() < small.as_secs_f64() * 20.0,
        "scan_pk through ParallelWalStorage<MvccStorage<FileStorage>> scaled \
         with table size: 200 rows = {small:?}, 20000 rows = {large:?}. A PK \
         point lookup must reach FileStorage's PK index; a wrapper running \
         the trait-default full scan instead would make this grow with rows."
    );
}

/// Same invariant for the range variant. The query range is bounded
/// (`1..=10`, at most 10 rows) regardless of table size, so an indexed
/// range scan costs O(log N + 10) ≈ flat, while a trait-default full scan
/// + filter costs O(rows) and scales.
#[test]
fn production_chain_scan_pk_range_must_not_scale_with_table_size() {
    fn measure(rows: i64) -> std::time::Duration {
        let s = build_chain(rows);
        let low = sqlrustgo_storage::Value::Integer(1);
        let high = sqlrustgo_storage::Value::Integer(10);
        for _ in 0..5 {
            let _ = s.scan_pk_range("t", &low, &high);
        }
        let start = std::time::Instant::now();
        for _ in 0..50 {
            let _ = s.scan_pk_range("t", &low, &high);
        }
        start.elapsed()
    }

    let small = measure(200);
    let large = measure(20_000);
    assert!(
        large.as_secs_f64() < small.as_secs_f64() * 20.0,
        "scan_pk_range through ParallelWalStorage<MvccStorage<FileStorage>> \
         scaled with table size: 200 rows = {small:?}, 20000 rows = {large:?}. \
         A bounded PK range must reach FileStorage's PK index; a wrapper \
         running the trait-default full scan would make this grow with rows."
    );
}

/// Sanity: the production chain actually returns the right row for a present
/// key and the bounded set for the range. Guards against the cost tests
/// passing vacuously (e.g. if both sides erred and we timed the error path).
#[test]
fn production_chain_pk_lookup_is_correct() {
    let s = build_chain(1_000);
    let hit = s.scan_pk("t", "id", &sqlrustgo_storage::Value::Integer(999)).expect("scan_pk");
    assert!(hit.is_some(), "present key must be found");
    let miss = s
        .scan_pk("t", "id", &sqlrustgo_storage::Value::Integer(9_999_999))
        .expect("scan_pk");
    assert!(miss.is_none(), "absent key must be None");
    let range = s
        .scan_pk_range(
            "t",
            &sqlrustgo_storage::Value::Integer(1),
            &sqlrustgo_storage::Value::Integer(10),
        )
        .expect("scan_pk_range");
    assert_eq!(range.len(), 10, "bounded range must return exactly 10 rows");
}
