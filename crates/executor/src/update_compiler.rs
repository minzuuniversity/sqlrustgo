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
        let predicate = stmt
            .where_clause
            .as_ref()
            .map(|e| PredicateCompiler::compile(e))
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
