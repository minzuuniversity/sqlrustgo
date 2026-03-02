//! Query Planner Module
//!
//! This module implements the query planning layer for SQLRustGo.
//! It transforms parsed SQL statements into executable plans.
//!
//! # Architecture
//!
//! ```mermaid
//! graph LR
//!     SQL --> Parser
//!     Parser --> Statement
//!     Statement --> LogicalPlan
//!     LogicalPlan --> Analyzer
//! Analyzer --> PhysicalPlan
//! PhysicalPlan --> Executor
//! ```
//!
//! ## Layers
//!
//! 1. **LogicalPlan**: High-level query representation (what to compute)
//! 2. **Analyzer/Binder**: Type checking, column resolution, semantic analysis
//! 3. **PhysicalPlan**: Low-level execution plan (how to compute)
//! 4. **Executor**: Actual execution of the physical plan

use crate::parser::{Expression, SelectColumn};
use crate::types::{SqlResult, Value};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Logical plan node representing a query operation
///
/// Each variant represents a different relational algebra operation:
/// - TableScan: Read from a table
/// - Projection: SELECT columns
/// - Filter: WHERE clause
/// - Join: Table join (inner, left, right, full)
/// - Aggregate: GROUP BY and aggregation
#[derive(Debug, Clone, PartialEq)]
pub enum LogicalPlan {
    /// Scan a table (from clause)
    TableScan(TableScanPlan),
    /// Projection (SELECT columns)
    Projection(ProjectionPlan),
    /// Filter (WHERE clause)
    Filter(FilterPlan),
    /// Join two tables
    Join(JoinPlan),
    /// Aggregate (GROUP BY)
    Aggregate(AggregatePlan),
    /// Sort (ORDER BY)
    Sort(SortPlan),
    /// Limit rows
    Limit(LimitPlan),
    /// Cross product (implicit join)
    CrossJoin(CrossJoinPlan),
}

/// Table scan plan
#[derive(Debug, Clone, PartialEq)]
pub struct TableScanPlan {
    pub table_name: String,
    /// Column indices to read (None = all columns)
    pub projection: Option<Vec<usize>>,
    /// Table alias for disambiguation
    pub alias: Option<String>,
    /// Schema for this scan
    pub schema: Schema,
}

/// Projection plan (SELECT columns)
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectionPlan {
    pub input: Box<LogicalPlan>,
    /// Expressions to compute (column references, literals, expressions)
    pub expr: Vec<Expr>,
    /// Output column names
    pub schema: Schema,
}

/// Filter plan (WHERE clause)
#[derive(Debug, Clone, PartialEq)]
pub struct FilterPlan {
    pub input: Box<LogicalPlan>,
    /// Predicate expression
    pub predicate: Expr,
}

/// Join plan
#[derive(Debug, Clone, PartialEq)]
pub struct JoinPlan {
    pub left: Box<LogicalPlan>,
    pub right: Box<LogicalPlan>,
    /// Join condition (ON clause)
    pub on: Vec<(Expr, Expr)>,
    /// Join type
    pub join_type: JoinType,
    /// Output schema
    pub schema: Schema,
}

/// Join types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

/// Aggregate plan (GROUP BY)
#[derive(Debug, Clone, PartialEq)]
pub struct AggregatePlan {
    pub input: Box<LogicalPlan>,
    /// Grouping expressions
    pub group_expr: Vec<Expr>,
    /// Aggregate expressions
    pub aggr_expr: Vec<AggregateExpr>,
    /// Output schema
    pub schema: Schema,
}

/// Sort plan (ORDER BY)
#[derive(Debug, Clone, PartialEq)]
pub struct SortPlan {
    pub input: Box<LogicalPlan>,
    /// Sort expressions with direction
    pub sort_expr: Vec<(Expr, SortDirection)>,
}

/// Sort direction
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SortDirection {
    Asc,
    Desc,
}

/// Limit plan
#[derive(Debug, Clone, PartialEq)]
pub struct LimitPlan {
    pub input: Box<LogicalPlan>,
    pub limit: usize,
    pub offset: Option<usize>,
}

/// Cross join plan
#[derive(Debug, Clone, PartialEq)]
pub struct CrossJoinPlan {
    pub left: Box<LogicalPlan>,
    pub right: Box<LogicalPlan>,
    pub schema: Schema,
}

