//! SQLRustGo Planner Module
//!
//! This module provides query planning and optimization interfaces.

pub mod logical_plan;
pub mod optimizer;
pub mod physical_plan;
pub mod planner;
pub mod statement;

// TODO: Add these modules after migration
// pub mod analyzer;
// pub mod cost;
// pub mod executor;

pub use logical_plan::{LogicalPlan, ParamMode, ProcedureParam, ProcedureStatement};
pub use optimizer::{DefaultOptimizer, NoOpOptimizer, Optimizer, OptimizerRule};
pub use physical_plan::PhysicalPlan;
pub use physical_plan::{
    AggregateExec, DeleteExec, FilterExec, HashJoinExec, IndexScanExec, LimitExec, ProjectionExec,
    SeqScanExec, SortExec,
};
pub use planner::{DefaultPlanner, NoOpPlanner, Planner};
pub use statement::{MergeClause, MergeStatement};

use sqlrustgo_types::Value;
use std::fmt;

/// Type of join operation in query execution
#[derive(Debug, Clone, PartialEq)]
pub enum JoinType {
    /// Inner join - returns matching rows from both tables
    Inner,
    /// Left outer join - returns all rows from left table
    Left,
    /// Right outer join - returns all rows from right table
    Right,
    /// Full outer join - returns all rows from both tables
    Full,
    /// Cross join - Cartesian product of both tables
    Cross,
    /// Left semi join - returns rows from left that have match in right
    LeftSemi,
    /// Left anti join - returns rows from left that have no match in right
    LeftAnti,
    /// Right semi join - returns rows from right that have match in left
    RightSemi,
    /// Right anti join - returns rows from right that have no match in left
    RightAnti,
}

/// Aggregate function types
#[derive(Debug, Clone, PartialEq)]
pub enum AggregateFunction {
    /// COUNT - counts number of rows
    Count,
    /// SUM - sum of values
    Sum,
    /// AVG - average of values
    Avg,
    /// MIN - minimum value
    Min,
    /// MAX - maximum value
    Max,
}

/// Binary and unary operators in expressions
#[derive(Debug, Clone, PartialEq)]
pub enum Operator {
    /// Equal (=)
    Eq,
    /// Not equal (!=)
    NotEq,
    /// Less than (<)
    Lt,
    /// Less than or equal (<=)
    LtEq,
    /// Greater than (>)
    Gt,
    /// Greater than or equal (>=)
    GtEq,
    /// Addition (+)
    Plus,
    /// Subtraction (-)
    Minus,
    /// Multiplication (*)
    Multiply,
    /// Division (/)
    Divide,
    /// Modulo (%)
    Modulo,
    /// Logical AND
    And,
    /// Logical OR
    Or,
    /// Logical NOT
    Not,
    /// LIKE pattern matching
    Like,
}

/// Sort expression with direction and null ordering
#[derive(Debug, Clone, PartialEq)]
pub struct SortExpr {
    /// Expression to sort by
    pub expr: Expr,
    /// Sort in ascending order if true, descending if false
    pub asc: bool,
    /// Place nulls first if true, last if false
    pub nulls_first: bool,
}

/// Column reference in query
#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    /// Table alias or name (None for unqualified columns)
    pub relation: Option<String>,
    /// Column name
    pub name: String,
}

impl Column {
    pub fn new(name: String) -> Self {
        Self {
            relation: None,
            name,
        }
    }

    pub fn new_qualified(relation: String, name: String) -> Self {
        Self {
            relation: Some(relation),
            name,
        }
    }
}

impl fmt::Display for Column {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.relation {
            Some(rel) => write!(f, "{}.{}", rel, self.name),
            None => write!(f, "{}", self.name),
        }
    }
}

