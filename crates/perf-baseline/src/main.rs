fn main() {
    sqlrustgo_perf_baseline::run().unwrap_or_else(|e| {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    });
}
