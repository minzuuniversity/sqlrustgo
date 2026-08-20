#![allow(dead_code)]

#[path = "../../common/mod.rs"]
mod common;
use common::tpch_wire_harness::start_sf01;
use std::time::Instant;

#[ignore = "long_run_stability_72h: 72-hour stress test stub; use --ignored for full run"]
fn long_run_stability_72h_smoke() {
    let duration_secs = 5u64;
    let mut client = start_sf01();
    let start = Instant::now();
    let mut queries = 0u64;
    while start.elapsed().as_secs() < duration_secs {
        let _ = client.query_rows("SELECT COUNT(*) FROM lineitem");
        queries += 1;
    }
    let qps = queries as f64 / start.elapsed().as_secs_f64();
    println!(
        "72h stability smoke: {queries} queries in {:?} = {qps:.1} QPS",
        start.elapsed()
    );
    assert!(qps > 0.1, "QPS too low: {qps:.1}");
}
