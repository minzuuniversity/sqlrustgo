use sqlrustgo_types::Value;

use super::{DmlOperation, TelemetryCollector};

#[derive(Debug, Clone)]
pub struct QueryContext {
    pub sql: String,
    pub params: Vec<Value>,
    pub txn_id: Option<u64>,
    pub trace_id: Option<String>,
    pub op_type: Option<DmlOperation>,
    pub telemetry: Option<TelemetryCollector>,
}

impl QueryContext {
    pub fn new(sql: String) -> Self {
        let op_type = detect_dml_operation(&sql);
        Self {
            sql,
            params: vec![],
            txn_id: None,
            trace_id: None,
            op_type: Some(op_type),
            telemetry: None,
        }
    }

    pub fn with_params(mut self, params: Vec<Value>) -> Self {
        self.params = params;
        self
    }

    pub fn with_txn(mut self, txn_id: u64) -> Self {
        self.txn_id = Some(txn_id);
        self
    }

    pub fn with_telemetry(mut self, telemetry: TelemetryCollector) -> Self {
        self.telemetry = Some(telemetry);
        self
    }

    pub fn requires_txn(&self) -> bool {
        self.op_type.is_some()
    }
}

fn detect_dml_operation(sql: &str) -> DmlOperation {
    let sql_upper = sql.trim().to_uppercase();
    if sql_upper.starts_with("INSERT") {
        DmlOperation::Insert
    } else if sql_upper.starts_with("UPDATE") {
        DmlOperation::Update
    } else {
        DmlOperation::Delete
    }
}
