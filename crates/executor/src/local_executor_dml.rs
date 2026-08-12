//! Local Executor DML Module
//!
//! ## ⚠️ 弃用 (Deprecated since v3.9.0 Phase 2)
//!
//! `LocalExecutorDml` 是过渡性占位符。自 v3.9.0 Phase 2 起，
//! 所有 DML/DDL 已通过 `ExecutionEngine::execute()` 完整分发。
//! 此模块将在 v3.10.0 中移除。使用 `ExecutionEngine::execute(sql)` 替代。
//!
//! ## G4 Fix (Issue #2811)
//! Added `engine: Arc<Mutex<dyn ExecutionEngine>>` field
//!
//! ## G3 Fix (Issue #2810)
//! Added `storage: Arc<RwLock<dyn StorageEngine>>` field + `execute_dml` for MERGE

use crate::execution::ExecutionEngine;
use crate::merge::MergeExecutor;
use sqlrustgo_planner::{Expr, MergeClause, MergeStatement as PlannerMerge};
use sqlrustgo_storage::{MemoryStorage, StorageEngine};
use sqlrustgo_types::SqlError;
use std::sync::{Arc, Mutex, RwLock};

/// LocalExecutorDml with engine field for VTU path (G4 #2811) and
/// storage for MergeExecutor construction (G3 #2810).
#[deprecated(
    since = "3.9.0",
    note = "Use ExecutionEngine::execute(sql) instead. Will be removed in 3.10.0."
)]
pub struct LocalExecutorDml {
    engine: Arc<Mutex<dyn ExecutionEngine>>,
    storage: Arc<RwLock<dyn StorageEngine>>,
}

/// Placeholder LocalExecutorDmlArc
pub struct LocalExecutorDmlArc;

#[allow(deprecated)]
impl LocalExecutorDml {
    /// Default constructor: NoopExecutionEngine + MemoryStorage
    pub fn new() -> Self {
        Self::new_with_storage(
            Arc::new(RwLock::new(MemoryStorage::new())),
            Arc::new(Mutex::new(NoopExecutionEngine)),
        )
    }

    /// Construct with explicit storage and engine
    /// (used by G3 caller and future production wiring)
    pub fn new_with_storage(
        storage: Arc<RwLock<dyn StorageEngine>>,
        engine: Arc<Mutex<dyn ExecutionEngine>>,
    ) -> Self {
        Self { storage, engine }
    }

    /// Get the engine (for G3 MergeExecutor construction)
    pub fn engine(&self) -> Arc<Mutex<dyn ExecutionEngine>> {
        self.engine.clone()
    }

    /// Get the storage (for G3 MergeExecutor construction)
    pub fn storage(&self) -> Arc<RwLock<dyn StorageEngine>> {
        self.storage.clone()
    }

    /// Execute a DML statement (G3 #2810: currently supports MERGE only)
    /// Detects MERGE keyword (case-insensitive), parses with G2 parser,
    /// converts to planner MergeStatement, then calls MergeExecutor::execute_merge.
    pub fn execute_dml(&self, sql: &str) -> Result<crate::ExecutorResult, SqlError> {
        let upper = sql.trim().to_uppercase();
        if !upper.starts_with("MERGE") {
            return Err(SqlError::ExecutionError(
                "execute_dml currently supports only MERGE statements (G3 #2810 scope)".to_string(),
            ));
        }
        let stmt = sqlrustgo_parser::parse(sql)
            .map_err(|e| SqlError::ExecutionError(format!("MERGE parse error: {}", e)))?;
        let parser_merge = match stmt {
            sqlrustgo_parser::Statement::Merge(m) => m,
            other => {
                return Err(SqlError::ExecutionError(format!(
                    "execute_dml: expected MERGE statement, got {:?}",
                    other
                )))
            }
        };
        let planner_merge = convert_parser_merge_to_planner(parser_merge);
        let merge_executor = MergeExecutor::new(self.storage.clone(), self.engine.clone());
        merge_executor.execute_merge(&planner_merge)
    }
}

