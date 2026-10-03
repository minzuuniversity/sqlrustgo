// BLK-1 — AUTO_INCREMENT allocation must happen under the storage write
// lock, not from a pre-lock scan.
//
// docs/releases/v4.1.0/ISSUES_PLAN.md §4.1 (BLK-1):
// `src/engine_dml.rs` computed the next AUTO_INCREMENT id as
// `MAX(existing) + 1` from `pre_scanned_rows`, which is captured *before*
// `engine.storage.write()` is taken. Two INSERTs arriving together
// therefore both read the same MAX and both start from the same value,
// and the second one fails with
//     Duplicate entry '...' for key 'PRIMARY'
//
// That is what made every insert-bearing sysbench workload abort at
// 8 threads (`oltp_read_write`, `oltp_write_only`), on this commit and on
// the baseline be665d6bc1 alike.
//
// Concurrency model matches production and the existing benchmark tests
// (tests/benchmark/qps_benchmark_test.rs): one ExecutionEngine per thread
// — one per connection — over a shared Arc<RwLock<MemoryStorage>>.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, SqlResult};
use sqlrustgo_storage::{MemoryStorage, StorageEngine};
use std::sync::Arc;

/// Shared storage, with an AUTO_INCREMENT table already created.
fn setup() -> Arc<RwLock<MemoryStorage>> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    engine
        .execute("CREATE TABLE t (id INTEGER AUTO_INCREMENT PRIMARY KEY, v INTEGER)")
        .expect("CREATE TABLE must succeed");
    storage
}

/// Run `threads` concurrent INSERTs of `per_thread` rows each; return
/// every error any thread saw.
fn concurrent_insert_errors(
    storage: Arc<RwLock<MemoryStorage>>,
    threads: usize,
    per_thread: usize,
) -> Vec<String> {
    let errors: Arc<parking_lot::Mutex<Vec<String>>> =
        Arc::new(parking_lot::Mutex::new(Vec::new()));
    let mut handles = Vec::with_capacity(threads);

    for t in 0..threads {
        let storage = storage.clone();
        let errors = errors.clone();
        handles.push(std::thread::spawn(move || {
            let mut engine = ExecutionEngine::new(storage);
            for i in 0..per_thread {
                let sql = format!("INSERT INTO t (v) VALUES ({})", t * per_thread + i);
                if let Err(e) = engine.execute(&sql) {
                    errors.lock().push(format!("{}", e));
                }
            }
        }));
    }
    for h in handles {
        h.join().expect("insert thread must not panic");
    }

    let out = errors.lock().clone();
    out
}

/// Read back the `id` column, sorted.
fn sorted_ids(storage: &Arc<RwLock<MemoryStorage>>) -> Vec<i64> {
    let mut engine = ExecutionEngine::new(storage.clone());
    let result: SqlResult<_> = engine.execute("SELECT id FROM t");
    let result = result.expect("SELECT must succeed");
    let mut ids: Vec<i64> = result
        .rows
        .iter()
        .filter_map(|r| r.first().and_then(|v| v.as_integer()))
        .collect();
    ids.sort_unstable();
    ids
}

#[test]
fn concurrent_inserts_get_distinct_auto_increment_ids() {
    let storage = setup();

    let errs = concurrent_insert_errors(storage, 4, 25);

    assert!(
        errs.is_empty(),
        "4 threads x 25 concurrent INSERTs produced {} error(s); \
         AUTO_INCREMENT ids were allocated outside the write lock. \
         First: {}",
        errs.len(),
        errs.first().map(String::as_str).unwrap_or("<none>")
    );
}

#[test]
fn concurrent_inserts_leave_no_gaps_in_auto_increment_sequence() {
    let storage = setup();

    let errs = concurrent_insert_errors(storage.clone(), 4, 25);
    assert!(errs.is_empty(), "insert errors: {:?}", errs);

    let ids = sorted_ids(&storage);

    // 100 rows inserted, so ids must be exactly 1..=100. A gap would
    // mean an id was minted for a row that never landed.
    assert_eq!(
        ids.len(),
        100,
        "expected 100 rows to land, got {}",
        ids.len()
    );
    let expected: Vec<i64> = (1..=100).collect();
    assert_eq!(
        ids, expected,
        "AUTO_INCREMENT ids must be contiguous 1..=100 — no gaps, no duplicates"
    );
}

#[test]
fn explicit_id_advances_the_auto_increment_sequence() {
    // MySQL semantics: after an explicit id, the next generated id is
    // max(explicit) + 1. This guards that the move under the lock did
    // not change which value the sequence resumes from.
    let storage = setup();
    {
        let mut engine = ExecutionEngine::new(storage.clone());
        engine
            .execute("INSERT INTO t (id, v) VALUES (500, 0)")
            .expect("explicit-id insert must succeed");
    }

    let errs = concurrent_insert_errors(storage.clone(), 4, 10);
    assert!(errs.is_empty(), "insert errors: {:?}", errs);

    let ids = sorted_ids(&storage);
    assert_eq!(
        ids.len(),
        41,
        "1 explicit + 40 generated, got {}",
        ids.len()
    );
    // MySQL semantics: the sequence resumes at max(explicit) + 1 = 501,
    // so there is no id in 2..=500.
    assert_eq!(ids[0], 500, "the explicit id must be present");
    assert!(
        ids.contains(&501),
        "sequence must resume at 501 after explicit id 500; tail was {:?}",
        &ids[ids.len().saturating_sub(5)..]
    );
    assert_eq!(
        *ids.last().unwrap(),
        540,
        "40 generated ids after 500 must end at 540"
    );
}

#[test]
fn single_threaded_auto_increment_still_starts_at_one() {
    // Guards pre-existing behaviour the fix must not disturb.
    let storage = setup();
    {
        let mut engine = ExecutionEngine::new(storage.clone());
        engine
            .execute("INSERT INTO t (v) VALUES (1)")
            .expect("insert must succeed");
        engine
            .execute("INSERT INTO t (v) VALUES (2)")
            .expect("insert must succeed");
    }
    assert_eq!(sorted_ids(&storage), vec![1, 2]);
}

#[test]
fn null_in_explicit_column_is_replaced_by_generated_id() {
    // A row that supplies NULL for the AUTO_INCREMENT column must get a
    // generated id, not a NULL.
    let storage = setup();
    {
        let mut engine = ExecutionEngine::new(storage.clone());
        engine
            .execute("INSERT INTO t (id, v) VALUES (NULL, 7)")
            .expect("insert must succeed");
    }
    assert_eq!(sorted_ids(&storage), vec![1]);
}
