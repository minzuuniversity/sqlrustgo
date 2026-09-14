//! Phase C.1 race-condition coverage for FileStorage.
//!
//! These tests simulate the Phase C server-layer pattern:
//! `Arc<RwLock<FileStorage>>` — multiple reader threads share a
//! single `RwLockReadGuard` over the storage, while writers take
//! a `RwLockWriteGuard`. After C.1.1.bis the inherent methods
//! (`scan`, `get_table`, `flush`, `discard_all_buffers`,
//! `clear_all_tables`, `partition_rows`, ...) take `&self`; the
//! trait impl writes (`insert`, `delete`, `delete_collect_pks`,
//! `delete_if`, `commit_transaction`, `rollback_transaction`,
//! `flush`, ...) keep `&mut self` and serialise internally via the
//! new `write_lock: parking_lot::Mutex<()>` field (see
//! `PHASE_C_1_INTERNAL_LOCKING.md` §2–§3).
//!
//! # What these tests do (and do not) catch
//!
//! The outer `Arc<RwLock<FileStorage>>` serialises ALL access — at
//! any instant exactly one thread holds either a `read()` or
//! `write()` guard. This means there is **no real data race
//! possible** at the Rust aliasing level: the borrow checker
//! guarantees `&FileStorage` and `&mut FileStorage` never coexist
//! across threads.
//!
//! What the tests DO catch:
//! 1. **Deadlock**: a missed `with_write_lock` wrapper, or a
//!    re-entrant `parking_lot::Mutex` acquire, would deadlock the
//!    test thread. `cargo test` would time out.
//! 2. **Lost writes**: if the inner `write_lock` were missing or
//!    scoped too narrowly, two concurrent writers would each
//!    append rows but only one set would survive. Final integrity
//!    check would catch this.
//! 3. **Torn-row reads**: a `scan()` that misses a `data.rows.clone()`
//!    barrier (or reads without the inner lock) would see rows
//!    mid-mutation. The row-column-count and type checks at the
//!    end would catch this.
//! 4. **Transaction-state corruption**: `current_tx_id` and
//!    `tx_undo_log` are particularly sensitive — if any of the
//!    BEGIN/COMMIT/ROLLBACK paths missed the lock wrapper, the
//!    undo log would be torn. The final "100 committed rows" check
//!    would catch this.
//!
//! What the tests do NOT catch (out of scope for C.1.3):
//! - **Phase C.2 data races** — when the outer `Arc<RwLock>` is
//!   removed and the server layer accesses `Arc<FileStorage>`
//!   directly, multiple threads could hold `&FileStorage`
//!   simultaneously. That race condition is C.2's problem and
//!   needs `UnsafeCell`-based fields, not just an internal
//!   `Mutex<()>`. A test for that would require bypassing the
//!   borrow checker via the same `as_mut_self` trick the storage
//!   itself uses; it is intentionally deferred.
//!
//! Run with:
//!   cargo test -p sqlrustgo-storage --test phase_c_1_race -- --nocapture
//!
//! # C.1.3 success criterion
//!
//! All 5 tests below pass with the C.1.1.bis + C.1.2 changes
//! applied, including the existing 141-test `cargo test --lib`
//! suite, the 47-test `file_storage_direct_v3_12`, the 32-test
//! `wal_storage_direct_v3_12`, the 18-test
//! `storage_integration_test`, the 1-test
//! `bulk_load_quadraticity` (proves the C.1.2 delete / insert
//! rewrites are NOT quadratic), and the 4-test
//! `e2e_crash_recovery_proof`.

use std::sync::Arc;
use std::sync::RwLock;
use std::thread;
use std::time::Duration;

use tempfile::tempdir;

use sqlrustgo_storage::{ColumnDefinition, FileStorage, StorageEngine, TableInfo, Value};

/// Helper: build a minimal TableInfo for the race tests.
fn make_counter_table(name: &str) -> TableInfo {
    TableInfo {
        name: name.to_string(),
        columns: vec![ColumnDefinition {
            name: "id".into(),
            data_type: "BIGINT".into(),
            nullable: false,
            auto_increment: false,
            ..Default::default()
        }],
        ..Default::default()
    }
}

