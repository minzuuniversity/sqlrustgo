//! v3.8.0 Point Query + Aggregation 性能基准 (实测)
//!
//! **Date**: 2026-06-03
//! **Issue**: V380 PERFORMANCE_TARGETS.md (P0 followup)
//!
//! 工作负载:
//! 1. Point Query (Primary Key)
//! 2. Point Query Batch
//! 3. Point Query Range
//! 4. Aggregation (COUNT/SUM/AVG)
//! 5. Aggregation with WHERE filter
//!
//! **Mode**: --ignored, --release
//! **Run**: cargo test --release --test bench_v380_point_agg -- --ignored --nocapture

use sqlrustgo::MemoryExecutionEngine;
use std::time::Instant;

const ITER: usize = 10_000;

fn create_engine() -> MemoryExecutionEngine {
    let storage = std::sync::Arc::new(std::sync::RwLock::new(
        sqlrustgo_storage::MemoryStorage::new(),
    ));
    MemoryExecutionEngine::new(storage)
}

fn setup_users_table(engine: &mut MemoryExecutionEngine) {
    let _ = engine.execute("CREATE TABLE IF NOT EXISTS users (id INTEGER, name TEXT, age INTEGER)");
    for i in 0..1000 {
        let _ = engine.execute(&format!(
            "INSERT INTO users VALUES ({}, 'user_{}', {})",
            i,
            i,
            20 + (i % 50)
        ));
    }
}

#[test]
#[ignore = "performance benchmark, run with --ignored --release"]
fn bench_pkey_lookup() {
    let mut engine = create_engine();
    setup_users_table(&mut engine);

    // Warmup
    for i in 0..100 {
        let _ = engine.execute(&format!("SELECT * FROM users WHERE id = {}", i));
    }

    let start = Instant::now();
    let mut found = 0;
    for i in 0..ITER {
        let r = engine.execute(&format!("SELECT * FROM users WHERE id = {}", i % 1000));
        if r.is_ok() {
            found += 1;
        }
    }
    let elapsed = start.elapsed();
    let total_us = elapsed.as_micros();
    let avg_us = total_us / ITER as u128;
    let qps = (ITER as u128 * 1_000_000) / total_us.max(1);

    println!("=== Point Query: PKey Lookup (v3.8.0) ===");
    println!("  Iterations: {}", ITER);
    println!("  Total: {} ms", total_us / 1000);
    println!("  Avg latency: {} us", avg_us);
    println!("  QPS: {}", qps);
    println!("  Found (iter ok): {}/{}", found, ITER);

    write_artifact(
        "pkey_lookup.txt",
        &format!(
            "PKey Lookup (v3.8.0)\n\
             Date: 2026-06-03\n\
             Iter: {}\n\
             Total: {} ms\n\
             Avg: {} us\n\
             QPS: {}\n",
            ITER,
            total_us / 1000,
            avg_us,
            qps
        ),
    );

    assert!(found >= ITER - 100, "Should find most rows");
}

#[test]
#[ignore = "performance benchmark, run with --ignored --release"]
fn bench_pkey_batch() {
    let mut engine = create_engine();
    setup_users_table(&mut engine);

    let start = Instant::now();
    let mut found = 0;
    for _ in 0..100 {
        for k in 0..100 {
            let r = engine.execute(&format!("SELECT * FROM users WHERE id = {}", k));
            if r.is_ok() {
                found += 1;
            }
        }
    }
    let elapsed = start.elapsed();
    let total_us = elapsed.as_micros();
    let avg_us = total_us / ITER as u128;
    let qps = (ITER as u128 * 1_000_000) / total_us.max(1);

    println!("=== Point Query: Batch (v3.8.0) ===");
    println!("  Total: {} ms", total_us / 1000);
    println!("  Avg latency: {} us", avg_us);
    println!("  QPS: {}", qps);
    println!("  Found: {}/{}", found, ITER);

    write_artifact(
        "pkey_batch.txt",
        &format!(
            "PKey Batch (v3.8.0)\n\
             Date: 2026-06-03\n\
             Iter: {}\n\
             Total: {} ms\n\
             Avg: {} us\n\
             QPS: {}\n",
            ITER,
            total_us / 1000,
            avg_us,
            qps
        ),
    );

    assert!(found >= 5000);
}

#[test]
#[ignore = "performance benchmark, run with --ignored --release"]
fn bench_pkey_range() {
    let mut engine = create_engine();
    setup_users_table(&mut engine);

    let start = Instant::now();
    let mut found = 0;
    for iter in 0..100 {
        for offset in 0..100 {
            let k = (iter * 10 + offset) % 1000;
            let r = engine.execute(&format!("SELECT * FROM users WHERE id = {}", k));
            if r.is_ok() {
                found += 1;
            }
        }
    }
    let elapsed = start.elapsed();
    let total_us = elapsed.as_micros();
    let avg_us = total_us / ITER as u128;
    let qps = (ITER as u128 * 1_000_000) / total_us.max(1);

    println!("=== Point Query: Range (v3.8.0) ===");
    println!("  Total: {} ms", total_us / 1000);
    println!("  Avg latency: {} us", avg_us);
    println!("  QPS: {}", qps);
    println!("  Found: {}/{}", found, ITER);

    write_artifact(
        "pkey_range.txt",
        &format!(
            "PKey Range (v3.8.0)\n\
             Date: 2026-06-03\n\
             Iter: {}\n\
             Total: {} ms\n\
             Avg: {} us\n\
             QPS: {}\n",
            ITER,
            total_us / 1000,
            avg_us,
            qps
        ),
    );

    assert!(found >= 5000);
}

