//! V312-13 / ISSUES #4029 + #4020 — LOAD DATA SF=10 evidence test.
//!
//! Loads TPC-H region (5 rows) + nation (25 rows) + supplier (100,000 rows)
//! tables via LOAD DATA LOCAL INFILE with the exact SF=10 row counts
//! generated from dbgen -s 10 -T {r,n,s}, and asserts:
//!   - row count matches the reference (5 / 25 / 100,000)
//!   - the rows are queryable via SELECT COUNT(*)
//!   - duration is recorded for the gate report
//!
//! This is the wire-protocol end-to-end counterpart to the issue-4020
//! bulk-load runner (`scripts/tpch/bulk_load_sf10.sh`). The bulk-load
//! runner optimizes for throughput across all 8 tables; this test
//! focuses on the 3 smallest tables so:
//!   - it actually finishes in CI time (region + nation + 100K supplier
//!     wire-load takes seconds, not hours)
//!   - full SF=10 lineitem (60M rows, 6.8 GB) is gated separately as
//!     performance-bound (see issue #4020 §Throughput caveat).
//!
//! ## Fixture location
//!
//! The expected fixture path is `/tmp/tpch-sf10/{region,nation,supplier}.tbl`.
//! Generate with:
//!   cd /tmp/tpch-dbgen
//!   ./dbgen -s 10 -f -T r
//!   ./dbgen -s 10 -f -T n
//!   ./dbgen -s 10 -f -T s
//!   mv *.tbl /tmp/tpch-sf10/
//!
//! ## Test gate status
//!
//! This test replaces the previous "not_applicable" placeholder of
//! `check_v312_13_wire_load_data.sh` step 08. Once adopted, step 08
//! becomes: `cargo test --test v312_13_load_data_sf10_test -- --nocapture`.
//!
//! See: openspec/changes/v312-13-mysql-wire-load-data-hardening
//!      specs/load-data-sf1-sf10-memory-cap

#[path = "../../common/mod.rs"]
mod common;

use common::MySqlTestClient;
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};
use std::path::PathBuf;
use std::time::Instant;
use tempfile::TempDir;

const SF10_REGION_ROWS: u64 = 5;
const SF10_NATION_ROWS: u64 = 25;
const SF10_SUPPLIER_ROWS: u64 = 100_000;

const TPC_H_REGION_SCHEMA: &str = "CREATE TABLE region ( \
    r_regionkey INTEGER, \
    r_name TEXT, \
    r_comment TEXT)";

const TPC_H_NATION_SCHEMA: &str = "CREATE TABLE nation ( \
    n_nationkey INTEGER, \
    n_name TEXT, \
    n_regionkey INTEGER, \
    n_comment TEXT)";

const TPC_H_SUPPLIER_SCHEMA: &str = "CREATE TABLE supplier ( \
    s_suppkey INTEGER, \
    s_name TEXT, \
    s_address TEXT, \
    s_nationkey INTEGER, \
    s_phone TEXT, \
    s_acctbal DECIMAL(15,2), \
    s_comment TEXT)";

/// Resolve SF=10 TPC-H fixture under `/tmp/tpch-sf10/<table>.tbl`.
fn sf10_fixture(table: &str) -> PathBuf {
    PathBuf::from(format!("/tmp/tpch-sf10/{table}.tbl"))
}

