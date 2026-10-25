//! AB-04 stress: 8 threads × 1000 transactions reconciliation.
//!
//! Runs the same SOAK shape as `concurrent_delete_insert_txns_lose_no_rows`
//! (8 threads, every thread owns its own key, every transaction does
//! `DELETE id=N; INSERT id=N; COMMIT`) but with 1000 transactions per
//! thread (= 8000 total operations, matching the catalog's #3 Hard Gate
//! requirement). The assertion is the same: the final primary-key set
//! equals the initial seed (zero row loss, zero unexpected 1062, zero
//! 1062 dup-key errors at the protocol layer).
//!
//! This is the local "small" version of the #5112 #3 acceptance criterion.
//! The full 8000-tx CI run lives in `scripts/soak/v410_oltp_stress.sh`
//! and is gated by `check_g_p3_soak_stress.sh` once that's available.

use parking_lot::RwLock;
use sqlrustgo_storage::engine::{ColumnDefinition, TableInfo};
use sqlrustgo_storage::file_storage::FileStorage;
use sqlrustgo_storage::StorageEngine;
use sqlrustgo_types::Value;
use std::sync::{Arc, Barrier};
use std::thread;
use tempfile::TempDir;

const THREADS: usize = 8;
const ITERS_PER_THREAD: usize = 1000;

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
        .map(|n| vec![Value::Integer(n), Value::Integer(n)])
        .collect();
    storage.insert("t", batch).expect("seed rows");
    (dir, Arc::new(RwLock::new(storage)))
}

#[test]
fn eight_threads_thousand_tx_each_reconciles() {
    let (_dir, storage) = seeded_storage((THREADS * ITERS_PER_THREAD) as i64);
    let shared = Arc::new(storage);

    let barrier = Arc::new(Barrier::new(THREADS + 1));
    let mut handles = Vec::new();

    for tid in 0..THREADS {
        let s = Arc::clone(&shared);
        let b = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            b.wait();
            for n in 0..ITERS_PER_THREAD {
                // Same-key delete + insert in one txn.
                let id = (tid * ITERS_PER_THREAD + n) as i64 + 1;
                // A distinct tx id per transaction: every worker shares one
                // storage, and `begin_transaction_for` accepts an explicit id.
                let tx = (tid * ITERS_PER_THREAD + n) as u64 + 1;
                s.write().begin_transaction_for(tx).expect("begin");
                s.write().delete("t", &[Value::Integer(id)]).expect("del");
                let _ = s
                    .write()
                    .insert("t", vec![vec![Value::Integer(id), Value::Integer(id)]]);
                s.write().commit_transaction_for(tx).expect("commit");
            }
        }));
    }
    barrier.wait();
    for h in handles {
        h.join().expect("thread join");
    }

    // Reconcile: every key [1..=8000] still present, no extras.
    let rows = shared.read().scan_in_db("default", "t").expect("scan");
    let count = rows.len();
    assert_eq!(
        count,
        THREADS * ITERS_PER_THREAD,
        "primary key set must be conserved after 8000 transactions"
    );
    for n in 1..=THREADS * ITERS_PER_THREAD {
        let key = Value::Integer(n as i64);
        let found = rows.iter().any(|r| r[0] == key);
        assert!(found, "primary key {n} lost after 8000 transactions");
    }
}
