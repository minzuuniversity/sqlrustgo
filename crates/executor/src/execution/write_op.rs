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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_op_insert_table_name() {
        let op = WriteOp::Insert {
            table: "users".into(),
            columns: vec!["id".into(), "name".into()],
            values: vec![vec![Value::Integer(1), Value::Text("alice".into())]],
        };
        assert_eq!(op.table_name(), "users");
        assert_eq!(op.operation_type(), "INSERT");
    }

    #[test]
    fn test_write_op_update_table_name() {
        let op = WriteOp::Update {
            table: "users".into(),
            set: vec![("name".into(), Value::Text("bob".into()))],
            filter: "id = 1".into(),
        };
        assert_eq!(op.table_name(), "users");
        assert_eq!(op.operation_type(), "UPDATE");
    }

    #[test]
    fn test_write_op_delete_table_name() {
        let op = WriteOp::Delete {
            table: "users".into(),
            filter: "id = 1".into(),
        };
        assert_eq!(op.table_name(), "users");
        assert_eq!(op.operation_type(), "DELETE");
    }

    #[test]
    fn test_write_op_debug() {
        let op = WriteOp::Insert {
            table: "t".into(),
            columns: vec!["c".into()],
            values: vec![],
        };
        let debug = format!("{:?}", op);
        assert!(debug.contains("Insert"));
    }
}
