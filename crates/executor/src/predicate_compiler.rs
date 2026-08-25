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

#[cfg(test)]
mod tests {
    use super::*;
    use sqlrustgo_planner::{Expr, Operator};
    use sqlrustgo_types::Value;

    fn bool_record(vals: &[bool]) -> Record {
        vals.iter().map(|&b| Value::Boolean(b)).collect()
    }

    #[test]
    fn test_compile_literal() {
        let filter = compile(&Expr::Literal(Value::Integer(42)));
        assert!(filter(&bool_record(&[true])));
    }

    #[test]
    fn test_compile_wildcard() {
        let filter = compile(&Expr::Wildcard);
        assert!(filter(&bool_record(&[true])));
    }

    #[test]
    fn test_compile_unknown() {
        let filter = compile(&Expr::Literal(Value::Null));
        assert!(filter(&bool_record(&[false; 2])));
    }

    #[test]
    fn test_predicate_compiler_new() {
        use sqlrustgo_planner::{DataType, Field, Schema};
        let schema = Schema::new(vec![Field::new_not_null(
            "x".to_string(),
            DataType::Integer,
        )]);
        let compiler = PredicateCompiler::new(schema);
        let filter = compiler.compile(&Expr::Literal(Value::Integer(1)));
        assert!(filter(&vec![]));
    }

    #[test]
    fn test_column_index() {
        use sqlrustgo_planner::{DataType, Field, Schema};
        let schema = Schema::new(vec![
            Field::new_not_null("a".to_string(), DataType::Integer),
            Field::new_not_null("b".to_string(), DataType::Boolean),
        ]);
        let compiler = PredicateCompiler::new(schema);
        let filter = compiler.compile(&Expr::Column(sqlrustgo_planner::Column::new(
            "b".to_string(),
        )));
        let true_row: Record = vec![Value::Integer(1), Value::Boolean(true)];
        let false_row: Record = vec![Value::Integer(1), Value::Boolean(false)];
        assert!(filter(&true_row));
        assert!(!filter(&false_row));
    }

    #[test]
    fn test_compile_optional_some() {
        use sqlrustgo_planner::{DataType, Field, Schema};
        let schema = Schema::new(vec![Field::new_not_null(
            "x".to_string(),
            DataType::Integer,
        )]);
        let compiler = PredicateCompiler::new(schema);
        assert!(compiler
            .compile_optional(Some(&Expr::Literal(Value::Integer(5))))
            .is_some());
    }

    #[test]
    fn test_compile_optional_none() {
        use sqlrustgo_planner::{DataType, Field, Schema};
        let schema = Schema::new(vec![Field::new_not_null(
            "x".to_string(),
            DataType::Integer,
        )]);
        let compiler = PredicateCompiler::new(schema);
        assert!(compiler.compile_optional(None).is_none());
    }

    #[test]
    fn test_module_compile_optional_some() {
        assert!(compile_optional(Some(&Expr::Literal(Value::Text("hello".into())))).is_some());
    }

    #[test]
    fn test_module_compile_optional_none() {
        assert!(compile_optional(None).is_none());
    }

    // ---- Additional coverage: BinaryExpr + UnaryExpr + edge cases ----

    fn two_col_schema() -> Schema {
        use sqlrustgo_planner::{DataType, Field};
        Schema::new(vec![
            Field::new_not_null("a".to_string(), DataType::Boolean),
            Field::new_not_null("b".to_string(), DataType::Boolean),
        ])
    }

    #[test]
    fn compile_binary_and() {
        let c = PredicateCompiler::new(two_col_schema());
        let expr = Expr::BinaryExpr {
            left: Box::new(Expr::Column(sqlrustgo_planner::Column::new("a".into()))),
            op: Operator::And,
            right: Box::new(Expr::Column(sqlrustgo_planner::Column::new("b".into()))),
        };
        let f = c.compile(&expr);
        assert!(f(&vec![Value::Boolean(true), Value::Boolean(true)]));
        assert!(!f(&vec![Value::Boolean(true), Value::Boolean(false)]));
        assert!(!f(&vec![Value::Boolean(false), Value::Boolean(true)]));
    }

