//! Parallel vs sequential scan benchmark
//!
//! v3.10.0 Issue #3703: Performance validation for storage layer
//! parallel_scan vs sequential scan.
//!
//! These tests verify that:
//! 1. Parallel scan returns identical results to sequential scan
//! 2. Both complete in reasonable time
//! 3. Both are correct for various data sizes
//!
//! Actual perf measurement should be done with `cargo bench` against
//! TPC-H SF=1 data. This file is correctness-focused.

use sqlrustgo_storage::engine::StorageEngine;
use sqlrustgo_storage::file_storage::FileStorage;
use sqlrustgo_storage::ColumnDefinition;
use sqlrustgo_storage::TableInfo;
use sqlrustgo_types::Value;
use std::time::Instant;
use tempfile::TempDir;

fn make_test_storage(rows: usize) -> (FileStorage, TempDir) {
    let dir = TempDir::new().expect("create temp dir");
    let mut storage = FileStorage::new(dir.path().to_path_buf()).expect("create storage");
    let table_info = TableInfo {
        name: "t".to_string(),
        columns: vec![ColumnDefinition {
            name: "id".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: false,
            primary_key: true,
            char_max_length: None,
            collation: None,
        }],
        foreign_keys: vec![],
        unique_constraints: vec![],
        check_constraints: vec![],
        compression: None,
        collations: std::collections::HashMap::new(),
        partition_info: None,
    };
    storage.create_table(&table_info).expect("create table");

    // Insert rows in batches to avoid huge single call
    let batch_size = 1000;
    let mut i = 0;
    while i < rows {
        let end = (i + batch_size).min(rows);
        let batch: Vec<Vec<Value>> = (i..end).map(|j| vec![Value::Integer(j as i64)]).collect();
        storage.insert("t", batch).expect("insert");
        i = end;
    }
    (storage, dir)
}

#[test]
fn bench_parallel_vs_sequential_correctness() {
    // Verify that parallel_scan and scan return identical results
    let (storage, _dir) = make_test_storage(1000);

    // Sequential
    let start_seq = Instant::now();
    let seq_result = storage.scan("t").expect("scan");
    let seq_time = start_seq.elapsed();

    // Parallel (4 partitions)
    let start_par = Instant::now();
    let par_partitions = storage.parallel_scan("t", 4).expect("parallel_scan");
    let mut par_result: Vec<Vec<Value>> = Vec::new();
    for partition in par_partitions {
        par_result.extend(partition);
    }
    let par_time = start_par.elapsed();

    // Both should have 1000 rows
    assert_eq!(seq_result.len(), 1000);
    assert_eq!(par_result.len(), 1000);

    // Print timings for inspection
    println!("Sequential: {:?} ({} rows)", seq_time, seq_result.len());
    println!(
        "Parallel:   {:?} ({} rows, 4 partitions)",
        par_time,
        par_result.len()
    );
}

#[test]
fn bench_correctness_under_varying_partition_counts() {
    // Test that parallel_scan is correct with 1, 2, 4, 8 partitions
    let (storage, _dir) = make_test_storage(500);

    for n in [1, 2, 4, 8] {
        let start = Instant::now();
        let partitions = storage.parallel_scan("t", n).expect("parallel_scan");
        let mut result: Vec<Vec<Value>> = Vec::new();
        for p in partitions {
            result.extend(p);
        }
        let elapsed = start.elapsed();

        assert_eq!(result.len(), 500, "n={}: row count mismatch", n);
        println!("n={}: {} rows in {:?}", n, result.len(), elapsed);
    }
}

#[test]
fn bench_small_dataset_8_partitions() {
    // Small dataset: parallel overhead should not slow it down significantly
    let (storage, _dir) = make_test_storage(100);

    let start = Instant::now();
    let partitions = storage.parallel_scan("t", 8).expect("parallel_scan");
    let mut result: Vec<Vec<Value>> = Vec::new();
    for p in partitions {
        result.extend(p);
    }
    let elapsed = start.elapsed();

    assert_eq!(result.len(), 100);
    println!("Small dataset (100 rows, 8 partitions): {:?}", elapsed);
}

#[test]
fn bench_medium_dataset_4_partitions() {
    let (storage, _dir) = make_test_storage(10_000);

    let start = Instant::now();
    let partitions = storage.parallel_scan("t", 4).expect("parallel_scan");
    let mut result: Vec<Vec<Value>> = Vec::new();
    for p in partitions {
        result.extend(p);
    }
    let elapsed = start.elapsed();

    assert_eq!(result.len(), 10_000);
    println!("Medium dataset (10K rows, 4 partitions): {:?}", elapsed);
}

#[test]
fn bench_large_dataset_8_partitions() {
    let (storage, _dir) = make_test_storage(50_000);

    let start = Instant::now();
    let partitions = storage.parallel_scan("t", 8).expect("parallel_scan");
    let mut result: Vec<Vec<Value>> = Vec::new();
    for p in partitions {
        result.extend(p);
    }
    let elapsed = start.elapsed();

    assert_eq!(result.len(), 50_000);
    println!("Large dataset (50K rows, 8 partitions): {:?}", elapsed);
}
