//! G11 QPS/TPS Benchmark — 5 workloads × 4 thread counts
//!
//! Refs: docs/openspec/3182-cost-optimizer.md (借力 SQLRustGo StorageEngine)
//!       V390_TEST_PLAN_SUPPLEMENT_PERF.md §G11
//!
//! Uses the actual MemoryStorage / StorageEngine API:
//!   - scan(table) -> Vec<Record>
//!   - insert(table, records) -> SqlResult<()>
//!   - update(table, filters, updates) -> SqlResult<usize>
//!   - delete_if(table, filter) -> SqlResult<usize>

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sqlrustgo_storage::{
    ColumnDefinition, MemoryStorage, Record, RowFilter, RowMutation, StorageEngine, TableInfo,
};
use sqlrustgo_types::Value;
use std::hint::black_box;
use std::sync::{Arc, RwLock};
use std::thread;

/// Generate test rows
fn generate_rows(count: usize) -> Vec<Record> {
    (0..count)
        .map(|i| {
            vec![
                Value::Integer(i as i64),
                Value::Integer((i % 1000) as i64),
                Value::Text(format!("pad-{}", i)),
            ]
        })
        .collect()
}

/// Create test table info
fn create_table_info() -> TableInfo {
    TableInfo {
        name: "qps_bench".to_string(),
        columns: vec![
            ColumnDefinition {
                name: "id".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: true,
                char_max_length: None,
                collation: None,

                default_value: None,
            },
            ColumnDefinition {
                name: "k".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                collation: None,

                default_value: None,
            },
            ColumnDefinition {
                name: "c".to_string(),
                data_type: "TEXT".to_string(),
                nullable: true,
                primary_key: false,
                char_max_length: None,
                collation: None,

                default_value: None,
            },
        ],
        foreign_keys: vec![],
        unique_constraints: vec![],
        check_constraints: vec![],
        compression: None,
        collations: std::collections::HashMap::new(),
        partition_info: None,
    }
}

/// Setup: shared storage with pre-populated data
fn setup_storage(rows: usize) -> Arc<RwLock<MemoryStorage>> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    {
        let mut s = storage.write().unwrap();
        s.create_table(&create_table_info()).unwrap();
        s.insert("qps_bench", generate_rows(rows)).unwrap();
    }
    storage
}

/// Filter that matches id == target (uses a thread-local)
thread_local! {
    static TARGET_ID: std::cell::Cell<i64> = const { std::cell::Cell::new(0) };
}

fn make_id_filter(target: i64) -> RowFilter {
    Box::new(move |row: &Record| -> bool {
        matches!(row.first(), Some(Value::Integer(v)) if *v == target)
    })
}

fn make_k_filter(target: i64) -> RowFilter {
    Box::new(move |row: &Record| -> bool {
        matches!(row.get(1), Some(Value::Integer(v)) if *v == target)
    })
}

/// 1. Point SELECT (主键查询) - via scan + filter
fn bench_point_select(c: &mut Criterion) {
    let storage = setup_storage(10_000);
    let mut group = c.benchmark_group("qps_point_select");
    for &threads in &[1usize, 4, 8, 16] {
        group.throughput(Throughput::Elements(threads as u64 * 1000));
        group.bench_with_input(BenchmarkId::from_parameter(threads), &threads, |b, &t| {
            b.iter(|| {
                let handles: Vec<_> = (0..t)
                    .map(|tid| {
                        let s = Arc::clone(&storage);
                        thread::spawn(move || {
                            for i in 0..1000 {
                                let id = ((tid * 1000 + i) % 10_000) as i64;
                                let filter = make_id_filter(id);
                                let guard = s.read().unwrap();
                                let rows: Vec<Record> = guard
                                    .scan("qps_bench")
                                    .unwrap_or_default()
                                    .into_iter()
                                    .filter(|r| filter(r))
                                    .collect();
                                black_box(rows);
                            }
                        })
                    })
                    .collect();
                for h in handles {
                    h.join().unwrap();
                }
            });
        });
    }
    group.finish();
}

/// 2. Range SELECT (k 列 1% 选择率)
fn bench_range_select(c: &mut Criterion) {
    let storage = setup_storage(10_000);
    let mut group = c.benchmark_group("qps_range_select");
    for &threads in &[1usize, 4, 8] {
        group.throughput(Throughput::Elements(threads as u64 * 1000));
        group.bench_with_input(BenchmarkId::from_parameter(threads), &threads, |b, &t| {
            b.iter(|| {
                let handles: Vec<_> = (0..t)
                    .map(|tid| {
                        let s = Arc::clone(&storage);
                        thread::spawn(move || {
                            for i in 0..1000 {
                                let target = ((tid * 1000 + i) % 1000) as i64;
                                let filter = make_k_filter(target);
                                let guard = s.read().unwrap();
                                let rows: Vec<Record> = guard
                                    .scan("qps_bench")
                                    .unwrap_or_default()
                                    .into_iter()
                                    .filter(|r| filter(r))
                                    .collect();
                                black_box(rows);
                            }
                        })
                    })
                    .collect();
                for h in handles {
                    h.join().unwrap();
                }
            });
        });
    }
    group.finish();
}

