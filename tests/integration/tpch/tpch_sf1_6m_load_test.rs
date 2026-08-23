//! L5: TPC-H SF=1 lineitem load-time gate.
//!
//! Asserts that loading 6,001,215 TPC-H lineitem rows through
//! `BinaryTableStorageV2::insert_streaming + flush` completes within
//! a configurable wall-time budget (default 60 s).
//!
//! Run (ignored by default; gated run):
//!
//! ```bash
//! cargo test --test tpch_sf1_6m_load_test -- --ignored --nocapture
//! ```
//!
//! Override the budget for slower hardware:
//!
//! ```bash
//! TPCH_SF1_LOAD_BUDGET_S=180 cargo test --test tpch_sf1_6m_load_test -- --ignored --nocapture
//! ```
//!
//! Notes:
//! - Synthetic 6M rows are generated in-test (no fixture dependency).
//! - Schema mirrors `benches/tpch_load_bench.rs` (T6.3) so the 1M
//!   criterion measurement and the 6M assertion are directly comparable.
//! - All integer / date columns use BIGINT (i64) to avoid the
//!   INTEGER-vs-BIGINT column_width trap from T6.1 / T6.2.

use sqlrustgo_storage::binary_storage_v2::BinaryTableStorageV2;
use sqlrustgo_storage::engine::{ColumnDefinition, Record, Value};
use std::time::{Duration, Instant};
use tempfile::TempDir;

/// TPC-H SF=1 lineitem row count per dbgen.
const SF1_LINEITEM_ROWS: usize = 6_001_215;

/// Default wall-time budget for the 6M load. Override with TPCH_SF1_LOAD_BUDGET_S.
const DEFAULT_LOAD_BUDGET_S: u64 = 60;

/// Batch size for `insert_streaming` calls. Smaller than the 6M total
/// so each call fits within segment-cap rollover logic (T6.3 discovery:
/// `get_or_open_writer` only checks cap at start of call, so single
/// huge inserts bypass rollover). 100_000 matches T6.3's working
/// configuration.
const BATCH_SIZE: usize = 100_000;

/// A smaller smoke row count used to verify the test path is wired up
/// without paying the 6M cost. The smoke run is NOT ignored (so a
/// reviewer can confirm the test works in <1s); the 6M run is ignored
/// because it costs 30-90s of CI time.
const SMOKE_LINEITEM_ROWS: usize = 1_000;

fn lineitem_schema() -> Vec<ColumnDefinition> {
    vec![
        ColumnDefinition::new("l_orderkey", "BIGINT"),
        ColumnDefinition::new("l_partkey", "BIGINT"),
        ColumnDefinition::new("l_suppkey", "BIGINT"),
        ColumnDefinition::new("l_linenumber", "BIGINT"), // INT in TPC-H; BIGINT avoids width trap
        ColumnDefinition::new("l_quantity", "DOUBLE PRECISION"),
        ColumnDefinition::new("l_extendedprice", "DOUBLE PRECISION"),
        ColumnDefinition::new("l_discount", "DOUBLE PRECISION"),
        ColumnDefinition::new("l_tax", "DOUBLE PRECISION"),
        ColumnDefinition::new("l_returnflag", "TEXT"),
        ColumnDefinition::new("l_linestatus", "TEXT"),
        ColumnDefinition::new("l_shipdate", "BIGINT"), // encoded as days since epoch
        ColumnDefinition::new("l_commitdate", "BIGINT"),
        ColumnDefinition::new("l_receiptdate", "BIGINT"),
        ColumnDefinition::new("l_shipinstruct", "TEXT"),
        ColumnDefinition::new("l_shipmode", "TEXT"),
        ColumnDefinition::new("l_comment", "TEXT"),
    ]
}

