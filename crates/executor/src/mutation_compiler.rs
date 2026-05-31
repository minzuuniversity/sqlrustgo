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
    use sqlrustgo_planner::Expr;
    use sqlrustgo_types::Value;

    #[test]
    fn test_canonicalize_literal() {
        let expr = Expr::Literal(Value::Integer(42));
        let canon = canonicalize_expr(&expr);
        assert!(matches!(canon, CanonicalExpr::Const(Value::Integer(42))));
    }

    #[test]
    fn test_canonicalize_column() {
        let expr = Expr::Column(sqlrustgo_planner::Column { name: "id".into(), relation: None });
        let canon = canonicalize_expr(&expr);
        assert!(matches!(canon, CanonicalExpr::Column(ref s) if s == "id"));
    }

    #[test]
    fn test_mutation_compiler_compile() {
        let assignments = vec![
            Assignment { column: "name".into(), expr: Expr::Literal(Value::Text("Alice".into())) },
        ];
        let mutation = MutationCompiler::compile(assignments.clone());
        assert_eq!(mutation.assignments().len(), 1);
        assert_eq!(mutation.assignments()[0].column, "name");
    }

    #[test]
    fn test_row_mutation_new() {
        let mutation = RowMutation::new(vec![], 42);
        assert_eq!(mutation.mutation_hash(), 42);
    }

    #[test]
    fn test_canonical_expr_debug() {
        let c = CanonicalExpr::Column("x".into());
        assert!(!format!("{:?}", c).is_empty());
    }
}
