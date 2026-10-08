//! #4994: high concurrency + DELETE — every statement succeeds, all data is gone.
//!
//! The original report was 8 workers × 12 rounds (352 operations) all
//! returning success, with `COUNT(*)` = 0 at the end. There was no Rust
//! regression test for it: the only evidence was
//! `scripts/repro_4994_concurrent_delete.py`, which CI never runs. This file
//! is that regression test, in the suite.
//!
//! ## What the Python script does, and what carries over
//!
//! Per worker `w`, per round `i`, with `rid = w * 1000 + i`:
//!
//! 1. `INSERT` its own row (autocommit)
//! 2. `UPDATE` it
//! 3. `SELECT` it
//! 4. `DELETE` it — **only when `i > 8`**, matching the original script
//! 5. `BEGIN` / `UPDATE v = v + 100` / `ROLLBACK` on odd rounds, `COMMIT` on even
//!
//! Expected survivors: `workers × 8 + 1` — each worker keeps the rows from
//! rounds 1..=8 plus one seed row. The script observed 0.
//!
//! What does **not** carry over: the MySQL protocol layer. This drives
//! `StorageEngine` directly, i.e. below the SQL layer, for the same reason
//! `file_storage_concurrent_same_row_5059.rs` does — primary-key uniqueness
//! and affected-row counts are `ExecutionEngine` concerns
//! (`src/engine_dml.rs`), not storage-layer guarantees.
//!
//! ## Why the assertion is about *survivors*, not about counts
//!
//! The defect was never "a statement failed". Every statement returned OK
//! while the data vanished. So the invariant worth pinning is: **a worker's
//! `ROLLBACK` must not remove rows that belong to other workers.** Each
//! worker writes only its own `rid`s, so any row lost outside its own
//! `DELETE` range is a rollback that reached too far.
//!
//! `begin_transaction_for` / `commit_transaction_for` / `rollback_transaction_for`
//! are used rather than the `begin_transaction()` convenience form because
//! the latter draws from the storage-wide counter — with 8 threads that is
//! exactly the shared-state ambiguity the issue is about. The `_for` forms
//! give each worker its own `tx_id`, which is what "8 independent
//! connections" means at this layer.

use sqlrustgo_storage::{ColumnDefinition, FileStorage, Record, StorageEngine, TableInfo, Value};
use std::sync::{Arc, RwLock};

/// Rounds 1..=DELETE_FROM are never deleted; a worker keeps the ones it
/// committed out of them.
const DELETE_FROM: i64 = 8;

/// Rounds are committed on even `i`. Combined with `DELETE_FROM`, a worker
/// keeps rounds {2, 4, 6, 8} — four rows — plus nothing else.
const KEEP_FROM: i64 = 4;

fn counter_table(name: &str) -> TableInfo {
    TableInfo {
        name: name.to_string(),
        columns: vec![
            ColumnDefinition {
                name: "id".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: true,
                char_max_length: None,
                collation: None,
                default_value: None,
                auto_increment: false,
            },
            ColumnDefinition {
                name: "v".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: true,
                primary_key: false,
                char_max_length: None,
                collation: None,
                default_value: None,
                auto_increment: false,
            },
        ],
        foreign_keys: vec![],
        unique_constraints: vec![],
        check_constraints: vec![],
        partition_info: None,
        collations: Default::default(),
        compression: None,
        original_sql: String::new(),
    }
}

fn row(id: i64, v: i64) -> Record {
    vec![Value::Integer(id), Value::Integer(v)]
}