/// Expression in query (column reference, literal, computation, etc.)
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Column reference
    Column(Column),
    /// Literal value
    Literal(Value),
    /// Binary expression (left op right)
    BinaryExpr {
        left: Box<Expr>,
        op: Operator,
        right: Box<Expr>,
    },
    /// Unary expression (op expr)
    UnaryExpr { op: Operator, expr: Box<Expr> },
    /// Aggregate function call
    AggregateFunction {
        func: AggregateFunction,
        args: Vec<Expr>,
        distinct: bool,
    },
    /// Alias expression (expr AS name)
    Alias { expr: Box<Expr>, name: String },
    /// Wildcard (*) select
    Wildcard,
    /// Qualified wildcard (table.*)
    QualifiedWildcard { qualifier: String },
    /// Window function expression (func() OVER (PARTITION BY ... ORDER BY ...))
    WindowFunction {
        /// The window function to compute
        func: WindowFunction,
        /// Arguments to the function
        args: Vec<Expr>,
        /// PARTITION BY expressions
        partition_by: Vec<Expr>,
        /// ORDER BY expressions
        order_by: Vec<SortExpr>,
        /// Window frame specification
        frame: Option<WindowFrame>,
    },
}

/// Schema containing field definitions
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Schema {
    /// Fields in this schema
    pub fields: Vec<Field>,
}

/// Field definition in a schema
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    /// Field name
    pub name: String,
    /// Data type
    pub data_type: DataType,
    /// Whether field can be null
    pub nullable: bool,
}

/// Planner data types (logical types)
#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    /// Boolean type
    Boolean,
    /// Integer type
    Integer,
    /// Floating point type
    Float,
    /// Text/string type
    Text,
    /// Binary data type
    Blob,
    /// Null type
    Null,
}

/// Window function types for window expressions
#[derive(Debug, Clone, PartialEq)]
pub enum WindowFunction {
    /// ROW_NUMBER - returns the number of the row within its partition
    RowNumber,
    /// RANK - rank of the current row with gaps
    Rank,
    /// DENSE_RANK - rank of the current row without gaps
    DenseRank,
    /// PERCENT_RANK - percentage rank
    PercentRank,
    /// CUME_DIST - cumulative distribution
    CumeDist,
    /// LEAD - value after current row
    Lead {
        offset: usize,
        default: Option<Value>,
    },
    /// LAG - value before current row
    Lag {
        offset: usize,
        default: Option<Value>,
    },
    /// FIRST_VALUE - first value in window frame
    FirstValue,
    /// LAST_VALUE - last value in window frame
    LastValue,
    /// NTH_VALUE - nth value in window frame
    NthValue { n: usize },
    /// COUNT - count of values
    Count,
    /// SUM - sum of values
    Sum,
    /// AVG - average of values
    Avg,
    /// MIN - minimum value
    Min,
    /// MAX - maximum value
    Max,
}

/// Frame bound types for window frames
#[derive(Debug, Clone, PartialEq)]
pub enum FrameBound {
    /// UNBOUNDED PRECEDING - start of partition
    UnboundedPreceding,
    /// UNBOUNDED FOLLOWING - end of partition
    UnboundedFollowing,
    /// CURRENT ROW - current row position
    CurrentRow,
    /// PRECEDING n rows
    Preceding(usize),
    /// FOLLOWING n rows
    Following(usize),
}

/// Window frame specification (ROWS/RANGE/GROUPS BETWEEN ...)
#[derive(Debug, Clone, PartialEq)]
pub struct WindowFrame {
    /// Frame mode: ROWS, RANGE, or GROUPS
    pub mode: FrameMode,
    /// Start bound
    pub start: FrameBound,
    /// End bound
    pub end: FrameBound,
    /// Exclusion mode
    pub exclude: ExcludeMode,
}

/// Frame mode for window frames
#[derive(Debug, Clone, PartialEq)]
pub enum FrameMode {
    /// ROWS mode - physical row offset
    Rows,
    /// RANGE mode - logical range based on ORDER BY values
    Range,
    /// GROUPS mode - group of equal values
    Groups,
}

/// Frame exclusion mode for window frames
#[derive(Debug, Clone, PartialEq)]
pub enum ExcludeMode {
    /// EXCLUDE NO OTHERS (default)
    None,
    /// EXCLUDE CURRENT ROW
    CurrentRow,
    /// EXCLUDE GROUP
    Group,
    /// EXCLUDE TIES
    Ties,
}

