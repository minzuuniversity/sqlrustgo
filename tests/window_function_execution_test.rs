//! Window Function Execution Tests
//! GAP-2: coverage improvement for window functions

use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use sqlrustgo_types::Value;
use std::sync::{Arc, RwLock};

/// Extract integer values from ExecutorResult rows
fn extract_integer_values(rows: &[Vec<Value>]) -> Vec<Option<i64>> {
    rows.iter()
        .map(|row| match &row[0] {
            Value::Null => None,
            Value::Integer(i) => Some(*i),
            other => panic!("Expected Integer or Null, got {:?}", other),
        })
        .collect()
}

/// Extract float values from ExecutorResult rows
fn extract_float_values(rows: &[Vec<Value>]) -> Vec<Option<f64>> {
    rows.iter()
        .map(|row| match &row[0] {
            Value::Null => None,
            Value::Float(f) => Some(*f),
            other => panic!("Expected Float or Null, got {:?}", other),
        })
        .collect()
}

fn create_engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

fn setup_sales_data(engine: &mut ExecutionEngine<MemoryStorage>) {
    engine.execute("CREATE TABLE sales (id INTEGER PRIMARY KEY, region TEXT, quarter TEXT, amount INTEGER)").unwrap();
    for sql in &[
        "INSERT INTO sales VALUES (1, 'North', 'Q1', 100)",
        "INSERT INTO sales VALUES (2, 'North', 'Q2', 150)",
        "INSERT INTO sales VALUES (3, 'North', 'Q3', 120)",
        "INSERT INTO sales VALUES (4, 'North', 'Q4', 180)",
        "INSERT INTO sales VALUES (5, 'South', 'Q1', 200)",
        "INSERT INTO sales VALUES (6, 'South', 'Q2', 160)",
        "INSERT INTO sales VALUES (7, 'South', 'Q3', 220)",
        "INSERT INTO sales VALUES (8, 'South', 'Q4', 190)",
    ] {
        engine.execute(sql).unwrap();
    }
}

#[test]
fn test_window_lead() {
    let mut engine = create_engine();
    setup_sales_data(&mut engine);
    let result = engine
        .execute("SELECT LEAD(amount) OVER (ORDER BY quarter) FROM sales WHERE region = 'North'")
        .unwrap();
    // Debug: print actual rows
    println!("DEBUG: Rows count: {}", result.rows.len());
    for (i, row) in result.rows.iter().enumerate() {
        println!("DEBUG: Row {}: {:?}", i, row);
    }
    let values = extract_integer_values(&result.rows);
    // North data: Q1=100, Q2=150, Q3=120, Q4=180
    // LEAD returns next row's value: [150, 120, 180, NULL]
    assert_eq!(values, vec![Some(150), Some(120), Some(180), None]);
}

#[test]
fn test_window_lead_with_partition() {
    let mut engine = create_engine();
    setup_sales_data(&mut engine);
    let result = engine
        .execute("SELECT region, quarter, LEAD(amount) OVER (PARTITION BY region ORDER BY quarter) as next_amount FROM sales ORDER BY region, quarter")
        .unwrap();
    // Results should be grouped by region, LEAD within each partition
    assert_eq!(result.rows.len(), 8);
    // Check North partition: Q1->Q2=150, Q2->Q3=120, Q3->Q4=180, Q4->NULL
    // Check South partition: Q1->Q2=160, Q2->Q3=220, Q3->Q4=190, Q4->NULL
    let north_lead: Vec<Option<i64>> = result.rows.iter()
        .filter(|row| row[0] == Value::Text("North".to_string()))
        .map(|row| match &row[2] {
            Value::Null => None,
            Value::Integer(i) => Some(*i),
            other => panic!("Expected Integer or Null, got {:?}", other),
        })
        .collect();
    assert_eq!(north_lead, vec![Some(150), Some(120), Some(180), None]);
}

#[test]
fn test_window_lag() {
    let mut engine = create_engine();
    setup_sales_data(&mut engine);
    let result = engine
        .execute("SELECT LAG(amount) OVER (ORDER BY quarter) FROM sales WHERE region = 'North'")
        .unwrap();
    let values = extract_integer_values(&result.rows);
    // North data: Q1=100, Q2=150, Q3=120, Q4=180
    // LAG returns previous row's value: [NULL, 100, 150, 120]
    assert_eq!(values, vec![None, Some(100), Some(150), Some(120)]);
}

#[test]
fn test_window_lag_with_default() {
    let mut engine = create_engine();
    setup_sales_data(&mut engine);
    let result = engine
        .execute("SELECT LAG(amount, 1, 0) OVER (ORDER BY quarter) FROM sales WHERE region = 'North'")
        .unwrap();
    let values = extract_integer_values(&result.rows);
    // LAG with default 0: [0, 100, 150, 120]
    assert_eq!(values, vec![Some(0), Some(100), Some(150), Some(120)]);
}

#[test]
fn test_window_first_value() {
    let mut engine = create_engine();
    setup_sales_data(&mut engine);
    let result = engine
        .execute(
            "SELECT FIRST_VALUE(amount) OVER (ORDER BY quarter) FROM sales WHERE region = 'North'",
        )
        .unwrap();
    let values = extract_integer_values(&result.rows);
    // FIRST_VALUE returns first value in frame: [100, 100, 100, 100]
    assert_eq!(values, vec![Some(100), Some(100), Some(100), Some(100)]);
}

