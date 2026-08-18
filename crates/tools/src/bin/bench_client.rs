use clap::Parser;
use rand::Rng;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    #[arg(long, default_value = "3306")]
    port: u16,

    #[arg(long, default_value = "8")]
    threads: usize,

    #[arg(long, default_value = "10")]
    duration: u64,

    #[arg(long, default_value = "3")]
    oltp_weight: usize,

    #[arg(long, default_value = "7")]
    olap_weight: usize,

    #[arg(long, default_value = "100")]
    batch_size: usize,
}

fn run_thread(
    thread_id: usize,
    host: String,
    port: u16,
    oltp_weight: usize,
    olap_weight: usize,
    batch_size: usize,
    duration: u64,
    total_oltp: Arc<AtomicUsize>,
    total_olap: Arc<AtomicUsize>,
    error_count: Arc<AtomicUsize>,
) {
    let mut rng = rand::thread_rng();
    let mut oltp_count = 0usize;
    let mut olap_count = 0usize;
    let end = Instant::now() + Duration::from_secs(duration);

    while Instant::now() < end {
        // Build batch of queries
        let mut queries = Vec::with_capacity(batch_size);
        let mut batch_oltp = 0usize;
        let mut batch_olap = 0usize;

        for _ in 0..batch_size {
            if Instant::now() >= end {
                break;
            }

            let is_oltp = rng.gen_ratio(oltp_weight as u32, (oltp_weight + olap_weight) as u32);

            let query = if is_oltp {
                let orderkey = rng.gen_range(1..10000);
                match rng.gen_range(0..3) {
                    0 => format!("SELECT * FROM orders WHERE o_orderkey = {}", orderkey),
                    1 => format!("UPDATE orders SET o_orderstatus = 'P' WHERE o_orderkey = {}", orderkey),
                    _ => format!("INSERT INTO orders (o_orderkey, o_custkey) VALUES ({}, {}) ON DUPLICATE KEY UPDATE o_custkey = o_custkey", orderkey, orderkey % 1000),
                }
            } else {
                match rng.gen_range(0..3) {
                    0 => "SELECT l_returnflag, COUNT(*) FROM lineitem GROUP BY l_returnflag".to_string(),
                    1 => "SELECT o_orderdate, SUM(o_totalprice) FROM orders GROUP BY o_orderdate LIMIT 100".to_string(),
                    _ => "SELECT c_nationkey, COUNT(*), AVG(c_acctbal) FROM customer GROUP BY c_nationkey".to_string(),
                }
            };

            if is_oltp {
                batch_oltp += 1;
            } else {
                batch_olap += 1;
            }
            queries.push(query);
        }

        // Execute batch via mysql -e with piped input
        let _input = queries.join(";");
        let output = Command::new("mysql")
            .args(["-h", &host, "-P", &port.to_string(), "-u", "root", "-N"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let _lines = String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .filter(|l| !l.is_empty())
                    .count();
                // Count successful queries (some may have been no-row selects)
                oltp_count += batch_oltp;
                olap_count += batch_olap;
            }
            Ok(out) => {
                // Some queries failed, count partial
                let failed = String::from_utf8_lossy(&out.stderr).len();
                if failed > 0 {
                    error_count.fetch_add(1, Ordering::Relaxed);
                }
            }
            Err(_) => {
                error_count.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    total_oltp.fetch_add(oltp_count, Ordering::Relaxed);
    total_olap.fetch_add(olap_count, Ordering::Relaxed);
    println!(
        "Thread {}: {} OLTP, {} OLAP",
        thread_id, oltp_count, olap_count
    );
}

fn main() {
    let args = Args::parse();

    println!("=== Multi-threaded MySQL OLTP+OLAP Benchmark (Batch Mode) ===");
    println!("Host: {}:{}", args.host, args.port);
    println!("Threads: {}", args.threads);
    println!("Duration: {}s", args.duration);
    println!(
        "OLTP:OLAP ratio = {}:{}",
        args.oltp_weight, args.olap_weight
    );
    println!("Batch size: {}", args.batch_size);
    println!("");

    let total_oltp = Arc::new(AtomicUsize::new(0));
    let total_olap = Arc::new(AtomicUsize::new(0));
    let error_count = Arc::new(AtomicUsize::new(0));

    let start = Instant::now();

    let handles: Vec<_> = (0..args.threads)
        .map(|thread_id| {
            let host = args.host.clone();
            let total_oltp = Arc::clone(&total_oltp);
            let total_olap = Arc::clone(&total_olap);
            let error_count = Arc::clone(&error_count);

            thread::spawn(move || {
                run_thread(
                    thread_id,
                    host,
                    args.port,
                    args.oltp_weight,
                    args.olap_weight,
                    args.batch_size,
                    args.duration,
                    total_oltp,
                    total_olap,
                    error_count,
                );
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    let elapsed = start.elapsed();
    let oltp = total_oltp.load(Ordering::Relaxed);
    let olap = total_olap.load(Ordering::Relaxed);
    let errors = error_count.load(Ordering::Relaxed);
    let total = oltp + olap;

    println!("");
    println!("=== RESULTS ===");
    println!("OLTP queries: {}", oltp);
    println!("OLAP queries: {}", olap);
    println!("Total queries: {}", total);
    println!("Errors: {}", errors);
    if total + errors > 0 {
        println!(
            "Success rate: {:.1}%",
            100.0 * total as f64 / (total + errors) as f64
        );
    }
    println!("Duration: {:.2}s", elapsed.as_secs_f64());
    if elapsed.as_secs_f64() > 0.0 {
        println!("OPS: {:.1}", total as f64 / elapsed.as_secs_f64());
    }
}
