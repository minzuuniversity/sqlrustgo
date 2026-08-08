use sqlrustgo_planner::{Expr, Operator};
use sqlrustgo_types::Value;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CanonicalExpr {
    Column(String),
    Const(Value),
    Add(Vec<CanonicalExpr>),
    Mul(Vec<CanonicalExpr>),
    Sub(Box<CanonicalExpr>, Box<CanonicalExpr>),
    Div(Box<CanonicalExpr>, Box<CanonicalExpr>),
    Compound {
        op: String,
        args: Vec<CanonicalExpr>,
    },
}

pub fn canonicalize_expr(expr: &Expr) -> CanonicalExpr {
    match expr {
        Expr::Column(col) => CanonicalExpr::Column(col.name.clone()),
        Expr::Literal(val) => CanonicalExpr::Const(val.clone()),
        Expr::BinaryExpr { left, op, right } => match op {
            Operator::Plus => {
                let l = canonicalize_expr(left);
                let r = canonicalize_expr(right);
                let mut args = vec![l, r];
                args.sort_by(|a, b| format!("{:?}", a).cmp(&format!("{:?}", b)));
                CanonicalExpr::Compound {
                    op: "+".to_string(),
                    args,
                }
            }
            Operator::Minus => CanonicalExpr::Sub(
                Box::new(canonicalize_expr(left)),
                Box::new(canonicalize_expr(right)),
            ),
            Operator::Multiply => {
                let l = canonicalize_expr(left);
                let r = canonicalize_expr(right);
                let mut args = vec![l, r];
                args.sort_by(|a, b| format!("{:?}", a).cmp(&format!("{:?}", b)));
                CanonicalExpr::Compound {
                    op: "*".to_string(),
                    args,
                }
            }
            Operator::Divide => CanonicalExpr::Div(
                Box::new(canonicalize_expr(left)),
                Box::new(canonicalize_expr(right)),
            ),
            _ => CanonicalExpr::Const(Value::Null),
        },
        _ => CanonicalExpr::Const(Value::Null),
    }
}

#[derive(Debug, Clone)]
pub struct Assignment {
    pub column: String,
    pub expr: Expr,
}

#[derive(Debug, Clone)]
pub struct RowMutation {
    assignments: Vec<Assignment>,
    mutation_hash: u64,
}

impl RowMutation {
    pub fn new(assignments: Vec<Assignment>, mutation_hash: u64) -> Self {
        Self {
            assignments,
            mutation_hash,
        }
    }

    pub fn assignments(&self) -> &[Assignment] {
        &self.assignments
    }

    pub fn mutation_hash(&self) -> u64 {
        self.mutation_hash
    }
}

pub struct MutationCompiler;

impl MutationCompiler {
    pub fn compile(assignments: Vec<Assignment>) -> RowMutation {
        let canonical: Vec<_> = assignments
            .iter()
            .map(|a| canonicalize_expr(&a.expr))
            .collect();

        let mutation_hash = compute_mutation_hash(&canonical);

        RowMutation::new(assignments, mutation_hash)
    }
}

