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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ok_sets_affected_rows_only() {
        let r = ExecutionResult::ok(7);
        assert_eq!(r.affected_rows, 7);
        assert_eq!(r.last_insert_id, None);
        assert!(r.payload.is_none());
    }

    #[test]
    fn with_payload_sets_payload_only() {
        let r = ExecutionResult::with_payload(vec![Value::Integer(1), Value::Integer(2)]);
        assert_eq!(r.affected_rows, 0);
        assert_eq!(r.last_insert_id, None);
        assert_eq!(r.payload.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn with_insert_id_chains_after_ok() {
        let r = ExecutionResult::ok(3).with_insert_id(42);
        assert_eq!(r.affected_rows, 3);
        assert_eq!(r.last_insert_id, Some(42));
        assert!(r.payload.is_none());
    }

    #[test]
    fn with_insert_id_chains_after_with_payload() {
        let r = ExecutionResult::with_payload(vec![Value::Null]).with_insert_id(99);
        assert_eq!(r.affected_rows, 0);
        assert_eq!(r.last_insert_id, Some(99));
        assert!(r.payload.is_some());
    }
}
