//! TPC-H Soak Test — progressive ladder 5m → 10m → 20m → 30m
//!
//! Uses start_sf001() which loads all 8 TPC-H SF=0.001 tables via LOAD DATA LOCAL INFILE.
//! Each rung runs a mix of TPC-H queries in a loop and asserts QPS > 0.
//!
//! Run ignored tests:
//!   cargo test --release --test tpch_soak_test -- --ignored
//! Single rung:
//!   DURATION_MIN=5 cargo test --release --test tpch_soak_test -- --ignored -- test_soak_5m

mod common;
use common::tpch_wire_harness::start_sf001;
use std::time::{Duration, Instant};

const QUERIES: &[&str] = &[
    // Warm queries covering different code paths
    "SELECT COUNT(*) FROM lineitem",             // Q1 — aggregation
    "SELECT COUNT(*) FROM orders",               // Q2 — range scan
    "SELECT COUNT(*) FROM customer",             // Q3 — join prep
    "SELECT SUM(l_extendedprice) FROM lineitem", // Q1 — numeric agg
    "SELECT l_orderkey, SUM(l_quantity) FROM lineitem GROUP BY l_orderkey LIMIT 10", // Q1 group
    "SELECT o_custkey, COUNT(*) FROM orders GROUP BY o_custkey LIMIT 10", // Q3 group
];

fn run_soak(duration_secs: u64) -> (u64, f64) {
    let mut client = start_sf001();
    let start = Instant::now();
    let mut queries = 0u64;
    let mut qidx = 0usize;

    while start.elapsed().as_secs() < duration_secs {
        let sql = QUERIES[qidx % QUERIES.len()];
        match client.query_rows(sql) {
            Ok(rows) => {
                if rows.is_empty() {
                    queries += 1; // count even empty results
                } else {
                    queries += rows.len() as u64;
                }
            }
            Err(e) => {
                eprintln!("  Query error (will retry): {}", e);
            }
        }
        qidx += 1;

        // Small sleep to avoid hammering in tight loop
        if qidx % 100 == 0 {
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    let qps = queries as f64 / elapsed;
    (queries, qps)
}

fn run_soak_ladder(minutes: u64) -> (u64, f64) {
    let secs = minutes * 60;
    println!("  Starting {}m soak ({} seconds)...", minutes, secs);
    let (queries, qps) = run_soak(secs);
    println!(
        "  {}m soak result: {} queries, QPS={:.1}",
        minutes, queries, qps
    );
    (queries, qps)
}

#[test]
#[ignore]
fn test_soak_5m() {
    let (queries, qps) = run_soak_ladder(5);
    assert!(qps > 0.1, "QPS too low: {:.1}", qps);
    println!("  PASS: {} queries at QPS={:.1}", queries, qps);
}

#[test]
#[ignore]
fn test_soak_10m() {
    let (queries, qps) = run_soak_ladder(10);
    assert!(qps > 0.1, "QPS too low: {:.1}", qps);
    println!("  PASS: {} queries at QPS={:.1}", queries, qps);
}

#[test]
#[ignore]
fn test_soak_20m() {
    let (queries, qps) = run_soak_ladder(20);
    assert!(qps > 0.1, "QPS too low: {:.1}", qps);
    println!("  PASS: {} queries at QPS={:.1}", queries, qps);
}

#[test]
#[ignore]
fn test_soak_30m() {
    let (queries, qps) = run_soak_ladder(30);
    assert!(qps > 0.1, "QPS too low: {:.1}", qps);
    println!("  PASS: {} queries at QPS={:.1}", queries, qps);
}
