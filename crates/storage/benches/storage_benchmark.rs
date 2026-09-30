//! FileStorage insert buffering performance benchmarks
//!
//! Performance targets (Issue #1667):
//! - Single row INSERT: < 1ms (direct write)
//! - Batch INSERT (1000 rows): < 50ms (buffered)
//! - Buffer flush: < 10ms per 100 records

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use sqlrustgo_storage::{
    engine::{ColumnDefinition, StorageEngine, TableInfo},
    file_storage::FileStorage,
};
use std::fs::remove_dir_all;
use std::hint::black_box;
use std::path::PathBuf;

/// Generate test records for benchmarking
fn generate_records(count: usize) -> Vec<Vec<sqlrustgo_types::Value>> {
    (0..count)
        .map(|i| vec![sqlrustgo_types::Value::Integer(i as i64)])
        .collect()
}

/// Create a test table in temporary storage
fn setup_test_storage(
    temp_dir: &PathBuf,
    buffer_threshold: usize,
    enable_buffer: bool,
) -> FileStorage {
    let mut storage =
        FileStorage::new_with_buffer_config(temp_dir.clone(), buffer_threshold, enable_buffer)
            .unwrap();

    let table_info = TableInfo {
        name: "test_table".to_string(),
        columns: vec![ColumnDefinition {
            name: "id".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: false,
            primary_key: true,
            char_max_length: None,
            collation: None,

            default_value: None,
            auto_increment: false,
        }],
        foreign_keys: vec![],
        unique_constraints: vec![],
        check_constraints: vec![],
        compression: None,
        collations: std::collections::HashMap::new(),
        partition_info: None,
        original_sql: String::new(),
    };
    storage.create_table(&table_info).unwrap();

    storage
}

/// Benchmark: Single row INSERT with buffering disabled (direct write)
fn bench_single_insert_direct(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_single_no_buffer");

    for i in 0..100 {
        let temp_dir = std::env::temp_dir().join(format!("sqlrustgo_bench_direct_{}", i));
        let _ = remove_dir_all(&temp_dir);

        let mut storage = setup_test_storage(&temp_dir, 100, false);

        group.bench_function(BenchmarkId::from_parameter(i), |b| {
            b.iter(|| {
                let records = generate_records(1);
                let _ = storage.insert("test_table", records);
            });
        });

        let _ = remove_dir_all(&temp_dir);
    }

    group.finish();
}

/// Benchmark: Single row INSERT with buffering enabled
fn bench_single_insert_buffered(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_single_buffered");

    for i in 0..100 {
        let temp_dir = std::env::temp_dir().join(format!("sqlrustgo_bench_buffered_{}", i));
        let _ = remove_dir_all(&temp_dir);

        let mut storage = setup_test_storage(&temp_dir, 100, true);

        group.bench_function(BenchmarkId::from_parameter(i), |b| {
            b.iter(|| {
                let records = generate_records(1);
                let _ = storage.insert("test_table", records);
            });
        });

        let _ = remove_dir_all(&temp_dir);
    }

    group.finish();
}

/// Benchmark: Small batch INSERT (10 rows) - triggers buffered path
fn bench_batch_insert_small(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_batch_small_10");

    for size in [10].iter() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_bench_small_batch");
        let _ = remove_dir_all(&temp_dir);

        let mut storage = setup_test_storage(&temp_dir, 100, true);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let records = generate_records(size);
                let _ = storage.insert("test_table", records);
            });
        });

        let _ = remove_dir_all(&temp_dir);
    }

    group.finish();
}

/// Benchmark: Medium batch INSERT (100 rows) - at threshold
fn bench_batch_insert_medium(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_batch_medium_100");

    for size in [100].iter() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_bench_medium_batch");
        let _ = remove_dir_all(&temp_dir);

        let mut storage = setup_test_storage(&temp_dir, 100, true);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let records = generate_records(size);
                let _ = storage.insert("test_table", records);
            });
        });

        let _ = remove_dir_all(&temp_dir);
    }

    group.finish();
}

/// Benchmark: Large batch INSERT (1000 rows) - exceeds threshold, triggers flush
fn bench_batch_insert_large(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_batch_large_1000");

    for size in [1000].iter() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_bench_large_batch");
        let _ = remove_dir_all(&temp_dir);

        let mut storage = setup_test_storage(&temp_dir, 100, true);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let records = generate_records(size);
                let _ = storage.insert("test_table", records);
            });
        });

        let _ = remove_dir_all(&temp_dir);
    }

    group.finish();
}

