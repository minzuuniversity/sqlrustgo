//! Window Executor Tests (VTU Phase 2)
//!
//! Tests for window_executor module:
//! - WindowVolcanoExecutor construction
//! - Window functions: ROW_NUMBER, RANK, DENSE_RANK
//! - Window aggregate functions: SUM, AVG, COUNT
//! - PARTITION BY and ORDER BY behavior

use sqlrustgo_executor::executor::{ExecutorResult, VolcanoExecutor};
use sqlrustgo_executor::window_executor::WindowVolcanoExecutor;
use sqlrustgo_planner::{DataType, Expr, Field, Schema, SortExpr, WindowFunction};
use sqlrustgo_types::{SqlResult, Value};
use std::any::Any;
use std::sync::atomic::{AtomicUsize, Ordering};

// ============================================================================
// Mock VolcanoExecutor for testing
// ============================================================================

/// Mock child executor that returns fixed data rows
struct MockChildExecutor {
    rows: Vec<Vec<Value>>,
    position: AtomicUsize,
    schema: Schema,
    initialized: bool,
}

impl MockChildExecutor {
    fn new(rows: Vec<Vec<Value>>, schema: Schema) -> Self {
        Self {
            rows,
            position: AtomicUsize::new(0),
            schema,
            initialized: false,
        }
    }
}

impl VolcanoExecutor for MockChildExecutor {
    fn init(&mut self) -> SqlResult<()> {
        self.initialized = true;
        self.position.store(0, Ordering::SeqCst);
        Ok(())
    }

    fn next(&mut self) -> SqlResult<Option<ExecutorResult>> {
        if !self.initialized {
            self.initialized = true;
        }
        let pos = self.position.fetch_add(1, Ordering::SeqCst);
        if pos < self.rows.len() {
            Ok(Some(ExecutorResult::new(vec![self.rows[pos].clone()], 1)))
        } else {
            Ok(None)
        }
    }

    fn close(&mut self) -> SqlResult<()> {
        Ok(())
    }

    fn schema(&self) -> &Schema {
        &self.schema
    }

    fn name(&self) -> &str {
        "mock_child"
    }

