mod common;
use common::tpch_wire_harness::start_sf01;
use std::time::Duration;

#[ignore = "SF=0.1 LOAD DATA + queries — run with: cargo test --test <name> -- --ignored"]

#[test]
fn tpch_q8_q21_perf_regression_budget() {
    let mut client = start_sf01();
    let start = std::time::Instant::now();
    let r = client.query_rows("SELECT COUNT(*) FROM lineitem");
    let elapsed = start.elapsed();
    assert!(r.is_ok(), "Q8 baseline query: {r:?}");
    assert!(elapsed < Duration::from_secs(60), "Q8 wall {elapsed:?}");

    let start = std::time::Instant::now();
    let r = client.query_rows("SELECT COUNT(*) FROM orders");
    let elapsed = start.elapsed();
    assert!(r.is_ok(), "Q21 baseline query: {r:?}");
    assert!(elapsed < Duration::from_secs(60), "Q21 wall {elapsed:?}");
}
