//! Test Harness for Executor Testing
//!
//! This module provides utilities and helpers for testing executor operations,
//! including test fixtures, assertions, and common test patterns.

use crate::{ExecutorResult, LocalExecutor};
use sqlrustgo_planner::{PhysicalPlan, Schema};
use sqlrustgo_storage::StorageEngine;
use sqlrustgo_types::{SqlResult, Value};

/// TestHarness - Main test harness for executor testing
pub struct TestHarness<S: StorageEngine> {
    storage: S,
}

impl<S: StorageEngine> TestHarness<S> {
    /// Create a new TestHarness with the given storage
    pub fn new(storage: S) -> Self {
        Self { storage }
    }

    /// Execute a physical plan and return the result
    pub fn execute(&self, plan: &dyn PhysicalPlan) -> SqlResult<ExecutorResult> {
        let executor = LocalExecutor::new(&self.storage);
        executor.execute(plan)
    }

    /// Get the storage engine
    pub fn storage(&self) -> &S {
        &self.storage
    }
}

/// ExecutorTestCase - Structure for defining test cases
pub struct ExecutorTestCase {
    pub name: String,
    pub plan: Box<dyn PhysicalPlan>,
    pub expected_rows: usize,
    pub expected_first_row: Option<Vec<Value>>,
}

impl ExecutorTestCase {
    /// Create a new test case
    pub fn new(name: &str, plan: Box<dyn PhysicalPlan>) -> Self {
        Self {
            name: name.to_string(),
            plan,
            expected_rows: 0,
            expected_first_row: None,
        }
    }

    /// Set expected row count
    pub fn expect_rows(mut self, count: usize) -> Self {
        self.expected_rows = count;
        self
    }

    /// Set expected first row
    pub fn expect_first_row(mut self, row: Vec<Value>) -> Self {
        self.expected_first_row = Some(row);
        self
    }

    /// Run the test case
    pub fn run<S: StorageEngine>(&self, harness: &TestHarness<S>) -> SqlResult<()> {
        let result = harness.execute(self.plan.as_ref())?;

        assert_eq!(
            result.rows.len(),
            self.expected_rows,
            "Test '{}' failed: expected {} rows, got {}",
            self.name,
            self.expected_rows,
            result.rows.len()
        );

        if let Some(ref expected) = self.expected_first_row {
            assert!(
                !result.rows.is_empty(),
                "Test '{}' failed: expected first row but result is empty",
                self.name
            );
            assert_eq!(
                result.rows[0], *expected,
                "Test '{}' failed: first row mismatch",
                self.name
            );
        }

        Ok(())
    }
}

/// Assertion helpers for executor tests
pub mod assertions {
    use super::*;

    /// Assert that an executor result has the expected number of rows
    pub fn assert_row_count(result: &ExecutorResult, expected: usize) {
        assert_eq!(
            result.rows.len(),
            expected,
            "Expected {} rows, got {}",
            expected,
            result.rows.len()
        );
    }

    /// Assert that an executor result has rows
    pub fn assert_has_rows(result: &ExecutorResult) {
        assert!(
            !result.rows.is_empty(),
            "Expected at least one row, but result is empty"
        );
    }

    /// Assert that an executor result is empty
    pub fn assert_no_rows(result: &ExecutorResult) {
        assert!(
            result.rows.is_empty(),
            "Expected no rows, but got {}",
            result.rows.len()
        );
    }

    /// Assert that the first row matches expected values
    pub fn assert_first_row_equals(result: &ExecutorResult, expected: &[Value]) {
        assert!(
            !result.rows.is_empty(),
            "Cannot assert first row: result is empty"
        );
        assert_eq!(result.rows[0], expected, "First row mismatch");
    }

    /// Assert that a specific row matches
    pub fn assert_row_equals(result: &ExecutorResult, index: usize, expected: &[Value]) {
        assert!(
            index < result.rows.len(),
            "Row index {} out of bounds (total rows: {})",
            index,
            result.rows.len()
        );
        assert_eq!(result.rows[index], expected, "Row {} mismatch", index);
    }

    /// Assert that affected_rows matches expected
    pub fn assert_affected_rows(result: &ExecutorResult, expected: usize) {
        assert_eq!(
            result.affected_rows, expected,
            "Expected {} affected rows, got {}",
            expected, result.affected_rows
        );
    }

    /// Assert that result contains a specific value in any row
    pub fn assert_contains_value(result: &ExecutorResult, value: &Value) {
        let found = result.rows.iter().any(|row| row.contains(value));
        assert!(
            found,
            "Expected to find value {:?} in result, but it was not found",
            value
        );
    }
}

/// Helper functions for creating test plans
pub mod helpers {
    use super::*;
    use sqlrustgo_planner::{Expr, Field, SeqScanExec};

    /// Create a simple SeqScan plan for testing
    pub fn create_seq_scan_plan(table_name: &str, schema: Schema) -> Box<dyn PhysicalPlan> {
        Box::new(SeqScanExec::new(table_name.to_string(), schema))
    }

