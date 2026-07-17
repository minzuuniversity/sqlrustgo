use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::process::Command;
use clap::Parser;
use std::thread;

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

    #[arg(long)]
    query: Option<String>,
}

fn main() {
    let args = Args::parse();

    let query = args.query.unwrap_or_else(|| "SELECT 1".to_string());

    println!("=== Multi-threaded MySQL Benchmark ===");
    println!("Host: {}:{}", args.host, args.port);
    println!("Threads: {}", args.threads);
    println!("Duration: {}s", args.duration);
    println!("Query: {}", query);
    println!("");

    let total_count = Arc::new(AtomicUsize::new(0));
    let error_count = Arc::new(AtomicUsize::new(0));

    let start = Instant::now();
    let duration = Duration::from_secs(args.duration);

    let handles: Vec<_> = (0..args.threads).map(|thread_id| {
        let host = args.host.clone();
        let port = args.port;
        let query = query.clone();
        let total_count = Arc::clone(&total_count);
        let error_count = Arc::clone(&error_count);
        let duration = duration;

        thread::spawn(move || {
            let mut count = 0usize;
            let end = Instant::now() + duration;

            while Instant::now() < end {
                let output = Command::new("mysql")
                    .args(["-h", &host, "-P", &port.to_string(), "-u", "root", "-e", &query])
                    .output();

                match output {
                    Ok(out) if out.status.success() => {
                        count += 1;
                    }
                    _ => {
                        error_count.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }

            total_count.fetch_add(count, Ordering::Relaxed);
            println!("Thread {}: {} queries", thread_id, count);
        })
    }).collect();

    for handle in handles {
        handle.join().unwrap();
    }

    let elapsed = start.elapsed();
    let total = total_count.load(Ordering::Relaxed);
    let errors = error_count.load(Ordering::Relaxed);

    println!("");
    println!("=== RESULTS ===");
    println!("Total queries: {}", total);
    println!("Errors: {}", errors);
    println!("Duration: {:.2}s", elapsed.as_secs_f64());
    println!("OPS: {:.1}", total as f64 / elapsed.as_secs_f64());
}
