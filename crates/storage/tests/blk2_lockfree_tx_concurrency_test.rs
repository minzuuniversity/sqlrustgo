// BLK-2 — the `*_transaction_lockfree(&self)` paths must not derive a
// `&mut S` from a shared reference.
//
// docs/releases/v4.1.0/ISSUES_PLAN.md §4.1 (BLK-2).
//
// `WalStorage` keeps its backend in an `UnsafeCell<S>` so the lockfree
// transaction methods can be declared `&self` and skip the global
// `Arc<RwLock<Storage>>` write lock. It previously did that via
// `as_inner_mut()`, which returned `&mut S` derived from `&self`. The
// documented invariant — "the engine serializes the calls via its own
// mutex" — does not hold in the server, where every connection has its
// own `ExecutionEngine` over one shared storage. Connection A's
// `storage.read()` + `&mut` therefore aliased connection B's
// `storage.write()` guard.
//
// The failure is not a subtle one: `sample` on the wedged server put
// all 2458 samples across all threads in
// `WalStorage<...>::rollback_transaction_lockfree` and
// `RawRwLock::lock_{shared,exclusive}_slow`. A single query then hangs
// forever.
//
// These tests drive many threads through transactions against one
// shared storage and assert they all complete. A hang fails the test by
// timeout rather than by assertion, which is the honest signature of
// this bug.

use parking_lot::RwLock;
use sqlrustgo_storage::engine::StorageEngine;
use sqlrustgo_storage::file_storage::FileStorage;
use sqlrustgo_storage::mvcc_storage::MvccStorage;
use sqlrustgo_storage::wal::file_backed_wal_manager::FileBackedWalManager;
use sqlrustgo_storage::wal_storage::WalStorage;
use std::sync::Arc;
use std::time::Duration;

type Inner = MvccStorage<FileStorage>;
type Storage = WalStorage<Inner, FileBackedWalManager>;

fn make_storage(dir: &std::path::Path) -> Arc<RwLock<Storage>> {
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir).unwrap();

    let file = FileStorage::new(dir.to_path_buf()).expect("FileStorage::new");
    let mvcc = MvccStorage::new(file);
    let wal_path = dir.join("test.wal");
    let wal = FileBackedWalManager::new(wal_path).expect("wal manager");
    let storage = WalStorage::new(mvcc, wal).expect("WalStorage::new");
    let storage = Arc::new(RwLock::new(storage));

    {
        let mut s = storage.write();
        s.create_table(&sqlrustgo_storage::engine::TableInfo {
            name: "t".to_string(),
            columns: vec![sqlrustgo_storage::engine::ColumnDefinition {
                name: "id".to_string(),
                data_type: "INTEGER".to_string(),
                primary_key: true,
                ..Default::default()
            }],
            ..Default::default()
        })
        .expect("create_table");

        // Seed a few rows so the undo log has something to roll back.
        for i in 0..5i64 {
            s.insert("t", vec![vec![sqlrustgo_types::Value::Integer(i)]])
                .expect("seed insert");
        }
    }
    storage
}