    /// Create a simple projection plan
    pub fn create_projection_plan(
        child: Box<dyn PhysicalPlan>,
        exprs: Vec<Expr>,
        output_schema: Schema,
    ) -> Box<dyn PhysicalPlan> {
        use sqlrustgo_planner::ProjectionExec;
        Box::new(ProjectionExec::new(child, exprs, output_schema))
    }

    /// Create a simple filter plan
    pub fn create_filter_plan(
        child: Box<dyn PhysicalPlan>,
        predicate: Expr,
    ) -> Box<dyn PhysicalPlan> {
        use sqlrustgo_planner::FilterExec;
        Box::new(FilterExec::new(child, predicate))
    }

    /// Create a simple aggregate plan
    pub fn create_aggregate_plan(
        child: Box<dyn PhysicalPlan>,
        group_expr: Vec<Expr>,
        aggregate_expr: Vec<Expr>,
        output_schema: Schema,
    ) -> Box<dyn PhysicalPlan> {
        use sqlrustgo_planner::AggregateExec;
        Box::new(AggregateExec::new(
            child,
            group_expr,
            aggregate_expr,
            None,
            output_schema,
        ))
    }

    /// Create a schema for common test tables
    pub fn users_schema() -> Schema {
        Schema::new(vec![
            Field::new("id".to_string(), sqlrustgo_planner::DataType::Integer),
            Field::new("name".to_string(), sqlrustgo_planner::DataType::Text),
        ])
    }

    /// Create a schema for orders table
    pub fn orders_schema() -> Schema {
        Schema::new(vec![
            Field::new("order_id".to_string(), sqlrustgo_planner::DataType::Integer),
            Field::new("user_id".to_string(), sqlrustgo_planner::DataType::Integer),
            Field::new("amount".to_string(), sqlrustgo_planner::DataType::Integer),
        ])
    }

