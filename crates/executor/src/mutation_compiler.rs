//! Canonical expression forms for row mutations.
//!
//! # Commutative vs Non-Commutative Operators
//!
//! - **Add/Mul** are commutative (`a + b == b + a`, `a * b == b * a`) → use sorted `Vec`
//!   to normalize `b + a` into the same form as `a + b`
//! - **Sub/Div** are NOT commutative (`a - b != b - a`, `a / b != b / a`) → use `Box`
//!   to preserve the original left-to-right order

use sqlrustgo_planner::{Expr, Operator};
use sqlrustgo_types::Value;

/// Canonical expression representation for row-level operations.
///
/// Represents SQL expressions in a normalized form suitable for
/// compilation into row mutations. Handles column references,
/// constants, and binary operations.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CanonicalExpr {
    /// Reference to a column by name
    Column(String),
    /// Constant value
    Const(Value),
    /// Addition - commutative, uses sorted Vec to normalize operand order
    Add(Vec<CanonicalExpr>),
    /// Multiplication - commutative, uses sorted Vec to normalize operand order
    Mul(Vec<CanonicalExpr>),
    /// Subtraction - NOT commutative, Box preserves left-to-right order
    Sub(Box<CanonicalExpr>, Box<CanonicalExpr>),
    /// Division - NOT commutative, Box preserves left-to-right order
    Div(Box<CanonicalExpr>, Box<CanonicalExpr>),
    /// Compound operator with sorted operands
    Compound {
        /// Operator symbol
        op: String,
        /// Sorted operands for commutative operators
        args: Vec<CanonicalExpr>,
    },
}

/// Transforms a planner Expr into a canonical form for row mutations.
///
/// Handles column references, literals, and binary expressions,
/// normalizing commutative operators for consistent hashing.
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
            #[allow(unused_variables)]
            other => {
                // Unsupported operators (And, Or, Eq, etc.) fall through to Null.
                // These are filtered out earlier in query planning.
                CanonicalExpr::Const(Value::Null)
            }
        },
        #[allow(unused_variables)]
        _ => {
            // Non-expression nodes (wildcards, subqueries, etc.) are not
            // valid in SET clauses and produce Null.
            CanonicalExpr::Const(Value::Null)
        }
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

use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

pub struct MutationCompiler;

impl MutationCompiler {
    pub fn compile(assignments: Vec<Assignment>) -> RowMutation {
        let canonical: Vec<_> = assignments.iter()
            .map(|a| canonicalize_expr(&a.expr))
            .collect();

        let mutation_hash = compute_mutation_hash(&canonical);

        RowMutation::new(assignments, mutation_hash)
    }
}

fn compute_mutation_hash(canonical: &[CanonicalExpr]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for expr in canonical {
        expr.hash(&mut hasher);
    }
    hasher.finish()
}
