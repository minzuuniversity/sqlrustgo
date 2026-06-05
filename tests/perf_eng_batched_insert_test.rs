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
//! **Strategy**: drive the batched INSERT through the canonical
//!   `start_ephemeral` MySQL server. Internally the server uses the same
//!   `ExecutionEngine<WalStorage<FileStorage, FileBackedWalManager>>` stack,
//!   so this exercises the exact user-facing insert path plus the wire
//!   protocol round-trip overhead. Pre-fix O(N²) manifests as a wire-level
//!   timeout; post-fix the same path lands within the thresholds below.
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

mod common;

use common::MySqlTestClient;
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};
use std::time::Instant;
use tempfile::TempDir;

fn open_client(data_dir: &std::path::Path) -> MySqlTestClient {
    let cfg = EphemeralConfig {
        host: "127.0.0.1".to_string(),
        bootstrap_tables: false,
        bootstrap_users: true,
        data_dir: Some(data_dir.to_path_buf()),
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
    };
    let handle = start_ephemeral(cfg).expect("start_ephemeral");
    MySqlTestClient::connect_handle(handle).expect("MySqlTestClient::connect_handle")
}

#[test]
#[ignore = "performance gate, run with --ignored --release"]
fn perf_1000_row_batched_insert_under_1s() {
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();
    let mut client = open_client(&data_dir);

    client
        .exec("CREATE TABLE t (id INTEGER, name TEXT, value INTEGER)")
        .expect("CREATE TABLE failed");

    let mut sql = String::from("INSERT INTO t VALUES ");
    for i in 0..1000i64 {
        if i > 0 {
            sql.push(',');
        }
        sql.push_str(&format!("({}, 'name_{}', {})", i, i, i * 10));
    }

    let start = Instant::now();
    client.exec(&sql).expect("INSERT 1000 rows failed");
    let elapsed = start.elapsed();

    let count = client
        .query_one_i64("SELECT COUNT(*) FROM t")
        .expect("COUNT");
    assert_eq!(count, 1000, "row count mismatch after batched INSERT");

    println!("=== 1000-row batched INSERT (issue #3013 P1 fix verification, wire) ===");
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
    let mut client = open_client(&data_dir);

    client
        .exec("CREATE TABLE t (id INTEGER, name TEXT, value INTEGER)")
        .expect("CREATE TABLE failed");

    let mut sql = String::from("INSERT INTO t VALUES ");
    for i in 0..10000i64 {
        if i > 0 {
            sql.push(',');
        }
        sql.push_str(&format!("({}, 'name_{}', {})", i, i, i * 10));
    }

    let start = Instant::now();
    client.exec(&sql).expect("INSERT 10000 rows failed");
    let elapsed = start.elapsed();

    let count = client
        .query_one_i64("SELECT COUNT(*) FROM t")
        .expect("COUNT");
    assert_eq!(count, 10000, "row count mismatch after batched INSERT");

    println!("=== 10000-row batched INSERT (issue #3013 P1 fix verification, wire) ===");
    println!("  Elapsed:   {} ms", elapsed.as_millis());
    println!("  Threshold: < 10000 ms (release build)");
    println!("  Rows:      {}", count);
    assert!(
        elapsed.as_secs_f64() < 10.0,
        "10000-row batched INSERT took too long: {:?} (threshold 10s)",
        elapsed
    );
}
