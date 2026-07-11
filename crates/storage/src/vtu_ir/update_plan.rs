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

#[cfg(test)]
mod tests {
    use super::super::predicate_ir::ExprIR;
    use super::*;
    use sqlrustgo_types::Value;

    fn make_mutation() -> MutationIR {
        MutationIR::new(vec![])
    }

    #[test]
    fn test_plan_trace_new() {
        let trace = PlanTrace::new(100, 200, 5);
        assert_eq!(trace.predicate_hash, 100);
        assert_eq!(trace.mutation_hash, 200);
        assert_eq!(trace.rows_affected, 5);
        assert!(trace.plan_id.starts_with("update_"));
    }

    #[test]
    fn test_plan_trace_rows_affected_accessor() {
        let trace = PlanTrace::new(1, 2, 42);
        assert_eq!(trace.rows_affected(), 42);
    }

    #[test]
    fn test_plan_trace_combined_hash_deterministic() {
        let t1 = PlanTrace::new(100, 200, 5);
        let t2 = PlanTrace::new(100, 200, 5);
        assert_eq!(t1.combined_hash, t2.combined_hash);
    }

    #[test]
    fn test_plan_trace_combined_hash_varies() {
        let t1 = PlanTrace::new(100, 200, 5);
        let t2 = PlanTrace::new(100, 300, 5);
        assert_ne!(t1.combined_hash, t2.combined_hash);
    }

    #[test]
    fn test_update_plan_new() {
        let plan = UpdatePlan::new("users".to_string(), PredicateIR::All, make_mutation(), 10);
        assert_eq!(plan.table, "users");
    }

    #[test]
    fn test_update_plan_predicate_accessor() {
        let plan = UpdatePlan::new("t".to_string(), PredicateIR::All, make_mutation(), 0);
        assert!(matches!(plan.predicate(), PredicateIR::All));
    }

    #[test]
    fn test_update_plan_mutation_accessor() {
        let plan = UpdatePlan::new("t".to_string(), PredicateIR::All, make_mutation(), 0);
        assert_eq!(plan.mutation().assignments().len(), 0);
    }

    #[test]
    fn test_update_plan_trace_accessor() {
        let plan = UpdatePlan::new("t".to_string(), PredicateIR::All, make_mutation(), 7);
        assert_eq!(plan.trace().rows_affected, 7);
    }

    #[test]
    fn test_update_plan_with_expr_predicate() {
        let plan = UpdatePlan::new(
            "t".to_string(),
            PredicateIR::Expr(ExprIR::Literal(Value::Boolean(true))),
            make_mutation(),
            1,
        );
        assert!(plan.trace().predicate_hash != 0);
    }
}
