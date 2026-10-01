//! #4915 / B2.2 A/B measurement harness — reader latency under lock-held I/O.
//!
//! `#[ignore]`d: this is a measurement, not a correctness test. It is run
//! explicitly against two builds that differ ONLY in whether
//! `FileStorage::flush` performs its per-table serialization + `write()`
//! inside the internal `with_write_lock` (pre-B2.2, `be665d6bc1`) or after
//! releasing it (post-B2.2, `e2355c0680` + `ff34478830`).
//!
//! Run:
//!   cargo test -p sqlrustgo-storage --release \
//!       --test b22_concurrent_flush_ab -- --ignored --nocapture
//!
//! Design: one shared `Arc<RwLock<FileStorage>>`, the shape the MySQL server
//! uses. Readers hold the read guard across `StorageEngine::scan`; the
//! writer takes the write guard to insert (so it also drains the insert
//! buffer and persists the window) and calls `flush()` with NO outer guard,
//! mirroring the server. Any I/O performed while the *internal*
//! `with_write_lock` is held therefore shows up directly as reader latency,
//! because `scan` also needs that internal lock.

use parking_lot::RwLock;
use sqlrustgo_storage::mvcc_storage::MvccStorage;
use sqlrustgo_storage::{ColumnDefinition, FileStorage, Record, StorageEngine, TableInfo, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

const READER_THREADS: usize = 4;
const WRITER_ROUNDS: usize = 400;
const BATCH: usize = 100; // window size per explicit flush
const PREFILL: usize = 20_000;

fn table_info(name: &str) -> TableInfo {
    TableInfo {
        name: name.to_string(),
        columns: vec![ColumnDefinition {
            name: "a".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: false,
            primary_key: false,
            char_max_length: None,
            collation: None,
            default_value: None,
            auto_increment: false,
        }],
        foreign_keys: vec![],
        unique_constraints: vec![],
        check_constraints: vec![],
        partition_info: None,
        collations: Default::default(),
        compression: None,
        original_sql: String::new(),
    }
}

fn pct(v: &mut [f64], p: f64) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let k = ((p / 100.0) * (v.len() as f64 - 1.0)).round() as usize;
    v[k.min(v.len() - 1)]
}

fn rows(base: i64, tag: &str) -> Vec<Record> {
    (0..BATCH)
        .map(|i| {
            vec![
                Value::Integer(base + i as i64),
                Value::Text(format!("{tag}{}", "x".repeat(32))),
            ]
        })
        .collect()
}

#[test]
#[ignore]
fn b22_reader_latency_under_lock_held_io() {
    let dir = tempfile::tempdir().expect("tempdir");
    // Threshold above the per-round batch so the explicit `flush()` carries
    // each round's delta; its internal `with_write_lock` hold time is exactly
    // what B2.2 changed.
    const BUFFER_THRESHOLD: usize = 10_000;
    let inner =
        FileStorage::new_with_buffer_config(dir.path().to_path_buf(), BUFFER_THRESHOLD, true)
            .expect("FileStorage");
    // Wrap in MvccStorage: this is what the MySQL server actually shares
    // across connections, and unlike a bare `FileStorage` it is `Send`
    // (`parking_lot::RwLockWriteGuard<FileStorage>` is `!Send` because
    // `with_write_lock` erases a guard lifetime through `*mut ()`).
    let mut storage = MvccStorage::new(inner);
    storage
        .create_table(&table_info("t"))
        .expect("create_table");

    // Prefill so the table is large enough that a full-table serialization
    // (the pre-B2.2 shape) is measurably different from a window write.
    for chunk in 0..(PREFILL / BATCH) {
        let base = (chunk * BATCH) as i64;
        storage
            .insert("t", rows(base, "p"))
            .expect("prefill insert");
    }
    storage.flush().expect("prefill flush");

    let shared = Arc::new(RwLock::new(storage));
    let stop = Arc::new(AtomicBool::new(false));

    let mut reader_handles = Vec::new();
    for _ in 0..READER_THREADS {
        let shared = Arc::clone(&shared);
        let stop = Arc::clone(&stop);
        reader_handles.push(std::thread::spawn(move || {
            let mut lats = Vec::new();
            while !stop.load(Ordering::Relaxed) {
                let t0 = Instant::now();
                {
                    // Write guard, not read guard: `StorageEngine::scan` takes
                    // `&self`, but we want reader/reader parallelism to be
                    // possible, so use the shared read side.
                    let guard = shared.read();
                    let scanned = guard.scan("t").expect("scan");
                    std::hint::black_box(scanned.len());
                }
                lats.push(t0.elapsed().as_secs_f64() * 1e3);
            }
            lats
        }));
    }

    let shared_w = Arc::clone(&shared);
    let writer = std::thread::spawn(move || {
        for round in 0..WRITER_ROUNDS {
            let base = (PREFILL + round * BATCH) as i64;
            {
                let mut guard = shared_w.write();
                guard.insert("t", rows(base, "w")).expect("writer insert");
            }
            // No outer guard held: exactly the server's checkpoint shape.
            shared_w.write().flush().expect("flush");
        }
    });

    writer.join().expect("writer");
    stop.store(true, Ordering::Relaxed);

    let mut all = Vec::new();
    for h in reader_handles {
        all.extend(h.join().expect("reader"));
    }

    let n = all.len();
    println!(
        "B22_RESULT n={n} p50={:.4} p95={:.4} p99={:.4}",
        pct(&mut all.clone(), 50.0),
        pct(&mut all.clone(), 95.0),
        pct(&mut all.clone(), 99.0),
    );
    assert!(n > 0, "no reader samples collected");
}