impl Expr {
    pub fn column(name: &str) -> Self {
        Expr::Column(Column::new(name.to_string()))
    }

    pub fn literal(value: Value) -> Self {
        Expr::Literal(value)
    }

    pub fn binary_expr(left: Expr, op: Operator, right: Expr) -> Self {
        Expr::BinaryExpr {
            left: Box::new(left),
            op,
            right: Box::new(right),
        }
    }

    /// Evaluate this expression against a row with a given schema.
    /// Returns `Some(Value)` for column references and literals,
    /// `None` for complex expressions (computed separately by executors).
    pub fn evaluate(&self, row: &[Value], schema: &Schema) -> Option<Value> {
        match self {
            Expr::Column(col) => {
                // Find column index in schema
                if let Some(idx) = schema.field_index(&col.name) {
                    row.get(idx).cloned()
                } else {
                    None
                }
            }
            Expr::Literal(value) => Some(value.clone()),
            // Binary and unary expressions require computation at runtime
            // They are not evaluated here - handled by execution layer
            Expr::BinaryExpr { .. } => None,
            Expr::UnaryExpr { .. } => None,
            Expr::AggregateFunction { .. } => {
                // Aggregates are computed separately, not here
                None
            }
            Expr::Alias { expr, .. } => expr.evaluate(row, schema),
            Expr::Wildcard => None,
            Expr::QualifiedWildcard { .. } => None,
            Expr::WindowFunction { .. } => {
                // Window functions are computed by WindowVolcanoExecutor
                None
            }
        }
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Column(col) => write!(f, "{}", col),
            Expr::Literal(val) => write!(f, "{}", val),
            Expr::BinaryExpr { left, op, right } => {
                write!(f, "({} {} {})", left, op, right)
            }
            Expr::UnaryExpr { op, expr } => write!(f, "({} {})", op, expr),
            Expr::AggregateFunction {
                func,
                args,
                distinct,
            } => {
                let func_name = match func {
                    AggregateFunction::Count => "COUNT",
                    AggregateFunction::Sum => "SUM",
                    AggregateFunction::Avg => "AVG",
                    AggregateFunction::Min => "MIN",
                    AggregateFunction::Max => "MAX",
                };
                let distinct_str = if *distinct { "DISTINCT " } else { "" };
                let args_str: Vec<String> = args.iter().map(|a| a.to_string()).collect();
                write!(f, "{}({}{})", func_name, distinct_str, args_str.join(", "))
            }
            Expr::Alias { expr, name } => write!(f, "{} AS {}", expr, name),
            Expr::Wildcard => write!(f, "*"),
            Expr::QualifiedWildcard { qualifier } => write!(f, "{}.*", qualifier),
            Expr::WindowFunction { func, args, .. } => {
                let func_name = match func {
                    WindowFunction::RowNumber => "ROW_NUMBER",
                    WindowFunction::Rank => "RANK",
                    WindowFunction::DenseRank => "DENSE_RANK",
                    WindowFunction::PercentRank => "PERCENT_RANK",
                    WindowFunction::CumeDist => "CUME_DIST",
                    WindowFunction::Lead { .. } => "LEAD",
                    WindowFunction::Lag { .. } => "LAG",
                    WindowFunction::FirstValue => "FIRST_VALUE",
                    WindowFunction::LastValue => "LAST_VALUE",
                    WindowFunction::NthValue { .. } => "NTH_VALUE",
                    WindowFunction::Count => "COUNT",
                    WindowFunction::Sum => "SUM",
                    WindowFunction::Avg => "AVG",
                    WindowFunction::Min => "MIN",
                    WindowFunction::Max => "MAX",
                };
                let args_str: Vec<String> = args.iter().map(|a| a.to_string()).collect();
                write!(f, "{}({})", func_name, args_str.join(", "))
            }
        }
    }
}

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operator::Eq => write!(f, "="),
            Operator::NotEq => write!(f, "!="),
            Operator::Lt => write!(f, "<"),
            Operator::LtEq => write!(f, "<="),
            Operator::Gt => write!(f, ">"),
            Operator::GtEq => write!(f, ">="),
            Operator::Plus => write!(f, "+"),
            Operator::Minus => write!(f, "-"),
            Operator::Multiply => write!(f, "*"),
            Operator::Divide => write!(f, "/"),
            Operator::Modulo => write!(f, "%"),
            Operator::And => write!(f, "AND"),
            Operator::Or => write!(f, "OR"),
            Operator::Not => write!(f, "NOT"),
            Operator::Like => write!(f, "LIKE"),
        }
    }
}

