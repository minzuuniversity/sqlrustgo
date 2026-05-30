//! Write operations tracked by the TransactionalFacade

use sqlrustgo_types::Value;

/// Write operation types — enforced by DriftGate
#[derive(Debug, Clone)]
pub enum WriteOp {
    Insert {
        table: String,
        columns: Vec<String>,
        values: Vec<Vec<Value>>,
    },
    Update {
        table: String,
        set: Vec<(String, Value)>,
        filter: String,
    },
    Delete {
        table: String,
        filter: String,
    },
}

impl WriteOp {
    pub fn table_name(&self) -> &str {
        match self {
            WriteOp::Insert { table, .. } => table,
            WriteOp::Update { table, .. } => table,
            WriteOp::Delete { table, .. } => table,
        }
    }

    pub fn operation_type(&self) -> &'static str {
        match self {
            WriteOp::Insert { .. } => "INSERT",
            WriteOp::Update { .. } => "UPDATE",
            WriteOp::Delete { .. } => "DELETE",
        }
    }
}