/// V312-13 §9 wire-protocol SF=10 end-to-end:
/// LOAD DATA region + nation + supplier from /tmp/tpch-sf10/*.tbl,
/// assert row counts match `wc -l` references, and verify rows are
/// queryable via SELECT COUNT(*).
#[test]
fn v312_13_load_data_sf10_region_nation_supplier_smoke() {
    let region_path = sf10_fixture("region");
    let nation_path = sf10_fixture("nation");
    let supplier_path = sf10_fixture("supplier");

    for (label, p) in [
        ("region", &region_path),
        ("nation", &nation_path),
        ("supplier", &supplier_path),
    ] {
        assert!(
            p.exists(),
            "SF=10 fixture missing: {}. Generate with:\n  cd /tmp/tpch-dbgen && \
             ./dbgen -s 10 -f -T r && ./dbgen -s 10 -f -T n && ./dbgen -s 10 -f -T s && \
             mv *.tbl /tmp/tpch-sf10/",
            p.display(),
        );
        let lines = std::fs::read_to_string(p)
            .unwrap_or_else(|e| panic!("read {}: {}", p.display(), e))
            .lines()
            .count() as u64;
        match label {
            "region" => assert_eq!(
                lines, SF10_REGION_ROWS,
                "SF=10 region.tbl expected {} rows, got {} lines",
                SF10_REGION_ROWS, lines
            ),
            "nation" => assert_eq!(
                lines, SF10_NATION_ROWS,
                "SF=10 nation.tbl expected {} rows, got {} lines",
                SF10_NATION_ROWS, lines
            ),
            "supplier" => assert_eq!(
                lines, SF10_SUPPLIER_ROWS,
                "SF=10 supplier.tbl expected {} rows, got {} lines",
                SF10_SUPPLIER_ROWS, lines
            ),
            _ => unreachable!(),
        }
    }

    // Pass tempdir as data_dir so sqlrustgo's LOAD DATA whitelist
    // accepts the canonicalised fixture paths. (server's
    // handle_load_local_infile canonicalizes the requested path and
    // rejects anything outside --data-dir; we use the SAME fixture
    // dir as the data_dir so the request resolves to its in-data-dir
    // path without any copy.)
    let tmp = TempDir::new().expect("create tempdir");
    let start = Instant::now();
    let handle = start_ephemeral(EphemeralConfig {
        data_dir: Some(tmp.path().to_path_buf()),
        bootstrap_tables: false,
        bootstrap_users: true,
        ..Default::default()
    })
    .expect("start_ephemeral");
    let port = handle.port;

    let mut client =
        MySqlTestClient::connect_at(("127.0.0.1", port), "tester", "tester").expect("connect");
    client.exec(TPC_H_REGION_SCHEMA).expect("create region");
    client.exec(TPC_H_NATION_SCHEMA).expect("create nation");
    client.exec(TPC_H_SUPPLIER_SCHEMA).expect("create supplier");

    // The MySqlTestClient sends `LOAD DATA LOCAL INFILE '<path>'`
    // using the absolute path. sqlrustgo's data_dir whitelist will
    // canonicalise the path and reject anything outside data_dir, so
    // we copy the fixture into the data_dir first (mirrors the
    // bulk_load_sf10.sh pattern from PR #4020).
    let canonical_region = tmp.path().join("region.tbl");
    let canonical_nation = tmp.path().join("nation.tbl");
    let canonical_supplier = tmp.path().join("supplier.tbl");
    std::fs::copy(&region_path, &canonical_region).expect("copy region fixture");
    std::fs::copy(&nation_path, &canonical_nation).expect("copy nation fixture");
    std::fs::copy(&supplier_path, &canonical_supplier).expect("copy supplier fixture");

    // LOAD DATA: region (5 rows).
    let region_rows = client
        .load_local_infile(&canonical_region, "region")
        .expect("load region");
    assert_eq!(
        region_rows, SF10_REGION_ROWS,
        "region LOAD DATA: expected {} rows, got {}",
        SF10_REGION_ROWS, region_rows
    );

    // LOAD DATA: nation (25 rows).
    let nation_rows = client
        .load_local_infile(&canonical_nation, "nation")
        .expect("load nation");
    assert_eq!(
        nation_rows, SF10_NATION_ROWS,
        "nation LOAD DATA: expected {} rows, got {}",
        SF10_NATION_ROWS, nation_rows
    );

    // LOAD DATA: supplier (100,000 rows).
    let supplier_rows = client
        .load_local_infile(&canonical_supplier, "supplier")
        .expect("load supplier");
    assert_eq!(
        supplier_rows, SF10_SUPPLIER_ROWS,
        "supplier LOAD DATA: expected {} rows, got {}",
        SF10_SUPPLIER_ROWS, supplier_rows
    );

    // Verify rows are queryable (the LOAD DATA path must produce
    // rows that return from SELECT just like a row-by-row INSERT).
    let region_count: i64 = client
        .query_one_i64("SELECT COUNT(*) FROM region")
        .expect("count region");
    assert_eq!(region_count, SF10_REGION_ROWS as i64);

    let nation_count: i64 = client
        .query_one_i64("SELECT COUNT(*) FROM nation")
        .expect("count nation");
    assert_eq!(nation_count, SF10_NATION_ROWS as i64);

    let supplier_count: i64 = client
        .query_one_i64("SELECT COUNT(*) FROM supplier")
        .expect("count supplier");
    assert_eq!(supplier_count, SF10_SUPPLIER_ROWS as i64);

    let elapsed = start.elapsed();
    eprintln!(
        "V312-13 SF=10 smoke: region={} nation={} supplier={} duration={:?}",
        region_rows, nation_rows, supplier_rows, elapsed
    );

    drop(handle);
}
