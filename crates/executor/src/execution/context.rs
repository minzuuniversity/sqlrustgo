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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_context_new() {
        let ctx = QueryContext::new("SELECT 1".into());
        assert_eq!(ctx.sql, "SELECT 1");
        assert!(ctx.params.is_empty());
        assert!(ctx.txn_id.is_none());
        assert!(ctx.trace_id.is_none());
    }

    #[test]
    fn test_query_context_with_params() {
        let ctx = QueryContext::new("SELECT ?".into()).with_params(vec![Value::Integer(42)]);
        assert_eq!(ctx.params.len(), 1);
        assert_eq!(ctx.params[0], Value::Integer(42));
    }

    #[test]
    fn test_query_context_with_txn() {
        let ctx = QueryContext::new("INSERT INTO t VALUES (1)".into()).with_txn(100);
        assert_eq!(ctx.txn_id, Some(100));
    }

    #[test]
    fn test_requires_txn_insert() {
        let ctx = QueryContext::new("INSERT INTO t VALUES (1)".into());
        assert!(ctx.requires_txn());
    }

    #[test]
    fn test_requires_txn_update() {
        let ctx = QueryContext::new("UPDATE t SET x = 1".into());
        assert!(ctx.requires_txn());
    }

    #[test]
    fn test_requires_txn_delete() {
        let ctx = QueryContext::new("DELETE FROM t WHERE x = 1".into());
        assert!(ctx.requires_txn());
    }

    #[test]
    fn test_requires_txn_select() {
        let ctx = QueryContext::new("SELECT * FROM t".into());
        assert!(!ctx.requires_txn());
    }

    #[test]
    fn test_is_dml_various() {
        assert!(is_dml("INSERT INTO t VALUES (1)"));
        assert!(is_dml("UPDATE t SET x=1"));
        assert!(is_dml("DELETE FROM t"));
        assert!(!is_dml("SELECT * FROM t"));
        assert!(!is_dml("CREATE TABLE t (id INT)"));
        assert!(!is_dml("DROP TABLE t"));
    }
}
