use super::context::QueryContext;
use super::engine::ExecutionEngine;
use super::result::ExecutionResult;
use sqlrustgo_types::{SqlError, Value};

pub struct ExecutionFacade<E: ExecutionEngine> {
    engine: E,
}

impl<E: ExecutionEngine> ExecutionFacade<E> {
    pub fn new(engine: E) -> Self {
        Self { engine }
    }

    pub fn execute_sql(&mut self, sql: String) -> Result<ExecutionResult, SqlError> {
        let mut ctx = QueryContext::new(sql);
        self.engine.execute(&mut ctx)
    }

    pub fn execute_with_params(
        &mut self,
        sql: String,
        params: Vec<Value>,
    ) -> Result<ExecutionResult, SqlError> {
        let mut ctx = QueryContext::new(sql).with_params(params);
        self.engine.execute(&mut ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlrustgo_types::Value;

    struct MockEngine {
        last_sql: Option<String>,
        last_params: Option<Vec<Value>>,
        result: Result<ExecutionResult, SqlError>,
    }

    impl ExecutionEngine for MockEngine {
        fn execute(&mut self, ctx: &mut QueryContext) -> Result<ExecutionResult, SqlError> {
            self.last_sql = Some(ctx.sql.clone());
            self.last_params = Some(ctx.params.clone());
            // Build a fresh result: if params were passed, surface them as payload.
            // Move the engine's stored result out so we can return it directly.
            let stored = std::mem::replace(&mut self.result, Ok(ExecutionResult::ok(0)));
            match stored {
                Ok(base) => {
                    let mut res = ExecutionResult {
                        affected_rows: base.affected_rows,
                        last_insert_id: base.last_insert_id,
                        payload: None,
                    };
                    if !ctx.params.is_empty() {
                        res.payload = Some(ctx.params.clone());
                    }
                    Ok(res)
                }
                Err(e) => Err(e),
            }
        }
        fn begin(&mut self) -> Result<u64, SqlError> {
            Ok(1)
        }
        fn commit(&mut self, _txn: u64) -> Result<(), SqlError> {
            Ok(())
        }
        fn rollback(&mut self, _txn: u64) -> Result<(), SqlError> {
            Ok(())
        }
    }

    #[test]
    fn facade_execute_sql_passes_query_through() {
        let engine = MockEngine {
            last_sql: None,
            last_params: None,
            result: Ok(ExecutionResult::ok(3)),
        };
        let mut facade = ExecutionFacade::new(engine);
        let res = facade.execute_sql("SELECT 1".to_string()).unwrap();
        assert_eq!(res.affected_rows, 3);
        assert_eq!(facade.engine.last_sql.as_deref(), Some("SELECT 1"));
    }

    #[test]
    fn facade_execute_with_params_passes_params_through() {
        let engine = MockEngine {
            last_sql: None,
            last_params: None,
            result: Ok(ExecutionResult::ok(0)),
        };
        let mut facade = ExecutionFacade::new(engine);
        let params = vec![Value::Integer(42), Value::Text("x".to_string())];
        facade
            .execute_with_params("INSERT INTO t VALUES (?)".to_string(), params.clone())
            .unwrap();
        assert_eq!(facade.engine.last_params.as_ref(), Some(&params));
    }

    #[test]
    fn facade_execute_propagates_engine_error() {
        let engine = MockEngine {
            last_sql: None,
            last_params: None,
            result: Err(SqlError::ParseError("syntax error".to_string())),
        };
        let mut facade = ExecutionFacade::new(engine);
        let err = facade.execute_sql("BAD".to_string()).unwrap_err();
        assert!(matches!(err, SqlError::ParseError(_)));
    }
}
