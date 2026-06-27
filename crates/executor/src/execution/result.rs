use sqlrustgo_types::Value;

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub affected_rows: usize,
    pub last_insert_id: Option<u64>,
    pub payload: Option<Vec<Value>>,
}

impl ExecutionResult {
    pub fn ok(affected_rows: usize) -> Self {
        Self {
            affected_rows,
            last_insert_id: None,
            payload: None,
        }
    }

    pub fn with_payload(payload: Vec<Value>) -> Self {
        Self {
            affected_rows: 0,
            last_insert_id: None,
            payload: Some(payload),
        }
    }

    pub fn with_insert_id(mut self, id: u64) -> Self {
        self.last_insert_id = Some(id);
        self
    }
}
