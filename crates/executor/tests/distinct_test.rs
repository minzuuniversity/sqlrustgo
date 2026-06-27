//! DISTINCT Execution Tests (EXEC-06, #2972)
//!
//! End-to-end tests verifying that `SELECT DISTINCT` actually deduplicates
//! rows at execution time. The parser already sets `select.distinct = true`
//! on `SelectStatement`; this test file exercises the executor path in
//! `src/engine_select.rs` (Step 6) which applies the dedup via a HashSet
//! over projected `Vec<Value>` rows.
//!
//! These tests live in `crates/executor/tests/` so they are picked up by
//! `cargo test -p sqlrustgo-executor` (the executor crate's integration
//! test directory). Tests placed in `crates/executor/src/local_executor.rs`
//! are dead code — that file is NOT in the executor's lib.rs mod tree.

use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use sqlrustgo_types::Value;
use std::sync::{Arc, RwLock};

fn create_engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

#[test]
fn test_distinct_single_column() {
    // SELECT DISTINCT col FROM t must collapse duplicate values.
    let mut engine = create_engine();
    engine
        .execute("CREATE TABLE users (id INTEGER, country TEXT)")
        .unwrap();
    engine
        .execute(
            "INSERT INTO users VALUES \
             (1, 'US'), (2, 'US'), (3, 'JP'), (4, 'JP'), (5, 'US'), (6, 'CN')",
        )
        .unwrap();

    let result = engine
        .execute("SELECT DISTINCT country FROM users")
        .unwrap();

    // 3 unique countries: US, JP, CN
    assert_eq!(
        result.rows.len(),
        3,
        "expected 3 distinct countries, got {} rows: {:?}",
        result.rows.len(),
        result.rows
    );

    // Each row should be a single column (the projected country).
    for row in &result.rows {
        assert_eq!(
            row.len(),
            1,
            "DISTINCT single col should yield 1 column, got row {:?}",
            row
        );
    }

    // Collect the distinct values and verify the set is exactly {US, JP, CN}.
    let mut seen: Vec<String> = result
        .rows
        .iter()
        .map(|r| match &r[0] {
            Value::Text(s) => s.clone(),
            other => panic!("expected Text, got {:?}", other),
        })
        .collect();
    seen.sort();
    assert_eq!(seen, vec!["CN", "JP", "US"]);
}

#[test]
fn test_distinct_multiple_columns() {
    // SELECT DISTINCT col1, col2 FROM t must collapse rows that agree on
    // the entire projected tuple, not just the first column.
    let mut engine = create_engine();
    engine
        .execute("CREATE TABLE orders (id INTEGER, region TEXT, status TEXT)")
        .unwrap();
    engine
        .execute(
            "INSERT INTO orders VALUES \
             (1, 'NA', 'NEW'), \
             (2, 'NA', 'NEW'), \
             (3, 'NA', 'DONE'), \
             (4, 'EU', 'NEW'), \
             (5, 'EU', 'NEW'), \
             (6, 'NA', 'NEW')",
        )
        .unwrap();

    let result = engine
        .execute("SELECT DISTINCT region, status FROM orders")
        .unwrap();

    // Distinct tuples: (NA,NEW), (NA,DONE), (EU,NEW) -> 3 rows
    assert_eq!(
        result.rows.len(),
        3,
        "expected 3 distinct (region,status) tuples, got {} rows: {:?}",
        result.rows.len(),
        result.rows
    );

    for row in &result.rows {
        assert_eq!(
            row.len(),
            2,
            "DISTINCT two cols should yield 2 columns, got row {:?}",
            row
        );
    }

    // Verify the exact set of tuples.
    let mut tuples: Vec<(String, String)> = result
        .rows
        .iter()
        .map(|r| {
            (
                match &r[0] {
                    Value::Text(s) => s.clone(),
                    other => panic!("expected Text, got {:?}", other),
                },
                match &r[1] {
                    Value::Text(s) => s.clone(),
                    other => panic!("expected Text, got {:?}", other),
                },
            )
        })
        .collect();
    tuples.sort();
    assert_eq!(
        tuples,
        vec![
            ("EU".to_string(), "NEW".to_string()),
            ("NA".to_string(), "DONE".to_string()),
            ("NA".to_string(), "NEW".to_string()),
        ]
    );
}

#[test]
fn test_distinct_no_duplicates_preserves_all_rows() {
    // When no duplicates exist, DISTINCT must be a no-op (all rows pass through).
    let mut engine = create_engine();
    engine
        .execute("CREATE TABLE t (id INTEGER, name TEXT)")
        .unwrap();
    engine
        .execute("INSERT INTO t VALUES (1, 'a'), (2, 'b'), (3, 'c')")
        .unwrap();

    let result = engine.execute("SELECT DISTINCT name FROM t").unwrap();
    assert_eq!(result.rows.len(), 3);
}

#[test]
fn test_distinct_all_same_collapses_to_one() {
    // When every row is the same, DISTINCT must return exactly one row.
    let mut engine = create_engine();
    engine
        .execute("CREATE TABLE t (id INTEGER, val TEXT)")
        .unwrap();
    engine
        .execute("INSERT INTO t VALUES (1, 'x'), (2, 'x'), (3, 'x'), (4, 'x')")
        .unwrap();

    let result = engine.execute("SELECT DISTINCT val FROM t").unwrap();
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0], vec![Value::Text("x".to_string())]);
}

#[test]
fn test_select_without_distinct_returns_duplicates() {
    // Regression guard: plain SELECT (no DISTINCT) must NOT dedupe.
    // This is what makes Step 6 in engine_select.rs a real behavior change
    // gated on `select.distinct` rather than unconditional.
    let mut engine = create_engine();
    engine.execute("CREATE TABLE t (val TEXT)").unwrap();
    engine
        .execute("INSERT INTO t VALUES ('a'), ('a'), ('b')")
        .unwrap();

    let result = engine.execute("SELECT val FROM t").unwrap();
    assert_eq!(result.rows.len(), 3, "plain SELECT must not dedupe");
}
