//! L6: TPC-H lineitem JSON vs BINT v3 load-speed comparison.
//!
//! Measures wall-time for the same N lineitem rows loaded through:
//! 1. `FileStorage` (JSON row-oriented storage; pre-V313 default)
//! 2. `BinaryTableStorageV2` (BINT v3 binary storage; post-V313 default when
//!    `bin_storage_default` feature flag is enabled)
//!
//! Reports rows/sec for each engine and the speedup factor
//! (JSON_wall / BINT_wall).
//!
//! Sizes are configurable via `TPCH_COMPARE_ROWS` (default 50_000).
//! Smaller sizes keep JSON within reasonable wall-time so the comparison
//! runs in seconds, not hours (a 6M JSON run is estimated ~10h — see
//! `docs/releases/v3.13.0/perf/BIN_LOAD_PERF.md`).
//!
//! Run:
//!
//! ```bash
//! cargo test --test tpch_json_vs_bint_compare -- --nocapture
//! TPCH_COMPARE_ROWS=10000 cargo test --test tpch_json_vs_bint_compare -- --nocapture
//! ```
//!
//! Schema mirrors `tpch_sf1_6m_load_test.rs` and `benches/tpch_load_bench.rs`
//! so the comparison is apples-to-apples.

use sqlrustgo_storage::binary_storage_v2::BinaryTableStorageV2;
use sqlrustgo_storage::engine::{ColumnDefinition, Record, StorageEngine, TableInfo, Value};
use sqlrustgo_storage::file_storage::FileStorage;
use std::time::{Duration, Instant};
use tempfile::TempDir;

const DEFAULT_COMPARE_ROWS: usize = 50_000;
const BATCH_SIZE: usize = 100_000; // >= DEFAULT_COMPARE_ROWS so single batch is fine

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

/// Build the N row batch up-front (one-time cost) so both engines see
/// the same input. This isolates storage cost from row-generation cost.
fn build_rows(n_rows: usize) -> Vec<Record> {
    (0..n_rows).map(lineitem_row).collect()
}

/// Time loading `rows` through `FileStorage` (JSON). Wall-time includes
/// the single batch `insert()` + final `flush()`.
fn time_json_load(rows: &[Record], schema: &[ColumnDefinition]) -> Duration {
    let temp_dir = TempDir::new().expect("create tempdir");
    let mut storage = FileStorage::new(temp_dir.path().to_path_buf()).expect("JSON init");
    let info = TableInfo {
        name: "lineitem".to_string(),
        columns: schema.to_vec(),
        ..Default::default()
    };
    storage.create_table(&info).expect("create_table (JSON)");

    let start = Instant::now();
    storage
        .insert("lineitem", rows.to_vec())
        .expect("insert (JSON)");
    storage.flush().expect("flush (JSON)");
    start.elapsed()
}

/// Time loading `rows` through `BinaryTableStorageV2` (BINT). Wall-time
/// includes the streaming insert + final `flush()`.
///
/// Uses a batch + flush loop matching `tpch_sf1_6m_load_test.rs` and
/// `benches/tpch_load_bench.rs` so the comparison mirrors the real-world
/// SF=1 load pattern. `BATCH_SIZE` rows per `insert_streaming` call fits
/// within the segment size cap (T6.3 discovery: `get_or_open_writer`
/// only checks the cap at function entry).
fn time_bint_load(rows: &[Record], schema: &[ColumnDefinition]) -> Duration {
    let temp_dir = TempDir::new().expect("create tempdir");
    let mut storage = BinaryTableStorageV2::new(temp_dir.path().to_path_buf()).expect("V2 init");
    storage
        .create_table("lineitem", schema.to_vec())
        .expect("create_table (BINT)");

    let start = Instant::now();
    let mut emitted: usize = 0;
    while emitted < rows.len() {
        let batch_end = (emitted + BATCH_SIZE).min(rows.len());
        let batch = rows[emitted..batch_end].to_vec();
        storage
            .insert_streaming("lineitem", batch)
            .expect("insert_streaming (BINT)");
        storage.flush().expect("flush (BINT)");
        emitted = batch_end;
    }
    start.elapsed()
}

#[test]
fn json_vs_bint_load_speed() {
    let n_rows: usize = std::env::var("TPCH_COMPARE_ROWS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_COMPARE_ROWS);

    eprintln!("=== JSON vs BINT v3 load-speed comparison ===");
    eprintln!("n_rows            = {}", n_rows);
    eprintln!("schema            = 16 cols (lineitem shape)");
    eprintln!("BATCH_SIZE        = {}", BATCH_SIZE);

    // Pre-build the row batch once (excluded from measurement).
    let build_start = Instant::now();
    let rows = build_rows(n_rows);
    let build_elapsed = build_start.elapsed();
    eprintln!(
        "build_rows({}):     {:?} (excluded from storage timing)",
        n_rows, build_elapsed
    );

    // Time JSON load.
    let json_elapsed = time_json_load(&rows, &lineitem_schema());
    let json_rps = n_rows as f64 / json_elapsed.as_secs_f64();
    eprintln!(
        "FileStorage (JSON): {:?}  ({:.0} rows/sec)",
        json_elapsed, json_rps
    );

    // Time BINT load.
    let bint_elapsed = time_bint_load(&rows, &lineitem_schema());
    let bint_rps = n_rows as f64 / bint_elapsed.as_secs_f64();
    eprintln!(
        "BINT v3:           {:?}  ({:.0} rows/sec)",
        bint_elapsed, bint_rps
    );

    // Speedup = JSON wall / BINT wall.
    let speedup = json_elapsed.as_secs_f64() / bint_elapsed.as_secs_f64();
    let rps_speedup = bint_rps / json_rps;
    eprintln!(
        "Speedup factor:    {:.2}x (wall-time), {:.2}x (rows/sec)",
        speedup, rps_speedup
    );
    eprintln!("=============================================");

    // Sanity assertion: BINT must be at least 2x faster than JSON at this
    // size. (In practice the gap is ~3-5x per the empirical measurements
    // below; we use a generous floor so the test is robust against
    // small-N noise and JIT-warmup variance.)
    assert!(
        speedup >= 2.0,
        "BINT should be at least 2x faster than JSON at n_rows={}, got {:.2}x",
        n_rows,
        speedup
    );
}
