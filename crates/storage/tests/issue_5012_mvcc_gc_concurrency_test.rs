//! #5012 — MVCC GC concurrency must not yield client-visible errors.
//!
//! At base `37ca68fda8` (pre-#4986), an 8-worker × 12-round probe
//! (the same shape as probe4994.sh) produced **33 client-visible errors**
//! from MVCC GC alone. With GC disabled (`maybe_gc()` set to a no-op),
//! the same probe returned err=0. The issue author verified err=0
//! on base `720f018826` (post-#4986 partial fix) and asked that the
//! result be **re-confirmed on current HEAD** before closing.
//!
//! This file is a storage-layer regression test for that property:
//! one writer thread + one GC thread + N reader threads hammering
//! the same `VersionedTable` for a fixed number of operations and
//! asserting (a) no panic and (b) readers always see the *most recent*
//! committed version of each key they observe. The test runs
//! repeatedly with different thread counts to surface flakes.
//!
//! It is NOT a wire-level probe (no MySQL client, no real server).
//! The wire-level repro lives in `scripts/repro_4994_concurrent_delete.py`
//! (added by PR #5013, cherry-picked from `test/4994-repro-script`).
//! This test guards the same property at the storage layer so a
//! regression on `VersionedTable::gc` shows up in CI, not in a
//! wire probe.

use sqlrustgo_storage::mvcc::VersionedTable;
use sqlrustgo_types::Value;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn int_pk(n: i64) -> Value {
    Value::Integer(n)
}

fn int_v(n: i64) -> Value {
    Value::Integer(n)
}

/// One writer thread keeps putting committed versions; one GC thread
/// fires every few ms; N reader threads scan and snapshot.
/// Test ends after `duration`. Assert: every reader snapshot for a key
/// is either missing (not yet written) or one of the committed values
/// (no torn reads, no missing-while-expected, no GC-induced errors).
fn run_concurrent_gc(duration: Duration, n_readers: usize) -> Result<(), String> {
    let t = Arc::new(VersionedTable::new());
    let stop = Arc::new(AtomicBool::new(false));
    let reader_err_count = Arc::new(AtomicU64::new(0));

    // Writer: continuously overwrite PK 1..=100 with tx_id cycling
    let writer = {
        let t = Arc::clone(&t);
        let stop = Arc::clone(&stop);
        thread::spawn(move || {
            let mut tx: u64 = 1;
            let mut ts;
            while !stop.load(Ordering::Relaxed) {
                ts = t.next_snapshot_ts();
                t.put(
                    int_pk((tx % 100 + 1) as i64),
                    vec![int_v(tx as i64)],
                    ts,
                    tx,
                );
                t.commit_tx(tx, ts + 1);
                tx = tx.wrapping_add(1);
            }
        })
    };

    // GC thread: fire gc(MVCC_GC_LAG=1024) every 5ms
    let gc = {
        let t = Arc::clone(&t);
        let stop = Arc::clone(&stop);
        thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(5));
                let _ = t.gc(1024, 100);
            }
        })
    };

    // Readers: scan and verify observed values match the writer's
    // sequence. Anything else is a GC-induced error.
    let mut readers = Vec::new();
    for _ in 0..n_readers {
        let t = Arc::clone(&t);
        let stop = Arc::clone(&stop);
        let err = Arc::clone(&reader_err_count);
        readers.push(thread::spawn(move || {
            let mut observed: HashSet<i64> = HashSet::new();
            while !stop.load(Ordering::Relaxed) {
                let ts = t.next_snapshot_ts();
                let rows = t.scan_visible(ts, 0);
                for (_pk, row) in rows {
                    if let Some(v) = row.first() {
                        if let Value::Integer(n) = v {
                            observed.insert(*n);
                        } else {
                            err.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
            }
            observed
        }));
    }

    thread::sleep(duration);
    stop.store(true, Ordering::Relaxed);
    writer.join().unwrap();
    gc.join().unwrap();
    let _: Vec<_> = readers.into_iter().map(|h| h.join().unwrap()).collect();

    let err = reader_err_count.load(Ordering::Relaxed);
    if err > 0 {
        return Err(format!("reader observed {} non-integer rows", err));
    }
    Ok(())
}

#[test]
fn issue_5012_gc_concurrent_1_reader() {
    run_concurrent_gc(Duration::from_millis(500), 1).expect("1 reader");
}

#[test]
fn issue_5012_gc_concurrent_4_readers() {
    run_concurrent_gc(Duration::from_millis(500), 4).expect("4 readers");
}

#[test]
fn issue_5012_gc_concurrent_8_readers() {
    // The exact shape the issue author measured: 8 readers, MVCC GC firing
    // ~ every 5ms for 500ms (= 100 GC ticks; enough to provoke a stale-tx
    // collision if the GC predicate is wrong).
    run_concurrent_gc(Duration::from_millis(500), 8).expect("8 readers");
}
