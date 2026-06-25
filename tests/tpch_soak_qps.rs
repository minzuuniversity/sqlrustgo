//! Quick QPS benchmark — runs 6 TPC-H queries in a loop for 30s to measure throughput.
//! Run: cargo test --release --test tpch_soak_qps -- --nocapture

mod common;
use common::tpch_wire_harness::start_sf001;
use std::time::{Duration, Instant};

const QUERIES: &[&str] = &[
    "SELECT COUNT(*) FROM lineitem",
    "SELECT COUNT(*) FROM orders",
    "SELECT COUNT(*) FROM customer",
    "SELECT SUM(l_extendedprice) FROM lineitem",
    "SELECT l_orderkey, SUM(l_quantity) FROM lineitem GROUP BY l_orderkey LIMIT 10",
    "SELECT o_custkey, COUNT(*) FROM orders GROUP BY o_custkey LIMIT 10",
];

#[test]
fn test_tpch_qps_30s() {
    let mut client = start_sf001();
    let duration_secs = 30u64;
    let start = Instant::now();
    let mut queries = 0u64;
    let mut qidx = 0usize;

    println!("Starting 30s QPS benchmark...");

    while start.elapsed().as_secs() < duration_secs {
        let sql = QUERIES[qidx % QUERIES.len()];
        match client.query_rows(sql) {
            Ok(rows) => {
                queries += rows.len() as u64;
            }
            Err(e) => {
                eprintln!("Query error (will retry): {}", e);
            }
        }
        qidx += 1;
        if qidx % 100 == 0 {
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    let qps = queries as f64 / elapsed;
    println!("30s QPS benchmark: {} queries, QPS={:.1}, elapsed={:.1}s", queries, qps, elapsed);
    assert!(qps > 0.1, "QPS too low: {:.1}", qps);
}