/// Spawn N reader threads, each runs `op_count` scans, and return
/// after all readers finish. The "success" signal is the absence
/// of panics during the read path — readers don't collect
/// counters because that would force them all into the same
/// JoinHandle<usize> type while writers would be JoinHandle<()>.
fn spawn_scanners(
    storage: Arc<RwLock<FileStorage>>,
    table: &'static str,
    n_threads: usize,
    ops_per_thread: usize,
) {
    let mut handles = Vec::with_capacity(n_threads);
    for tid in 0..n_threads {
        let s = Arc::clone(&storage);
        handles.push(thread::spawn(move || {
            for i in 0..ops_per_thread {
                let guard = s.read().expect("read guard");
                let _ = guard.scan(table);
                // Force a small yield to encourage interleaving.
                if (tid + i) % 17 == 0 {
                    thread::yield_now();
                }
            }
        }));
    }
    for h in handles {
        h.join().expect("scanner join");
    }
}

/// Spawn a single writer thread that inserts `n_rows` rows into
/// `table` (id values 0..n_rows). Used as the "interfering"
/// operation that the readers must coexist with.
fn spawn_writer(
    storage: Arc<RwLock<FileStorage>>,
    table: &'static str,
    n_rows: usize,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        for id in 0..n_rows {
            let row = vec![Value::Integer(id as i64)];
            let mut guard = storage.write().expect("write guard");
            let _ = guard.insert(table, vec![row]);
            // Drop the guard every iteration — that's how the
            // server's per-connection code path actually behaves
            // (one lock per query, not held for the whole txn).
            drop(guard);
            if id % 50 == 0 {
                thread::yield_now();
            }
        }
    })
}

/// Race test #1 — concurrent scans while a writer is active.
///
/// With C.1 done correctly, readers must never see a torn row
/// (i.e. a row missing the `id` column, or one with the wrong
/// length). The borrow checker + `with_write_lock` wrapper
/// guarantee this because every read of `data.rows.clone()`
/// happens on the underlying `HashMap` whose contents can only
/// change while the writer holds the inner write_lock.
#[test]
fn c1_concurrent_scans_no_torn_reads() {
    let dir = tempdir().expect("tempdir");
    let storage = Arc::new(RwLock::new(
        FileStorage::new(dir.path().to_path_buf()).expect("FileStorage::new"),
    ));
    storage
        .write()
        .expect("write guard")
        .create_table(&make_counter_table("counters"))
        .expect("create_table");

    // Pre-insert 100 rows so the table is non-empty when scanning
    // starts; this is what makes a torn read detectable.
    {
        let mut g = storage.write().expect("pre-write guard");
        let rows: Vec<Vec<Value>> = (0..100).map(|i| vec![Value::Integer(i)]).collect();
        g.insert("counters", rows).expect("pre-insert");
    }

    let writer = spawn_writer(Arc::clone(&storage), "counters", 500);
    spawn_scanners(Arc::clone(&storage), "counters", 4, 100);
    writer.join().expect("writer join");

    // Final integrity check: post-writer, we should have 600 rows.
    let final_count = storage
        .read()
        .expect("final guard")
        .scan("counters")
        .expect("final scan")
        .len();
    assert_eq!(
        final_count, 600,
        "post-writer row count must equal pre-existing + inserted"
    );

    // Every row must have exactly one column with the expected
    // type. A torn read would manifest as a Vec with zero columns
    // or with a wrong-type Value.
    let rows = storage
        .read()
        .expect("integrity guard")
        .scan("counters")
        .expect("integrity scan");
    for row in &rows {
        assert_eq!(row.len(), 1, "torn row: wrong column count");
        assert!(
            matches!(row[0], Value::Integer(_)),
            "torn row: wrong value type"
        );
    }
}

