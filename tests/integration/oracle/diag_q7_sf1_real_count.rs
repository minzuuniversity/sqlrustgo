//! Diagnostic: actually run queries/q7.sql on the sqlrustgo engine against
//! SF=1 fixture and report exact row count.
//!
//! V312-58 / Issue #4376 — empirical verification of "175 vs 7" claim.
//!
//! Uses `tpch_wire_harness::start_sf01()` (the constant name is misleading —
//! it actually points at `/home/openclaw/tpch_baseline/sf1`, the SF=1 fixture,
//! because `tests/data/tpch-sf01` is a symlink).
//!
//! Run:
//!   cargo test --test diag_q7_sf1_real_count --all-features -- \
//!     --ignored --nocapture q7_sf1_real_count

#[path = "../../common/mod.rs"]
mod common;
use common::tpch_wire_harness::start_sf01;

#[test]
#[ignore] // Heavy: SF=1 lineitem is 6M rows
fn q7_sf1_real_count() {
    let data_dir = std::path::PathBuf::from("tests/data/tpch-sf01");
    if !data_dir.exists() {
        panic!("fixture missing: {}", data_dir.display());
    }
    let mut client = start_sf01();
    let sql = std::fs::read_to_string("queries/q7.sql").unwrap();
    let t0 = std::time::Instant::now();
    let rows = client
        .query_rows(&sql)
        .unwrap_or_else(|e| panic!("Q7 failed: {}", e));
    let elapsed = t0.elapsed();
    eprintln!("Q7 SF=1 returned {} rows in {:?}", rows.len(), elapsed);
    for (i, row) in rows.iter().take(15).enumerate() {
        eprintln!("  row[{:2}] = {:?}", i, row);
    }
    eprintln!("NOTE: SQLite oracle for queries/q7.sql at SF=1 = 3600 rows");
    eprintln!("NOTE: TPC-H standard Q7 at SF=1 = 4 rows");
}
