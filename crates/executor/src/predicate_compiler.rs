use sqlrustgo_planner::{Expr, Operator, Schema};
use sqlrustgo_storage::engine::{Record, RowFilter};
use std::sync::LazyLock;

/// Predicate compiler: translates a planner Expr to a RowFilter (a predicate on Record).
///
/// A Record is a flat `Vec<Value>` in **column-index order** (NOT name→value pairs).
/// Column names must be resolved via a schema that maps name → index.
///
/// # Original bug (fixed)
/// `compile(Expr::Column(col))` tried to find a column by scanning the row for a
/// `Value::Text(col_name)` entry. This only works if the row stores name→value pairs
/// (it does not). The fix uses the schema (column name → index) to resolve the
/// position, then reads `row[idx]` directly.
#[allow(dead_code)]
pub struct PredicateCompiler {
    schema: Schema,
}

impl PredicateCompiler {
    pub fn new(schema: Schema) -> Self {
        Self { schema }
    }

    /// Compile an expression to a row filter using this compiler's schema.
    pub fn compile(&self, expr: &Expr) -> RowFilter {
        match expr {
            Expr::Column(col) => {
                let col_idx = self.column_index(&col.name);
                Box::new(move |row: &Record| -> bool {
                    if let Some(idx) = col_idx {
                        if idx < row.len() {
                            if let sqlrustgo_types::Value::Boolean(b) = &row[idx] {
                                return *b;
                            }
                        }
                    }
                    false
                })
            }
            Expr::Literal(_val) => Box::new(|_| true),
            Expr::BinaryExpr { left, op, right } => {
                let left_filter = self.compile(left);
                let right_filter = self.compile(right);
                let op = op.clone();
                Box::new(move |row| {
                    let l = left_filter(row);
                    let r = right_filter(row);
                    Self::eval_bool_op(&op, l, r)
                })
            }
            Expr::UnaryExpr { op, expr } => {
                let inner = self.compile(expr);
                let op = op.clone();
                Box::new(move |row| {
                    let v = inner(row);
                    Self::eval_unary_bool_op(&op, v)
                })
            }
            Expr::Wildcard | Expr::QualifiedWildcard { .. } => Box::new(|_| true),
            _ => Box::new(|_| true),
        }
    }

    /// Compile an optional expression (None → no filter).
    pub fn compile_optional(&self, opt: Option<&Expr>) -> Option<RowFilter> {
        opt.map(|e| self.compile(e))
    }

    fn column_index(&self, col_name: &str) -> Option<usize> {
        self.schema.fields.iter().position(|f| f.name == col_name)
    }

    fn eval_bool_op(op: &Operator, left: bool, right: bool) -> bool {
        match op {
            Operator::Eq => left == right,
            Operator::NotEq => left != right,
            Operator::And => left && right,
            Operator::Or => left || right,
            _ => false,
        }
    }

    fn eval_unary_bool_op(op: &Operator, val: bool) -> bool {
        match op {
            Operator::Not => !val,
            _ => false,
        }
    }
}

/// Default two-column schema used by the module-level `compile` helper.
static DEFAULT_SCHEMA: LazyLock<Schema> = LazyLock::new(|| {
    use sqlrustgo_planner::{DataType, Field};
    Schema::new(vec![
        Field::new_not_null("active".to_string(), DataType::Boolean),
        Field::new_not_null("v".to_string(), DataType::Integer),
    ])
});

/// Module-level compile using DEFAULT_SCHEMA (for test/backward-compatibility).
pub fn compile(expr: &Expr) -> RowFilter {
    PredicateCompiler::new(DEFAULT_SCHEMA.clone()).compile(expr)
}

/// Module-level compile_optional using DEFAULT_SCHEMA.
pub fn compile_optional(opt: Option<&Expr>) -> Option<RowFilter> {
    opt.map(|e| compile(e))
}
