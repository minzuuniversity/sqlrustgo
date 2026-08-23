//! Profiling harness: prove that FileStorage::insert exhibits
//! O(N²) write amplification on bulk loads, then measure the speedup
//! from raising `buffer_threshold`.
//!
//! Issue #4020 TPC-H SF=10 evidence: LOAD DATA LOCAL INFILE on
//! `supplier` sustained only ~111 rows/s, while `region` and `nation`
//! (a handful of rows) finished in under a second. Code reading
//! localized the cost to:
//!
//!   FileStorage::insert_buffered
//!     -> flush_buffer (every `buffer_threshold` = 100 rows)
//!       -> insert_direct (full `data.clone()` + `save_table`)
//!         -> serde_json::to_string_pretty over the WHOLE table.
//!
//! If true, total work to insert N rows is ~Σ_{k=1..N/100} k * rowsize,
//! i.e. O(N²). This test prints a per-batch timing table for several
//! `buffer_threshold` settings so we can compare:
//!   threshold=100  (current default)  — should exhibit O(N²)
//!   threshold=10000                   — should be near-linear
//!
//! Run with:
//!   cargo test -p sqlrustgo-storage --test bulk_load_quadraticity -- --nocapture

use std::time::Instant;
use tempfile::tempdir;

use sqlrustgo_storage::{ColumnDefinition, FileStorage, StorageEngine, TableInfo};
use sqlrustgo_types::Value;

fn make_supplier_info() -> TableInfo {
    TableInfo {
        name: "supplier".to_string(),
        columns: vec![
            ColumnDefinition {
                name: "s_suppkey".into(),
                data_type: "BIGINT".into(),
                nullable: false,
                auto_increment: false,
                ..Default::default()
            },
            ColumnDefinition {
                name: "s_name".into(),
                data_type: "TEXT".into(),
                nullable: true,
                auto_increment: false,
                ..Default::default()
            },
            ColumnDefinition {
                name: "s_address".into(),
                data_type: "TEXT".into(),
                nullable: true,
                auto_increment: false,
                ..Default::default()
            },
            ColumnDefinition {
                name: "s_nationkey".into(),
                data_type: "INT".into(),
                nullable: true,
                auto_increment: false,
                ..Default::default()
            },
            ColumnDefinition {
                name: "s_phone".into(),
                data_type: "TEXT".into(),
                nullable: true,
                auto_increment: false,
                ..Default::default()
            },
            ColumnDefinition {
                name: "s_acctbal".into(),
                data_type: "DECIMAL".into(),
                nullable: true,
                auto_increment: false,
                ..Default::default()
            },
            ColumnDefinition {
                name: "s_comment".into(),
                data_type: "TEXT".into(),
                nullable: true,
                auto_increment: false,
                ..Default::default()
            },
        ],
        ..Default::default()
    }
}

fn row(id: i64) -> Vec<Value> {
    vec![
        Value::Integer(id),
        Value::Text(format!("Supplier#{id:010}")),
        Value::Text("X-".repeat(40)),
        Value::Integer((id % 25) as i64),
        Value::Text(format!("{:0>15}", id)),
        Value::Float((id as f64) * 0.01),
        Value::Text("comment ".repeat(20)),
    ]
}

fn run_with_threshold(dir: &tempfile::TempDir, threshold: usize, total_rows: usize) -> (f64, f64) {
    let mut storage =
        FileStorage::new_with_buffer_config(dir.path().to_path_buf(), threshold, true)
            .expect("FileStorage::new_with_buffer_config");

    storage
        .create_table(&make_supplier_info())
        .expect("create_table");

    eprintln!(
        "\n=== threshold={} total_rows={} ===",
        threshold, total_rows
    );
    eprintln!("rows_so_far,batch_us,since_start_us,json_bytes");

    let t_total_start = Instant::now();
    let batch = threshold;
    for batch_idx in 0..(total_rows / batch) {
        let rows: Vec<Vec<Value>> = (0..batch)
            .map(|j| row((batch_idx * batch + j) as i64))
            .collect();

        let t0 = Instant::now();
        storage.insert("supplier", rows).expect("insert");
        let dt = t0.elapsed();

        let rows_so_far = (batch_idx + 1) * batch;
        let json_bytes = std::fs::metadata(dir.path().join("supplier.json"))
            .map(|m| m.len())
            .unwrap_or(0);

        eprintln!(
            "{},{},{},{}",
            rows_so_far,
            dt.as_micros(),
            t_total_start.elapsed().as_micros(),
            json_bytes
        );
    }

    storage.flush().expect("flush");

    let total_us = t_total_start.elapsed().as_micros();
    let total_s = total_us as f64 / 1_000_000.0;
    let rows_per_s = total_rows as f64 / total_s;
    let last_json_bytes = std::fs::metadata(dir.path().join("supplier.json"))
        .map(|m| m.len())
        .unwrap_or(0);

    eprintln!(
        "TOTAL: {} rows in {:.2}s ({:.0} rows/s), final json={} bytes",
        total_rows, total_s, rows_per_s, last_json_bytes
    );
    (total_s, rows_per_s)
}

#[test]
fn file_storage_insert_grows_quadratically() {
    let dir = tempdir().expect("tempdir");
    // Current production default: 100 rows. N=30k ⇒ 300 flushes.
    let (s_low, rps_low) = run_with_threshold(&dir, 100, 30_000);

    let dir2 = tempdir().expect("tempdir2");
    // Hypothesis: raise threshold to 10 000. N=30k ⇒ 3 flushes.
    let (s_high, rps_high) = run_with_threshold(&dir2, 10_000, 30_000);

    eprintln!(
        "\nSPEEDUP: threshold=100 -> {:.0} rows/s ; threshold=10000 -> {:.0} rows/s ; {:.1}x faster",
        rps_low, rps_high, rps_high / rps_low
    );

    assert!(s_high <= s_low, "higher threshold must not be slower");
    let _ = (s_low, s_high); // suppress unused if test is skipped
}