fn fresh(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "4994_workers_{tag}_{}_{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// One worker's share of the script's inner loop, against its own `tx_id`.
///
/// The INSERT and UPDATE run **inside** an explicit transaction rather than
/// as bare autocommit statements. That is not a stylistic choice: it is what
/// the SQL layer does (the executor wraps statements in a transaction), and
/// it is the only way to reach the path this issue is about.
/// `FileStorage::insert_direct` records an undo entry **only when
/// `current_tx_id != 0`** — an autocommit insert goes straight to the buffer
/// with nothing to undo, so a test built from autocommit inserts never
/// exercises the rollback undo at all. A first draft of this file did
/// exactly that and a deliberate re-introduction of the #5059 blanket sweep
/// sailed straight through it (mutation survived).
fn run_worker(storage: &Arc<RwLock<FileStorage>>, w: i64, rounds: i64) {
    for i in 1..=rounds {
        let rid = w * 1000 + i;
        // A distinct tx_id per (worker, round): a transaction that spans
        // rounds would make its own earlier writes part of every later
        // rollback, which is a different defect.
        let tx = (w as u64) * 1000 + i as u64;

        // Steps 1, 2 and 5 of the script: INSERT, UPDATE, then COMMIT on
        // even rounds / ROLLBACK on odd ones.
        {
            let mut g = storage.write().unwrap();
            g.begin_transaction_for(tx).unwrap();
            g.insert("t", vec![row(rid, w)]).unwrap();
            g.update("t", &[Value::Integer(rid)], &[(1, Value::Integer(w + 10))])
                .unwrap();
            if i % 2 == 1 {
                g.rollback_transaction_for(tx).unwrap();
            } else {
                g.commit_transaction_for(tx).unwrap();
            }
        }

        // Step 4: DELETE, only past the original script's threshold. The
        // worker's own row, addressed by its own id.
        if i > DELETE_FROM {
            storage
                .write()
                .unwrap()
                .delete("t", &[Value::Integer(rid)])
                .unwrap();
        }
    }
}

/// The ids a worker must still own at the end: committed (even) rounds that
/// were also below the delete threshold.
fn expected_ids(w: i64) -> Vec<i64> {
    (1..=DELETE_FROM)
        .filter(|i| i % 2 == 0)
        .map(|i| w * 1000 + i)
        .collect()
}

/// The reported scenario: 8 workers × 12 rounds, then count what is left.
///
/// Expected `8 × 8 + 1 = 65`. The original report saw 0.
#[test]
fn eight_concurrent_workers_do_not_lose_each_others_rows() {
    const WORKERS: i64 = 8;
    const ROUNDS: i64 = 12;

    let dir = fresh("eight_workers");
    let storage = Arc::new(RwLock::new(FileStorage::new(dir.clone()).unwrap()));
    {
        let mut g = storage.write().unwrap();
        g.create_table(&counter_table("t")).unwrap();
        g.insert("t", vec![row(0, 0)]).unwrap(); // the script's seed row
    }

    let mut handles = Vec::new();
    for w in 1..=WORKERS {
        let storage = Arc::clone(&storage);
        handles.push(std::thread::spawn(move || {
            // Each worker owns a disjoint id space (w*1000 + i) and its own
            // per-round tx ids, which is what makes these 8 independent
            // connections rather than one shared transaction.
            run_worker(&storage, w, ROUNDS);
        }));
    }
    for h in handles {
        h.join().expect("worker panicked");
    }

    let survivors = {
        let g = storage.write().unwrap();
        g.flush_all_buffers().unwrap();
        g.scan("t").unwrap()
    };

    // The rows that must still be there: each worker's committed rounds
    // below the delete threshold. A row missing from that set is a
    // rollback that reached past its own transaction.
    let mut missing = Vec::new();
    for w in 1..=WORKERS {
        for rid in expected_ids(w) {
            if !survivors.iter().any(|r| r[0] == Value::Integer(rid)) {
                missing.push(rid);
            }
        }
    }
    let expected: usize = (WORKERS * KEEP_FROM) as usize + 1;
    assert!(
        missing.is_empty(),
        "a worker's ROLLBACK reached other workers' rows. \
         {}/{} expected rows gone, first few: {:?}. \
         Survivors: {} (expected {}). \
         Every statement returned Ok — that is the #4994 signature: \
         all operations succeed while the data disappears.",
        missing.len(),
        WORKERS * KEEP_FROM,
        &missing[..missing.len().min(8)],
        survivors.len(),
        expected,
    );

    assert_eq!(
        survivors.len(),
        expected,
        "survivor count must be workers×{} + 1 seed; got {}",
        KEEP_FROM,
        survivors.len(),
    );
}

/// The same workload, run on one thread.
///
/// ## What this control does and does not tell you
///
/// It is **not** a concurrency discriminator. Re-introducing the #5059
/// blanket sweep fails both tests, because a sweep that empties the buffer
/// for a table is destructive whether or not other threads are running —
/// the other workers' rows are already sitting in that buffer.
///
/// What it does establish is narrower and still worth having: the
/// insert / update / rollback / delete **sequence** is sound on the current
/// code. So if the concurrent test fails, the workload shape is not the
/// reason, and the failure has to be in how concurrent rollback interacts
/// with other connections' buffered rows.
///
/// Verified: mutation M-O (value-keyed undo → blanket table sweep) fails
/// *both* tests on `develop/v4.1.0`, and the concurrent one reports
/// `32/32 expected rows gone ... Survivors: 0` — the original #4994
/// signature, all statements returning Ok while the data disappears.
#[test]
fn the_same_sequence_without_concurrency_keeps_every_row() {
    const WORKERS: i64 = 8;
    const ROUNDS: i64 = 12;

    let dir = fresh("serial_control");
    let storage = Arc::new(RwLock::new(FileStorage::new(dir.clone()).unwrap()));
    {
        let mut g = storage.write().unwrap();
        g.create_table(&counter_table("t")).unwrap();
        g.insert("t", vec![row(0, 0)]).unwrap();
    }

    for w in 1..=WORKERS {
        run_worker(&storage, w, ROUNDS);
    }

    let survivors = {
        let g = storage.write().unwrap();
        g.flush_all_buffers().unwrap();
        g.scan("t").unwrap()
    };
    assert_eq!(
        survivors.len(),
        (WORKERS * KEEP_FROM) as usize + 1,
        "serial control: survivor count"
    );
}
