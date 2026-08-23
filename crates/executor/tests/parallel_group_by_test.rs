//! Parallel GROUP BY integration tests
//!
//! v3.10.0 Issue #3703 Phase 3 follow-up for #3736.

use sqlrustgo_executor::parallel_group_by::{
    compute_group_keys, evaluate_simple_expr, ParallelGroupBy, PartialAggregate,
};
use sqlrustgo_parser::{AggregateCall, AggregateFunction, Expression};
use sqlrustgo_storage::TableInfo;
use sqlrustgo_types::Value;

fn make_table_info() -> TableInfo {
    TableInfo {
        name: "t".to_string(),
        columns: vec![
            sqlrustgo_storage::ColumnDefinition {
                name: "k".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                collation: None,
                default_value: None,
                auto_increment: false,
            },
            sqlrustgo_storage::ColumnDefinition {
                name: "v".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                collation: None,
                default_value: None,
                auto_increment: false,
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

fn count_call() -> AggregateCall {
    AggregateCall {
        func: AggregateFunction::Count,
        args: vec![],
        distinct: false,
    }
}

fn sum_call(col: &str) -> AggregateCall {
    AggregateCall {
        func: AggregateFunction::Sum,
        args: vec![Expression::Identifier(col.to_string())],
        distinct: false,
    }
}

fn avg_call(col: &str) -> AggregateCall {
    AggregateCall {
        func: AggregateFunction::Avg,
        args: vec![Expression::Identifier(col.to_string())],
        distinct: false,
    }
}

#[test]
fn test_parallel_group_by_sum_correctness() {
    // 1000 rows with 10 distinct groups (i % 10)
    let n = 1000;
    let rows: Vec<Vec<Value>> = (0..n)
        .map(|i| vec![Value::Integer((i % 10) as i64), Value::Integer(i as i64)])
        .collect();

    let table_info = make_table_info();
    let group_keys = compute_group_keys(
        &rows,
        &[Expression::Identifier("k".to_string())],
        &table_info,
    );

    let pg = ParallelGroupBy::new(4);
    let result = pg.execute(rows, group_keys, &[sum_call("v")], &table_info);

    assert_eq!(result.len(), 10, "Should have 10 distinct groups");

    // Total sum should be 0+1+2+...+(n-1) = n*(n-1)/2 = 499500
    let total: i64 = result
        .iter()
        .map(|(_, vals)| {
            if let Value::Integer(n) = &vals[0] {
                *n
            } else {
                0
            }
        })
        .sum();
    assert_eq!(total, (n * (n - 1) / 2) as i64);
}

#[test]
fn test_parallel_group_by_correctness_n1_vs_n4() {
    // Cell-level equivalence: same query with N=1 and N=4 must give same results
    let n = 500;
    let rows: Vec<Vec<Value>> = (0..n)
        .map(|i| {
            vec![
                Value::Integer((i % 7) as i64),
                Value::Integer((i * 3) as i64),
            ]
        })
        .collect();

    let table_info = make_table_info();
    let aggs = vec![count_call(), sum_call("v")];

    // Sequential (N=1)
    let keys_seq = compute_group_keys(
        &rows,
        &[Expression::Identifier("k".to_string())],
        &table_info,
    );
    let pg_seq = ParallelGroupBy::new(1);
    let seq_result = pg_seq.execute(rows.clone(), keys_seq.clone(), &aggs, &table_info);

    // Parallel (N=4)
    let keys_par = compute_group_keys(
        &rows,
        &[Expression::Identifier("k".to_string())],
        &table_info,
    );
    let pg_par = ParallelGroupBy::new(4);
    let par_result = pg_par.execute(rows, keys_par, &aggs, &table_info);

    // Both should have 7 groups (i % 7)
    assert_eq!(seq_result.len(), 7);
    assert_eq!(par_result.len(), 7);

    // Aggregate values should match (bit-exact for integers)
    let mut seq_sorted: Vec<_> = seq_result.iter().collect();
    seq_sorted.sort_by_key(|(k, _)| k.0.clone());
    let mut par_sorted: Vec<_> = par_result.iter().collect();
    par_sorted.sort_by_key(|(k, _)| k.0.clone());

    for (s, p) in seq_sorted.iter().zip(par_sorted.iter()) {
        assert_eq!(s.0 .0, p.0 .0, "Group keys should match");
        assert_eq!(s.1[0], p.1[0], "COUNT should match");
        assert_eq!(s.1[1], p.1[1], "SUM should match");
    }
}

#[test]
fn test_parallel_group_by_avg_correctness() {
    // AVG correctness: same query with N=1 and N=4 must give same average
    let n = 500;
    let rows: Vec<Vec<Value>> = (0..n)
        .map(|i| {
            vec![
                Value::Integer((i % 5) as i64),
                Value::Integer((i + 1) as i64),
            ]
        })
        .collect();

    let table_info = make_table_info();
    let aggs = vec![avg_call("v"), count_call()];

    // Sequential
    let keys_seq = compute_group_keys(
        &rows,
        &[Expression::Identifier("k".to_string())],
        &table_info,
    );
    let pg_seq = ParallelGroupBy::new(1);
    let seq_result = pg_seq.execute(rows.clone(), keys_seq.clone(), &aggs, &table_info);

    // Parallel
    let keys_par = compute_group_keys(
        &rows,
        &[Expression::Identifier("k".to_string())],
        &table_info,
    );
    let pg_par = ParallelGroupBy::new(4);
    let par_result = pg_par.execute(rows, keys_par, &aggs, &table_info);

    // 5 groups
    assert_eq!(seq_result.len(), 5);
    assert_eq!(par_result.len(), 5);

    // Sort by group key for comparison
    let mut seq_sorted: Vec<_> = seq_result.iter().collect();
    seq_sorted.sort_by_key(|(k, _)| k.0.clone());
    let mut par_sorted: Vec<_> = par_result.iter().collect();
    par_sorted.sort_by_key(|(k, _)| k.0.clone());

    for (s, p) in seq_sorted.iter().zip(par_sorted.iter()) {
        assert_eq!(s.0 .0, p.0 .0);
        // AVG and COUNT must match within 1e-9 floating-point tolerance
        let s_avg = match &s.1[0] {
            Value::Float(f) => *f,
            _ => panic!("Expected Float"),
        };
        let p_avg = match &p.1[0] {
            Value::Float(f) => *f,
            _ => panic!("Expected Float"),
        };
        assert!(
            (s_avg - p_avg).abs() < 1e-9,
            "AVG mismatch: seq={} par={}",
            s_avg,
            p_avg
        );
        assert_eq!(s.1[1], p.1[1], "COUNT must match exactly");
    }
}

#[test]
fn test_compute_group_keys_basic() {
    let rows = vec![
        vec![Value::Integer(1), Value::Text("a".to_string())],
        vec![Value::Integer(1), Value::Text("b".to_string())],
        vec![Value::Integer(2), Value::Text("c".to_string())],
    ];
    let table_info = make_table_info();
    let keys = compute_group_keys(
        &rows,
        &[Expression::Identifier("k".to_string())],
        &table_info,
    );
    // First two rows should have same key (k=1), last should be different (k=2)
    assert_eq!(keys.len(), 3);
    assert_eq!(keys[0], keys[1]);
    assert_ne!(keys[0], keys[2]);
}

#[test]
fn test_evaluate_simple_expr_identifier() {
    let row = vec![Value::Integer(42), Value::Text("hello".to_string())];
    let table_info = make_table_info();
    // Identifier "k" -> row[0]
    let v = evaluate_simple_expr(&Expression::Identifier("k".to_string()), &row, &table_info);
    assert_eq!(v, Value::Integer(42));
    // Identifier "v" -> row[1]
    let v = evaluate_simple_expr(&Expression::Identifier("v".to_string()), &row, &table_info);
    assert_eq!(v, Value::Text("hello".to_string()));
}

#[test]
fn test_evaluate_simple_expr_literal() {
    let row = vec![];
    let table_info = make_table_info();
    assert_eq!(
        evaluate_simple_expr(&Expression::Literal("42".to_string()), &row, &table_info),
        Value::Integer(42)
    );
    assert_eq!(
        evaluate_simple_expr(&Expression::Literal("3.14".to_string()), &row, &table_info),
        Value::Float(3.14)
    );
    assert_eq!(
        evaluate_simple_expr(&Expression::Literal("hello".to_string()), &row, &table_info),
        Value::Text("hello".to_string())
    );
}

#[test]
fn test_parallel_group_by_empty_input() {
    let pg = ParallelGroupBy::new(4);
    let rows: Vec<Vec<Value>> = vec![];
    let keys: Vec<sqlrustgo_executor::parallel_group_by::GroupKey> = vec![];
    let table_info = make_table_info();
    let result = pg.execute(rows, keys, &[count_call()], &table_info);
    assert!(result.is_empty());
}

#[test]
fn test_parallel_group_by_constant_key() {
    // All rows have same group key (GROUP BY 1)
    let n = 100;
    let rows: Vec<Vec<Value>> = (0..n).map(|i| vec![Value::Integer(i as i64)]).collect();

    let table_info = make_table_info();
    let keys = compute_group_keys(&rows, &[Expression::Literal("1".to_string())], &table_info);
    let pg = ParallelGroupBy::new(4);
    let result = pg.execute(rows, keys, &[count_call(), sum_call("k")], &table_info);
    // All rows should be in one group
    assert_eq!(result.len(), 1);
}
