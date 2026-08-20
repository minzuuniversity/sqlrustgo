#[path = "../../common/mod.rs"]
mod common;
use std::thread;
use std::time::{Duration, Instant};

const QUERIES: &[&str] = &[
    "SELECT COUNT(*) FROM lineitem",
    "SELECT COUNT(*) FROM orders",
    "SELECT COUNT(*) FROM customer",
    "SELECT SUM(l_extendedprice) FROM lineitem",
    "SELECT l_orderkey, SUM(l_quantity) FROM lineitem GROUP BY l_orderkey LIMIT 10",
    "SELECT o_custkey, COUNT(*) FROM orders GROUP BY o_custkey LIMIT 10",
];

fn run_thread_benchmark(tid: usize, duration: Duration) -> (u64, u64, u64) {
    let mut client = common::tpch_wire_harness::start_sf01();
    let mut local_q = 0u64;
    let mut local_err = 0u64;
    let mut local_rows = 0u64;
    let thread_start = Instant::now();
    let mut qidx = 0usize;

    while thread_start.elapsed() < duration {
        let sql = QUERIES[qidx % QUERIES.len()];

        match client.query_rows(sql) {
            Ok(rows) => {
                local_q += 1;
                local_rows += rows.len() as u64;
            }
            Err(e) => {
                local_err += 1;
                eprintln!("Thread {} error: {}", tid, e);
            }
        }
        qidx += 1;

        if qidx.is_multiple_of(50) {
            thread::sleep(Duration::from_micros(100));
        }
    }

    (local_q, local_err, local_rows)
}

#[test]
fn test_tpch_qps_30s_multi_threaded() {
    let threads: usize = std::env::var("MT_SOAK_THREADS")
        .unwrap_or_else(|_| "4".to_string())
        .parse()
        .unwrap_or(4);
    let duration_secs: u64 = std::env::var("MT_SOAK_DURATION")
        .unwrap_or_else(|_| "30".to_string())
        .parse()
        .unwrap_or(30);

    println!("=== Multi-threaded QPS Benchmark (SF=0.1) ===");
    println!("Threads: {}", threads);
    println!("Duration: {}s", duration_secs);
    println!();

    let duration = Duration::from_secs(duration_secs);
    let start = Instant::now();

    let handles: Vec<_> = (0..threads)
        .map(|tid| thread::spawn(move || run_thread_benchmark(tid, duration)))
        .collect();

    let mut total_q = 0u64;
    let mut total_err = 0u64;
    let mut total_rows = 0u64;

    for (i, h) in handles.into_iter().enumerate() {
        let (q, e, r) = h.join().unwrap();
        total_q += q;
        total_err += e;
        total_rows += r;
        println!("Thread {} done: {} queries, {} errors, {} rows", i, q, e, r);
    }

    let elapsed = start.elapsed().as_secs_f64();
    let qps = total_q as f64 / elapsed;
    let error_rate = if total_q + total_err > 0 {
        total_err as f64 / (total_q + total_err) as f64
    } else {
        0.0
    };

    println!();
    println!("====================== QPS SUMMARY ======================");
    println!("Threads:           {}", threads);
    println!("Duration:          {:.2}s", elapsed);
    println!("Total queries:     {}", total_q);
    println!("Total errors:      {}", total_err);
    println!("Total rows:        {}", total_rows);
    println!("QPS:               {:.1} queries/sec", qps);
    println!("Error rate:        {:.2}%", error_rate * 100.0);
    println!("========================================================");

    assert!(qps > 0.1, "QPS too low: {:.1}", qps);
    assert!(
        error_rate < 0.01,
        "Error rate too high: {:.2}%",
        error_rate * 100.0
    );
}