/// Run `threads` threads through BEGIN / INSERT / ROLLBACK against the
/// shared storage, holding the *read* guard while touching tx state —
/// exactly what `ExecutionEngine::rollback_transaction` does
/// (`execution_engine_methods.rs:1776`: `let storage_read =
/// self.storage.read();`).
fn run_concurrent_rollbacks(storage: Arc<RwLock<Storage>>, threads: usize, rounds: usize) {
    let mut handles = Vec::with_capacity(threads);
    for t in 0..threads {
        let storage = storage.clone();
        handles.push(std::thread::spawn(move || {
            for r in 0..rounds {
                let tx_id = (t * 1000 + r + 1) as u64;
                {
                    // Read guard + mutation — the aliasing pattern.
                    let guard = storage.read();
                    guard.begin_transaction_lockfree(tx_id).expect("begin");
                }
                {
                    let mut guard = storage.write();
                    guard
                        .insert(
                            "t",
                            vec![vec![sqlrustgo_types::Value::Integer(
                                (t * 100 + r) as i64 + 1000,
                            )]],
                        )
                        .expect("insert inside tx");
                }
                {
                    let guard = storage.read();
                    guard.rollback_transaction_lockfree().expect("rollback");
                }
                {
                    let guard = storage.read();
                    guard
                        .begin_transaction_lockfree(tx_id + 500_000)
                        .expect("begin 2");
                }
                {
                    let mut guard = storage.write();
                    guard
                        .insert(
                            "t",
                            vec![vec![sqlrustgo_types::Value::Integer(
                                (t * 100 + r) as i64 + 2000,
                            )]],
                        )
                        .expect("insert inside tx 2");
                }
                {
                    let guard = storage.read();
                    guard.commit_transaction_lockfree().expect("commit");
                }
            }
        }));
    }
    for h in handles {
        h.join().expect("worker must not panic");
    }
}

#[test]
fn concurrent_transactions_over_shared_storage_complete() {
    let storage = make_storage(std::path::Path::new("/tmp/sqlrustgo_blk2_concurrent_tx"));
    run_concurrent_rollbacks(storage, 8, 25);
}

#[test]
fn concurrent_transactions_interleaved_with_writers_complete() {
    // Mix tx-control traffic with plain writes, which take the write
    // guard. The deadlock needs both sides present: the writer holds
    // `write()` while a rollback path is mid-flight on `read()`.
    let storage = make_storage(std::path::Path::new("/tmp/sqlrustgo_blk2_mixed_rw"));
    let storage2 = storage.clone();
    let mut handles = Vec::new();

    for t in 0..6 {
        let s = storage.clone();
        handles.push(std::thread::spawn(move || {
            for r in 0..40 {
                let tx_id = (t * 100 + r + 1) as u64;
                {
                    let g = s.read();
                    g.begin_transaction_lockfree(tx_id).unwrap();
                }
                {
                    let mut g = s.write();
                    g.insert(
                        "t",
                        vec![vec![sqlrustgo_types::Value::Integer(
                            3000 + (t * 100 + r) as i64,
                        )]],
                    )
                    .unwrap();
                }
                {
                    let g = s.read();
                    g.rollback_transaction_lockfree().unwrap();
                }
            }
        }));
    }
    for t in 0..4 {
        let s = storage2.clone();
        handles.push(std::thread::spawn(move || {
            for r in 0..60 {
                let mut g = s.write();
                g.insert(
                    "t",
                    vec![vec![sqlrustgo_types::Value::Integer(
                        5000 + (t * 100 + r) as i64,
                    )]],
                )
                .unwrap();
            }
        }));
    }
    for h in handles {
        h.join().expect("worker must not panic");
    }
}

#[test]
fn lockfree_paths_do_not_require_exclusive_storage_access() {
    // The contract the fix restores: tx control must be able to run
    // while another thread holds the write guard for an ordinary write.
    // If `rollback_transaction_lockfree` needed `&mut S` it could not.
    let storage = make_storage(std::path::Path::new("/tmp/sqlrustgo_blk2_no_excl"));

    let s1 = storage.clone();
    let holder = std::thread::spawn(move || {
        let _guard = s1.write();
        // Hold the exclusive guard for a measurable window.
        std::thread::sleep(Duration::from_millis(200));
    });

    std::thread::sleep(Duration::from_millis(20));
    {
        let guard = storage.read();
        // parking_lot RwLock is not reentrant, so this read blocks until
        // the writer above releases. The point is that it *completes*.
        guard
            .begin_transaction_lockfree(42)
            .expect("begin while a writer holds the guard");
        guard
            .rollback_transaction_lockfree()
            .expect("rollback while a writer holds the guard");
    }

    holder.join().expect("holder must not panic");
}