/// 3. INSERT (写)
fn bench_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("qps_insert");
    for &threads in &[1usize, 4, 8] {
        group.throughput(Throughput::Elements(threads as u64 * 1000));
        group.bench_with_input(BenchmarkId::from_parameter(threads), &threads, |b, &t| {
            b.iter(|| {
                let storage = Arc::new(RwLock::new(MemoryStorage::new()));
                storage
                    .write()
                    .unwrap()
                    .create_table(&create_table_info())
                    .unwrap();
                let handles: Vec<_> = (0..t)
                    .map(|tid| {
                        let s = Arc::clone(&storage);
                        thread::spawn(move || {
                            for i in 0..1000 {
                                let rows = vec![vec![
                                    Value::Integer((tid * 1000 + i) as i64),
                                    Value::Integer(i as i64),
                                    Value::Text(format!("row-{}", i)),
                                ]];
                                let _ = s.write().unwrap().insert("qps_bench", rows);
                            }
                        })
                    })
                    .collect();
                for h in handles {
                    h.join().unwrap();
                }
            });
        });
    }
    group.finish();
}

/// 4. UPDATE (索引列 id) - via update(table, filters, updates)
fn bench_update(c: &mut Criterion) {
    let storage = setup_storage(10_000);
    let mut group = c.benchmark_group("qps_update");
    for &threads in &[1usize, 4, 8] {
        group.throughput(Throughput::Elements(threads as u64 * 500));
        group.bench_with_input(BenchmarkId::from_parameter(threads), &threads, |b, &t| {
            b.iter(|| {
                let handles: Vec<_> = (0..t)
                    .map(|tid| {
                        let s = Arc::clone(&storage);
                        thread::spawn(move || {
                            for i in 0..500 {
                                let id = ((tid * 500 + i) % 10_000) as i64;
                                // update col 2 to "updated-i"
                                let updates = vec![(2usize, Value::Text(format!("upd-{}", i)))];
                                let _ = s.write().unwrap().update(
                                    "qps_bench",
                                    &[Value::Integer(id)],
                                    &updates,
                                );
                            }
                        })
                    })
                    .collect();
                for h in handles {
                    h.join().unwrap();
                }
            });
        });
    }
    group.finish();
}

/// 5. Mixed OLTP (point_select + insert + update, sysbench-like)
fn bench_mixed_oltp(c: &mut Criterion) {
    let storage = setup_storage(10_000);
    let mut group = c.benchmark_group("qps_mixed_oltp");
    for &threads in &[4usize, 8] {
        group.throughput(Throughput::Elements(threads as u64 * 1000));
        group.bench_with_input(BenchmarkId::from_parameter(threads), &threads, |b, &t| {
            b.iter(|| {
                let handles: Vec<_> = (0..t)
                    .map(|tid| {
                        let s = Arc::clone(&storage);
                        thread::spawn(move || {
                            for i in 0..1000 {
                                let op = i % 10;
                                if op < 7 {
                                    // SELECT 70%
                                    let id = ((tid * 1000 + i) % 10_000) as i64;
                                    let filter = make_id_filter(id);
                                    let guard = s.read().unwrap();
                                    let rows: Vec<Record> = guard
                                        .scan("qps_bench")
                                        .unwrap_or_default()
                                        .into_iter()
                                        .filter(|r| filter(r))
                                        .collect();
                                    black_box(rows);
                                } else if op < 9 {
                                    // UPDATE 20%
                                    let id = ((tid * 1000 + i) % 10_000) as i64;
                                    let updates = vec![(2usize, Value::Text(format!("mix-{}", i)))];
                                    let _ = s.write().unwrap().update(
                                        "qps_bench",
                                        &[Value::Integer(id)],
                                        &updates,
                                    );
                                } else {
                                    // INSERT 10% (rare)
                                    let rows = vec![vec![
                                        Value::Integer((100_000 + tid * 1000 + i) as i64),
                                        Value::Integer(i as i64),
                                        Value::Text(format!("new-{}", i)),
                                    ]];
                                    let _ = s.write().unwrap().insert("qps_bench", rows);
                                }
                            }
                        })
                    })
                    .collect();
                for h in handles {
                    h.join().unwrap();
                }
            });
        });
    }
    group.finish();
}

criterion_group!(
    qps_benches,
    bench_point_select,
    bench_range_select,
    bench_insert,
    bench_update,
    bench_mixed_oltp
);
criterion_main!(qps_benches);