/// Expression in logical plan
///
/// This is a simplified expression type for logical planning.
/// It differs from parser::Expression in that it includes column references
/// with table qualifications.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    /// Column reference (possibly qualified with table name)
    Column(ColumnRef),
    /// Literal value
    Literal(Value),
    /// Binary operation
    BinaryOp(Box<Expr>, BinaryOperator, Box<Expr>),
    /// Unary operation
    UnaryOp(UnaryOperator, Box<Expr>),
    /// Function call
    Function(FunctionCall),
    /// Aggregate function
    Aggregate(AggregateExpr),
    /// Wildcard (*)
    Wildcard,
}

/// Column reference
#[derive(Debug, Clone, PartialEq, Hash, Eq, Serialize, Deserialize)]
pub struct ColumnRef {
    /// Table name (optional for unqualified columns)
    pub table: Option<String>,
    /// Column name
    pub name: String,
}

impl ColumnRef {
    /// Create a new column reference
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            table: None,
            name: name.into(),
        }
    }

    /// Create a qualified column reference (table.column)
    pub fn qualified(table: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            table: Some(table.into()),
            name: name.into(),
        }
    }
}

/// Binary operators
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BinaryOperator {
    Eq,       // =
    Ne,       // != or <>
    Lt,       // <
    Le,       // <=
    Gt,       // >
    Ge,       // >=
    And,      // AND
    Or,       // OR
    Plus,     // +
    Minus,    // -
    Mul,      // *
    Div,      // /
    Mod,      // %
}

/// Unary operators
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnaryOperator {
    Not,  // NOT
    Minus, // - (unary minus)
    Plus, // + (unary plus)
}

/// Function call
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub args: Vec<Expr>,
}

/// Aggregate expression
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AggregateExpr {
    pub func: AggregateFunc,
    pub args: Vec<Expr>,
    pub alias: Option<String>,
}

/// Aggregate functions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AggregateFunc {
    Count,
    Sum,
    Avg,
    Min,
    Max,
}

/// Schema definition (table column information)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Schema {
    pub columns: Vec<Column>,
}

/// Column definition in schema
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Column {
    pub name: String,
    pub data_type: ColumnType,
    pub nullable: bool,
}

/// Column data types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ColumnType {
    Null,
    Boolean,
    Integer,
    Float,
    Text,
    Blob,
}

impl Schema {
    /// Create a new schema from column definitions
    pub fn new(columns: Vec<Column>) -> Self {
        Self { columns }
    }

    /// Get column index by name
    pub fn column_index(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|c| c.name == name)
    }

    /// Get column by name
    pub fn column(&self, name: &str) -> Option<&Column> {
        self.columns.iter().find(|c| c.name == name)
    }

    /// Get column names
    pub fn column_names(&self) -> Vec<String> {
        self.columns.iter().map(|c| c.name.clone()).collect()
    }
}

impl LogicalPlan {
    /// Get the output schema of this plan
    pub fn schema(&self) -> &Schema {
        match self {
            LogicalPlan::TableScan(scan) => &scan.schema,
            LogicalPlan::Projection(p) => &p.schema,
            LogicalPlan::Filter(f) => f.input.schema(),
            LogicalPlan::Join(j) => &j.schema,
            LogicalPlan::Aggregate(a) => &a.schema,
            LogicalPlan::Sort(s) => s.input.schema(),
            LogicalPlan::Limit(l) => l.input.schema(),
            LogicalPlan::CrossJoin(c) => &c.schema,
        }
    }

    /// Get all tables referenced in this plan
    pub fn tables(&self) -> Vec<String> {
        match self {
            LogicalPlan::TableScan(scan) => vec![scan.table_name.clone()],
            LogicalPlan::Projection(p) => p.input.tables(),
            LogicalPlan::Filter(f) => f.input.tables(),
            LogicalPlan::Join(j) => {
                let mut tables = j.left.tables();
                tables.extend(j.right.tables());
                tables
            }
            LogicalPlan::Aggregate(a) => a.input.tables(),
            LogicalPlan::Sort(s) => s.input.tables(),
            LogicalPlan::Limit(l) => l.input.tables(),
            LogicalPlan::CrossJoin(c) => {
                let mut tables = c.left.tables();
                tables.extend(c.right.tables());
                tables
            }
        }
    }
}