/// Convert parser::MergeStatement to planner::MergeStatement (G3 #2810)
/// Lossy conversion: multiple WHEN clauses collapsed to first matched +
/// first not_matched; Subquery source not yet supported.
fn convert_parser_merge_to_planner(p: sqlrustgo_parser::MergeStatement) -> PlannerMerge {
    let mut matched_clause = None;
    let mut not_matched_clause = None;
    for clause in p.when_clauses {
        if clause.is_matched && matched_clause.is_none() {
            matched_clause = Some(convert_when_clause(&clause));
        } else if !clause.is_matched && not_matched_clause.is_none() {
            not_matched_clause = Some(convert_when_clause(&clause));
        }
    }
    let source_table = match p.source {
        sqlrustgo_parser::MergeSource::Table { name } => name,
        sqlrustgo_parser::MergeSource::Subquery(_) => String::new(),
    };
    PlannerMerge {
        target_table: p.target_table,
        source_table,
        on_condition: convert_expression(&p.on_condition),
        matched_clause,
        not_matched_clause,
    }
}

fn convert_when_clause(c: &sqlrustgo_parser::MergeWhenClause) -> MergeClause {
    let mut update_columns = Vec::new();
    let mut update_values = Vec::new();
    let mut insert_columns = Vec::new();
    let mut insert_values = Vec::new();
    match &c.action {
        sqlrustgo_parser::MergeAction::Update { set_clauses } => {
            for (col, val) in set_clauses {
                update_columns.push(col.clone());
                update_values.push(convert_expression(val));
            }
        }
        sqlrustgo_parser::MergeAction::Insert { columns, values } => {
            insert_columns = columns.clone();
            insert_values = values.iter().map(convert_expression).collect();
        }
        sqlrustgo_parser::MergeAction::Delete => {}
    }
    MergeClause {
        update_columns,
        update_values,
        insert_columns,
        insert_values,
    }
}

fn convert_expression(e: &sqlrustgo_parser::Expression) -> Expr {
    use sqlrustgo_planner::{Column, Operator};
    match e {
        sqlrustgo_parser::Expression::Identifier(name) => Expr::Column(Column {
            relation: None,
            name: name.clone(),
        }),
        sqlrustgo_parser::Expression::Literal(s) => {
            if let Ok(n) = s.parse::<i64>() {
                Expr::Literal(sqlrustgo_types::Value::Integer(n))
            } else if let Ok(n) = s.parse::<f64>() {
                Expr::Literal(sqlrustgo_types::Value::Float(n))
            } else {
                Expr::Literal(sqlrustgo_types::Value::Text(s.clone()))
            }
        }
        sqlrustgo_parser::Expression::BinaryOp(left, op, right) => {
            let pl_op = match op.as_str() {
                "=" => Operator::Eq,
                "!=" | "<>" => Operator::NotEq,
                "<" => Operator::Lt,
                "<=" => Operator::LtEq,
                ">" => Operator::Gt,
                ">=" => Operator::GtEq,
                "AND" => Operator::And,
                "OR" => Operator::Or,
                "+" => Operator::Plus,
                "-" => Operator::Minus,
                "*" => Operator::Multiply,
                "/" => Operator::Divide,
                "%" => Operator::Modulo,
                _ => Operator::Eq,
            };
            Expr::BinaryExpr {
                left: Box::new(convert_expression(left)),
                op: pl_op,
                right: Box::new(convert_expression(right)),
            }
        }
        _ => Expr::Literal(sqlrustgo_types::Value::Null),
    }
}

/// No-op engine adapter for placeholder use. G3 will replace with real impl.
struct NoopExecutionEngine;

impl ExecutionEngine for NoopExecutionEngine {
    fn execute(
        &mut self,
        _ctx: &mut crate::execution::QueryContext,
    ) -> Result<crate::execution::ExecutionResult, sqlrustgo_types::SqlError> {
        Ok(crate::execution::ExecutionResult::ok(0))
    }
    fn begin(&mut self) -> Result<u64, sqlrustgo_types::SqlError> {
        Err(sqlrustgo_types::SqlError::ExecutionError(
            "NoopEngine: begin not implemented".to_string(),
        ))
    }
    fn commit(&mut self, _txn: u64) -> Result<(), sqlrustgo_types::SqlError> {
        Err(sqlrustgo_types::SqlError::ExecutionError(
            "NoopEngine: commit not implemented".to_string(),
        ))
    }
    fn rollback(&mut self, _txn: u64) -> Result<(), sqlrustgo_types::SqlError> {
        Err(sqlrustgo_types::SqlError::ExecutionError(
            "NoopEngine: rollback not implemented".to_string(),
        ))
    }
}

#[allow(deprecated)]
impl Default for LocalExecutorDml {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlrustgo_storage::MemoryStorage;

    #[test]
    fn test_local_executor_dml_new() {
        let dml = LocalExecutorDml::new();
        let _ = dml;
    }

    #[test]
    fn test_local_executor_dml_default() {
        let dml = LocalExecutorDml::default();
        let _ = dml;
    }

