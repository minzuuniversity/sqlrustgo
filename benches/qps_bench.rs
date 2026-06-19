//! QPS/TPS Benchmark — Issue #847
//!
//! Measures queries per second for various workload types:
//! - qps_point_select: Simple point selects
//! - qps_range_select: Range scan queries
//! - qps_insert: Insert operations
//! - qps_update: Update operations
//! - qps_mixed_oltp: Mixed OLTP workload
//!
//! This benchmark is used by G11 gate (scripts/gate/check_g11_qps.sh)

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use sqlrustgo::MemoryExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use std::sync::{Arc, RwLock};
use std::time::Instant;

#[allow(dead_code)]
const ITERATIONS: usize = 1000;
#[allow(dead_code)]
const CONCURRENT_THREADS: usize = 4;

fn create_engine() -> MemoryExecutionEngine {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    MemoryExecutionEngine::new(storage)
}

fn setup_tables(engine: &mut MemoryExecutionEngine) {
    let _ = engine.execute("CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, name TEXT, age INTEGER)");
    let _ = engine.execute("CREATE TABLE IF NOT EXISTS orders (id INTEGER PRIMARY KEY, user_id INTEGER, amount INTEGER)");
    let _ = engine.execute("CREATE TABLE IF NOT EXISTS products (id INTEGER PRIMARY KEY, name TEXT, price INTEGER)");
}

fn insert_test_data(engine: &mut MemoryExecutionEngine) {
    for i in 0..100 {
        let _ = engine.execute(&format!(
            "INSERT INTO users VALUES ({}, 'user_{}', {})",
            i, i, 20 + (i % 50)
        ));
    }
    for i in 0..500 {
        let _ = engine.execute(&format!(
            "INSERT INTO orders VALUES ({}, {}, {})",
            i, i % 100, 100 + (i % 1000)
        ));
    }
    for i in 0..100 {
        let _ = engine.execute(&format!(
            "INSERT INTO products VALUES ({}, 'product_{}', {})",
            i, i, 1000 + (i % 500)
        ));
    }
}

fn cleanup(engine: &mut MemoryExecutionEngine) {
    let _ = engine.execute("DROP TABLE IF EXISTS users");
    let _ = engine.execute("DROP TABLE IF EXISTS orders");
    let _ = engine.execute("DROP TABLE IF EXISTS products");
}

fn qps_point_select(c: &mut Criterion) {
    let mut engine = create_engine();
    setup_tables(&mut engine);
    insert_test_data(&mut engine);

    let mut g = c.benchmark_group("qps_point_select");
    for i in 0..10 {
        g.bench_with_input(BenchmarkId::new("users", i), &i, |b, _i| {
            b.iter(|| {
                let _ = engine.execute("SELECT * FROM users WHERE id = 50");
            });
        });
    }
    cleanup(&mut engine);
}

fn qps_range_select(c: &mut Criterion) {
    let mut engine = create_engine();
    setup_tables(&mut engine);
    insert_test_data(&mut engine);

    let mut g = c.benchmark_group("qps_range_select");
    for i in 0..10 {
        g.bench_with_input(BenchmarkId::new("orders", i), &i, |b, _i| {
            b.iter(|| {
                let _ = engine.execute("SELECT * FROM orders WHERE amount > 500");
            });
        });
    }
    cleanup(&mut engine);
}

fn qps_insert(c: &mut Criterion) {
    let mut engine = create_engine();
    setup_tables(&mut engine);

    let mut g = c.benchmark_group("qps_insert");
    for i in 0..10 {
        let idx = i * 1000;
        g.bench_with_input(BenchmarkId::new("products", i), &i, |b, _i| {
            b.iter(|| {
                let _ = engine.execute(&format!(
                    "INSERT INTO products VALUES ({}, 'new_product', 999)",
                    idx
                ));
            });
        });
    }
    cleanup(&mut engine);
}

fn qps_update(c: &mut Criterion) {
    let mut engine = create_engine();
    setup_tables(&mut engine);
    insert_test_data(&mut engine);

    let mut g = c.benchmark_group("qps_update");
    for i in 0..10 {
        g.bench_with_input(BenchmarkId::new("users", i), &i, |b, _i| {
            b.iter(|| {
                let _ = engine.execute("UPDATE users SET age = age + 1 WHERE id % 10 = 0");
            });
        });
    }
    cleanup(&mut engine);
}

fn qps_mixed_oltp(c: &mut Criterion) {
    let mut engine = create_engine();
    setup_tables(&mut engine);
    insert_test_data(&mut engine);

    let mut g = c.benchmark_group("qps_mixed_oltp");
    for i in 0..10 {
        g.bench_with_input(BenchmarkId::new("mixed", i), &i, |b, _i| {
            b.iter(|| {
                let _ = engine.execute("SELECT * FROM orders WHERE user_id IN (SELECT id FROM users WHERE age > 30)");
            });
        });
    }
    cleanup(&mut engine);
}

criterion_group!(
    benches,
    qps_point_select,
    qps_range_select,
    qps_insert,
    qps_update,
    qps_mixed_oltp
);
criterion_main!(benches);
