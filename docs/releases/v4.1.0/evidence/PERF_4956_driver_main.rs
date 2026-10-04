//! #4956 A/B driver — commit-path flush cost.
//!
//! Measures what a client actually waits for per committed transaction.
//! Three scenarios, all against `--auth-mode none`:
//!   S1 single-thread, 1 row per autocommit  → per-commit latency, purest signal
//!   S2 8 threads,     1 row per autocommit  → throughput + write-lock contention
//!   S3 single-thread, 1000 rows per explicit tx → flush cost amortisation
//!
//! Reports per-op latency percentiles so a mean can hide a bimodal
//! "mostly cheap, occasionally stalls on a snapshot" distribution.

use sqlrustgo_mysql_client::{MySqlConnection, ResultSet};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

fn connect(port: u16) -> MySqlConnection {
    let addr: SocketAddr = format!("127.0.0.1:{}", port).parse().expect("addr");
    MySqlConnection::connect(&addr, "", "", "").expect("connect")
}

fn is_err(r: &ResultSet) -> bool {
    matches!(r, ResultSet::Error { .. })
}

fn percentile(sorted_us: &[u64], p: f64) -> u64 {
    if sorted_us.is_empty() {
        return 0;
    }
    let idx = ((sorted_us.len() as f64 - 1.0) * p).round() as usize;
    sorted_us[idx.min(sorted_us.len() - 1)]
}

struct Stats {
    label: String,
    n: usize,
    wall_ms: u128,
    lat: Vec<u64>,
    errors: u64,
}

fn report(s: &Stats) {
    let mut v = s.lat.clone();
    v.sort_unstable();
    let sum: u64 = v.iter().sum();
    let mean = if v.is_empty() { 0 } else { sum / v.len() as u64 };
    println!(
        "RESULT|{}|n={}|wall_ms={}|tps={:.1}|mean_us={}|p50_us={}|p90_us={}|p99_us={}|max_us={}|errors={}",
        s.label,
        s.n,
        s.wall_ms,
        s.n as f64 / (s.wall_ms as f64 / 1000.0),
        mean,
        percentile(&v, 0.50),
        percentile(&v, 0.90),
        percentile(&v, 0.99),
        v.last().copied().unwrap_or(0),
        s.errors,
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // perf4956 <port> <scenario> <threads> <ops_per_thread> <rows_per_tx>
    let port: u16 = args[1].parse().expect("port");
    let scenario = args[2].clone();
    let threads: usize = args[3].parse().expect("threads");
    let ops: usize = args[4].parse().expect("ops");
    let rows_per_tx: usize = args[5].parse().expect("rows_per_tx");

    let table = format!("bench_{}", scenario.replace('-', "_"));
    {
        let mut c = connect(port);
        c.execute(&format!("DROP TABLE IF EXISTS {}", table)).ok();
        c.execute(&format!(
            "CREATE TABLE {} (id INT, pad TEXT)",
            table
        ))
        .expect("create");
    }

    let table = Arc::new(table);
    let base_id = Arc::new(AtomicU64::new(0));
    let errors = Arc::new(AtomicU64::new(0));

    let t0 = Instant::now();
    let mut handles = Vec::new();
    for t in 0..threads {
        let table = Arc::clone(&table);
        let base_id = Arc::clone(&base_id);
        let errors = Arc::clone(&errors);
        handles.push(std::thread::spawn(move || {
            let mut conn = connect(port);
            let mut lat: Vec<u64> = Vec::with_capacity(ops);
            for _ in 0..ops {
                let s = Instant::now();
                let mut bad = false;
                if rows_per_tx == 1 {
                    let id = base_id.fetch_add(1, Ordering::Relaxed);
                    let sql =
                        format!("INSERT INTO {} VALUES ({}, 'x')", table, id);
                    if is_err(&conn.execute(&sql).expect("insert")) {
                        bad = true;
                    }
                } else {
                    // One explicit transaction carrying rows_per_tx rows:
                    // BEGIN + rows + COMMIT, so the commit path is
                    // exercised once per group instead of per row.
                    let mut sql = String::from("BEGIN;");
                    for _ in 0..rows_per_tx {
                        let id = base_id.fetch_add(1, Ordering::Relaxed);
                        sql.push_str(&format!("INSERT INTO {} VALUES ({}, 'y');", table, id));
                    }
                    let r = conn.execute_multi(&sql).expect("multi");
                    if r.iter().any(is_err) {
                        bad = true;
                    }
                }
                lat.push(s.elapsed().as_micros() as u64);
                if bad {
                    errors.fetch_add(1, Ordering::Relaxed);
                }
            }
            let _ = t;
            lat
        }));
    }
    let mut all: Vec<u64> = Vec::new();
    for h in handles {
        all.extend(h.join().expect("thread"));
    }
    let stats = Stats {
        label: format!("{}t{}r{}", scenario, threads, rows_per_tx),
        n: all.len(),
        wall_ms: t0.elapsed().as_millis(),
        lat: all,
        errors: errors.load(Ordering::Relaxed),
    };
    report(&stats);
}