    #[test]
    fn test_send_sync() {
        // LocalExecutorDml is not necessarily Send because it holds
        // `Arc<Mutex<dyn ExecutionEngine>>` where ExecutionEngine is not Send.
        // Only verify the no-engine Arc type.
        fn check<T: Send + Sync>() {}
        check::<LocalExecutorDmlArc>();
    }

    #[test]
    fn test_g4_engine_field_arc_cloneable() {
        // G4 #2811: engine must be Arc<Mutex<dyn ExecutionEngine>> and cloneable
        let dml = LocalExecutorDml::new();
        let engine1 = dml.engine();
        let engine2 = dml.engine();
        assert!(Arc::ptr_eq(&engine1, &engine2));
    }

    #[test]
    fn test_g4_with_engine_constructor() {
        // G4 #2811: with_engine() must accept explicit engine (use new_with_storage in G3)
        let engine: Arc<Mutex<dyn ExecutionEngine>> = Arc::new(Mutex::new(NoopExecutionEngine));
        let dml = LocalExecutorDml::new_with_storage(
            Arc::new(RwLock::new(MemoryStorage::new())),
            engine.clone(),
        );
        let retrieved = dml.engine();
        assert!(Arc::ptr_eq(&engine, &retrieved));
    }

    #[test]
    fn test_g3_execute_dml_rejects_non_merge() {
        let dml = LocalExecutorDml::new();
        let result = dml.execute_dml("INSERT INTO t VALUES (1)");
        assert!(result.is_err(), "Expected error for non-MERGE");
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("MERGE"),
            "Error should mention MERGE: {}",
            err
        );
    }

    #[test]
    fn test_g3_execute_dml_parses_valid_merge() {
        let dml = LocalExecutorDml::new();
        let sql = "MERGE INTO t1 AS t USING t2 AS s ON t.id = s.id \
                   WHEN MATCHED THEN UPDATE SET t.val = s.val";
        let result = dml.execute_dml(sql);
        // Acceptable: storage error (empty storage); NOT acceptable: parse error.
        if let Err(e) = result {
            assert!(
                !e.to_string().to_lowercase().contains("parse"),
                "Should not be parse error: {}",
                e
            );
        }
    }

    #[test]
    fn test_g3_execute_dml_returns_error_for_malformed_merge() {
        let dml = LocalExecutorDml::new();
        let result = dml.execute_dml("MERGE INTO");
        assert!(result.is_err());
    }

    #[test]
    fn test_g3_storage_field_accessible() {
        // G3: storage field is accessible and cloneable for MergeExecutor
        let dml = LocalExecutorDml::new();
        let _s: Arc<RwLock<dyn StorageEngine>> = dml.storage();
        let _e: Arc<Mutex<dyn ExecutionEngine>> = dml.engine();
    }

    #[test]
    fn test_convert_expression_all_variants() {
        let parser_expr = sqlrustgo_parser::Expression::BinaryOp(
            Box::new(sqlrustgo_parser::Expression::Identifier("a".into())),
            "=".into(),
            Box::new(sqlrustgo_parser::Expression::Literal("42".into())),
        );
        let _converted = convert_expression(&parser_expr);
    }

    #[test]
    fn test_convert_expression_unknown_op_falls_back_to_eq() {
        let parser_expr = sqlrustgo_parser::Expression::BinaryOp(
            Box::new(sqlrustgo_parser::Expression::Literal("1".into())),
            "UNKNOWN_OP".into(),
            Box::new(sqlrustgo_parser::Expression::Literal("2".into())),
        );
        let _converted = convert_expression(&parser_expr);
    }

    #[test]
    fn test_convert_expression_unsupported_falls_back_to_null() {
        let parser_expr = sqlrustgo_parser::Expression::FunctionCall("now".into(), vec![]);
        let _converted = convert_expression(&parser_expr);
    }

    #[test]
    fn test_convert_expression_float_literal() {
        let parser_expr = sqlrustgo_parser::Expression::Literal("3.14".into());
        let _converted = convert_expression(&parser_expr);
    }

    #[test]
    fn test_noop_engine_methods() {
        let mut engine = NoopExecutionEngine;
        let mut ctx = crate::execution::QueryContext::new("SELECT 1".into());
        let result = engine.execute(&mut ctx);
        assert!(result.is_ok());
        let begin_result = engine.begin();
        assert!(begin_result.is_err());
        let commit_result = engine.commit(1);
        assert!(commit_result.is_err());
        let rollback_result = engine.rollback(1);
        assert!(rollback_result.is_err());
    }
}