/// Benchmark: Compare buffered vs direct for various batch sizes
fn bench_buffer_vs_direct(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_vs_direct");

    for size in [10, 50, 100, 500, 1000].iter() {
        // Direct write (buffering disabled)
        let temp_dir_direct = std::env::temp_dir().join(format!("sqlrustgo_bench_direct_{}", size));
        let _ = remove_dir_all(&temp_dir_direct);

        group.bench_with_input(BenchmarkId::new("direct", size), size, |b, &size| {
            let mut storage = setup_test_storage(&temp_dir_direct, 100, false);
            b.iter(|| {
                let records = generate_records(size);
                let _ = storage.insert("test_table", records);
            });
        });

        let _ = remove_dir_all(&temp_dir_direct);

        // Buffered write (buffering enabled)
        let temp_dir_buffered =
            std::env::temp_dir().join(format!("sqlrustgo_bench_buffered_{}", size));
        let _ = remove_dir_all(&temp_dir_buffered);

        group.bench_with_input(BenchmarkId::new("buffered", size), size, |b, &size| {
            let mut storage = setup_test_storage(&temp_dir_buffered, 100, true);
            b.iter(|| {
                let records = generate_records(size);
                let _ = storage.insert("test_table", records);
            });
        });

        let _ = remove_dir_all(&temp_dir_buffered);
    }

    group.finish();
}

/// Benchmark: Buffer flush performance
fn bench_buffer_flush(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_flush");

    for size in [100, 500, 1000].iter() {
        let temp_dir = std::env::temp_dir().join(format!("sqlrustgo_bench_flush_{}", size));
        let _ = remove_dir_all(&temp_dir);

        let mut storage = setup_test_storage(&temp_dir, 10000, true); // High threshold to prevent auto-flush

        // Pre-fill buffer
        let pre_records = generate_records(*size);
        let _ = storage.insert("test_table", pre_records);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                // Reset and pre-fill for each iteration
                let mut s = setup_test_storage(&temp_dir, 10000, true);
                let pre_records = generate_records(size);
                let _ = s.insert("test_table", pre_records);

                // Flush all buffers
                let _ = s.flush_all_buffers();
                black_box(size);
            });
        });

        let _ = remove_dir_all(&temp_dir);
    }

    group.finish();
}

/// Benchmark: Insert throughput (records per second)
fn bench_insert_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_throughput");

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let temp_dir =
                std::env::temp_dir().join(format!("sqlrustgo_bench_throughput_{}", size));
            let _ = remove_dir_all(&temp_dir);

            let mut storage = setup_test_storage(&temp_dir, 100, true);

            b.iter(|| {
                let records = generate_records(size);
                let _ = storage.insert("test_table", records);
            });

            let _ = remove_dir_all(&temp_dir);
        });
    }

    group.finish();
}

/// Benchmark: Multiple table inserts with shared buffer
fn bench_multi_table_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_multi_table");

    let tables = ["table1", "table2", "table3", "table4", "table5"];
    let records_per_table = 50;

    let temp_dir = std::env::temp_dir().join("sqlrustgo_bench_multi_table");
    let _ = remove_dir_all(&temp_dir);

    group.bench_function("5_tables_50_records_each", |b| {
        b.iter(|| {
            let mut storage = setup_test_storage(&temp_dir, 100, true);

            for table_name in tables.iter() {
                let table_info = TableInfo {
                    name: table_name.to_string(),
                    columns: vec![ColumnDefinition {
                        name: "id".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        primary_key: true,
                        char_max_length: None,
                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    }],
                    foreign_keys: vec![],
                    unique_constraints: vec![],
                    check_constraints: vec![],
                    compression: None,
                    collations: std::collections::HashMap::new(),
                    partition_info: None,
                    original_sql: String::new(),
                };
                let _ = storage.create_table(&table_info);
            }

            for table_name in tables.iter() {
                let records = generate_records(records_per_table);
                let _ = storage.insert(table_name, records);
            }

            let _ = storage.flush_all_buffers();
        });
    });

    let _ = remove_dir_all(&temp_dir);
    group.finish();
}

// --- Phase B2 / #4915 A/B benchmarks -------------------------------------
//
// The benchmarks above all insert into a table that starts empty, so they
// cannot see B2.1: `insert_direct` used to clone the *whole* `TableData`
// on every call, which costs O(table_size) and grows with every insert.
// The cost only shows up when the table is already large, so these
// pre-fill first and then measure a single insert into the large table.

/// Build a `FileStorage` with `rows` already in `test_table`.
fn setup_prefilled(temp_dir: &PathBuf, rows: usize) -> FileStorage {
    let _ = remove_dir_all(temp_dir);
    let mut storage = setup_test_storage(temp_dir, 100, false);
    storage
        .insert("test_table", generate_records(rows))
        .unwrap();
    storage
}

/// B2.1: one insert into an already-large table.
///
/// Pre-fix this is dominated by `data.clone()` — a full copy of the
/// `TableData` for every single row inserted. Post-fix it copies only
/// the appended window.
fn bench_b2_insert_into_large_table(c: &mut Criterion) {
    let mut group = c.benchmark_group("b2_insert_into_large_table");

    for preexisting in [1_000usize, 10_000, 50_000] {
        let temp_dir = std::env::temp_dir().join(format!("b2_large_{}", preexisting));
        let mut storage = setup_prefilled(&temp_dir, preexisting);

        group.bench_with_input(
            BenchmarkId::from_parameter(preexisting),
            &preexisting,
            |b, _| {
                b.iter(|| {
                    let records = generate_records(1);
                    black_box(storage.insert("test_table", records).unwrap());
                });
            },
        );

        let _ = remove_dir_all(&temp_dir);
    }

    group.finish();
}

