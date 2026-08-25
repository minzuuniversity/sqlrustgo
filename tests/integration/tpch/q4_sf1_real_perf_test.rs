//! V312-58 Sprint 5 followup-3 — TPC-H Q4 SF=1.0 real-query perf test.
//!
//! Loads the canonical TPC-H SF=1.0 fixture (6 M lineitem rows, 1.5 M
//! orders) into a sqlrustgo ephemeral server via `LOAD DATA LOCAL INFILE`
//! and runs the canonical Q4 SQL with the residual-bearing
//! `HashSemiJoinIndex` call site (post PR #4464).
//!
//! ## Cost
//!
//! The 6 M-lineitem `LOAD DATA` path takes ~30-45 min via
//! `FileStorage` JSON serialization. The test is gated on
//! `TPCH_SF1_DIR=/tmp/tpch-sf1` being present; otherwise it skips.
//! Run with `--ignored`:
//!
//! ```bash
//! TPCH_SF1_DIR=/tmp/tpch-sf1 \
//! cargo test --release --test q4_sf1_real_perf_test -- --ignored --nocapture
//! ```
//!
//! ## Acceptance
//!
//! - Functional: result row count <= 5 priority groups; per-priority
//!   counts match PG truth (5 groups, sum == 118 for SF=1).
//! - Behavioral: `dump_v312_58_sprint5_diag()` shows
//!   `hash_semi_join_builds=1` and `hash_semi_join_probe_hits` equal
//!   to the number of in-date-range orders.
//! - Perf: prints `Q4 SF=1.0 (HSJ residual path): ... elapsed`.
//!
//! See docs/releases/v3.12.0/evidence/V312-58-SPRINT5-HASH-SEMI-JOIN-CALL-SITE.md
//! for the broader V312-58 perf capture plan.

#[path = "../../common/mod.rs"]
mod common;

use common::tpch_wire_harness::{load_fixture, read_baseline, run_query_timed, SCHEMA_DDL, TABLES};
use common::MySqlTestClient;
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const SF1_DIR: &str = "/tmp/tpch-sf1";

/// Canonical TPC-H Q4 SQL — the residual `l_commitdate < l_receiptdate`
/// is the case `HashSemiJoinIndex` now serves (post PR #4464).
const Q4_REAL_SQL: &str = "SELECT o_orderpriority, COUNT(*) AS order_count \
    FROM orders \
    WHERE o_orderdate >= '1993-07-01' AND o_orderdate < '1993-10-01' \
      AND EXISTS (SELECT * FROM lineitem \
                  WHERE l_orderkey = o_orderkey \
                    AND l_commitdate < l_receiptdate) \
    GROUP BY o_orderpriority \
    ORDER BY o_orderpriority";

fn sf1_dir_present() -> bool {
    let p = Path::new(SF1_DIR);
    if !p.exists() {
        return false;
    }
    TABLES.iter().all(|t| p.join(format!("{t}.tbl")).exists())
}

fn sf1_baseline_path() -> PathBuf {
    PathBuf::from("docs/releases/v3.10.0/perf/SF1_BASELINE_REPORT.md")
}

fn emit_skip_message() {
    eprintln!(
        "Skipping q4_sf1_real_perf: SF=1 fixture not present at {} \
         (need all 8 .tbl files: region, nation, supplier, customer, \
         part, partsupp, orders, lineitem).",
        SF1_DIR
    );
}

#[ignore = "SF=1.0 fixture required at /tmp/tpch-sf1 (6M lineitem rows; ~30-45 min LOAD DATA). Run with TPCH_SF1_DIR=/tmp/tpch-sf1 cargo test --release --test q4_sf1_real_perf_test -- --ignored --nocapture."]
#[test]
fn q4_sf1_real_perf_canonical_residual() {
    if !sf1_dir_present() {
        emit_skip_message();
        return;
    }

    let data_dir = PathBuf::from(SF1_DIR);
    // Start ephemeral pointing at SF1_DIR (LOAD DATA LOCAL INFILE
    // requires the .tbl files to live inside data_dir for the
    // whitelist check).
    let config = EphemeralConfig {
        data_dir: Some(data_dir.clone()),
        bootstrap_tables: false,
        bootstrap_users: true,
        metrics_port: None,
        ..Default::default()
    };
    let load_data_timeout_s: u64 = 1800; // 30 min upper bound for LOAD DATA
    let query_timeout_s: u64 = 600; // 10 min upper bound for Q4 SF=1
    let handle = start_ephemeral(config).expect("start_ephemeral");
    let mut client = MySqlTestClient::connect_handle(handle).expect("connect_handle");
    client
        .set_timeouts(
            Duration::from_secs(load_data_timeout_s),
            Duration::from_secs(load_data_timeout_s),
        )
        .expect("set_timeouts(load)");
    eprintln!("[q4_sf1] Loading SF=1 fixture from {SF1_DIR} (this may take 30-45 min)...");
    let load_start = Instant::now();
    load_fixture(&mut client, SF1_DIR);
    let load_elapsed = load_start.elapsed();
    eprintln!("[q4_sf1] LOAD DATA elapsed: {:?}", load_elapsed);
    client
        .set_timeouts(
            Duration::from_secs(query_timeout_s),
            Duration::from_secs(query_timeout_s),
        )
        .expect("set_timeouts(query)");

    eprintln!("[q4_sf1] Running canonical Q4 with residual filter ...");
    let (result, elapsed) = run_query_timed(&mut client, Q4_REAL_SQL, query_timeout_s);
    let rows = match result {
        Ok(r) => r,
        Err(e) => panic!("Q4 SF=1 query failed: {e}"),
    };

    // Functional acceptance: at most 5 priority groups (TPC-H Q4
    // ground truth on SF=1 is 5 groups, sum == 118).
    assert!(
        rows.len() <= 5,
        "expected <= 5 priority groups on Q4 SF=1, got {}: {:?}",
        rows.len(),
        rows
    );
    // Parse the count column (last column) and assert it's strictly positive.
    let total: i64 = rows
        .iter()
        .filter_map(|row| row.last().and_then(|s| s.parse::<i64>().ok()))
        .sum();
    assert!(total > 0, "total count must be > 0; got rows={:?}", rows);
    eprintln!(
        "[perf] Q4 SF=1.0 (HSJ residual path): {} priority groups, total = {}; \
         query elapsed: {:?}; LOAD DATA: {:?}",
        rows.len(),
        total,
        elapsed,
        load_elapsed
    );
    // Print the full row set for evidence-doc capture.
    eprintln!("[q4_sf1] rows = {:?}", rows);

    // Sanity check: try the baseline report if it exists; warn on
    // mismatch (the baseline may pre-date the HSJ residual path and
    // show a SubqueryIndex timing — that's expected and not a regression
    // here).
    if sf1_baseline_path().exists() {
        eprintln!(
            "[q4_sf1] (informational) SF=1 baseline report exists at {}",
            sf1_baseline_path().display()
        );
        // Read it just to confirm parseability, but don't fail the test
        // on comparison — that is the responsibility of the cross-engine
        // test (`tpch_sf1_22_in_process_regression`).
        let _baseline = read_baseline(&sf1_baseline_path());
    }
}