impl Schema {
    pub fn new(fields: Vec<Field>) -> Self {
        Self { fields }
    }

    pub fn empty() -> Self {
        Self { fields: vec![] }
    }

    pub fn field(&self, name: &str) -> Option<&Field> {
        self.fields.iter().find(|f| f.name == name)
    }

    pub fn field_index(&self, name: &str) -> Option<usize> {
        self.fields.iter().position(|f| f.name == name)
    }
}

impl Field {
    pub fn new(name: String, data_type: DataType) -> Self {
        Self {
            name,
            data_type,
            nullable: true,
        }
    }

    pub fn new_not_null(name: String, data_type: DataType) -> Self {
        Self {
            name,
            data_type,
            nullable: false,
        }
    }
}

impl DataType {
    pub fn from_sql_type(type_name: &str) -> Self {
        match type_name.to_uppercase().as_str() {
            "INTEGER" | "INT" => DataType::Integer,
            "FLOAT" | "DOUBLE" | "REAL" => DataType::Float,
            "TEXT" | "VARCHAR" | "CHAR" => DataType::Text,
            "BLOB" | "BINARY" => DataType::Blob,
            "BOOLEAN" | "BOOL" => DataType::Boolean,
            _ => DataType::Null,
        }
    }
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataType::Boolean => write!(f, "BOOLEAN"),
            DataType::Integer => write!(f, "INTEGER"),
            DataType::Float => write!(f, "FLOAT"),
            DataType::Text => write!(f, "TEXT"),
            DataType::Blob => write!(f, "BLOB"),
            DataType::Null => write!(f, "NULL"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_join_type_values() {
        assert_eq!(format!("{:?}", JoinType::Inner), "Inner");
        assert_eq!(format!("{:?}", JoinType::Left), "Left");
        assert_eq!(format!("{:?}", JoinType::Right), "Right");
        assert_eq!(format!("{:?}", JoinType::Full), "Full");
        assert_eq!(format!("{:?}", JoinType::Cross), "Cross");
        assert_eq!(format!("{:?}", JoinType::LeftSemi), "LeftSemi");
        assert_eq!(format!("{:?}", JoinType::LeftAnti), "LeftAnti");
        assert_eq!(format!("{:?}", JoinType::RightSemi), "RightSemi");
        assert_eq!(format!("{:?}", JoinType::RightAnti), "RightAnti");
    }

    #[test]
    fn test_join_type_clone() {
        let jt = JoinType::Left.clone();
        assert_eq!(jt, JoinType::Left);
    }

    #[test]
    fn test_aggregate_function_values() {
        assert_eq!(format!("{:?}", AggregateFunction::Count), "Count");
        assert_eq!(format!("{:?}", AggregateFunction::Sum), "Sum");
        assert_eq!(format!("{:?}", AggregateFunction::Avg), "Avg");
        assert_eq!(format!("{:?}", AggregateFunction::Min), "Min");
        assert_eq!(format!("{:?}", AggregateFunction::Max), "Max");
    }

    #[test]
    fn test_aggregate_function_clone() {
        let af = AggregateFunction::Count.clone();
        assert_eq!(af, AggregateFunction::Count);
    }

    #[test]
    fn test_operator_display() {
        assert_eq!(format!("{}", Operator::Eq), "=");
        assert_eq!(format!("{}", Operator::NotEq), "!=");
        assert_eq!(format!("{}", Operator::Lt), "<");
        assert_eq!(format!("{}", Operator::LtEq), "<=");
        assert_eq!(format!("{}", Operator::Gt), ">");
        assert_eq!(format!("{}", Operator::GtEq), ">=");
        assert_eq!(format!("{}", Operator::Plus), "+");
        assert_eq!(format!("{}", Operator::Minus), "-");
        assert_eq!(format!("{}", Operator::Multiply), "*");
        assert_eq!(format!("{}", Operator::Divide), "/");
        assert_eq!(format!("{}", Operator::Modulo), "%");
        assert_eq!(format!("{}", Operator::And), "AND");
        assert_eq!(format!("{}", Operator::Or), "OR");
        assert_eq!(format!("{}", Operator::Not), "NOT");
        assert_eq!(format!("{}", Operator::Like), "LIKE");
    }

    #[test]
    fn test_operator_clone() {
        let op = Operator::Eq.clone();
        assert_eq!(op, Operator::Eq);
    }

    #[test]
    fn test_operator_partial_eq() {
        assert_eq!(Operator::Eq, Operator::Eq);
        assert_ne!(Operator::Eq, Operator::NotEq);
    }

    #[test]
    fn test_sort_expr() {
        let expr = Expr::column("x");
        let sort = SortExpr {
            expr,
            asc: true,
            nulls_first: true,
        };
        assert!(sort.asc);
        assert!(sort.nulls_first);
    }

    #[test]
    fn test_column_new() {
        let col = Column::new("id".to_string());
        assert_eq!(col.name, "id");
        assert!(col.relation.is_none());
    }

    #[test]
    fn test_column_new_qualified() {
        let col = Column::new_qualified("users".to_string(), "id".to_string());
        assert_eq!(col.name, "id");
        assert_eq!(col.relation, Some("users".to_string()));
    }

    #[test]
    fn test_column_display() {
        let col = Column::new("id".to_string());
        assert_eq!(format!("{}", col), "id");
        let qualified = Column::new_qualified("users".to_string(), "id".to_string());
        assert_eq!(format!("{}", qualified), "users.id");
    }

    #[test]
    fn test_column_clone() {
        let col = Column::new("id".to_string());
        let cloned = col.clone();
        assert_eq!(col, cloned);
    }

    #[test]
    fn test_expr_column() {
        let expr = Expr::column("name");
        match expr {
            Expr::Column(col) => assert_eq!(col.name, "name"),
            _ => panic!("Expected Column variant"),
        }
    }

    #[test]
    fn test_expr_literal() {
        let expr = Expr::literal(Value::Integer(42));
        match expr {
            Expr::Literal(v) => assert_eq!(v, Value::Integer(42)),
            _ => panic!("Expected Literal variant"),
        }
    }

    #[test]
    fn test_expr_binary_expr() {
        let expr = Expr::binary_expr(Expr::column("a"), Operator::Plus, Expr::column("b"));
        match expr {
            Expr::BinaryExpr { left, op, right } => {
                assert!(matches!(*left, Expr::Column(_)));
                assert_eq!(op, Operator::Plus);
                assert!(matches!(*right, Expr::Column(_)));
            }
            _ => panic!("Expected BinaryExpr variant"),
        }
    }

    #[test]
    fn test_expr_display() {
        let col = Expr::column("x");
        assert_eq!(format!("{}", col), "x");
        let lit = Expr::literal(Value::Integer(5));
        assert_eq!(format!("{}", lit), "5");
    }

    #[test]
    fn test_expr_clone() {
        let expr = Expr::column("x");
        let cloned = expr.clone();
        assert_eq!(expr, cloned);
    }

    #[test]
    fn test_expr_aggregate_function() {
        let expr = Expr::AggregateFunction {
            func: AggregateFunction::Count,
            args: vec![Expr::Wildcard],
            distinct: false,
        };
        assert_eq!(format!("{}", expr), "COUNT(*)");
    }

    #[test]
    fn test_expr_alias() {
        let expr = Expr::Alias {
            expr: Box::new(Expr::column("x")),
            name: "alias_x".to_string(),
        };
        assert_eq!(format!("{}", expr), "x AS alias_x");
    }

    #[test]
    fn test_expr_wildcard() {
        let expr = Expr::Wildcard;
        assert_eq!(format!("{}", expr), "*");
    }

    #[test]
    fn test_expr_qualified_wildcard() {
        let expr = Expr::QualifiedWildcard {
            qualifier: "users".to_string(),
        };
        assert_eq!(format!("{}", expr), "users.*");
    }

    #[test]
    fn test_schema_new() {
        let field = Field::new("id".to_string(), DataType::Integer);
        let schema = Schema::new(vec![field]);
        assert_eq!(schema.fields.len(), 1);
    }

    #[test]
    fn test_schema_empty() {
        let schema = Schema::empty();
        assert!(schema.fields.is_empty());
    }

    #[test]
    fn test_schema_field() {
        let field = Field::new("id".to_string(), DataType::Integer);
        let schema = Schema::new(vec![field]);
        assert!(schema.field("id").is_some());
        assert!(schema.field("name").is_none());
    }

    #[test]
    fn test_schema_field_index() {
        let field = Field::new("id".to_string(), DataType::Integer);
        let schema = Schema::new(vec![field]);
        assert_eq!(schema.field_index("id"), Some(0));
        assert_eq!(schema.field_index("name"), None);
    }

    #[test]
    fn test_schema_clone() {
        let schema = Schema::empty();
        let cloned = schema.clone();
        assert_eq!(schema, cloned);
    }

    #[test]
    fn test_field_new() {
        let field = Field::new("name".to_string(), DataType::Text);
        assert_eq!(field.name, "name");
        assert_eq!(field.data_type, DataType::Text);
        assert!(field.nullable);
    }

    #[test]
    fn test_field_new_not_null() {
        let field = Field::new_not_null("id".to_string(), DataType::Integer);
        assert!(!field.nullable);
        assert_eq!(field.data_type, DataType::Integer);
    }

    #[test]
    fn test_field_clone() {
        let field = Field::new("x".to_string(), DataType::Integer);
        let cloned = field.clone();
        assert_eq!(field, cloned);
    }

    #[test]
    fn test_data_type_from_sql_type() {
        assert_eq!(DataType::from_sql_type("INTEGER"), DataType::Integer);
        assert_eq!(DataType::from_sql_type("INT"), DataType::Integer);
        assert_eq!(DataType::from_sql_type("FLOAT"), DataType::Float);
        assert_eq!(DataType::from_sql_type("DOUBLE"), DataType::Float);
        assert_eq!(DataType::from_sql_type("TEXT"), DataType::Text);
        assert_eq!(DataType::from_sql_type("VARCHAR"), DataType::Text);
        assert_eq!(DataType::from_sql_type("BLOB"), DataType::Blob);
        assert_eq!(DataType::from_sql_type("BOOLEAN"), DataType::Boolean);
        assert_eq!(DataType::from_sql_type("UNKNOWN"), DataType::Null);
    }

    #[test]
    fn test_data_type_display() {
        assert_eq!(format!("{}", DataType::Boolean), "BOOLEAN");
        assert_eq!(format!("{}", DataType::Integer), "INTEGER");
        assert_eq!(format!("{}", DataType::Float), "FLOAT");
        assert_eq!(format!("{}", DataType::Text), "TEXT");
        assert_eq!(format!("{}", DataType::Blob), "BLOB");
        assert_eq!(format!("{}", DataType::Null), "NULL");
    }

    #[test]
    fn test_data_type_clone() {
        let dt = DataType::Integer.clone();
        assert_eq!(dt, DataType::Integer);
    }

    #[test]
    fn test_data_type_partial_eq() {
        assert_eq!(DataType::Integer, DataType::Integer);
        assert_ne!(DataType::Integer, DataType::Float);
    }
}
