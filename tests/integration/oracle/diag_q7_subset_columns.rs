//! Diagnostic v4: TPC-H Q7 on DE/FR subset (SF~0.001) — print per-row columns.
//!
//: V312-58 / Issue #4376 — root cause investigation v4.
//! Copies the DE/FR subset from /tmp/tpch-sf01_real to a local temp dir,
//! loads it via EphemeralConfig + LOAD DATA, runs queries/q7.sql, and
//! prints each row's column count + per-column display so we can see
//! the engine output structure.
//!
//! Run:
//!   cargo test --test diag_q7_subset_columns --all-features -- --nocapture
//!
//! Expected (SQLite oracle): 7 rows × 4 cols = [GERMANY, FRANCE, year, vol]
//! Observed (engine): 2 rows × 3 cols (root-cause TBD)

#[path = "../../common/mod.rs"]
mod common;
use common::tpch_wire_harness::load_fixture;
use common::MySqlTestClient;
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};
use std::path::PathBuf;
use std::time::Instant;

#[test]
fn diag_q7_subset_columns() {
    // 1. Copy /tmp/tpch-sf01_real to a local fixture dir under target/.
    let src = PathBuf::from("/tmp/tpch-sf01_real");
    if !src.exists() {
        panic!("missing source subset fixture: {}", src.display());
    }
    let dst = PathBuf::from("target/test_fixtures/tpch-de-fr-subset");
    if dst.exists() {
        std::fs::remove_dir_all(&dst).ok();
    }
    std::fs::create_dir_all(&dst).unwrap();
    for entry in std::fs::read_dir(&src).unwrap() {
        let entry = entry.unwrap();
        let src_path = entry.path();
        if src_path.extension().and_then(|s| s.to_str()) == Some("tbl") {
            let dst_path = dst.join(src_path.file_name().unwrap());
            std::fs::copy(&src_path, &dst_path).unwrap();
        }
    }
    // Also copy expected/ if any.
    let src_expected = src.join("expected");
    if src_expected.exists() {
        let dst_expected = dst.join("expected");
        std::fs::create_dir_all(&dst_expected).ok();
        for entry in std::fs::read_dir(&src_expected).unwrap() {
            let entry = entry.unwrap();
            let dst_path = dst_expected.join(entry.file_name());
            std::fs::copy(entry.path(), &dst_path).ok();
        }
    }

    // 2. Boot engine, load fixture.
    let config = EphemeralConfig {
        data_dir: Some(dst.clone()),
        bootstrap_tables: false,
        bootstrap_users: true,
        metrics_port: None,
        ..Default::default()
    };
    let handle = start_ephemeral(config).expect("start_ephemeral");
    let mut client = MySqlTestClient::connect_handle(handle).expect("connect_handle");
    client
        .set_timeouts(
            std::time::Duration::from_secs(60),
            std::time::Duration::from_secs(60),
        )
        .expect("set_timeouts");
    load_fixture(&mut client, dst.to_str().unwrap());

    // 3. Run Q7.
    let sql = std::fs::read_to_string("queries/q7.sql").unwrap();
    let t0 = Instant::now();
    let rows = client
        .query_rows(&sql)
        .unwrap_or_else(|e| panic!("Q7 failed: {}", e));
    let elapsed = t0.elapsed();
    eprintln!(
        "Q7 (DE/FR subset) returned {} rows in {:?}",
        rows.len(),
        elapsed
    );
    for (i, row) in rows.iter().enumerate() {
        eprintln!("row[{}] ncols={}", i, row.len());
        for (j, cell) in row.iter().enumerate() {
            eprintln!("  col[{}] = {:?}", j, cell);
        }
    }
    eprintln!("SQLite oracle: 7 rows × 4 cols each [GERMANY, FRANCE, year, vol]");
}