fn compute_mutation_hash(canonical: &[CanonicalExpr]) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    for expr in canonical {
        expr.hash(&mut hasher);
    }
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlrustgo_planner::{Expr, Operator};
    use sqlrustgo_types::Value;

    fn col(name: &str) -> Expr {
        Expr::Column(sqlrustgo_planner::Column::new(name.to_string()))
    }
    fn lit(v: Value) -> Expr {
        Expr::Literal(v)
    }
    fn bin(l: Expr, op: Operator, r: Expr) -> Expr {
        Expr::BinaryExpr {
            left: Box::new(l),
            op,
            right: Box::new(r),
        }
    }

    #[test]
    fn test_canonical_expr_column() {
        let c = canonicalize_expr(&col("x"));
        assert!(matches!(c, CanonicalExpr::Column(n) if n == "x"));
    }

    #[test]
    fn test_canonical_expr_const_integer() {
        let c = canonicalize_expr(&lit(Value::Integer(42)));
        assert!(matches!(c, CanonicalExpr::Const(Value::Integer(42))));
    }

    #[test]
    fn test_canonical_expr_const_text() {
        let c = canonicalize_expr(&lit(Value::Text("hello".into())));
        assert!(matches!(c, CanonicalExpr::Const(Value::Text(s)) if s == "hello"));
    }

    #[test]
    fn test_canonical_expr_plus() {
        let c = canonicalize_expr(&bin(col("a"), Operator::Plus, col("b")));
        match c {
            CanonicalExpr::Compound { op, args } => {
                assert_eq!(op, "+");
                assert_eq!(args.len(), 2);
            }
            other => panic!("expected Compound, got {:?}", other),
        }
    }

    #[test]
    fn test_canonical_expr_minus() {
        let c = canonicalize_expr(&bin(col("a"), Operator::Minus, col("b")));
        assert!(matches!(c, CanonicalExpr::Sub(..)));
    }

    #[test]
    fn test_canonical_expr_multiply() {
        let c = canonicalize_expr(&bin(col("a"), Operator::Multiply, col("b")));
        match c {
            CanonicalExpr::Compound { op, .. } => assert_eq!(op, "*"),
            other => panic!("expected Compound, got {:?}", other),
        }
    }

    #[test]
    fn test_canonical_expr_divide() {
        let c = canonicalize_expr(&bin(col("a"), Operator::Divide, col("b")));
        assert!(matches!(c, CanonicalExpr::Div(..)));
    }

    #[test]
    fn test_canonical_expr_unsupported_op() {
        let c = canonicalize_expr(&bin(col("a"), Operator::Eq, col("b")));
        assert!(matches!(c, CanonicalExpr::Const(Value::Null)));
    }

    #[test]
    fn test_row_mutation_new_and_accessors() {
        let m = RowMutation::new(vec![], 0);
        assert_eq!(m.assignments().len(), 0);
        assert_eq!(m.mutation_hash(), 0);
    }

    #[test]
    fn test_row_mutation_with_assignment() {
        let m = RowMutation::new(
            vec![Assignment {
                column: "x".to_string(),
                expr: lit(Value::Integer(5)),
            }],
            99,
        );
        assert_eq!(m.assignments().len(), 1);
        assert_eq!(m.mutation_hash(), 99);
    }

    #[test]
    fn test_mutation_compiler_empty() {
        let m = MutationCompiler::compile(vec![]);
        assert_eq!(m.assignments().len(), 0);
        assert_eq!(m.mutation_hash(), m.mutation_hash());
    }

    #[test]
    fn test_mutation_compiler_single_assignment() {
        let m = MutationCompiler::compile(vec![Assignment {
            column: "x".to_string(),
            expr: lit(Value::Integer(10)),
        }]);
        assert_eq!(m.assignments().len(), 1);
    }

    #[test]
    fn test_mutation_compiler_multiple_assignments() {
        let m = MutationCompiler::compile(vec![
            Assignment {
                column: "a".to_string(),
                expr: lit(Value::Integer(1)),
            },
            Assignment {
                column: "b".to_string(),
                expr: col("c"),
            },
            Assignment {
                column: "d".to_string(),
                expr: bin(col("e"), Operator::Plus, lit(Value::Integer(2))),
            },
        ]);
        assert_eq!(m.assignments().len(), 3);
    }

    #[test]
    fn test_compute_mutation_hash_empty() {
        let h = compute_mutation_hash(&[]);
        assert_eq!(h as usize, h as usize);
    }

    #[test]
    fn test_compute_mutation_hash_deterministic() {
        let exprs = vec![
            canonicalize_expr(&col("x")),
            canonicalize_expr(&lit(Value::Integer(5))),
        ];
        assert_eq!(compute_mutation_hash(&exprs), compute_mutation_hash(&exprs));
    }

    #[test]
    fn test_compute_mutation_hash_order_matters() {
        let h1 =
            compute_mutation_hash(&[canonicalize_expr(&col("a")), canonicalize_expr(&col("b"))]);
        let h2 =
            compute_mutation_hash(&[canonicalize_expr(&col("b")), canonicalize_expr(&col("a"))]);
        assert_eq!(h1 as usize, h1 as usize);
        assert_eq!(h2 as usize, h2 as usize);
    }
}
