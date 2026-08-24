//! V313.3 controlled-variable experiments for the 6M TPC-H SF=1 lineitem load.
//!
//! Goal: identify the dominant cost driver in the 6.3x extrapolation gap
//! (1M -> 17s predicted; actual 6M = 112.9s via insert_streaming_iter).
//!
//! Run all experiments (ignored by default; takes ~20 min total for 4 expts x 3 iter):
//!
//!   cargo test --release --test v313_3_profile_experiments \
//!     --features v313_3_profile -- --ignored --nocapture --test-threads=1
//!
//! Run a single experiment by name:
//!
//!   cargo test --release --test v313_3_profile_experiments \
//!     --features v313_3_profile -- --ignored --nocapture experiment_a_skip_rows_accumulator
//!
//! Smoke tests (NOT ignored, fast):
//!
//!   cargo test --test v313_3_profile_experiments --features v313_3_profile

#![cfg(feature = "v313_3_profile")]

use sqlrustgo_storage::binary_storage_v2::BinaryTableStorageV2;
use sqlrustgo_storage::bin_index::read_root_index_file;
use sqlrustgo_storage::engine::{ColumnDefinition, Record, Value};
use std::time::{Duration, Instant};
use tempfile::TempDir;

/// TPC-H SF=1 lineitem row count per dbgen.
const SF1_LINEITEM_ROWS: usize = 6_001_215;

/// Number of median iterations per 6M experiment.
const N_ITER: usize = 3;

/// 1k smoke row count.
const SMOKE_LINEITEM_ROWS: usize = 1_000;

fn lineitem_schema() -> Vec<ColumnDefinition> {
    vec![
        ColumnDefinition::new("l_orderkey", "BIGINT"),
        ColumnDefinition::new("l_partkey", "BIGINT"),
        ColumnDefinition::new("l_suppkey", "BIGINT"),
        ColumnDefinition::new("l_linenumber", "BIGINT"),
        ColumnDefinition::new("l_quantity", "DOUBLE PRECISION"),
        ColumnDefinition::new("l_extendedprice", "DOUBLE PRECISION"),
        ColumnDefinition::new("l_discount", "DOUBLE PRECISION"),
        ColumnDefinition::new("l_tax", "DOUBLE PRECISION"),
        ColumnDefinition::new("l_returnflag", "TEXT"),
        ColumnDefinition::new("l_linestatus", "TEXT"),
        ColumnDefinition::new("l_shipdate", "BIGINT"),
        ColumnDefinition::new("l_commitdate", "BIGINT"),
        ColumnDefinition::new("l_receiptdate", "BIGINT"),
        ColumnDefinition::new("l_shipinstruct", "TEXT"),
        ColumnDefinition::new("l_shipmode", "TEXT"),
        ColumnDefinition::new("l_comment", "TEXT"),
    ]
}

/// Same shape as `tpch_sf1_6m_load_iter_test::lineitem_row` (cross-test comparability).
fn lineitem_row(i: usize) -> Record {
    vec![
        Value::Integer(i as i64),
        Value::Integer((i % 200_000) as i64),
        Value::Integer((i % 10_000) as i64),
        Value::Integer(1i64),
        Value::Float(1.0),
        Value::Float(1000.0),
        Value::Float(0.05),
        Value::Float(0.01),
        Value::Text("R".to_string()),
        Value::Text("F".to_string()),
        Value::Integer(9000 + (i % 1000) as i64),
        Value::Integer(9100 + (i % 1000) as i64),
        Value::Integer(9150 + (i % 1000) as i64),
        Value::Text("DELIVER IN PERSON".to_string()),
        Value::Text("TRUCK".to_string()),
        Value::Text("comment".to_string()),
    ]
}

/// Run a single load of `n_rows` rows through the iter API and return
/// (wall-time, segment-count, row-count).
fn run_load(n_rows: usize) -> (Duration, usize, u64) {
    let temp_dir = TempDir::new().expect("tempdir");
    let mut storage = BinaryTableStorageV2::new(temp_dir.path().to_path_buf()).expect("V2 init");
    storage.create_table("lineitem", lineitem_schema()).expect("create_table");
    let start = Instant::now();
    storage
        .insert_streaming_iter("lineitem", (0..n_rows).map(lineitem_row))
        .expect("insert_streaming_iter");
    storage.flush().expect("final flush");
    let elapsed = start.elapsed();
    let root_path = temp_dir.path().join("lineitem.root.bin");
    let idx = read_root_index_file(&root_path).expect("read root index");
    (elapsed, idx.segments.len(), idx.total_rows)
}