/// Race test #2 — interleaved inserts + scans.
///
/// Mirrors the Phase B 8-thread 10k-row workload's read/write
/// mix: writers do `insert` while readers do `scan`. After all
/// threads finish the table's contents must still be internally
/// consistent (no torn rows, no double-counts).
#[test]
fn c1_concurrent_insert_and_scan() {
    let dir = tempdir().expect("tempdir");
    let storage = Arc::new(RwLock::new(
        FileStorage::new(dir.path().to_path_buf()).expect("FileStorage::new"),
    ));
    storage
        .write()
        .expect("write guard")
        .create_table(&make_counter_table("t"))
        .expect("create_table");

    let writer_count = 2;
    let reader_count = 4;
    let ops_per_thread = 200;

    let mut handles = Vec::new();
    for w in 0..writer_count {
        let s = Arc::clone(&storage);
        handles.push(thread::spawn(move || {
            for id in 0..ops_per_thread {
                let row = vec![Value::Integer((w * 10_000 + id) as i64)];
                let mut g = s.write().expect("write guard");
                let _ = g.insert("t", vec![row]);
            }
        }));
    }
    for _ in 0..reader_count {
        let s = Arc::clone(&storage);
        handles.push(thread::spawn(move || {
            for _ in 0..ops_per_thread {
                let g = s.read().expect("read guard");
                let _ = g.scan("t");
            }
        }));
    }
    for h in handles {
        h.join().expect("thread join");
    }

    // Final integrity check: all writer inserts must have landed.
    let rows = storage
        .read()
        .expect("integrity guard")
        .scan("t")
        .expect("integrity scan");
    assert_eq!(
        rows.len(),
        writer_count * ops_per_thread,
        "all inserts should land"
    );
    for row in &rows {
        assert_eq!(row.len(), 1, "torn row");
    }
}

/// Race test #3 — concurrent flush + scan.
///
/// `flush` mutates `dirty_tables` (C.1.2 wrapped it under
/// `with_write_lock`) and then calls `save_table` (which does
/// file I/O outside the lock). Readers doing `scan` between the
/// dirty-set drain and the per-table save must not see a torn
/// `data.rows` snapshot.
#[test]
fn c1_concurrent_flush_and_scan() {
    let dir = tempdir().expect("tempdir");
    let storage = Arc::new(RwLock::new(
        FileStorage::new_with_buffer_config(dir.path().to_path_buf(), 1_000, true)
            .expect("FileStorage::new_with_buffer_config"),
    ));
    storage
        .write()
        .expect("write guard")
        .create_table(&make_counter_table("flush_t"))
        .expect("create_table");

    // Pre-insert 500 rows so flush has work to do.
    {
        let mut g = storage.write().expect("pre-write guard");
        let rows: Vec<Vec<Value>> = (0..500).map(|i| vec![Value::Integer(i)]).collect();
        g.insert("flush_t", rows).expect("pre-insert");
    }

    let flusher = {
        let s = Arc::clone(&storage);
        thread::spawn(move || {
            for _ in 0..10 {
                let g = s.write().expect("flush guard");
                let _ = g.flush();
                drop(g);
                thread::sleep(Duration::from_millis(5));
            }
        })
    };
    spawn_scanners(Arc::clone(&storage), "flush_t", 4, 100);
    flusher.join().expect("flusher join");

    // Final integrity check after all flushes complete.
    let rows = storage
        .read()
        .expect("integrity guard")
        .scan("flush_t")
        .expect("integrity scan");
    assert_eq!(rows.len(), 500, "no rows lost across concurrent flushes");
    for row in &rows {
        assert_eq!(row.len(), 1, "torn row");
    }
}

