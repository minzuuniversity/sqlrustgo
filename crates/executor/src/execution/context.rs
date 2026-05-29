use sqlrustgo_types::Value;

#[derive(Debug, Clone)]
pub struct QueryContext {
    pub sql: String,
    pub params: Vec<Value>,
    pub txn_id: Option<u64>,
    pub trace_id: Option<String>,
}

impl QueryContext {
    pub fn new(sql: String) -> Self {
        Self {
            sql,
            params: vec![],
            txn_id: None,
            trace_id: None,
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

    pub fn requires_txn(&self) -> bool {
        is_dml(&self.sql)
    }
}

fn is_dml(sql: &str) -> bool {
    let sql_upper = sql.to_uppercase();
    sql_upper.starts_with("INSERT")
        || sql_upper.starts_with("UPDATE")
        || sql_upper.starts_with("DELETE")
}