    #[test]
    fn compile_binary_or() {
        let c = PredicateCompiler::new(two_col_schema());
        let expr = Expr::BinaryExpr {
            left: Box::new(Expr::Column(sqlrustgo_planner::Column::new("a".into()))),
            op: Operator::Or,
            right: Box::new(Expr::Column(sqlrustgo_planner::Column::new("b".into()))),
        };
        let f = c.compile(&expr);
        assert!(f(&vec![Value::Boolean(true), Value::Boolean(false)]));
        assert!(!f(&vec![Value::Boolean(false), Value::Boolean(false)]));
    }
    #[test]
    fn compile_binary_eq_on_columns() {
        // Both sides are column refs: left_filter reads col 'a',
        // right_filter reads col 'b'; equality compares their bool values.
        let c = PredicateCompiler::new(two_col_schema());
        let expr = Expr::BinaryExpr {
            left: Box::new(Expr::Column(sqlrustgo_planner::Column::new("a".into()))),
            op: Operator::Eq,
            right: Box::new(Expr::Column(sqlrustgo_planner::Column::new("b".into()))),
        };
        let f = c.compile(&expr);
        assert!(f(&vec![Value::Boolean(true), Value::Boolean(true)]));
        assert!(!f(&vec![Value::Boolean(true), Value::Boolean(false)]));
    }

    #[test]
    fn compile_binary_noteq_on_columns() {
        let c = PredicateCompiler::new(two_col_schema());
        let expr = Expr::BinaryExpr {
            left: Box::new(Expr::Column(sqlrustgo_planner::Column::new("a".into()))),
            op: Operator::NotEq,
            right: Box::new(Expr::Column(sqlrustgo_planner::Column::new("b".into()))),
        };
        let f = c.compile(&expr);
        assert!(f(&vec![Value::Boolean(true), Value::Boolean(false)]));
        assert!(!f(&vec![Value::Boolean(true), Value::Boolean(true)]));
    }

    #[test]
    fn compile_unary_not_on_column() {
        // Inner expr is a column ref so inner_filter reads actual values.
        let c = PredicateCompiler::new(two_col_schema());
        let expr = Expr::UnaryExpr {
            op: Operator::Not,
            expr: Box::new(Expr::Column(sqlrustgo_planner::Column::new("a".into()))),
        };
        let f = c.compile(&expr);
        assert!(!f(&vec![Value::Boolean(true), Value::Boolean(false)]));
        assert!(f(&vec![Value::Boolean(false), Value::Boolean(true)]));
    }

    #[test]
    fn compile_unary_unsupported_op_returns_false() {
        // Operator::Plus on unary → eval_unary_bool_op returns false.
        let c = PredicateCompiler::new(two_col_schema());
        let expr = Expr::UnaryExpr {
            op: Operator::Plus,
            expr: Box::new(Expr::Literal(Value::Integer(1))),
        };
        let f = c.compile(&expr);
        assert!(!f(&vec![]));
    }

    #[test]
    fn compile_qualified_wildcard_always_true() {
        let c = PredicateCompiler::new(two_col_schema());
        let expr = Expr::QualifiedWildcard {
            qualifier: "t".to_string(),
        };
        let f = c.compile(&expr);
        assert!(f(&vec![]));
    }

    #[test]
    fn compile_column_with_non_boolean_value_returns_false() {
        let c = PredicateCompiler::new(two_col_schema());
        let expr = Expr::Column(sqlrustgo_planner::Column::new("a".into()));
        let f = c.compile(&expr);
        let row = vec![Value::Integer(1), Value::Boolean(true)];
        // column 'a' holds Integer, not Boolean → filter returns false.
        assert!(!f(&row));
    }

    #[test]
    fn compile_unknown_column_returns_false() {
        let c = PredicateCompiler::new(two_col_schema());
        let expr = Expr::Column(sqlrustgo_planner::Column::new("missing".into()));
        let f = c.compile(&expr);
        assert!(!f(&vec![Value::Boolean(true), Value::Boolean(true)]));
    }

    #[test]
    fn compile_column_index_out_of_row_returns_false() {
        let c = PredicateCompiler::new(two_col_schema());
        let expr = Expr::Column(sqlrustgo_planner::Column::new("a".into()));
        let f = c.compile(&expr);
        // Row too short — idx 0 out of range.
        assert!(!f(&vec![]));
    }

    #[test]
    fn compile_nested_binary_unary() {
        // !(a AND b)
        let c = PredicateCompiler::new(two_col_schema());
        let inner = Expr::BinaryExpr {
            left: Box::new(Expr::Column(sqlrustgo_planner::Column::new("a".into()))),
            op: Operator::And,
            right: Box::new(Expr::Column(sqlrustgo_planner::Column::new("b".into()))),
        };
        let expr = Expr::UnaryExpr {
            op: Operator::Not,
            expr: Box::new(inner),
        };
        let f = c.compile(&expr);
        assert!(!f(&vec![Value::Boolean(true), Value::Boolean(true)]));
        assert!(f(&vec![Value::Boolean(true), Value::Boolean(false)]));
    }
}