/// Compute the median of `n` repeated runs. Returns median Duration.
fn median_of(n: usize, mut f: impl FnMut() -> Duration) -> Duration {
    let mut samples: Vec<Duration> = (0..n).map(|_| f()).collect();
    samples.sort();
    samples[samples.len() / 2]
}

// ============================================================
// Baseline re-measurement
// ============================================================

/// Re-measure the baseline (no hooks set) to confirm V313.2 reference is
/// reproducible on current HEAD. Expected ~112s; report actual value.
#[ignore = "6M load; run with --ignored. Captured for PROFILE_RESULTS.md baseline."]
#[test]
fn baseline_6m_no_hooks() {
    let elapsed = median_of(N_ITER, || run_load(SF1_LINEITEM_ROWS).0);
    let rows_per_sec = SF1_LINEITEM_ROWS as f64 / elapsed.as_secs_f64();
    eprintln!(
        "BASELINE 6M (no hooks): {:?} ({:.0} rows/sec)",
        elapsed, rows_per_sec
    );
    // Do NOT assert a specific budget — this is reference measurement.
    // Just print the value for the human to paste into PROFILE_RESULTS.md.
}

// ============================================================
// Smoke tests (NOT ignored) — verify the test harness compiles + runs
// ============================================================

#[test]
fn baseline_smoke_1k() {
    let (elapsed, seg_count, row_count) = run_load(SMOKE_LINEITEM_ROWS);
    eprintln!(
        "BASELINE 1k: {:?} ({} segments, {} rows)",
        elapsed, seg_count, row_count
    );
    assert!(elapsed < Duration::from_secs(5));
    assert_eq!(row_count, SMOKE_LINEITEM_ROWS as u64);
    assert_eq!(seg_count, 1);
}

// ============================================================
// Experiment A — skip tables.rows.push accumulator
// ============================================================

/// Run a load with the Experiment A hook set (skip in-memory rows).
///
/// Returns (wall-time, segment-count, row-count, in-mem rows after run).
fn run_load_experiment_a(n_rows: usize) -> (Duration, usize, u64, usize) {
    let temp_dir = TempDir::new().expect("tempdir");
    let mut storage = BinaryTableStorageV2::new(temp_dir.path().to_path_buf()).expect("V2 init");
    storage.create_table("lineitem", lineitem_schema()).expect("create_table");
    storage.skip_in_memory_rows_for_test();
    let start = Instant::now();
    storage
        .insert_streaming_iter("lineitem", (0..n_rows).map(lineitem_row))
        .expect("insert_streaming_iter");
    storage.flush().expect("final flush");
    let elapsed = start.elapsed();
    let root_path = temp_dir.path().join("lineitem.root.bin");
    let idx = read_root_index_file(&root_path).expect("read root index");
    let in_mem = storage.len_in_memory_rows_for_test("lineitem");
    (elapsed, idx.segments.len(), idx.total_rows, in_mem)
}

#[ignore = "6M load with Experiment A hook. Run with --ignored."]
#[test]
fn experiment_a_skip_rows_accumulator() {
    let elapsed = median_of(N_ITER, || run_load_experiment_a(SF1_LINEITEM_ROWS).0);
    let rows_per_sec = SF1_LINEITEM_ROWS as f64 / elapsed.as_secs_f64();
    eprintln!(
        "EXPERIMENT A (no tables.rows.push): 6M rows in {:?} ({:.0} rows/sec)",
        elapsed, rows_per_sec
    );
}

#[test]
fn experiment_a_smoke_1k() {
    let (elapsed, seg_count, row_count, in_mem) = run_load_experiment_a(SMOKE_LINEITEM_ROWS);
    eprintln!(
        "EXPERIMENT A 1k: {:?} ({} seg, {} rows, {} in-memory)",
        elapsed, seg_count, row_count, in_mem
    );
    assert!(elapsed < Duration::from_secs(5));
    assert_eq!(row_count, SMOKE_LINEITEM_ROWS as u64);
    assert_eq!(in_mem, 0, "hook failed: tables.rows should be empty");
}

// Experiment tests (B/C/D) are added in Task 3b-3d.