impl LogicalPlan {
    /// Create a table scan logical plan
    pub fn table_scan(table_name: String, schema: Schema) -> Self {
        LogicalPlan::TableScan(TableScanPlan {
            table_name,
            projection: None,
            alias: None,
            schema,
        })
    }

    /// Create a projection logical plan
    pub fn projection(input: LogicalPlan, expr: Vec<Expr>, schema: Schema) -> Self {
        LogicalPlan::Projection(ProjectionPlan {
            input: Box::new(input),
            expr,
            schema,
        })
    }

    /// Create a filter logical plan
    pub fn filter(input: LogicalPlan, predicate: Expr) -> Self {
        LogicalPlan::Filter(FilterPlan {
            input: Box::new(input),
            predicate,
        })
    }

    /// Create a join logical plan
    pub fn join(
        left: LogicalPlan,
        right: LogicalPlan,
        on: Vec<(Expr, Expr)>,
        join_type: JoinType,
        schema: Schema,
    ) -> Self {
        LogicalPlan::Join(JoinPlan {
            left: Box::new(left),
            right: Box::new(right),
            on,
            join_type,
            schema,
        })
    }
}

/// Analyzer/Binder transforms a parsed Statement into a LogicalPlan
pub struct Analyzer;

impl Analyzer {
    /// Analyze a SELECT statement into a logical plan
    pub fn analyze_select(
        table_name: &str,
        columns: &[SelectColumn],
        where_clause: Option<&Expression>,
        table_schema: &Schema,
    ) -> SqlResult<LogicalPlan> {
        // Start with table scan
        let mut plan = LogicalPlan::table_scan(table_name.to_string(), table_schema.clone());

        // Apply WHERE filter if present
        if let Some(expr) = where_clause {
            let bound_expr = Self::bind_expression(expr, table_schema)?;
            plan = LogicalPlan::filter(plan, bound_expr);
        }

        // Apply projection (SELECT columns)
        let proj_exprs: Vec<Expr> = columns
            .iter()
            .map(|c| {
                if c.name == "*" {
                    Expr::Wildcard
                } else {
                    Expr::Column(ColumnRef::new(&c.name))
                }
            })
            .collect();

        let proj_schema = Self::compute_projection_schema(columns, table_schema);
        plan = LogicalPlan::projection(plan, proj_exprs, proj_schema);

        Ok(plan)
    }

    /// Bind a parser expression to a logical plan expression
    #[allow(clippy::only_used_in_recursion)]
    fn bind_expression(expr: &Expression, schema: &Schema) -> SqlResult<Expr> {
        match expr {
            Expression::Literal(s) => Ok(Expr::Literal(crate::types::parse_sql_literal(s))),
            Expression::Identifier(name) => {
                // Check if it's a qualified name (table.column)
                if let Some((table, col)) = name.split_once('.') {
                    Ok(Expr::Column(ColumnRef::qualified(table, col)))
                } else {
                    Ok(Expr::Column(ColumnRef::new(name)))
                }
            }
            Expression::BinaryOp(left, op, right) => {
                let left = Self::bind_expression(left, schema)?;
                let right = Self::bind_expression(right, schema)?;
                let operator = Self::bind_operator(op);
                Ok(Expr::BinaryOp(Box::new(left), operator, Box::new(right)))
            }
        }
    }

    /// Bind binary operator
    fn bind_operator(op: &str) -> BinaryOperator {
        match op.to_uppercase().as_str() {
            "=" => BinaryOperator::Eq,
            "!=" | "<>" => BinaryOperator::Ne,
            "<" => BinaryOperator::Lt,
            "<=" => BinaryOperator::Le,
            ">" => BinaryOperator::Gt,
            ">=" => BinaryOperator::Ge,
            "AND" => BinaryOperator::And,
            "OR" => BinaryOperator::Or,
            "+" => BinaryOperator::Plus,
            "-" => BinaryOperator::Minus,
            "*" => BinaryOperator::Mul,
            "/" => BinaryOperator::Div,
            "%" => BinaryOperator::Mod,
            _ => BinaryOperator::Eq,
        }
    }