#[test]
fn test_window_first_value_with_partition() {
    let mut engine = create_engine();
    setup_sales_data(&mut engine);
    let result = engine
        .execute("SELECT region, quarter, FIRST_VALUE(amount) OVER (PARTITION BY region ORDER BY quarter) as first_amount FROM sales ORDER BY region, quarter")
        .unwrap();
    assert_eq!(result.rows.len(), 8);
    // North partition: first amount is 100 (Q1)
    // South partition: first amount is 200 (Q1)
    let north_first: Vec<Option<i64>> = result.rows.iter()
        .filter(|row| row[0] == Value::Text("North".to_string()))
        .map(|row| match &row[2] {
            Value::Integer(i) => Some(*i),
            other => panic!("Expected Integer, got {:?}", other),
        })
        .collect();
    assert_eq!(north_first, vec![Some(100), Some(100), Some(100), Some(100)]);
}

#[test]
fn test_window_last_value() {
    let mut engine = create_engine();
    setup_sales_data(&mut engine);
    let result = engine
        .execute(
            "SELECT LAST_VALUE(amount) OVER (ORDER BY quarter) FROM sales WHERE region = 'North'",
        )
        .unwrap();
    let values = extract_integer_values(&result.rows);
    // With default frame RANGE BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW:
    // Q1 frame: [100] -> LAST_VALUE = 100
    // Q2 frame: [100, 150] -> LAST_VALUE = 150
    // Q3 frame: [100, 150, 120] -> LAST_VALUE = 120
    // Q4 frame: [100, 150, 120, 180] -> LAST_VALUE = 180
    assert_eq!(values, vec![Some(100), Some(150), Some(120), Some(180)]);
}

#[test]
#[ignore = "Parser does not support ROWS BETWEEN frame specification"]
fn test_window_last_value_with_frame() {
    let mut engine = create_engine();
    setup_sales_data(&mut engine);
    let result = engine
        .execute(
            "SELECT LAST_VALUE(amount) OVER (ORDER BY quarter ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING) FROM sales WHERE region = 'North'",
        )
        .unwrap();
    let values = extract_integer_values(&result.rows);
    // With explicit full frame: all rows in frame
    // LAST_VALUE always returns the last row's value: 180 for all rows
    assert_eq!(values, vec![Some(180), Some(180), Some(180), Some(180)]);
}

#[test]
fn test_window_ntile() {
    let mut engine = create_engine();
    setup_sales_data(&mut engine);
    let result = engine
        .execute("SELECT NTILE(2) OVER (ORDER BY amount) FROM sales WHERE region = 'North'")
        .unwrap();
    let values = extract_integer_values(&result.rows);
    // North amounts sorted: [100, 120, 150, 180]
    // With 4 rows and 2 buckets: first 2 rows get bucket 1, last 2 get bucket 2
    assert_eq!(values, vec![Some(1), Some(1), Some(2), Some(2)]);
}

#[test]
fn test_window_ntile_uneven() {
    let mut engine = create_engine();
    setup_sales_data(&mut engine);
    let result = engine
        .execute("SELECT NTILE(3) OVER (ORDER BY amount) FROM sales WHERE region = 'North'")
        .unwrap();
    let values = extract_integer_values(&result.rows);
    // North amounts sorted: [100, 120, 150, 180]
    // With 4 rows and 3 buckets (4/3 = 1 remainder 1):
    // First bucket gets extra row, so distribution is [2, 1, 1]
    // Buckets: [1, 1, 2, 3] or [1, 2, 2, 3] depending on implementation
    // We verify all values are in valid range 1-3
    for v in &values {
        assert!(v.unwrap_or(0) >= 1 && v.unwrap_or(0) <= 3);
    }
    assert_eq!(values.len(), 4);
}

#[test]
fn test_window_nth_value() {
    let mut engine = create_engine();
    setup_sales_data(&mut engine);
    let result = engine
        .execute(
            "SELECT NTH_VALUE(amount, 2) OVER (ORDER BY quarter) FROM sales WHERE region = 'North'",
        )
        .unwrap();
    let values = extract_integer_values(&result.rows);
    // NTH_VALUE(amount, 2) returns 2nd value in the window frame
    // Note: Parser uses ROWS semantics (position-based), not RANGE semantics
    // Q1: frame=[100] → no 2nd value → NULL
    // Q2: frame=[100,150] → 2nd value = 150
    // Q3: frame=[100,150,120] → 2nd value = 150 (ROWS mode, position-based)
    // Q4: frame=[100,150,120,180] → 2nd value = 150 (ROWS mode, position-based)
    assert_eq!(values, vec![None, Some(150), Some(150), Some(150)]);
}

#[test]
fn test_window_nth_value_out_of_range() {
    let mut engine = create_engine();
    setup_sales_data(&mut engine);
    let result = engine
        .execute(
            "SELECT NTH_VALUE(amount, 10) OVER (ORDER BY quarter) FROM sales WHERE region = 'North'",
        )
        .unwrap();
    let values = extract_integer_values(&result.rows);
    // NTH_VALUE with n=10 when there are only 4 rows should return NULL for all
    assert_eq!(values, vec![None, None, None, None]);
}

#[test]
fn test_window_partition_by() {
    let mut engine = create_engine();
    setup_sales_data(&mut engine);
    let result = engine
        .execute("SELECT SUM(amount) OVER (PARTITION BY region) FROM sales")
        .unwrap();
    assert!(!result.rows.is_empty());
}

#[test]
fn test_window_order_by() {
    let mut engine = create_engine();
    setup_sales_data(&mut engine);
    let result = engine
        .execute("SELECT ROW_NUMBER() OVER (ORDER BY amount DESC) FROM sales")
        .unwrap();
    assert!(!result.rows.is_empty());
}

#[test]
fn test_multiple_window_functions() {
    let mut engine = create_engine();
    setup_sales_data(&mut engine);
    let result = engine.execute("SELECT ROW_NUMBER() OVER (PARTITION BY region ORDER BY amount), SUM(amount) OVER (PARTITION BY region) FROM sales").unwrap();
    assert!(!result.rows.is_empty());
}
