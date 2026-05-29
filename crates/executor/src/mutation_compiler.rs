use sqlrustgo_planner::Expr;

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
