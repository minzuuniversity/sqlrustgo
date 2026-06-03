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