    /// Compute projection schema from SELECT columns
    fn compute_projection_schema(columns: &[SelectColumn], input_schema: &Schema) -> Schema {
        let column_types: Vec<Column> = columns
            .iter()
            .map(|c| {
                if c.name == "*" {
                    Column {
                        name: "*".to_string(),
                        data_type: ColumnType::Text,
                        nullable: true,
                    }
                } else if let Some(col) = input_schema.column(&c.name) {
                    Column {
                        name: c.alias.clone().unwrap_or_else(|| c.name.clone()),
                        data_type: col.data_type.clone(),
                        nullable: col.nullable,
                    }
                } else {
                    Column {
                        name: c.alias.clone().unwrap_or_else(|| c.name.clone()),
                        data_type: ColumnType::Text,
                        nullable: true,
                    }
                }
            })
            .collect();
        Schema::new(column_types)
    }
}

/// Physical plan trait (execution-ready plan)
pub trait PhysicalPlan: Send + Sync {
    /// Get the output schema
    fn schema(&self) -> &Schema;

    /// Execute this plan and return an iterator over record batches
    fn execute(&self) -> SqlResult<Vec<RecordBatch>>;

    /// Get child plans
    fn children(&self) -> Vec<Arc<dyn PhysicalPlan>>;
}

/// Record batch (columnar data)
#[derive(Debug, Clone)]
pub struct RecordBatch {
    pub schema: Schema,
    pub columns: Vec<Vec<Value>>,
}

impl RecordBatch {
    /// Create a new record batch
    pub fn new(schema: Schema, columns: Vec<Vec<Value>>) -> Self {
        Self { schema, columns }
    }

    /// Get number of rows
    pub fn num_rows(&self) -> usize {
        self.columns.first().map(|c| c.len()).unwrap_or(0)
    }
}

/// Execution engine trait
pub trait ExecutionEngine: Send + Sync {
    /// Execute a physical plan
    fn execute(&self, plan: Arc<dyn PhysicalPlan>) -> SqlResult<RecordBatch>;

    /// Get engine name
    fn name(&self) -> &str;
}

#[allow(dead_code)]
/// Table scan physical operator
pub struct TableScanExec {
    table_name: String,
    schema: Schema,
}

impl TableScanExec {
    pub fn new(table_name: String, schema: Schema) -> Self {
        Self { table_name, schema }
    }
}

impl PhysicalPlan for TableScanExec {
    fn schema(&self) -> &Schema {
        &self.schema
    }

    fn execute(&self) -> SqlResult<Vec<RecordBatch>> {
        // TODO: Integrate with storage engine
        Ok(vec![])
    }

    fn children(&self) -> Vec<Arc<dyn PhysicalPlan>> {
        vec![]
    }
}

#[allow(dead_code)]
/// Projection physical operator
pub struct ProjectionExec {
    input: Arc<dyn PhysicalPlan>,
    expr: Vec<Expr>,
    schema: Schema,
}

impl ProjectionExec {
    pub fn new(input: Arc<dyn PhysicalPlan>, expr: Vec<Expr>, schema: Schema) -> Self {
        Self { input, expr, schema }
    }
}

impl PhysicalPlan for ProjectionExec {
    fn schema(&self) -> &Schema {
        &self.schema
    }

    fn execute(&self) -> SqlResult<Vec<RecordBatch>> {
        let input_batches = self.input.execute()?;
        // TODO: Implement projection execution
        Ok(input_batches)
    }

    fn children(&self) -> Vec<Arc<dyn PhysicalPlan>> {
        vec![self.input.clone()]
    }
}

#[allow(dead_code)]
/// Filter physical operator
pub struct FilterExec {
    input: Arc<dyn PhysicalPlan>,
    predicate: Expr,
    schema: Schema,
}

impl FilterExec {
    pub fn new(input: Arc<dyn PhysicalPlan>, predicate: Expr, schema: Schema) -> Self {
        Self {
            input,
            predicate,
            schema,
        }
    }
}

impl PhysicalPlan for FilterExec {
    fn schema(&self) -> &Schema {
        &self.schema
    }

    fn execute(&self) -> SqlResult<Vec<RecordBatch>> {
        let input_batches = self.input.execute()?;
        // TODO: Implement filter execution
        Ok(input_batches)
    }

