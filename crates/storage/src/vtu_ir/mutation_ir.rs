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
