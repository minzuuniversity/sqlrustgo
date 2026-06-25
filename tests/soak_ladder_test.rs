mod common;
// In-process soak test: progressive ladder 5m -> 10m -> 20m -> 30m
// Run with: cargo test --test soak_test --release -- --ignored
// Or single step: DURATION_MIN=30 cargo test --test soak_test --release -- --ignored -- test_soak_ladder_30m

use common::tpch_wire_harness::start_sf01;
use std::time::Instant;

fn run_soak(duration_secs: u64) -> (u64, f64) {
    let mut client = start_sf01();
    let start = Instant::now();
    let mut queries = 0u64;
    while start.elapsed().as_secs() < duration_secs {
        let _ = client.query_rows("SELECT COUNT(*) FROM lineitem");
        queries += 1;
    }
    let elapsed = start.elapsed().as_secs_f64();
    let qps = queries as f64 / elapsed;
    (queries, qps)
}

#[test]
#[ignore]
fn test_soak_ladder_5m() {
    let (queries, qps) = run_soak(5 * 60);
    println!("  5m  soak: {} queries, QPS={:.1}", queries, qps);
    assert!(qps > 0.1, "QPS too low: {:.1}", qps);
}

#[test]
#[ignore]
fn test_soak_ladder_10m() {
    let (queries, qps) = run_soak(10 * 60);
    println!("  10m soak: {} queries, QPS={:.1}", queries, qps);
    assert!(qps > 0.1, "QPS too low: {:.1}", qps);
}

#[test]
#[ignore]
fn test_soak_ladder_20m() {
    let (queries, qps) = run_soak(20 * 60);
    println!("  20m soak: {} queries, QPS={:.1}", queries, qps);
    assert!(qps > 0.1, "QPS too low: {:.1}", qps);
}

#[test]
#[ignore]
fn test_soak_ladder_30m() {
    let (queries, qps) = run_soak(30 * 60);
    println!("  30m soak: {} queries, QPS={:.1}", queries, qps);
    assert!(qps > 0.1, "QPS too low: {:.1}", qps);
}
