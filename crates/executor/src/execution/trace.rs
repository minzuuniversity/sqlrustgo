pub struct TxnStep {
    pub step_id: usize,
    pub sql: String,
    pub rows_affected: usize,
}

pub struct ExecutionTrace {
    steps: Vec<TxnStep>,
    plan_id: Option<String>,
    predicate_hash: Option<u64>,
    mutation_hash: Option<u64>,
    combined_hash: Option<u64>,
}

impl ExecutionTrace {
    pub fn new() -> Self {
        Self {
            steps: Vec::new(),
            plan_id: None,
            predicate_hash: None,
            mutation_hash: None,
            combined_hash: None,
        }
    }

    pub fn with_plan_id(mut self, plan_id: &str) -> Self {
        self.plan_id = Some(plan_id.to_string());
        self
    }

    pub fn with_predicate_hash(mut self, hash: u64) -> Self {
        self.predicate_hash = Some(hash);
        self
    }

    pub fn with_mutation_hash(mut self, hash: u64) -> Self {
        self.mutation_hash = Some(hash);
        self
    }

    pub fn with_combined_hash(mut self, hash: u64) -> Self {
        self.combined_hash = Some(hash);
        self
    }

    pub fn plan_id(&self) -> Option<&str> {
        self.plan_id.as_deref()
    }

    pub fn predicate_hash(&self) -> Option<u64> {
        self.predicate_hash
    }

    pub fn mutation_hash(&self) -> Option<u64> {
        self.mutation_hash
    }

    pub fn combined_hash(&self) -> Option<u64> {
        self.combined_hash
    }

    pub fn add_step(&mut self, step: TxnStep) {
        self.steps.push(step);
    }

    pub fn steps(&self) -> &[TxnStep] {
        &self.steps
    }
}

impl Default for ExecutionTrace {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_trace_mutation_hash() {
        let trace = ExecutionTrace::new()
            .with_plan_id("plan123")
            .with_predicate_hash(0xDEAD)
            .with_mutation_hash(0xBEEF)
            .with_combined_hash(0xFACEFEED);

        assert_eq!(trace.plan_id(), Some("plan123"));
        assert_eq!(trace.predicate_hash(), Some(0xDEAD));
        assert_eq!(trace.mutation_hash(), Some(0xBEEF));
        assert_eq!(trace.combined_hash(), Some(0xFACEFEED));
    }

    #[test]
    fn test_execution_trace_empty() {
        let trace = ExecutionTrace::new();
        assert_eq!(trace.plan_id(), None);
        assert_eq!(trace.predicate_hash(), None);
        assert_eq!(trace.mutation_hash(), None);
        assert_eq!(trace.combined_hash(), None);
        assert!(trace.steps().is_empty());
    }
}