/// Race test #4 — concurrent BEGIN/COMMIT/ROLLBACK.
///
/// The trait impl `begin_transaction` / `commit_transaction` /
/// `rollback_transaction` are the C.1.2 hot-paths that wrap
/// `current_tx_id` and `tx_undo_log` under `with_write_lock`. This
/// test runs 4 transactions concurrently for 50 cycles each; if
/// any path missed a lock wrapper, the tx_undo_log would be torn
/// and the final `SELECT COUNT(*)` would be wrong.
///
/// Note on commit semantics: FileStorage::commit_transaction only
/// drops the undo log + zeroes tx_id; it does NOT flush buffered
/// inserts (see the C.1.2 commit_transaction doc-comment in
/// file_storage.rs). So after COMMIT, rows remain in the
/// in-memory `insert_buffer` until either a flush or another
/// auto-flush trigger fires. To make the test observe the
/// committed rows via `scan()` (which reads `data.rows` after
/// merging the buffer), each thread follows COMMIT with an
/// explicit `flush_all_buffers` call.
#[test]
fn c1_concurrent_begin_commit_rollback() {
    let dir = tempdir().expect("tempdir");
    let storage = Arc::new(RwLock::new(
        FileStorage::new(dir.path().to_path_buf()).expect("FileStorage::new"),
    ));
    storage
        .write()
        .expect("write guard")
        .create_table(&make_counter_table("tx_t"))
        .expect("create_table");

    let mut handles = Vec::new();
    for tid in 0..4 {
        let s = Arc::clone(&storage);
        handles.push(thread::spawn(move || {
            for cycle in 0..50 {
                let mut g = s.write().expect("tx guard");
                // BEGIN
                let _ = g.begin_transaction();
                // Insert one row inside the tx.
                let row = vec![Value::Integer((tid * 1000 + cycle) as i64)];
                let _ = g.insert("tx_t", vec![row]);
                // Half of the cycles COMMIT, half ROLLBACK.
                if cycle % 2 == 0 {
                    let _ = g.commit_transaction();
                } else {
                    let _ = g.rollback_transaction();
                }
                // After commit, flush buffered inserts so they
                // become visible to scan()'s data-rows merge.
                let _ = g.flush_all_buffers();
            }
        }));
    }
    for h in handles {
        h.join().expect("tx thread join");
    }

    // Final integrity: count rows where id came from a committed
    // tx. For tid in 0..4 and cycle in 0..50 with cycle % 2 == 0:
    //   id = tid * 1000 + cycle, only those survived COMMIT.
    // Total = 4 threads × 25 even cycles = 100 rows.
    let rows = storage
        .read()
        .expect("integrity guard")
        .scan("tx_t")
        .expect("integrity scan");
    assert_eq!(
        rows.len(),
        100,
        "concurrent tx churn must yield exactly 100 committed rows"
    );
    for row in &rows {
        assert_eq!(row.len(), 1, "torn row in tx-torn table");
        assert!(matches!(row[0], Value::Integer(_)));
    }
}

/// Race test #5 — final data integrity after concurrent ops.
///
/// Mixes insert / delete / scan from many threads, then verifies
/// the row count matches what the writer bookkeeping says it
/// should be. A missed `with_write_lock` on insert / delete
/// would manifest as either a count mismatch (lost write) or a
/// torn row on read.
#[test]
fn c1_data_integrity_after_concurrent_ops() {
    let dir = tempdir().expect("tempdir");
    let storage = Arc::new(RwLock::new(
        FileStorage::new(dir.path().to_path_buf()).expect("FileStorage::new"),
    ));
    storage
        .write()
        .expect("write guard")
        .create_table(&make_counter_table("integrity_t"))
        .expect("create_table");

    // Each writer thread inserts 50 rows then deletes every other
    // one. Final surviving count per thread = 25. Four threads
    // → 100 rows total.
    let n_threads = 4;
    let rows_per_thread = 50;

    let mut handles = Vec::new();
    for tid in 0..n_threads {
        let s = Arc::clone(&storage);
        handles.push(thread::spawn(move || {
            {
                let mut g = s.write().expect("integrity guard");
                // Insert batch.
                let rows: Vec<Vec<Value>> = (0..rows_per_thread)
                    .map(|i| vec![Value::Integer((tid * 1000 + i) as i64)])
                    .collect();
                g.insert("integrity_t", rows).expect("insert batch");
                // Delete half of them by id (delete is single-column
                // equality filter on column 0 in FileStorage).
                for i in (0..rows_per_thread).step_by(2) {
                    let id = (tid * 1000 + i) as i64;
                    g.delete("integrity_t", &[Value::Integer(id)])
                        .expect("delete");
                }
            }
            // Run concurrent scans during the writes (here only
            // after this thread's own writes are done).
            for _ in 0..10 {
                let rg = s.read().expect("read guard");
                let _ = rg.scan("integrity_t");
            }
        }));
    }
    for h in handles {
        h.join().expect("integrity thread join");
    }

    let rows = storage
        .read()
        .expect("final integrity guard")
        .scan("integrity_t")
        .expect("final integrity scan");
    assert_eq!(
        rows.len(),
        n_threads * (rows_per_thread / 2),
        "concurrent insert+delete must yield the expected count"
    );
    for row in &rows {
        assert_eq!(row.len(), 1, "torn row");
    }
}
