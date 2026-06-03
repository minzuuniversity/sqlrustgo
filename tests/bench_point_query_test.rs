//! v3.8.0 Point Query 性能基准
//!
//! **Date**: 2026-06-03
//! **Issue**: V380 Performance Benchmark (P0 PERFORMANCE_TARGETS)
//! **Goal**: 测量 storage 层 point query 性能
//!
//! 工作负载:
//! 1. Primary key lookup (MemoryStorage 字典)
//! 2. Batch lookup
//! 3. Range scan

use sqlrustgo_storage::engine::StorageEngine;
use sqlrustgo_storage::MemoryStorage;
use std::time::Instant;

#[test]
#[ignore = "performance benchmark, run with --ignored --release"]
fn bench_point_query_pkey_lookup() {
    let mut storage = MemoryStorage::new();
    for k in 0..1000u64 {
        let val = format!("row_data_{}", k);
        storage.put(k, val.as_str().to_string());
    }

    let keys: Vec<u64> = (0..1000).collect();

    // Warmup
    for k in keys.iter().take(100) {
        let _ = storage.get(&k.to_string());
    }

    // Measure: 10K iterations
    let start = Instant::now();
    let mut found = 0;
    for i in 0..10_000 {
        let k = keys[i % keys.len()];
        if storage.get(&k.to_string()).is_ok() {
            found += 1;
        }
    }
    let elapsed = start.elapsed();
    let total_us = elapsed.as_micros();
    let avg_us = total_us / 10_000;
    let qps = 10_000_000_000 / total_us.max(1);

    println!("=== Point Query: Primary Key Lookup ===");
    println!("  Total time: {} ms", total_us / 1000);
    println!("  Avg latency: {} us", avg_us);
    println!("  QPS: {}", qps);
    println!("  Found: {}/10000", found);

    std::fs::create_dir_all("artifacts/bench/v3.8.0").ok();
    std::fs::write(
        "artifacts/bench/v3.8.0/point_query_pkey.txt",
        format!(
            "Point Query PKey Lookup (v3.8.0)\n\
             Date: 2026-06-03\n\
             Total: {} ms, Avg: {} us, QPS: {}\n\
             Found: {}/10000\n\
             Storage: MemoryStorage (in-process)\n\
             Rows: 1000\n\
             Iterations: 10000\n",
            total_us / 1000,
            avg_us,
            qps,
            found
        ),
    )
    .ok();

    assert!(found >= 9000, "Should find most keys, got {}", found);
}

#[test]
#[ignore = "performance benchmark, run with --ignored --release"]
fn bench_point_query_batch() {
    let mut storage = MemoryStorage::new();
    for k in 0..100u64 {
        let val = format!("row_data_{}", k);
        storage.put(k, val.as_str().to_string());
    }

    let start = Instant::now();
    let mut found = 0;
    // Batch of 100 keys per iteration x 100 iterations = 10K lookups
    for _ in 0..100 {
        for k in 0..100u64 {
            if storage.get(&k.to_string()).is_ok() {
                found += 1;
            }
        }
    }
    let elapsed = start.elapsed();
    let total_us = elapsed.as_micros();
    let avg_us = total_us / 10_000;
    let qps = 10_000_000_000 / total_us.max(1);

    println!("=== Point Query: Batch Lookup ===");
    println!("  Total time: {} ms", total_us / 1000);
    println!("  Avg latency: {} us", avg_us);
    println!("  QPS: {}", qps);
    println!("  Found: {}/10000", found);

    std::fs::write(
        "artifacts/bench/v3.8.0/point_query_batch.txt",
        format!(
            "Point Query Batch (v3.8.0)\n\
             Date: 2026-06-03\n\
             Total: {} ms, Avg: {} us, QPS: {}\n\
             Found: {}/10000\n\
             Batch: 100 keys x 100 iters\n",
            total_us / 1000,
            avg_us,
            qps,
            found
        ),
    )
    .ok();

    assert!(found >= 9000);
}

#[test]
#[ignore = "performance benchmark, run with --ignored --release"]
fn bench_point_query_range() {
    let mut storage = MemoryStorage::new();
    for k in 0..1000u64 {
        let val = format!("row_data_{}", k);
        storage.put(k, val.as_str().to_string());
    }

    let start = Instant::now();
    let mut found = 0;
    // 10K lookups with range pattern (mostly hits)
    for iter in 0..100u64 {
        for offset in 0..100u64 {
            let k = (iter * 10 + offset) % 1000;
            if storage.get(&k.to_string()).is_ok() {
                found += 1;
            }
        }
    }
    let elapsed = start.elapsed();
    let total_us = elapsed.as_micros();
    let avg_us = total_us / 10_000;
    let qps = 10_000_000_000 / total_us.max(1);

    println!("=== Point Query: Range Lookup ===");
    println!("  Total time: {} ms", total_us / 1000);
    println!("  Avg latency: {} us", avg_us);
    println!("  QPS: {}", qps);
    println!("  Found: {}/10000", found);

    std::fs::write(
        "artifacts/bench/v3.8.0/point_query_range.txt",
        format!(
            "Point Query Range (v3.8.0)\n\
             Date: 2026-06-03\n\
             Total: {} ms, Avg: {} us, QPS: {}\n\
             Found: {}/10000\n\
             Range: 100 consec keys x 100 iters\n",
            total_us / 1000,
            avg_us,
            qps,
            found
        ),
    )
    .ok();

    assert!(found >= 5000);
}
