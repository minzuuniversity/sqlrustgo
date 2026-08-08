use sqlrustgo_planner::{Expr, Operator, Schema};
use sqlrustgo_storage::engine::RowFilter;

use crate::mutation_compiler::{Assignment, MutationCompiler, RowMutation};
use crate::predicate_compiler::PredicateCompiler;
use sqlrustgo_types::SqlError;

pub struct UpdateStatement {
    pub table: String,
    pub where_clause: Option<Expr>,
    pub assignments: Vec<Assignment>,
}

pub struct UpdatePlan {
    predicate: RowFilter,
    mutation: RowMutation,
    predicate_hash: u64,
    mutation_hash: u64,
    combined_hash: u64,
}

impl UpdatePlan {
    pub fn predicate(&self) -> &RowFilter {
        &self.predicate
    }

    pub fn mutation(&self) -> &RowMutation {
        &self.mutation
    }

    pub fn predicate_hash(&self) -> u64 {
        self.predicate_hash
    }

    pub fn mutation_hash(&self) -> u64 {
        self.mutation_hash
    }

    pub fn combined_hash(&self) -> u64 {
        self.combined_hash
    }
}

pub struct UpdateCompiler;

impl UpdateCompiler {
    pub fn compile(stmt: &UpdateStatement, _schema: &Schema) -> Result<UpdatePlan, SqlError> {
        let compiler = PredicateCompiler::new(_schema.clone());
        let predicate = stmt
            .where_clause
            .as_ref()
            .map(|e| compiler.compile(e))
            .unwrap_or_else(|| Box::new(|_| true));

        let mutation = MutationCompiler::compile(stmt.assignments.clone());

        let predicate_hash = compute_predicate_hash(stmt.where_clause.as_ref());
        let mutation_hash = mutation.mutation_hash();
        let combined_hash = combine_hash(predicate_hash, mutation_hash);

        Ok(UpdatePlan {
            predicate,
            mutation,
            predicate_hash,
            mutation_hash,
            combined_hash,
        })
    }
}

fn compute_predicate_hash(expr: Option<&Expr>) -> u64 {
    match expr {
        Some(e) => {
            let canonical = canonicalize_predicate_expr(e);
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            canonical.hash(&mut hasher);
            hasher.finish()
        }
        None => 0,
    }
}

fn canonicalize_predicate_expr(expr: &Expr) -> crate::mutation_compiler::CanonicalExpr {
    match expr {
        Expr::BinaryExpr { left, op, right } => match op {
            Operator::Eq
            | Operator::NotEq
            | Operator::Lt
            | Operator::LtEq
            | Operator::Gt
            | Operator::GtEq => {
                let l = canonicalize_predicate_expr(left);
                let r = canonicalize_predicate_expr(right);
                crate::mutation_compiler::CanonicalExpr::Compound {
                    op: format!("{:?}", op),
                    args: {
                        let mut args = vec![l, r];
                        args.sort_by(|a, b| format!("{:?}", a).cmp(&format!("{:?}", b)));
                        args
                    },
                }
            }
            Operator::And => {
                let l = canonicalize_predicate_expr(left);
                let r = canonicalize_predicate_expr(right);
                crate::mutation_compiler::CanonicalExpr::Compound {
                    op: "AND".to_string(),
                    args: vec![l, r],
                }
            }
            Operator::Or => {
                let l = canonicalize_predicate_expr(left);
                let r = canonicalize_predicate_expr(right);
                crate::mutation_compiler::CanonicalExpr::Compound {
                    op: "OR".to_string(),
                    args: vec![l, r],
                }
            }
            Operator::Not => {
                let inner = canonicalize_predicate_expr(right);
                crate::mutation_compiler::CanonicalExpr::Compound {
                    op: "NOT".to_string(),
                    args: vec![inner],
                }
            }
            _ => crate::mutation_compiler::CanonicalExpr::Const(sqlrustgo_types::Value::Null),
        },
        Expr::Column(col) => crate::mutation_compiler::CanonicalExpr::Column(col.name.clone()),
        Expr::Literal(val) => crate::mutation_compiler::CanonicalExpr::Const(val.clone()),
        _ => crate::mutation_compiler::CanonicalExpr::Const(sqlrustgo_types::Value::Null),
    }
}

