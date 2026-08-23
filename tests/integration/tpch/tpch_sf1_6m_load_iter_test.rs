//! L5 alternative: TPC-H SF=1 lineitem load via `insert_streaming_iter`.
//!
//! V313.2 follow-up — measures the new `insert_streaming_iter` API that
//! eliminates the batch+flush workaround required by `insert_streaming`
//! for large loads (T6.3 / T6.4 work-around pattern).
//!
//! Run (ignored by default; gated run):
//!
//! ```bash
//! cargo test --test tpch_sf1_6m_load_iter_test -- --ignored --nocapture
//! ```
//!
//! Override the budget for slower hardware:
//!
//! ```bash
//! TPCH_SF1_ITER_LOAD_BUDGET_S=180 cargo test --test tpch_sf1_6m_load_iter_test -- --ignored --nocapture
//! ```
//!
//! Notes:
//! - Synthetic 6M rows generated in-test (no fixture dependency).
//! - Same schema + same synthetic row shape as
//     `tpch_sf1_6m_load_test.rs` for direct comparability.
//! - Asserts wall-time + asserts multiple segments were created
//!   (proving rollover happened transparently).
//!
//! Anti-Pattern (do NOT close the gate on these signals):
//! 1. **禁止** 把 "测试脚本可运行" 当作 PASS — 必须看到真实 wall-time 数字
//! 2. **禁止** 把 "BATCH_SIZE 仍然存在" 当作未消除 workaround —
//!    `insert_streaming_iter` 内部完成 rollover，调用方不再需要 BATCH_SIZE
//! 3. **禁止** 用本测试下调 6M < 60s 的目标 — 该目标独立于 API 选择
//! 4. **禁止** 把 "no error" 当作正确 — 必须断言 segment 数 > 1 (说明发生过 rollover)

use sqlrustgo_storage::binary_storage_v2::BinaryTableStorageV2;
use sqlrustgo_storage::bin_index::read_root_index_file;
use sqlrustgo_storage::engine::{ColumnDefinition, Record, Value};
use std::time::{Duration, Instant};
use tempfile::TempDir;

/// TPC-H SF=1 lineitem row count per dbgen.
const SF1_LINEITEM_ROWS: usize = 6_001_215;

/// Default wall-time budget for the 6M load via iter API. Override with
/// `TPCH_SF1_ITER_LOAD_BUDGET_S`. Set higher than the batch+flush budget
/// initially since this is a new code path; tune down as data accumulates.
const DEFAULT_LOAD_BUDGET_S: u64 = 180;

/// Smoke row count used to verify the test path is wired up. NOT ignored.
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

/// Build a single synthetic lineitem row at index `i`. Same shape as the
/// batch+flush test (tpch_sf1_6m_load_test.rs) for direct comparability.
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

/// Run a streaming insert of `n_rows` synthetic lineitem rows through
/// `BinaryTableStorageV2::insert_streaming_iter` and return wall-time
/// + segment count from the root index.
///
/// The iter API handles segment rollover internally — no manual
/// batch splitting or explicit `flush()` between batches.
fn run_load(n_rows: usize) -> (Duration, usize) {
    let temp_dir = TempDir::new().expect("create tempdir");
    let mut storage = BinaryTableStorageV2::new(temp_dir.path().to_path_buf()).expect("V2 init");
    storage
        .create_table("lineitem", lineitem_schema())
        .expect("create_table");

    let start = Instant::now();
    // Generator-style iterator: (0..n_rows).map(lineitem_row)
    // The iter API consumes any IntoIterator<Item = Record>.
    let records_iter = (0..n_rows).map(lineitem_row);
    storage
        .insert_streaming_iter("lineitem", records_iter)
        .expect("insert_streaming_iter");
    storage.flush().expect("final flush");
    let elapsed = start.elapsed();

    // Read root index to count segments (proves rollover happened)
    let root_path = temp_dir.path().join("lineitem.root.bin");
    let idx = read_root_index_file(&root_path).expect("read root index");
    (elapsed, idx.segments.len())
}

/// 1k-row smoke test (NOT ignored). Verifies the iter API path is wired up.
/// Wall-time should be < 5s; assertion is generous for CI.
#[test]
fn tpch_sf1_6m_load_iter_smoke_1k() {
    let (elapsed, seg_count) = run_load(SMOKE_LINEITEM_ROWS);
    eprintln!(
        "tpch_sf1_6m_load_iter_smoke_1k: {} rows in {:?} ({} segments)",
        SMOKE_LINEITEM_ROWS, elapsed, seg_count
    );
    assert!(
        elapsed < Duration::from_secs(5),
        "1k smoke load via iter took {:?}; expected < 5s",
        elapsed
    );
    assert_eq!(seg_count, 1, "1k rows should fit in 1 segment");
}

/// 6M-row load assertion (#[ignore]d). New V313.2 L5 gate.
///
/// Wall-time budget is configurable via `TPCH_SF1_ITER_LOAD_BUDGET_S`
/// (default 180s). Initial budget is 3× the batch+flush default (60s)
/// because this is a new code path; tighten as data accumulates.
#[ignore = "V313.2 L5 gate: 6M row load via insert_streaming_iter. \
            Run with: cargo test --test tpch_sf1_6m_load_iter_test -- --ignored --nocapture. \
            Override budget via TPCH_SF1_ITER_LOAD_BUDGET_S (default 180s)."]
#[test]
fn tpch_sf1_6m_load_iter_assertion() {
    let budget_s: u64 = std::env::var("TPCH_SF1_ITER_LOAD_BUDGET_S")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_LOAD_BUDGET_S);
    let budget = Duration::from_secs(budget_s);

    eprintln!(
        "tpch_sf1_6m_load_iter_assertion: starting 6M-row iter load (budget {}s)",
        budget_s
    );
    let (elapsed, seg_count) = run_load(SF1_LINEITEM_ROWS);
    let rows_per_sec = SF1_LINEITEM_ROWS as f64 / elapsed.as_secs_f64();
    eprintln!(
        "tpch_sf1_6m_load_iter_assertion: {} rows in {:?} ({:.0} rows/sec; \
         {} segments; budget {}s)",
        SF1_LINEITEM_ROWS, elapsed, rows_per_sec, seg_count, budget_s
    );

    assert!(
        elapsed < budget,
        "TPC-H SF=1 lineitem iter load regressed: {:?} (budget {}s)",
        elapsed,
        budget_s
    );
    // Lineitem rows are ~150 bytes; 64 MB segment cap holds ~430K rows.
    // 6M rows → at least 13 segments expected (often 14-15).
    // We assert >= 10 to leave headroom for variable-length encoding.
    assert!(
        seg_count >= 10,
        "expected >= 10 segments for 6M rows (~150B each, 64 MB cap); got {}",
        seg_count
    );
}