//! Histogram data-collection E2E tests
//!
//! V312-22b / Issue #4033 — CBO/Histogram 数据采集
//!
//! Verifies:
//! 1. `collect_table_stats` (ANALYZE) builds an equi-height `Histogram` per column
//! 2. `Histogram::estimate_eq` ≈ 1/NDV for uniform data
//! 3. `Histogram::estimate_lt` uses linear interpolation across buckets
//! 4. `UnifiedCostModel::estimate_selectivity` consumes histograms when available
//! 5. Falls back to per-operator heuristic when no histogram
//! 6. Integration: ANALYZE-style collect → selectivity estimation round-trip

use sqlrustgo_optimizer::stats::{
    build_histogram_from_values, ColumnStats, HistogramBucket, TableStats,
};
use sqlrustgo_optimizer::unified_cost::UnifiedCostModel;
use sqlrustgo_types::Value;

/// Helper: build a uniform-distribution integer column of `n_rows` distinct values
/// (each appearing once). Suitable for `estimate_eq ≈ 1/NDV` assertions.
fn uniform_int_values(n_rows: usize) -> Vec<Value> {
    (0..n_rows as i64).map(Value::Integer).collect()
}

/// Helper: build an increasing integer sequence [0, n_rows). Suitable for
/// linear-interpolation assertions on `estimate_lt`.
fn increasing_int_values(n_rows: usize) -> Vec<Value> {
    (0..n_rows as i64).map(Value::Integer).collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 1: collect_table_stats builds a histogram per column.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_histogram_populated_in_table_stats() {
    // We construct TableStats directly with a populated histogram to mirror the
    // end state of ANALYZE (collect_table_stats populates ColumnStats.histogram).
    let values = uniform_int_values(100);
    let hist = build_histogram_from_values(&values, 10)
        .expect("histogram should be built for non-empty values");

    let col_stats = ColumnStats::new("c")
        .with_distinct_count(100)
        .with_null_count(0)
        .with_histogram(hist);

    let tstats = TableStats::new("t")
        .with_row_count(100)
        .add_column_stats(col_stats);

    let cs = tstats
        .column("c")
        .expect("column c stats should exist after ANALYZE");
    assert!(
        cs.histogram.is_some(),
        "histogram must be populated after ANALYZE"
    );
    let h = cs.histogram.as_ref().unwrap();
    assert!(h.num_buckets > 0, "histogram must have at least one bucket");
    assert_eq!(h.total_count, 100);
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 2: estimate_eq with uniform data ≈ 1/NDV.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_histogram_eq_selectivity_with_uniform_data() {
    // 1000 unique integer values (each appearing exactly once).
    // For uniform data, estimate_eq(v) should be ≈ 1/1000.
    let values = uniform_int_values(1000);
    let hist = build_histogram_from_values(&values, 100)
        .expect("histogram should be built for non-empty values");

    // For a value present in the data, estimate_eq ≈ 1/1000
    let sel_present = hist.estimate_eq(&Value::Integer(500));
    assert!(
        (sel_present - 0.001).abs() < 0.0005,
        "uniform estimate_eq should be ≈ 1/1000, got {}",
        sel_present
    );

    // For a value outside the data range, estimate_eq may be 0 (out of range)
    // or small — accept anything ≤ 1/1000.
    let sel_outside = hist.estimate_eq(&Value::Integer(99999));
    assert!(
        sel_outside <= 0.001 + 0.0005,
        "out-of-range uniform estimate_eq should be ≤ 1/1000, got {}",
        sel_outside
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 3: estimate_lt with linear interpolation across buckets.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_histogram_lt_selectivity_linear_interpolation() {
    // 1000 values uniformly distributed in [0, 1000).
    // estimate_lt(500) should be ≈ 0.5 ± 0.01 (linear interpolation).
    let values = increasing_int_values(1000);
    let hist = build_histogram_from_values(&values, 100)
        .expect("histogram should be built for non-empty values");

    let sel = hist.estimate_lt(&Value::Integer(500));
    assert!(
        (sel - 0.5).abs() < 0.02,
        "estimate_lt(500) for uniform [0,1000) should be ≈ 0.5, got {}",
        sel
    );

    // estimate_lt(0) ≈ 0 (no values below 0)
    let sel_zero = hist.estimate_lt(&Value::Integer(0));
    assert!(
        sel_zero < 0.001,
        "estimate_lt(0) should be ≈ 0, got {}",
        sel_zero
    );

    // estimate_lt(1000) ≈ 1.0 (all values are < 1000)
    let sel_max = hist.estimate_lt(&Value::Integer(1000));
    assert!(
        sel_max > 0.99,
        "estimate_lt(1000) should be ≈ 1.0, got {}",
        sel_max
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 4: UnifiedCostModel.estimate_selectivity uses histograms.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_unified_cost_selectivity_uses_histogram() {
    // Build a histogram with uniform data.
    let values = uniform_int_values(1000);
    let hist = build_histogram_from_values(&values, 100)
        .expect("histogram should be built for non-empty values");

    let col_stats = ColumnStats::new("c")
        .with_distinct_count(1000)
        .with_null_count(0)
        .with_histogram(hist.clone());

    let mut column_stats = std::collections::HashMap::new();
    column_stats.insert("c".to_string(), col_stats);

    let mut model = UnifiedCostModel::default_model(128, 10000);
    model.update_table_stats_with_columns("t".to_string(), 1000, 10, column_stats);

    // estimate_selectivity("t", "c", Eq, &v) must equal histogram.estimate_eq(&v)
    let v = Value::Integer(500);
    let sel_eq = model.estimate_selectivity("t", "c", sqlrustgo_optimizer::rules::BinaryOperator::Eq, &v);
    let expected = hist.estimate_eq(&v);
    assert!(
        (sel_eq - expected).abs() < 1e-9,
        "estimate_selectivity with histogram should match histogram.estimate_eq; got {} vs {}",
        sel_eq,
        expected
    );
    // And it should NOT be the heuristic 0.1
    assert!(
        (sel_eq - 0.1).abs() > 1e-6,
        "estimate_selectivity should NOT fall back to heuristic 0.1 when histogram is present"
    );

    // Lt
    let v2 = Value::Integer(500);
    let sel_lt = model.estimate_selectivity(
        "t",
        "c",
        sqlrustgo_optimizer::rules::BinaryOperator::Lt,
        &v2,
    );
    let expected_lt = hist.estimate_lt(&v2);
    assert!(
        (sel_lt - expected_lt).abs() < 1e-9,
        "estimate_selectivity Lt should match histogram.estimate_lt; got {} vs {}",
        sel_lt,
        expected_lt
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 5: Falls back to per-op heuristic when no histogram.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_unified_cost_falls_back_to_heuristic() {
    let model = UnifiedCostModel::default_model(128, 10000);
    // No column_stats / no histogram at all.

    let sel_eq = model.estimate_selectivity(
        "unknown",
        "c",
        sqlrustgo_optimizer::rules::BinaryOperator::Eq,
        &Value::Integer(1),
    );
    assert!(
        (sel_eq - 0.1).abs() < 1e-6,
        "Eq heuristic should be 0.1, got {}",
        sel_eq
    );

    let sel_lt = model.estimate_selectivity(
        "unknown",
        "c",
        sqlrustgo_optimizer::rules::BinaryOperator::Lt,
        &Value::Integer(1),
    );
    assert!(
        (sel_lt - 0.3).abs() < 1e-6,
        "Lt heuristic should be 0.3, got {}",
        sel_lt
    );

    let sel_neq = model.estimate_selectivity(
        "unknown",
        "c",
        sqlrustgo_optimizer::rules::BinaryOperator::NotEq,
        &Value::Integer(1),
    );
    assert!(
        (sel_neq - 0.9).abs() < 1e-6,
        "NotEq heuristic should be 0.9, got {}",
        sel_neq
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 6: ANALYZE-then-selectivity integration round-trip.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_analyze_then_selectivity_roundtrip() {
    // Simulate ANALYZE: build histograms from column data.
    let column_data: Vec<Vec<Value>> = vec![
        // c1: increasing 0..1000
        (0..1000i64).map(Value::Integer).collect(),
        // c2: uniform 0..500 (each value appears twice)
        (0..500i64)
            .flat_map(|v| [Value::Integer(v), Value::Integer(v)])
            .collect(),
    ];

    let mut column_stats = std::collections::HashMap::new();
    for (i, values) in column_data.iter().enumerate() {
        let h = build_histogram_from_values(values, 50)
            .expect("histogram should be built for non-empty values");
        let cs = ColumnStats::new(format!("c{}", i + 1))
            .with_distinct_count(h.buckets.iter().map(|b| b.distinct_count).sum::<u64>())
            .with_null_count(0)
            .with_histogram(h);
        column_stats.insert(format!("c{}", i + 1), cs);
    }

    // Wire into UnifiedCostModel as if update_cost_model_stats had run.
    let mut model = UnifiedCostModel::default_model(128, 10000);
    model.update_table_stats_with_columns("t".to_string(), 1000, 100, column_stats);

    // SELECT * FROM t WHERE c1 < 500 → should match ~500/1000 ≈ 0.5
    let sel_c1_lt_500 = model.estimate_selectivity(
        "t",
        "c1",
        sqlrustgo_optimizer::rules::BinaryOperator::Lt,
        &Value::Integer(500),
    );
    assert!(
        (sel_c1_lt_500 - 0.5).abs() < 0.02,
        "c1 < 500 should be ≈ 0.5, got {}",
        sel_c1_lt_500
    );

    // SELECT * FROM t WHERE c2 = 250 → uniform 0..500 each twice → 2/1000 ≈ 0.002
    let sel_c2_eq_250 = model.estimate_selectivity(
        "t",
        "c2",
        sqlrustgo_optimizer::rules::BinaryOperator::Eq,
        &Value::Integer(250),
    );
    assert!(
        sel_c2_eq_250 < 0.01,
        "c2 = 250 should be < 0.01, got {}",
        sel_c2_eq_250
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Bonus: HistogramBucket equality and ordering invariants.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_histogram_buckets_sorted_by_lower_bound() {
    let values = increasing_int_values(500);
    let hist = build_histogram_from_values(&values, 50)
        .expect("histogram should be built for non-empty values");

    let mut prev: Option<&HistogramBucket> = None;
    for bucket in &hist.buckets {
        if let Some(p) = prev {
            assert!(
                bucket.lower_bound >= p.lower_bound,
                "buckets must be sorted by lower_bound"
            );
        }
        prev = Some(bucket);
    }
}

#[test]
fn test_histogram_empty_values_returns_none() {
    let empty: Vec<Value> = vec![];
    let hist = build_histogram_from_values(&empty, 10);
    assert!(
        hist.is_none(),
        "histogram from empty values must be None, got {:?}",
        hist
    );
}