fn combine_hash(a: u64, b: u64) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    a.hash(&mut hasher);
    b.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlrustgo_planner::{Expr, Operator};
    use sqlrustgo_types::Value;

    fn ident(name: &str) -> Expr {
        Expr::Column(sqlrustgo_planner::Column::new(name.to_string()))
    }
    fn lit(v: Value) -> Expr {
        Expr::Literal(v)
    }
    fn binary(left: Expr, op: Operator, right: Expr) -> Expr {
        Expr::BinaryExpr {
            left: Box::new(left),
            op,
            right: Box::new(right),
        }
    }

    #[test]
    fn test_compute_predicate_hash_none() {
        assert_eq!(compute_predicate_hash(None), 0);
    }

    #[test]
    fn test_compute_predicate_hash_deterministic() {
        let e = binary(ident("x"), Operator::Eq, lit(Value::Integer(5)));
        assert_eq!(
            compute_predicate_hash(Some(&e)),
            compute_predicate_hash(Some(&e))
        );
    }

    #[test]
    fn test_compute_predicate_hash_eq_vs_ne() {
        let eq = binary(ident("x"), Operator::Eq, lit(Value::Integer(5)));
        let ne = binary(ident("x"), Operator::NotEq, lit(Value::Integer(5)));
        assert_ne!(
            compute_predicate_hash(Some(&eq)),
            compute_predicate_hash(Some(&ne))
        );
    }

    #[test]
    fn test_canonicalize_column() {
        match canonicalize_predicate_expr(&ident("foo")) {
            crate::mutation_compiler::CanonicalExpr::Column(n) => assert_eq!(n, "foo"),
            other => panic!("expected Column, got {:?}", other),
        }
    }

    #[test]
    fn test_canonicalize_literal() {
        match canonicalize_predicate_expr(&lit(Value::Text("hello".into()))) {
            crate::mutation_compiler::CanonicalExpr::Const(Value::Text(s)) => {
                assert_eq!(s, "hello")
            }
            other => panic!("expected Const, got {:?}", other),
        }
    }

    #[test]
    fn test_canonicalize_eq() {
        match canonicalize_predicate_expr(&binary(
            ident("x"),
            Operator::Eq,
            lit(Value::Integer(42)),
        )) {
            crate::mutation_compiler::CanonicalExpr::Compound { op, args } => {
                assert_eq!(op, "Eq");
                assert_eq!(args.len(), 2);
            }
            other => panic!("expected Compound, got {:?}", other),
        }
    }

    #[test]
    fn test_canonicalize_and() {
        match canonicalize_predicate_expr(&binary(ident("a"), Operator::And, ident("b"))) {
            crate::mutation_compiler::CanonicalExpr::Compound { op, args } => {
                assert_eq!(op, "AND");
                assert_eq!(args.len(), 2);
            }
            other => panic!("expected AND, got {:?}", other),
        }
    }

    #[test]
    fn test_canonicalize_or() {
        match canonicalize_predicate_expr(&binary(ident("a"), Operator::Or, ident("b"))) {
            crate::mutation_compiler::CanonicalExpr::Compound { op, args } => {
                assert_eq!(op, "OR");
                assert_eq!(args.len(), 2);
            }
            other => panic!("expected OR, got {:?}", other),
        }
    }

    #[test]
    fn test_canonicalize_unsupported_op() {
        // Plus is not a comparison operator -> returns Null
        match canonicalize_predicate_expr(&binary(
            ident("x"),
            Operator::Plus,
            lit(Value::Integer(1)),
        )) {
            crate::mutation_compiler::CanonicalExpr::Const(Value::Null) => {}
            other => panic!("expected Null, got {:?}", other),
        }
    }

    #[test]
    fn test_combine_hash_deterministic() {
        assert_eq!(combine_hash(10, 20), combine_hash(10, 20));
    }

    #[test]
    fn test_combine_hash_zero() {
        assert_eq!(combine_hash(0, 0), combine_hash(0, 0));
    }

    #[test]
    fn test_update_plan_accessors() {
        let plan = UpdatePlan {
            predicate: Box::new(|_| true),
            mutation: MutationCompiler::compile(vec![]),
            predicate_hash: 100,
            mutation_hash: 200,
            combined_hash: 300,
        };
        assert_eq!(plan.predicate_hash(), 100);
        assert_eq!(plan.mutation_hash(), 200);
        assert_eq!(plan.combined_hash(), 300);
    }
}