/// Build a single synthetic lineitem row at index `i`. Same shape as T6.3
/// for direct comparability with the criterion bench.
fn lineitem_row(i: usize) -> Record {
    vec![
        Value::Integer(i as i64),
        Value::Integer((i % 200_000) as i64),     // l_partkey
        Value::Integer((i % 10_000) as i64),      // l_suppkey
        Value::Integer(1i64),                     // l_linenumber
        Value::Float(1.0),                        // l_quantity
        Value::Float(1000.0),                     // l_extendedprice
        Value::Float(0.05),                       // l_discount
        Value::Float(0.01),                       // l_tax
        Value::Text("R".to_string()),             // l_returnflag
        Value::Text("F".to_string()),             // l_linestatus
        Value::Integer(9000 + (i % 1000) as i64), // l_shipdate
        Value::Integer(9100 + (i % 1000) as i64), // l_commitdate
        Value::Integer(9150 + (i % 1000) as i64), // l_receiptdate
        Value::Text("DELIVER IN PERSON".to_string()),
        Value::Text("TRUCK".to_string()),
        Value::Text("comment".to_string()),
    ]
}

/// Run a streaming insert of `n_rows` synthetic lineitem rows through
/// `BinaryTableStorageV2` and return the wall-time. The `BinaryTableStorageV2`
/// instance is dropped at end of scope so its mmap is released.
///
/// Uses a batch + flush loop (BATCH_SIZE rows per `insert_streaming`
/// call) so segment rollover works correctly. See T6.3 discovery in
/// the brief.
fn run_load(n_rows: usize) -> Duration {
    let temp_dir = TempDir::new().expect("create tempdir");
    let mut storage = BinaryTableStorageV2::new(temp_dir.path().to_path_buf()).expect("V2 init");
    storage
        .create_table("lineitem", lineitem_schema())
        .expect("create_table");

    let start = Instant::now();
    let mut emitted: usize = 0;
    while emitted < n_rows {
        let batch_end = (emitted + BATCH_SIZE).min(n_rows);
        let batch: Vec<Record> = (emitted..batch_end).map(lineitem_row).collect();
        storage
            .insert_streaming("lineitem", batch)
            .expect("insert_streaming");
        // Force segment rollover: seal current writer + open fresh one
        // on the next batch.
        storage.flush().expect("flush");
        emitted = batch_end;
    }
    start.elapsed()
}

/// 1k-row smoke test (NOT ignored). Verifies the test path is wired up.
/// Wall-time should be < 1s; assertion is generous (5s) to allow for
/// CI machines.
#[test]
fn tpch_sf1_6m_load_smoke_1k() {
    let elapsed = run_load(SMOKE_LINEITEM_ROWS);
    eprintln!(
        "tpch_sf1_6m_load_smoke_1k: {} rows in {:?}",
        SMOKE_LINEITEM_ROWS, elapsed
    );
    assert!(
        elapsed < Duration::from_secs(5),
        "1k smoke load took {:?}; expected < 5s (likely a regression in the test wiring)",
        elapsed
    );
}

/// 6M-row load assertion (#[ignore]d). The plan's L5 gate.
///
/// Wall-time budget is configurable via `TPCH_SF1_LOAD_BUDGET_S`
/// (default 60s). Operators on slower hardware should set this higher.
#[ignore = "T6.4 L5 gate: 6M row load via BinaryTableStorageV2. \
            Run with: cargo test --test tpch_sf1_6m_load_test -- --ignored --nocapture. \
            Override budget via TPCH_SF1_LOAD_BUDGET_S (default 60s)."]
#[test]
fn tpch_sf1_6m_load_assertion() {
    let budget_s: u64 = std::env::var("TPCH_SF1_LOAD_BUDGET_S")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_LOAD_BUDGET_S);
    let budget = Duration::from_secs(budget_s);

    eprintln!(
        "tpch_sf1_6m_load_assertion: starting 6M-row load (budget {}s)",
        budget_s
    );
    let elapsed = run_load(SF1_LINEITEM_ROWS);
    let rows_per_sec = SF1_LINEITEM_ROWS as f64 / elapsed.as_secs_f64();
    eprintln!(
        "tpch_sf1_6m_load_assertion: {} rows in {:?} ({:.0} rows/sec; budget {}s)",
        SF1_LINEITEM_ROWS, elapsed, rows_per_sec, budget_s
    );

    assert!(
        elapsed < budget,
        "TPC-H SF=1 lineitem load regressed: {:?} (budget {}s, target < {}s)",
        elapsed,
        budget_s,
        budget_s
    );
}
