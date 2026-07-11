use sqlrustgo_types::Value;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use super::predicate_ir::ExprIR;

#[derive(Debug, Clone)]
pub struct AssignmentIR {
    pub column: String,
    pub column_index: usize,
    pub expr: ExprIR,
}

#[derive(Debug, Clone)]
pub struct MutationIR {
    pub assignments: Vec<AssignmentIR>,
    pub mutation_hash: u64,
}

impl MutationIR {
    pub fn new(assignments: Vec<AssignmentIR>) -> Self {
        let mut hasher = DefaultHasher::new();
        for a in &assignments {
            a.column.hash(&mut hasher);
            a.column_index.hash(&mut hasher);
            a.expr.hash(&mut hasher);
        }
        let mutation_hash = hasher.finish();
        Self {
            assignments,
            mutation_hash,
        }
    }

    pub fn assignments(&self) -> &[AssignmentIR] {
        &self.assignments
    }

    pub fn mutation_hash(&self) -> u64 {
        self.mutation_hash
    }

    pub fn to_row_mutation(&self) -> crate::engine::RowMutation {
        let assigns = self
            .assignments
            .iter()
            .map(|a| (a.column_index, Value::Null))
            .collect();
        crate::engine::RowMutation::new(assigns, self.mutation_hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_literal_assignment() -> AssignmentIR {
        AssignmentIR {
            column: "a".to_string(),
            column_index: 0,
            expr: ExprIR::Literal(Value::Integer(42)),
        }
    }

    #[test]
    fn test_mutation_new() {
        let m = MutationIR::new(vec![make_literal_assignment()]);
        assert_eq!(m.assignments.len(), 1);
        assert_eq!(m.assignments[0].column, "a");
        assert_eq!(m.assignments[0].column_index, 0);
    }

    #[test]
    fn test_mutation_empty() {
        let m = MutationIR::new(vec![]);
        assert_eq!(m.assignments.len(), 0);
    }

    #[test]
    fn test_mutation_assignments_accessor() {
        let m = MutationIR::new(vec![make_literal_assignment()]);
        assert_eq!(m.assignments().len(), 1);
    }

    #[test]
    fn test_mutation_hash_accessor() {
        let m = MutationIR::new(vec![make_literal_assignment()]);
        assert!(m.mutation_hash() != 0);
    }

    #[test]
    fn test_mutation_to_row_mutation() {
        let m = MutationIR::new(vec![make_literal_assignment()]);
        let row_mutation = m.to_row_mutation();
        assert_eq!(row_mutation.assignments().len(), 1);
        assert_eq!(row_mutation.assignments()[0].0, 0);
    }

    #[test]
    fn test_mutation_hash_deterministic() {
        let m1 = MutationIR::new(vec![make_literal_assignment()]);
        let m2 = MutationIR::new(vec![make_literal_assignment()]);
        assert_eq!(m1.mutation_hash(), m2.mutation_hash());
    }

    #[test]
    fn test_mutation_hash_differs_on_col() {
        let a1 = AssignmentIR {
            column: "a".to_string(),
            column_index: 0,
            expr: ExprIR::Literal(Value::Integer(1)),
        };
        let a2 = AssignmentIR {
            column: "b".to_string(),
            column_index: 1,
            expr: ExprIR::Literal(Value::Integer(1)),
        };
        let m1 = MutationIR::new(vec![a1]);
        let m2 = MutationIR::new(vec![a2]);
        assert_ne!(m1.mutation_hash(), m2.mutation_hash());
    }
}
