use super::{MutationIR, PredicateIR};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
pub struct PlanTrace {
    pub plan_id: String,
    pub predicate_hash: u64,
    pub mutation_hash: u64,
    pub combined_hash: u64,
    pub rows_affected: usize,
}

impl PlanTrace {
    pub fn new(predicate_hash: u64, mutation_hash: u64, rows_affected: usize) -> Self {
        let mut hasher = DefaultHasher::new();
        predicate_hash.hash(&mut hasher);
        mutation_hash.hash(&mut hasher);
        let combined_hash = hasher.finish();
        Self {
            plan_id: format!("update_{:x}", combined_hash),
            predicate_hash,
            mutation_hash,
            combined_hash,
            rows_affected,
        }
    }

    pub fn rows_affected(&self) -> usize {
        self.rows_affected
    }
}

fn compute_predicate_hash(predicate: &PredicateIR) -> u64 {
    let mut hasher = DefaultHasher::new();
    match predicate {
        PredicateIR::All => {
            "All".hash(&mut hasher);
        }
        PredicateIR::Expr(expr) => {
            expr.hash(&mut hasher);
        }
    }
    hasher.finish()
}

#[derive(Debug, Clone)]
pub struct UpdatePlan {
    pub table: String,
    pub predicate: PredicateIR,
    pub mutation: MutationIR,
    pub trace: PlanTrace,
}

impl UpdatePlan {
    pub fn new(
        table: String,
        predicate: PredicateIR,
        mutation: MutationIR,
        rows_affected: usize,
    ) -> Self {
        let predicate_hash = compute_predicate_hash(&predicate);
        let mutation_hash = mutation.mutation_hash();
        let trace = PlanTrace::new(predicate_hash, mutation_hash, rows_affected);
        Self {
            table,
            predicate,
            mutation,
            trace,
        }
    }

    pub fn predicate(&self) -> &PredicateIR {
        &self.predicate
    }

    pub fn mutation(&self) -> &MutationIR {
        &self.mutation
    }

    pub fn trace(&self) -> &PlanTrace {
        &self.trace
    }
}