    /// Create a schema for products table
    pub fn products_schema() -> Schema {
        Schema::new(vec![
            Field::new("id".to_string(), sqlrustgo_planner::DataType::Integer),
            Field::new("name".to_string(), sqlrustgo_planner::DataType::Text),
            Field::new("price".to_string(), sqlrustgo_planner::DataType::Integer),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlrustgo_planner::{
        DataType, Expr, Field, FilterExec, PhysicalPlan, ProjectionExec, Schema,
    };
    use sqlrustgo_storage::MemoryStorage;

    #[test]
    fn test_harness_creation() {
        let harness = TestHarness::new(MemoryStorage::new());
        assert_eq!(harness.storage().name(), "memory");
    }

    #[test]
    fn test_harness_execute_simple_plan() {
        let harness = TestHarness::new(MemoryStorage::new());
        let schema = Schema::new(vec![
            Field::new("id".to_string(), DataType::Integer),
        ]);
        let plan = Box::new(sqlrustgo_planner::SeqScanExec::new("t".to_string(), schema));
        let result = harness.execute(plan.as_ref());
        assert!(result.is_ok(), "execute should succeed");
    }

    #[test]
    fn test_test_case_builder() {
        let schema = Schema::new(vec![Field::new("id".to_string(), DataType::Integer)]);
        let plan = Box::new(sqlrustgo_planner::SeqScanExec::new("t".to_string(), schema));
        let mut tc = ExecutorTestCase::new("test_case", plan);
        tc.expect_rows(10);
        tc.expect_first_row(vec![Value::Integer(1)]);
        assert_eq!(tc.expected_rows, 10);
        assert!(tc.expected_first_row.is_some());
    }

    #[test]
    fn test_assertions_row_count() {
        let result = ExecutorResult::new(vec![
            vec![Value::Integer(1)],
            vec![Value::Integer(2)],
        ], 0);
        assertions::assert_row_count(&result, 2);
    }

    #[test]
    #[should_panic(expected = "Expected 3 rows, got 2")]
    fn test_assertions_row_count_fails() {
        let result = ExecutorResult::new(vec![
            vec![Value::Integer(1)],
            vec![Value::Integer(2)],
        ], 0);
        assertions::assert_row_count(&result, 3);
    }

    #[test]
    fn test_assertions_has_rows() {
        let result = ExecutorResult::new(vec![vec![Value::Integer(1)]], 0);
        assertions::assert_has_rows(&result);
    }

    #[test]
    fn test_assertions_no_rows() {
        let result = ExecutorResult::new(vec![], 0);
        assertions::assert_no_rows(&result);
    }

    #[test]
    fn test_assertions_first_row_equals() {
        let result = ExecutorResult::new(vec![
            vec![Value::Integer(1), Value::Text("Alice".to_string())],
        ], 0);
        assertions::assert_first_row_equals(
            &result,
            &[Value::Integer(1), Value::Text("Alice".to_string())],
        );
    }

    #[test]
    fn test_assertions_row_equals() {
        let result = ExecutorResult::new(vec![
            vec![Value::Integer(1)],
            vec![Value::Integer(2)],
        ], 0);
        assertions::assert_row_equals(&result, 1, &[Value::Integer(2)]);
    }

    #[test]
    fn test_assertions_affected_rows() {
        let result = ExecutorResult::new(vec![], 5);
        assertions::assert_affected_rows(&result, 5);
    }

    #[test]
    fn test_assertions_contains_value() {
        let result = ExecutorResult::new(vec![
            vec![Value::Integer(1)],
            vec![Value::Integer(2)],
        ], 0);
        assertions::assert_contains_value(&result, &Value::Integer(2));
    }

    #[test]
    fn test_helpers_users_schema() {
        let schema = helpers::users_schema();
        assert_eq!(schema.fields.len(), 2);
    }

    #[test]
    fn test_helpers_orders_schema() {
        let schema = helpers::orders_schema();
        assert_eq!(schema.fields.len(), 3);
    }

    #[test]
    fn test_helpers_products_schema() {
        let schema = helpers::products_schema();
        assert_eq!(schema.fields.len(), 3);
    }

    #[test]
    fn test_helpers_create_seq_scan_plan() {
        let schema = Schema::new(vec![Field::new("id".to_string(), DataType::Integer)]);
        let plan = helpers::create_seq_scan_plan("users", schema);
        assert!(plan.as_any().is::<sqlrustgo_planner::SeqScanExec>());
    }

    #[test]
    fn test_helpers_create_filter_plan() {
        let child = Box::new(sqlrustgo_planner::SeqScanExec::new(
            "users".to_string(),
            Schema::new(vec![Field::new("id".to_string(), DataType::Integer)]),
        ));
        let filter = Expr::BinaryExpr {
            left: Box::new(Expr::Column(sqlrustgo_planner::Column::new("id".to_string()))),
            op: sqlrustgo_planner::Operator::Gt,
            right: Box::new(Expr::Literal(Value::Integer(0))),
        };
        let plan = helpers::create_filter_plan(child, filter);
        assert!(plan.as_any().is::<FilterExec>());
    }

    #[test]
    fn test_helpers_create_projection_plan() {
        let child = Box::new(sqlrustgo_planner::SeqScanExec::new(
            "users".to_string(),
            Schema::new(vec![
                Field::new("id".to_string(), DataType::Integer),
                Field::new("name".to_string(), DataType::Text),
            ]),
        ));
        let exprs = vec![Expr::Column(sqlrustgo_planner::Column::new("id".to_string()))];
        let output_schema = Schema::new(vec![Field::new("id".to_string(), DataType::Integer)]);
        let plan = helpers::create_projection_plan(child, exprs, output_schema);
        assert!(plan.as_any().is::<ProjectionExec>());
    }

    #[test]
    fn test_helpers_create_aggregate_plan() {
        let child = Box::new(sqlrustgo_planner::SeqScanExec::new(
            "orders".to_string(),
            Schema::new(vec![
                Field::new("customer_id".to_string(), DataType::Integer),
                Field::new("amount".to_string(), DataType::Integer),
            ]),
        ));
        let group_expr = vec![Expr::Column(sqlrustgo_planner::Column::new("customer_id".to_string()))];
        let agg_expr = vec![Expr::AggregateFunction {
            func: sqlrustgo_planner::AggregateFunction::Sum,
            args: vec![Expr::Column(sqlrustgo_planner::Column::new("amount".to_string()))],
            distinct: false,
            order_by: None,
        }];
        let output_schema = Schema::new(vec![
            Field::new("customer_id".to_string(), DataType::Integer),
            Field::new("SUM(amount)".to_string(), DataType::Integer),
        ]);
        let plan = helpers::create_aggregate_plan(child, group_expr, agg_expr, output_schema);
        assert!(plan.as_any().is::<sqlrustgo_planner::AggregateExec>());
    }

    #[test]
    fn test_test_case_run() {
        let schema = Schema::new(vec![Field::new("id".to_string(), DataType::Integer)]);
        let plan = Box::new(sqlrustgo_planner::SeqScanExec::new("t".to_string(), schema));
        let tc = ExecutorTestCase::new("test_run", plan).expect_rows(0);
        let harness = TestHarness::new(MemoryStorage::new());
        assert!(tc.run(&harness).is_ok(), "test case should run successfully");
    }

    #[test]
    fn test_test_case_run_with_expected_rows() {
        let schema = Schema::new(vec![
            Field::new("id".to_string(), DataType::Integer),
            Field::new("name".to_string(), DataType::Text),
        ]);
        let plan = Box::new(sqlrustgo_planner::SeqScanExec::new("users".to_string(), schema));
        let tc = ExecutorTestCase::new("test_run_with_rows", plan).expect_rows(0);
        let harness = TestHarness::new(MemoryStorage::new());
        assert!(tc.run(&harness).is_ok(), "test case should run successfully");
    }
}