#[test]
#[ignore = "performance benchmark, run with --ignored --release"]
fn bench_aggregation_count() {
    let mut engine = create_engine();
    setup_users_table(&mut engine);

    // Warmup
    for _ in 0..50 {
        let _ = engine.execute("SELECT COUNT(*) FROM users");
    }

    let start = Instant::now();
    let mut found = 0;
    for _ in 0..ITER {
        let r = engine.execute("SELECT COUNT(*) FROM users");
        if r.is_ok() {
            found += 1;
        }
    }
    let elapsed = start.elapsed();
    let total_us = elapsed.as_micros();
    let avg_us = total_us / ITER as u128;
    let qps = (ITER as u128 * 1_000_000) / total_us.max(1);

    println!("=== Aggregation: COUNT(*) (v3.8.0) ===");
    println!("  Total: {} ms", total_us / 1000);
    println!("  Avg latency: {} us", avg_us);
    println!("  QPS: {}", qps);

    write_artifact(
        "agg_count.txt",
        &format!(
            "Aggregation COUNT(*) (v3.8.0)\n\
             Date: 2026-06-03\n\
             Iter: {}\n\
             Total: {} ms\n\
             Avg: {} us\n\
             QPS: {}\n",
            ITER,
            total_us / 1000,
            avg_us,
            qps
        ),
    );

    assert_eq!(found, ITER, "COUNT should always succeed");
}

#[test]
#[ignore = "performance benchmark, run with --ignored --release"]
fn bench_aggregation_sum_avg() {
    let mut engine = create_engine();
    setup_users_table(&mut engine);

    let start = Instant::now();
    let mut found = 0;
    for _ in 0..ITER {
        let r = engine.execute("SELECT SUM(age), AVG(age) FROM users");
        if r.is_ok() {
            found += 1;
        }
    }
    let elapsed = start.elapsed();
    let total_us = elapsed.as_micros();
    let avg_us = total_us / ITER as u128;
    let qps = (ITER as u128 * 1_000_000) / total_us.max(1);

    println!("=== Aggregation: SUM/AVG (v3.8.0) ===");
    println!("  Total: {} ms", total_us / 1000);
    println!("  Avg latency: {} us", avg_us);
    println!("  QPS: {}", qps);

    write_artifact(
        "agg_sum_avg.txt",
        &format!(
            "Aggregation SUM/AVG (v3.8.0)\n\
             Date: 2026-06-03\n\
             Iter: {}\n\
             Total: {} ms\n\
             Avg: {} us\n\
             QPS: {}\n",
            ITER,
            total_us / 1000,
            avg_us,
            qps
        ),
    );

    assert!(found > 0);
}

#[test]
#[ignore = "performance benchmark, run with --ignored --release"]
fn bench_aggregation_with_filter() {
    let mut engine = create_engine();
    setup_users_table(&mut engine);

    let start = Instant::now();
    let mut found = 0;
    for i in 0..ITER {
        let r = engine.execute(&format!(
            "SELECT COUNT(*), SUM(age) FROM users WHERE age > {}",
            20 + (i % 50)
        ));
        if r.is_ok() {
            found += 1;
        }
    }
    let elapsed = start.elapsed();
    let total_us = elapsed.as_micros();
    let avg_us = total_us / ITER as u128;
    let qps = (ITER as u128 * 1_000_000) / total_us.max(1);

    println!("=== Aggregation: COUNT+SUM with WHERE (v3.8.0) ===");
    println!("  Total: {} ms", total_us / 1000);
    println!("  Avg latency: {} us", avg_us);
    println!("  QPS: {}", qps);

    write_artifact(
        "agg_filter.txt",
        &format!(
            "Aggregation COUNT+SUM with WHERE (v3.8.0)\n\
             Date: 2026-06-03\n\
             Iter: {}\n\
             Total: {} ms\n\
             Avg: {} us\n\
             QPS: {}\n",
            ITER,
            total_us / 1000,
            avg_us,
            qps
        ),
    );

    assert!(found > 0);
}

fn write_artifact(name: &str, content: &str) {
    let dir = "artifacts/bench/v3.8.0";
    std::fs::create_dir_all(dir).ok();
    let path = format!("{}/{}", dir, name);
    std::fs::write(&path, content).ok();
    println!("  Wrote: {}", path);
}
