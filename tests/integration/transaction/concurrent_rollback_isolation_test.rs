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

use parking_lot::RwLock;
use sqlrustgo_storage::engine::{ColumnDefinition, TableInfo};
use sqlrustgo_storage::file_storage::FileStorage;
use sqlrustgo_storage::StorageEngine;
use sqlrustgo_types::Value;
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

/// A committed DELETE by one connection must not be undone by a peer's
/// ROLLBACK, and a rolled-back DELETE must be restored.
///
/// **The two transactions touch DIFFERENT rows on purpose.** An earlier
/// version of this test had both connections delete the same `id=42` and
/// asserted the row must survive. That oracle was wrong: when A commits a
/// delete of 42 and B rolls back a delete of the same 42, "row 42 present"
/// and "A's committed delete is permanent" are mutually exclusive. MySQL
/// resolves that with row locks — one transaction blocks — and does NOT
/// guarantee resurrection. Asserting resurrection therefore pinned a
/// behaviour the engine never promised, and the test could not distinguish
/// a real defect from correct behaviour.
///
/// With distinct keys the expected end state is unambiguous:
///   A: DELETE 42, COMMIT   -> 42 absent
///   B: DELETE 43, ROLLBACK -> 43 restored
///
/// Same-key concurrency is a different property (lock conflict / blocking /
/// first-committer-wins) and belongs in its own test, not here.
///
/// Baseline before the #5099 fix: 30/30 trials wrong, and BOTH halves fail
/// independently — a committed DELETE was undone 21/30 times, and a
/// rolled-back DELETE was not restored 9/30 times. Neither property holds
/// today; neither is an artifact of a wrong oracle.
///
/// Both handles share ONE `FileStorage`, exactly as the server passes
/// `storage.clone()` to every connection handler.
#[test]
fn committed_delete_is_not_undone_by_peer_rollback() {
    const TRIALS: usize = 30;
    let committed_target = 42i64;
    let rolled_back_target = 43i64;
    let mut wrong = 0usize;

    for _ in 0..TRIALS {
        // Fresh table per trial: one corrupted trial cannot mask another.
        let (_dir, storage) = seeded_storage(50);
        let barrier = Arc::new(Barrier::new(2));

        let a_storage = Arc::clone(&storage);
        let b_storage = Arc::clone(&storage);
        let a_barrier = Arc::clone(&barrier);
        let b_barrier = Arc::clone(&barrier);

        // Each connection names its own transaction. `begin_transaction()`
        // reads the single `current_tx_id` slot, so two concurrent BEGINs
        // collide: the second sees a non-zero slot and returns the first's
        // id, making both threads semantically the same transaction.
        let committer = thread::spawn(move || {
            a_storage.write().begin_transaction_for(1).ok();
            a_storage
                .write()
                .delete("t", &[Value::Integer(committed_target)])
                .ok();
            a_barrier.wait();
            thread::sleep(std::time::Duration::from_millis(20));
            a_storage.write().commit_transaction_for(1).ok();
        });
        let roller = thread::spawn(move || {
            b_storage.write().begin_transaction_for(2).ok();
            b_storage
                .write()
                .delete("t", &[Value::Integer(rolled_back_target)])
                .ok();
            b_barrier.wait();
            thread::sleep(std::time::Duration::from_millis(20));
            b_storage.write().rollback_transaction_for(2).ok();
        });

        committer.join().expect("committer thread");
        roller.join().expect("roller thread");

        // The committed delete stands...
        let committed_still_gone = count_id(&storage, committed_target) == 0;
        // ...and the rolled-back delete is undone.
        let rolled_back_restored = count_id(&storage, rolled_back_target) == 1;
        if !committed_still_gone || !rolled_back_restored {
            wrong += 1;
        }
    }

    assert_eq!(
        wrong, 0,
        "{wrong}/{TRIALS} trials had a wrong end state: a committed DELETE was undone \
         by a peer's ROLLBACK, or a rolled-back DELETE was not restored"
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
    // 8 threads x 100 iterations = 800 distinct keys, so every transaction
    // owns its row and no two race on one.
    const ROWS: i64 = (THREADS * ITERS) as i64;

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
                // Each transaction touches its OWN row. The previous
                // `(tid * 7 + n) % ROWS` allocation put 800 operations on
                // 200 rows, so concurrent transactions routinely deleted the
                // same key — and "row count is conserved" is not a property
                // that holds when two transactions race on one row: MySQL
                // resolves that with row locks (one blocks), it does not
                // guarantee neither's write is undone. The same-key case is
                // its own property, per the note on the test above.
                let id = (tid * ITERS + n) as i64 + 1;
                // A distinct tx id per transaction: every worker shares one
                // storage, and `begin_transaction()` would hand them all the
                // same id from the single `current_tx_id` slot.
                let tx = (tid * ITERS + n) as u64 + 1;
                s.write().begin_transaction_for(tx).ok();
                s.write().delete("t", &[Value::Integer(id)]).ok();
                let res = s
                    .write()
                    .insert("t", vec![vec![Value::Integer(id), Value::Integer(id)]]);
                match res {
                    Ok(_) => {
                        s.write().commit_transaction_for(tx).ok();
                    }
                    Err(e) => {
                        // Every insert follows its own delete of the same PK,
                        // so a duplicate means that delete was lost.
                        if format!("{e:?}").contains("1062") {
                            dup_errors += 1;
                        }
                        s.write().rollback_transaction_for(tx).ok();
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
        after,
        baseline,
        "row count drifted from {baseline} to {after} — concurrent transactions silently \
         lost {} rows while every rollback reported success",
        baseline - after
    );
}