/// B2.2: `flush()` of several dirty tables.
///
/// B2.2 moved the serialize + `write()` out of the inner write lock, so
/// this measures the flush itself, not concurrency. The lock-hold win
/// shows up under concurrent load (sysbench), not here — this bench
/// exists to confirm the move did not regress the I/O itself.
fn bench_b2_flush_dirty_tables(c: &mut Criterion) {
    let mut group = c.benchmark_group("b2_flush_dirty_tables");

    for table_count in [1usize, 5] {
        for rows_per_table in [1_000usize, 10_000] {
            let temp_dir =
                std::env::temp_dir().join(format!("b2_flush_{}_{}", table_count, rows_per_table));
            let _ = remove_dir_all(&temp_dir);

            let mut storage = setup_test_storage(&temp_dir, 1_000_000, true);
            for t in 0..table_count {
                let info = TableInfo {
                    name: format!("t{}", t),
                    columns: vec![ColumnDefinition {
                        name: "id".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        primary_key: true,
                        char_max_length: None,
                        collation: None,
                        default_value: None,
                        auto_increment: false,
                    }],
                    foreign_keys: vec![],
                    unique_constraints: vec![],
                    check_constraints: vec![],
                    compression: None,
                    collations: std::collections::HashMap::new(),
                    partition_info: None,
                    original_sql: String::new(),
                };
                storage.create_table(&info).unwrap();
                storage
                    .insert(&format!("t{}", t), generate_records(rows_per_table))
                    .unwrap();
            }

            let id = BenchmarkId::new(format!("{}tables_{}rows", table_count, rows_per_table), 0);
            group.bench_with_input(id, &(table_count, rows_per_table), |b, _| {
                b.iter(|| {
                    // Re-dirty every table each iteration, then flush.
                    for t in 0..table_count {
                        let _ = storage.insert(&format!("t{}", t), generate_records(1));
                    }
                    black_box(storage.flush().unwrap());
                });
            });

            let _ = remove_dir_all(&temp_dir);
        }
    }

    group.finish();
}

/// B2.3 / B2.4: full-snapshot write and filtered scan.
///
/// B2.3 made `save_table_full` stream instead of building an owned copy
/// plus a full `String`; B2.4 made `scan_with_filter` filter inside the
/// engine rather than materialising every row.
fn bench_b2_full_snapshot_and_filtered_scan(c: &mut Criterion) {
    let mut group = c.benchmark_group("b2_snapshot_and_scan");

    for rows in [10_000usize, 50_000] {
        let temp_dir = std::env::temp_dir().join(format!("b2_snap_{}", rows));
        let mut storage = setup_prefilled(&temp_dir, rows);

        // B2.3: full snapshot write. `save_table_full` is private and is
        // reached through `save_table`, so drive it the way production
        // does: dirty the table and call `flush()`.
        //
        // Each iteration builds a *fresh* FileStorage. Reusing one
        // storage across iterations lets the on-disk base snapshot and
        // the delta file grow without bound, so the measurement drifts
        // (an earlier version of this bench reported 133ms and 340ms for
        // the same binary on consecutive runs) and cannot be compared
        // A/B. Setup cost is inside the timed region, which is the
        // honest thing here: it is the same on both sides.
        group.bench_with_input(BenchmarkId::new("full_snapshot", rows), &rows, |b, _| {
            b.iter(|| {
                let _ = remove_dir_all(&temp_dir);
                let mut s = setup_test_storage(&temp_dir, 100, false);
                s.insert("test_table", generate_records(rows)).unwrap();
                black_box(s.flush().unwrap());
            });
        });

        // Filtered scan: a filter that matches almost nothing is the
        // case where pre-fix code paid to clone every row.
        let target = sqlrustgo_types::Value::Integer(-1);
        group.bench_with_input(
            BenchmarkId::new("filtered_scan_miss", rows),
            &rows,
            |b, _| {
                b.iter(|| {
                    let hit = storage
                        .scan_with_filter("test_table", |row| row.first() == Some(&target))
                        .unwrap();
                    black_box(hit.len());
                });
            },
        );

        let _ = remove_dir_all(&temp_dir);
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_single_insert_direct,
    bench_single_insert_buffered,
    bench_batch_insert_small,
    bench_batch_insert_medium,
    bench_batch_insert_large,
    bench_buffer_vs_direct,
    bench_buffer_flush,
    bench_insert_throughput,
    bench_multi_table_insert,
    bench_b2_insert_into_large_table,
    bench_b2_flush_dirty_tables,
    bench_b2_full_snapshot_and_filtered_scan
);
criterion_main!(benches);