    fn children(&self) -> Vec<Arc<dyn PhysicalPlan>> {
        vec![self.input.clone()]
    }
}

#[allow(dead_code)]
/// Hash join physical operator
///
/// Implements hash join algorithm for equi-joins.
/// Uses a hash table to efficiently match rows from left and right inputs.
pub struct HashJoinExec {
    left: Arc<dyn PhysicalPlan>,
    right: Arc<dyn PhysicalPlan>,
    /// Join condition (left column, right column)
    on: Vec<(usize, usize)>,
    /// Join type
    join_type: JoinType,
    /// Output schema
    schema: Schema,
}

impl HashJoinExec {
    pub fn new(
        left: Arc<dyn PhysicalPlan>,
        right: Arc<dyn PhysicalPlan>,
        on: Vec<(usize, usize)>,
        join_type: JoinType,
        schema: Schema,
    ) -> Self {
        Self {
            left,
            right,
            on,
            join_type,
            schema,
        }
    }

    /// Execute hash join
    pub fn execute(&self) -> SqlResult<Vec<RecordBatch>> {
        // Get left and right input batches
        let left_batches = self.left.execute()?;
        let _right_batches = self.right.execute()?;

        // Build hash table from right input
        let _hash_table: std::collections::HashMap<Vec<u8>, Vec<Vec<Value>>> =
            std::collections::HashMap::new();

        // TODO: Build hash table from right batches
        // For now, return empty result

        let result_rows: Vec<Vec<Value>> = vec![];

        // Probe with left input
        for _left_batch in &left_batches {
            // TODO: Implement actual hash join logic
            // For each left row, look up in hash table and emit matching rows
        }

        // Handle left outer join - emit unmatched left rows with NULL right columns
        if matches!(self.join_type, JoinType::Left | JoinType::Full) {
            // TODO: Add unmatched left rows with NULL values
        }

        // Handle right outer join - emit unmatched right rows with NULL left columns
        if matches!(self.join_type, JoinType::Right | JoinType::Full) {
            // TODO: Add unmatched right rows with NULL values
        }

        // Convert rows to record batch
        let num_columns = self.schema.columns.len();
        let _num_rows = result_rows.len();

        let mut columns: Vec<Vec<Value>> = vec![vec![]; num_columns];
        for row in result_rows {
            for (i, value) in row.into_iter().enumerate() {
                if i < num_columns {
                    columns[i].push(value);
                }
            }
        }

        Ok(vec![RecordBatch::new(self.schema.clone(), columns)])
    }
}

impl PhysicalPlan for HashJoinExec {
    fn schema(&self) -> &Schema {
        &self.schema
    }

    fn execute(&self) -> SqlResult<Vec<RecordBatch>> {
        self.execute()
    }

    fn children(&self) -> Vec<Arc<dyn PhysicalPlan>> {
        vec![self.left.clone(), self.right.clone()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logical_plan_schema() {
        let schema = Schema::new(vec![
            Column {
                name: "id".to_string(),
                data_type: ColumnType::Integer,
                nullable: false,
            },
            Column {
                name: "name".to_string(),
                data_type: ColumnType::Text,
                nullable: true,
            },
        ]);

        let plan = LogicalPlan::table_scan("users".to_string(), schema);
        assert_eq!(plan.schema().columns.len(), 2);
    }

    #[test]
    fn test_column_ref() {
        let col = ColumnRef::new("id");
        assert_eq!(col.name, "id");
        assert_eq!(col.table, None);

        let qualified = ColumnRef::qualified("users", "id");
        assert_eq!(qualified.name, "id");
        assert_eq!(qualified.table, Some("users".to_string()));
    }

    #[test]
    fn test_schema_column_index() {
        let schema = Schema::new(vec![
            Column {
                name: "id".to_string(),
                data_type: ColumnType::Integer,
                nullable: false,
            },
            Column {
                name: "name".to_string(),
                data_type: ColumnType::Text,
                nullable: true,
            },
        ]);

        assert_eq!(schema.column_index("id"), Some(0));
        assert_eq!(schema.column_index("name"), Some(1));
        assert_eq!(schema.column_index("nonexistent"), None);
    }
}
