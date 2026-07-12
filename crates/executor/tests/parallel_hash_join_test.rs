//! Parallel hash join integration tests
//!
//! v3.10.0 Issue #3703 Phase 2 follow-up for #3737.

use sqlrustgo_executor::parallel_hash_join::{JoinKey, ParallelHashJoin, Partition};
use sqlrustgo_types::Value;

#[test]
fn test_parallel_hash_join_simple() {
    let p = ParallelHashJoin::new(4);

    // Build side: id=1 has 2 rows, id=2 has 1 row
    let build = vec![
        vec![Value::Integer(1), Value::Text("a".to_string())],
        vec![Value::Integer(1), Value::Text("b".to_string())],
        vec![Value::Integer(2), Value::Text("c".to_string())],
    ];
    // Probe side: id=1 once, id=3 once
    let probe = vec![
        vec![Value::Integer(1), Value::Integer(100)],
        vec![Value::Integer(3), Value::Integer(300)],
    ];
    let result = p.execute(build, 0, probe, 0);
    // Only id=1 should match: 2 build rows × 1 probe row = 2 joined tuples
    assert_eq!(result.len(), 2);
    // Each result is (build_row, probe_row) where build_row[0]=1 and probe_row[0]=1
    for (b, pr) in &result {
        assert_eq!(b[0], Value::Integer(1));
        assert_eq!(pr[0], Value::Integer(1));
        assert_eq!(pr[1], Value::Integer(100));
    }
}

#[test]
fn test_correctness_n1_vs_n4_simple() {
    let build = vec![
        vec![Value::Integer(1), Value::Text("a".to_string())],
        vec![Value::Integer(2), Value::Text("b".to_string())],
        vec![Value::Integer(1), Value::Text("c".to_string())],
        vec![Value::Integer(3), Value::Text("d".to_string())],
    ];
    let probe = vec![
        vec![Value::Integer(1), Value::Integer(100)],
        vec![Value::Integer(2), Value::Integer(200)],
        vec![Value::Integer(1), Value::Integer(150)],
    ];

    let p = ParallelHashJoin::new(4);
    assert!(p.execute_correctness_check(build, 0, probe, 0));
}

#[test]
fn test_correctness_n1_vs_n4_larger() {
    // Larger test with 100 rows on each side
    let build: Vec<Vec<Value>> = (0..100)
        .map(|i| {
            vec![
                Value::Integer((i % 10) as i64),
                Value::Text(format!("b{}", i)),
            ]
        })
        .collect();
    let probe: Vec<Vec<Value>> = (0..50)
        .map(|i| vec![Value::Integer((i % 7) as i64), Value::Integer(i * 10)])
        .collect();

    let p = ParallelHashJoin::new(4);
    assert!(p.execute_correctness_check(build, 0, probe, 0));
}

#[test]
fn test_partition_deterministic_distribution() {
    // Same key should always hash to the same partition
    let p = ParallelHashJoin::new(4);
    let partitions1 = p.partition_relation(
        vec![vec![Value::Integer(42), Value::Text("a".to_string())]],
        0,
    );
    let partitions2 = p.partition_relation(
        vec![vec![Value::Integer(42), Value::Text("b".to_string())]],
        0,
    );
    let i1 = partitions1
        .iter()
        .position(|p| p.total_rows() == 1)
        .unwrap();
    let i2 = partitions2
        .iter()
        .position(|p| p.total_rows() == 1)
        .unwrap();
    assert_eq!(i1, i2);
}

#[test]
fn test_partition_row_count_preserved() {
    let p = ParallelHashJoin::new(4);
    let n = 100;
    let rows: Vec<Vec<Value>> = (0..n)
        .map(|i| vec![Value::Integer(i as i64), Value::Text(format!("row{}", i))])
        .collect();
    let partitions = p.partition_relation(rows, 0);
    let total: usize = partitions.iter().map(|p| p.total_rows()).sum();
    assert_eq!(total, n, "All rows should be in some partition");
}

#[test]
fn test_empty_inputs() {
    let p = ParallelHashJoin::new(4);
    let result = p.execute(vec![], 0, vec![], 0);
    assert!(result.is_empty());
}

#[test]
fn test_partition_basic_api() {
    let mut p = Partition::new();
    p.add(JoinKey("a".to_string()), vec![Value::Integer(1)]);
    p.add(JoinKey("a".to_string()), vec![Value::Integer(2)]);
    p.add(JoinKey("b".to_string()), vec![Value::Integer(3)]);
    assert_eq!(p.unique_keys(), 2);
    assert_eq!(p.total_rows(), 3);
}

#[test]
fn test_multi_dimensional_correctness() {
    // Join with multiple columns (compound key)
    // key is composite of [k, v] - we use index 0 only for simplicity
    let p = ParallelHashJoin::new(4);

    let build = vec![
        vec![Value::Integer(1), Value::Text("x".to_string())],
        vec![Value::Integer(2), Value::Text("y".to_string())],
        vec![Value::Integer(1), Value::Text("z".to_string())],
    ];
    let probe = vec![
        vec![Value::Integer(1), Value::Integer(10)],
        vec![Value::Integer(2), Value::Integer(20)],
    ];

    let r1 = ParallelHashJoin::new(1).execute(build.clone(), 0, probe.clone(), 0);
    let r4 = ParallelHashJoin::new(4).execute(build, 0, probe, 0);

    // Sort both for comparison
    let mut r1_sorted = r1.clone();
    let mut r4_sorted = r4.clone();
    r1_sorted.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    r4_sorted.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    assert_eq!(r1_sorted, r4_sorted);
}