    fn is_initialized(&self) -> bool {
        self.initialized
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Helper to create a simple test schema
fn test_schema(fields: &[(&str, DataType)]) -> Schema {
    Schema::new(
        fields
            .iter()
            .map(|(name, dt)| Field::new(name.to_string(), dt.clone()))
            .collect(),
    )
}

// ============================================================================
// WindowVolcanoExecutor Construction
// ============================================================================

#[test]
fn test_window_executor_new() {
    let schema = test_schema(&[("val", DataType::Integer)]);
    let child = MockChildExecutor::new(vec![vec![Value::Integer(1)]], schema.clone());
    let executor = WindowVolcanoExecutor::new(
        Box::new(child),
        vec![],
        schema.clone(),
        schema.clone(),
        vec![],
        vec![],
    );
    let _ = executor;
}

// ============================================================================
// ROW_NUMBER Without Partition
// ============================================================================

#[test]
fn test_row_number_no_partition() {
    let input_schema = test_schema(&[("val", DataType::Integer)]);
    let output_schema = test_schema(&[("val", DataType::Integer), ("rn", DataType::Integer)]);

    let rows = vec![
        vec![Value::Integer(10)],
        vec![Value::Integer(20)],
        vec![Value::Integer(30)],
    ];

    let child = MockChildExecutor::new(rows, input_schema.clone());

    let window_expr = Expr::WindowFunction {
        func: WindowFunction::RowNumber,
        args: vec![],
        partition_by: vec![],
        order_by: vec![],
        frame: None,
    };

    let mut executor = WindowVolcanoExecutor::new(
        Box::new(child),
        vec![window_expr],
        output_schema,
        input_schema,
        vec![],
        vec![],
    );

    executor.init().unwrap();
    let mut results = Vec::new();
    while let Some(batch) = executor.next().unwrap() {
        results.extend(batch.rows);
    }
    executor.close().unwrap();

    // ROW_NUMBER returns 1, 2, 3
    assert_eq!(results.len(), 3);
    // Each row should have 2 columns: original value + ROW_NUMBER
    for (i, row) in results.iter().enumerate() {
        assert_eq!(row.len(), 2);
        let expected_rn = (i + 1) as i64;
        assert_eq!(row[1], Value::Integer(expected_rn));
    }
}

// ============================================================================
// RANK and DENSE_RANK
// ============================================================================

#[test]
fn test_rank_dense_rank() {
    let input_schema = test_schema(&[("score", DataType::Integer)]);
    let output_schema = test_schema(&[
        ("score", DataType::Integer),
        ("rnk", DataType::Integer),
        ("dense_rnk", DataType::Integer),
    ]);

    let rows = vec![
        vec![Value::Integer(100)],
        vec![Value::Integer(100)],
        vec![Value::Integer(90)],
        vec![Value::Integer(80)],
    ];

    let child = MockChildExecutor::new(rows, input_schema.clone());

    let rank_expr = Expr::WindowFunction {
        func: WindowFunction::Rank,
        args: vec![],
        partition_by: vec![],
        order_by: vec![],
        frame: None,
    };

    let dense_rank_expr = Expr::WindowFunction {
        func: WindowFunction::DenseRank,
        args: vec![],
        partition_by: vec![],
        order_by: vec![],
        frame: None,
    };

    let mut executor = WindowVolcanoExecutor::new(
        Box::new(child),
        vec![rank_expr, dense_rank_expr],
        output_schema,
        input_schema,
        vec![],
        vec![],
    );

    executor.init().unwrap();
    let mut results = Vec::new();
    while let Some(batch) = executor.next().unwrap() {
        results.extend(batch.rows);
    }
    executor.close().unwrap();

    assert_eq!(results.len(), 4);
    // Each row: [score, rank, dense_rank]
    // Without ORDER BY, both RANK and DENSE_RANK return position+1
}

// ============================================================================
// SUM Window Aggregate
// ============================================================================

#[test]
fn test_sum_window_aggregate() {
    let input_schema = test_schema(&[("val", DataType::Integer)]);
    let output_schema = test_schema(&[("val", DataType::Integer), ("sum_val", DataType::Integer)]);

    let rows = vec![
        vec![Value::Integer(1)],
        vec![Value::Integer(2)],
        vec![Value::Integer(3)],
    ];

    let child = MockChildExecutor::new(rows, input_schema.clone());

    let sum_expr = Expr::WindowFunction {
        func: WindowFunction::Sum,
        args: vec![Expr::Column(sqlrustgo_planner::Column::new(
            "val".to_string(),
        ))],
        partition_by: vec![],
        order_by: vec![],
        frame: None,
    };

    let mut executor = WindowVolcanoExecutor::new(
        Box::new(child),
        vec![sum_expr],
        output_schema,
        input_schema,
        vec![],
        vec![],
    );

    executor.init().unwrap();
    let mut results = Vec::new();
    while let Some(batch) = executor.next().unwrap() {
        results.extend(batch.rows);
    }
    executor.close().unwrap();

    assert_eq!(results.len(), 3);
    // Default frame: UNBOUNDED PRECEDING TO CURRENT ROW
    // Row 0: SUM(val) over [0..0] = 1
    // Row 1: SUM(val) over [0..1] = 1+2 = 3
    // Row 2: SUM(val) over [0..2] = 1+2+3 = 6
    assert_eq!(results[0][1], Value::Integer(1));
    assert_eq!(results[1][1], Value::Integer(3));
    assert_eq!(results[2][1], Value::Integer(6));
}

// ============================================================================
// AVG Window Aggregate
// ============================================================================

#[test]
fn test_avg_window_aggregate() {
    let input_schema = test_schema(&[("val", DataType::Integer)]);
    let output_schema = test_schema(&[("val", DataType::Integer), ("avg_val", DataType::Float)]);

    let rows = vec![
        vec![Value::Integer(2)],
        vec![Value::Integer(4)],
        vec![Value::Integer(6)],
    ];

    let child = MockChildExecutor::new(rows, input_schema.clone());

    let avg_expr = Expr::WindowFunction {
        func: WindowFunction::Avg,
        args: vec![Expr::Column(sqlrustgo_planner::Column::new(
            "val".to_string(),
        ))],
        partition_by: vec![],
        order_by: vec![],
        frame: None,
    };

    let mut executor = WindowVolcanoExecutor::new(
        Box::new(child),
        vec![avg_expr],
        output_schema,
        input_schema,
        vec![],
        vec![],
    );

    executor.init().unwrap();
    let mut results = Vec::new();
    while let Some(batch) = executor.next().unwrap() {
        results.extend(batch.rows);
    }
    executor.close().unwrap();

    assert_eq!(results.len(), 3);
    // Default frame: UNBOUNDED PRECEDING TO CURRENT ROW
    // Row 0: AVG(2) = 2.0
    // Row 1: AVG(2,4) = 3.0
    // Row 2: AVG(2,4,6) = 4.0
    assert_eq!(results[0][1], Value::Float(2.0));
    assert_eq!(results[1][1], Value::Float(3.0));
    assert_eq!(results[2][1], Value::Float(4.0));
}

// ============================================================================
// COUNT Window Aggregate
// ============================================================================

#[test]
fn test_count_window_aggregate() {
    let input_schema = test_schema(&[("val", DataType::Integer)]);
    let output_schema = test_schema(&[("val", DataType::Integer), ("cnt", DataType::Integer)]);

    let rows = vec![
        vec![Value::Integer(1)],
        vec![Value::Integer(2)],
        vec![Value::Integer(3)],
    ];

    let child = MockChildExecutor::new(rows, input_schema.clone());

    let count_expr = Expr::WindowFunction {
        func: WindowFunction::Count,
        args: vec![Expr::Column(sqlrustgo_planner::Column::new(
            "val".to_string(),
        ))],
        partition_by: vec![],
        order_by: vec![],
        frame: None,
    };

    let mut executor = WindowVolcanoExecutor::new(
        Box::new(child),
        vec![count_expr],
        output_schema,
        input_schema,
        vec![],
        vec![],
    );

    executor.init().unwrap();
    let mut results = Vec::new();
    while let Some(batch) = executor.next().unwrap() {
        results.extend(batch.rows);
    }
    executor.close().unwrap();

    assert_eq!(results.len(), 3);
    // Default frame: UNBOUNDED PRECEDING TO CURRENT ROW
    // Row 0: COUNT(1) = 1
    // Row 1: COUNT(1,2) = 2
    // Row 2: COUNT(1,2,3) = 3
    assert_eq!(results[0][1], Value::Integer(1));
    assert_eq!(results[1][1], Value::Integer(2));
    assert_eq!(results[2][1], Value::Integer(3));
}

// ============================================================================
// PARTITION BY
// ============================================================================

#[test]
fn test_window_partition_by_single_partition() {
    let input_schema = test_schema(&[("grp", DataType::Integer), ("val", DataType::Integer)]);
    let output_schema = test_schema(&[
        ("grp", DataType::Integer),
        ("val", DataType::Integer),
        ("rn", DataType::Integer),
    ]);

    let rows = vec![
        vec![Value::Integer(1), Value::Integer(10)],
        vec![Value::Integer(1), Value::Integer(20)],
    ];

    let child = MockChildExecutor::new(rows, input_schema.clone());

    let partition_expr = Expr::Column(sqlrustgo_planner::Column::new("grp".to_string()));

    let rn_expr = Expr::WindowFunction {
        func: WindowFunction::RowNumber,
        args: vec![],
        partition_by: vec![],
        order_by: vec![],
        frame: None,
    };

    let mut executor = WindowVolcanoExecutor::new(
        Box::new(child),
        vec![rn_expr],
        output_schema,
        input_schema,
        vec![partition_expr],
        vec![],
    );

    executor.init().unwrap();
    let mut results = Vec::new();
    while let Some(batch) = executor.next().unwrap() {
        results.extend(batch.rows);
    }
    executor.close().unwrap();

    assert_eq!(results.len(), 2);
    // Single partition with 2 rows -> ROW_NUMBER 1, 2
    assert_eq!(results[0][0], Value::Integer(1));
    assert_eq!(results[0][2], Value::Integer(1));
    assert_eq!(results[1][0], Value::Integer(1));
    assert_eq!(results[1][2], Value::Integer(2));
}

// ============================================================================
// Empty Input
// ============================================================================

#[test]
fn test_window_executor_empty_input() {
    let input_schema = test_schema(&[("val", DataType::Integer)]);
    let output_schema = test_schema(&[("val", DataType::Integer), ("rn", DataType::Integer)]);

    let child = MockChildExecutor::new(vec![], input_schema.clone());

    let rn_expr = Expr::WindowFunction {
        func: WindowFunction::RowNumber,
        args: vec![],
        partition_by: vec![],
        order_by: vec![],
        frame: None,
    };

    let mut executor = WindowVolcanoExecutor::new(
        Box::new(child),
        vec![rn_expr],
        output_schema,
        input_schema,
        vec![],
        vec![],
    );

    executor.init().unwrap();
    let mut results = Vec::new();
    while let Some(batch) = executor.next().unwrap() {
        results.extend(batch.rows);
    }
    executor.close().unwrap();

    assert!(results.is_empty());
}

// ============================================================================
// Multiple Window Functions
// ============================================================================

#[test]
fn test_multiple_window_functions() {
    let input_schema = test_schema(&[("val", DataType::Integer)]);
    let output_schema = test_schema(&[
        ("val", DataType::Integer),
        ("rn", DataType::Integer),
        ("sum_val", DataType::Integer),
    ]);

    let rows = vec![vec![Value::Integer(5)], vec![Value::Integer(10)]];

    let child = MockChildExecutor::new(rows, input_schema.clone());

    let rn_expr = Expr::WindowFunction {
        func: WindowFunction::RowNumber,
        args: vec![],
        partition_by: vec![],
        order_by: vec![],
        frame: None,
    };

    let sum_expr = Expr::WindowFunction {
        func: WindowFunction::Sum,
        args: vec![Expr::Column(sqlrustgo_planner::Column::new(
            "val".to_string(),
        ))],
        partition_by: vec![],
        order_by: vec![],
        frame: None,
    };

    let mut executor = WindowVolcanoExecutor::new(
        Box::new(child),
        vec![rn_expr, sum_expr],
        output_schema,
        input_schema,
        vec![],
        vec![],
    );

    executor.init().unwrap();
    let mut results = Vec::new();
    while let Some(batch) = executor.next().unwrap() {
        results.extend(batch.rows);
    }
    executor.close().unwrap();

    assert_eq!(results.len(), 2);
    // Default frame: UNBOUNDED PRECEDING TO CURRENT ROW
    // Row 0: val=5, rn=1, sum over [0..0] = 5
    assert_eq!(results[0][0], Value::Integer(5));
    assert_eq!(results[0][1], Value::Integer(1));
    assert_eq!(results[0][2], Value::Integer(5));
    // Row 1: val=10, rn=2, sum over [0..1] = 5+10 = 15
    assert_eq!(results[1][0], Value::Integer(10));
    assert_eq!(results[1][1], Value::Integer(2));
    assert_eq!(results[1][2], Value::Integer(15));
}

// ============================================================================
// Window Function with ORDER BY
// ============================================================================

#[test]
fn test_window_order_by() {
    let input_schema = test_schema(&[("val", DataType::Integer)]);
    let output_schema = test_schema(&[("val", DataType::Integer), ("rn", DataType::Integer)]);

    // Input data is in descending order, but ORDER BY val ASC should sort
    let rows = vec![
        vec![Value::Integer(30)],
        vec![Value::Integer(10)],
        vec![Value::Integer(20)],
    ];

    let child = MockChildExecutor::new(rows, input_schema.clone());

    let rn_expr = Expr::WindowFunction {
        func: WindowFunction::RowNumber,
        args: vec![],
        partition_by: vec![],
        order_by: vec![],
        frame: None,
    };

    let mut executor = WindowVolcanoExecutor::new(
        Box::new(child),
        vec![rn_expr],
        output_schema,
        input_schema,
        vec![],
        vec![SortExpr {
            expr: Expr::Column(sqlrustgo_planner::Column::new("val".to_string())),
            asc: true,
            nulls_first: false,
        }],
    );

    executor.init().unwrap();
    let mut results = Vec::new();
    while let Some(batch) = executor.next().unwrap() {
        results.extend(batch.rows);
    }
    executor.close().unwrap();

    assert_eq!(results.len(), 3);
    // After sorting by val ASC, order should be 10, 20, 30
    assert_eq!(results[0][0], Value::Integer(10));
    assert_eq!(results[0][1], Value::Integer(1));
    assert_eq!(results[1][0], Value::Integer(20));
    assert_eq!(results[1][1], Value::Integer(2));
    assert_eq!(results[2][0], Value::Integer(30));
    assert_eq!(results[2][1], Value::Integer(3));
}

// ============================================================================
// Schema and Name Methods
// ============================================================================

#[test]
fn test_window_executor_name() {
    let schema = test_schema(&[("val", DataType::Integer)]);
    let child = MockChildExecutor::new(vec![vec![Value::Integer(1)]], schema.clone());
    let executor = WindowVolcanoExecutor::new(
        Box::new(child),
        vec![],
        schema.clone(),
        schema.clone(),
        vec![],
        vec![],
    );
    let _output_schema = executor.schema();
    drop(executor);
}

#[test]
fn test_window_executor_schema() {
    let input_schema = test_schema(&[("val", DataType::Integer)]);
    let output_schema = test_schema(&[("val", DataType::Integer), ("rn", DataType::Integer)]);
    let child = MockChildExecutor::new(vec![vec![Value::Integer(1)]], input_schema);
    let executor = WindowVolcanoExecutor::new(
        Box::new(child),
        vec![],
        output_schema.clone(),
        output_schema.clone(),
        vec![],
        vec![],
    );
    assert_eq!(executor.schema(), &output_schema);
}
