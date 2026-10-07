//! Regression: concurrent transactions must not corrupt each other's
//! rollback.
//!
//! # What was broken
//!
//! `FileStorage` keeps undo state in **instance-level** fields —
//! `tx_undo_log: Vec<UndoOp>`, `insert_buffer: HashMap<String, Vec<Record>>`
//! and `current_tx_id: AtomicU64` (see `crates/storage/src/file_storage.rs`).
//! The MySQL server hands every connection an `Arc` clone of **one shared**
//! `FileStorage` (`ExecutionEngine::new(storage.clone())`), so N concurrent
//! connections stamp and clear each other's transaction state.
//!
//! Observable consequence, measured before the fix (5 reps x 1600 txns):
//! 441 of 1000 rows (55%) silently disappeared with **zero** errors
//! reported by server or client — every `ROLLBACK` returned success. Two
//! connections that `DELETE` the same row behaved as if only one delete
//! ever happened:
//!
//!     conn A: BEGIN; DELETE FROM t WHERE id=42;  COMMIT
//!     conn B: BEGIN; DELETE FROM t WHERE id=42;  ROLLBACK   <-- row must survive
//!
//! MySQL semantics: B's rollback must restore the row. Before the fix it
//! did not, in 9/30 trials, silently at that.
//!
//! # What this test pins
//!
//! 1. `committed_delete_survives_peer_rollback` — the headline case.
//! 2. `concurrent_delete_insert_txns_lose_no_rows` — the SOAK shape
//!    (`DELETE id=N; INSERT id=N` in one txn, what sysbench's
//!    `oltp_read_write` / `execute_delete_inserts` issues).
//!
//! Both assert *data* outcomes (row presence / row count), not merely
//! that a call returned `Ok` — a call returning `Ok` is exactly what
//! stayed green while rows vanished.

use sqlrustgo_storage::engine::{ColumnDefinition, TableInfo};
use sqlrustgo_storage::file_storage::FileStorage;
use sqlrustgo_storage::StorageEngine;
use sqlrustgo_types::Value;
use parking_lot::RwLock;
use std::sync::{Arc, Barrier};
use std::thread;
use tempfile::TempDir;

/// A `t(id PK, k INT)` table seeded with `1..=rows`, wrapped in the same
/// `Arc<RwLock<_>>` the MySQL server hands to every connection handler
/// (`do_command_loop` takes `Arc<RwLock<BoxStorageEngine>>`).
fn seeded_storage(rows: i64) -> (TempDir, Arc<RwLock<FileStorage>>) {
    let dir = TempDir::new().expect("temp dir");
    let mut storage = FileStorage::new(dir.path().to_path_buf()).expect("open storage");
    let info = TableInfo {
        name: "t".to_string(),
        columns: vec![
            ColumnDefinition {
                name: "id".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: true,
                ..Default::default()
            },
            ColumnDefinition {
                name: "k".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: false,
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    storage.create_table(&info).expect("create table t");
    let batch: Vec<Vec<Value>> = (1..=rows)
        .map(|i| vec![Value::Integer(i), Value::Integer(0)])
        .collect();
    storage.insert("t", batch).expect("seed rows");
    (dir, Arc::new(RwLock::new(storage)))
}

fn count(storage: &Arc<RwLock<FileStorage>>) -> i64 {
    storage.read().scan("t").expect("scan t").len() as i64
}

fn count_id(storage: &Arc<RwLock<FileStorage>>, target: i64) -> i64 {
    storage
        .read()
        .scan("t")
        .expect("scan t")
        .iter()
        .filter(|r| r.first() == Some(&Value::Integer(target)))
        .count() as i64
}

/// The headline regression: two connections delete the SAME row; one
/// commits, the other rolls back. The row must survive — a rolled-back
/// DELETE must never erase a concurrently committed one.
///
/// Both handles share ONE `FileStorage`, exactly as the server passes
/// `storage.clone()` to every connection handler.
#[test]
fn committed_delete_survives_peer_rollback() {
    const TRIALS: usize = 30;
    let target = 42i64;
    let mut lost = 0usize;

    for _ in 0..TRIALS {
        // Fresh table per trial: one corrupted trial must not mask another.
        let (_dir, storage) = seeded_storage(50);
        let barrier = Arc::new(Barrier::new(2));

        let a_storage = Arc::clone(&storage);
        let b_storage = Arc::clone(&storage);
        let a_barrier = Arc::clone(&barrier);
        let b_barrier = Arc::clone(&barrier);

        let committer = thread::spawn(move || {
            a_storage.write().begin_transaction().ok();
            a_storage
                .write()
                .delete("t", &[Value::Integer(target)])
                .ok();
            a_barrier.wait();
            thread::sleep(std::time::Duration::from_millis(20));
            a_storage.write().commit_transaction().ok();
        });
        let roller = thread::spawn(move || {
            b_storage.write().begin_transaction().ok();
            b_storage
                .write()
                .delete("t", &[Value::Integer(target)])
                .ok();
            b_barrier.wait();
            thread::sleep(std::time::Duration::from_millis(20));
            b_storage.write().rollback_transaction().ok();
        });

        committer.join().expect("committer thread");
        roller.join().expect("roller thread");

        if count_id(&storage, target) == 0 {
            lost += 1;
        }
    }

    assert_eq!(
        lost, 0,
        "rolled-back DELETE erased a concurrently committed one in {lost}/{TRIALS} trials \
         — a rollback must never remove a row a peer transaction committed"
    );
}

/// The SOAK shape: `BEGIN; DELETE id=N; INSERT id=N;` run concurrently.
///
/// Two properties regressed and both are asserted:
///   * the row count is conserved (no silent data loss)
///   * no spurious `1062` escapes to the client (this aborted the SOAK)
#[test]
fn concurrent_delete_insert_txns_lose_no_rows() {
    const THREADS: usize = 8;
    const ITERS: usize = 100;
    const ROWS: i64 = 200;

    let (_dir, storage) = seeded_storage(ROWS);
    let baseline = count(&storage);
    assert_eq!(baseline, ROWS, "fixture must start with {ROWS} rows");

    let shared = Arc::new(storage);
    let mut handles = Vec::new();
    for tid in 0..THREADS {
        let s = Arc::clone(&shared);
        handles.push(thread::spawn(move || {
            let mut dup_errors = 0usize;
            for n in 0..ITERS {
                let id = ((tid * 7 + n) as i64) % ROWS + 1;
                s.write().begin_transaction().ok();
                s.write().delete("t", &[Value::Integer(id)]).ok();
                let res = s
                    .write()
                    .insert("t", vec![vec![Value::Integer(id), Value::Integer(id)]]);
                match res {
                    Ok(_) => {
                        s.write().commit_transaction().ok();
                    }
                    Err(e) => {
                        // Every insert follows its own delete of the same PK,
                        // so a duplicate means that delete was lost.
                        if format!("{e:?}").contains("1062") {
                            dup_errors += 1;
                        }
                        s.write().rollback_transaction().ok();
                    }
                }
            }
            dup_errors
        }));
    }

    let mut total_dup = 0usize;
    for h in handles {
        total_dup += h.join().expect("worker thread");
    }

    let after = count(&shared);
    assert_eq!(
        total_dup, 0,
        "{total_dup} transactions raised ERROR 1062 (Duplicate entry) for a row they \
         had just deleted in the same transaction"
    );
    assert_eq!(
        after, baseline,
        "row count drifted from {baseline} to {after} — concurrent transactions silently \
         lost {} rows while every rollback reported success",
        baseline - after
    );
}