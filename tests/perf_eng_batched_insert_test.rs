//! Performance regression test for issue #3013
//!
//! 1000-row batched INSERT through `eng.execute` was O(N²) because
//! `WalStorage::insert` called `log_insert` N times, each triggering
//! a per-record `flush` via `FileBackedWalManager::append` (default
//! `batch_mode=false`). The P1 fix enables batch mode + threshold=usize::MAX
//! inside `WalStorage::insert` and calls `wal.flush()` once at the end.
//!
//! **Date**: 2026-06-04
//! **Issue**: #3013 (P1 - TPC-H 22/22 sprint, unblocks SF=0.01 lineitem import)
//! **Strategy**: full `ExecutionEngine<WalStorage<FileStorage, FileBackedWalManager>>`
//!   stack so the path is the same as the user-facing `eng.execute("INSERT …")`.
//!
//! **Mode**: `#[ignore]` — run with `cargo test --release --test perf_eng_batched_insert_test -- --ignored --nocapture`
//!
//! **Thresholds** (release build, single-thread, debug off):
//! - 1000-row INSERT:  < 1 s
//! - 10000-row INSERT: < 10 s
//!
//! Pre-fix numbers (from issue #3013 reproduction):
//! - 1000-row INSERT:  > 60 s  (timeout in some workloads)
//! - 10000-row INSERT: > 300 s (timeout)
//!
//! Post-fix numbers (release, on this machine, 2026-06-04):
//! - 1000-row INSERT:  ~46 ms   (threshold 1 s  → ~22× headroom)
//! - 10000-row INSERT: ~3.87 s  (threshold 10 s → ~2.6× headroom)
//!
//! The 1 s / 10 s thresholds are intentionally tighter than the original
//! 5 s / 30 s to surface regressions earlier (see PR review).

use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::engine::Value;
use sqlrustgo_types::SqlResult;
use std::time::Instant;
use tempfile::TempDir;

fn make_wal_engine(
    dir: &std::path::Path,
) -> ExecutionEngine<
    sqlrustgo_storage::WalStorage<
        sqlrustgo_storage::FileStorage,
        sqlrustgo_storage::FileBackedWalManager,
    >,
> {
    ExecutionEngine::with_wal_file(dir.to_path_buf()).unwrap()
}

fn extract_count(result: SqlResult<sqlrustgo::ExecutorResult>) -> i64 {
    if let Ok(res) = result {
        if let Some(row) = res.rows.into_iter().next() {
            if let Some(v) = row.into_iter().next() {
                if let Value::Integer(n) = v {
                    return n;
                }
            }
        }
    }
    0
}

#[test]
#[ignore = "performance gate, run with --ignored --release"]
fn perf_1000_row_batched_insert_under_1s() {
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();

    let mut engine = make_wal_engine(&data_dir);

    engine
        .execute("CREATE TABLE t (id INTEGER, name TEXT, value INTEGER)")
        .expect("CREATE TABLE failed");

    // Build a single multi-row INSERT statement (the user's exact pain path).
    let mut sql = String::from("INSERT INTO t VALUES ");
    for i in 0..1000i64 {
        if i > 0 {
            sql.push(',');
        }
        sql.push_str(&format!("({}, 'name_{}', {})", i, i, i * 10));
    }

    let start = Instant::now();
    engine.execute(&sql).expect("INSERT 1000 rows failed");
    let elapsed = start.elapsed();

    let count = extract_count(engine.execute("SELECT COUNT(*) FROM t"));
    assert_eq!(count, 1000, "row count mismatch after batched INSERT");

    println!("=== 1000-row batched INSERT (issue #3013 P1 fix verification) ===");
    println!("  Elapsed:   {} ms", elapsed.as_millis());
    println!("  Threshold: < 1000 ms (release build)");
    println!("  Rows:      {}", count);
    assert!(
        elapsed.as_secs_f64() < 1.0,
        "1000-row batched INSERT took too long: {:?} (threshold 1s)",
        elapsed
    );
}

#[test]
#[ignore = "performance gate, run with --ignored --release"]
fn perf_10000_row_batched_insert_under_10s() {
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();

    let mut engine = make_wal_engine(&data_dir);

    engine
        .execute("CREATE TABLE t (id INTEGER, name TEXT, value INTEGER)")
        .expect("CREATE TABLE failed");

    // Build a single multi-row INSERT statement.
    let mut sql = String::from("INSERT INTO t VALUES ");
    for i in 0..10000i64 {
        if i > 0 {
            sql.push(',');
        }
        sql.push_str(&format!("({}, 'name_{}', {})", i, i, i * 10));
    }

    let start = Instant::now();
    engine.execute(&sql).expect("INSERT 10000 rows failed");
    let elapsed = start.elapsed();

    let count = extract_count(engine.execute("SELECT COUNT(*) FROM t"));
    assert_eq!(count, 10000, "row count mismatch after batched INSERT");

    println!("=== 10000-row batched INSERT (issue #3013 P1 fix verification) ===");
    println!("  Elapsed:   {} ms", elapsed.as_millis());
    println!("  Threshold: < 10000 ms (release build)");
    println!("  Rows:      {}", count);
    assert!(
        elapsed.as_secs_f64() < 10.0,
        "10000-row batched INSERT took too long: {:?} (threshold 10s)",
        elapsed
    );
}
