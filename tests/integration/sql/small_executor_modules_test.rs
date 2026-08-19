//! Integration tests for the small "0% coverage" executor modules:
//!
//! - `crates/executor/src/execution/facade.rs`           (ExecutionFacade)
//! - `crates/executor/src/execution/result.rs`          (ExecutionResult struct)
//! - `crates/executor/src/execution/recovery.rs`        (RecoveryPlanner + ReplayEngine + SafeExecutionController)
//! - `crates/executor/src/predicate_compiler.rs`        (PredicateCompiler)
//! - `crates/executor/src/mutation_compiler.rs`         (MutationCompiler + CanonicalExpr)
//! - `crates/executor/src/trigger_eval/resolver.rs`      (resolve_column stub)
//! - `crates/executor/src/ast_adapter.rs`               (AstAdapter::to_update_plan)
//!
//! Closes task-3.1 of #3537 (Round 3 coverage plan).
//!
//! Many of these modules already have rich `#[cfg(test)] mod tests` blocks;
//! the value-add of this file is exercising the **public** APIs from outside
//! the executor crate so coverage measurement sees them hit through the
//! integration test path.

use sqlrustgo_executor::execution::drift_gate::{
    DriftSeverity, DriftViolation, DriftViolationType,
};
use sqlrustgo_executor::execution::events::{RecoveryConfidence, RecoveryPlan, RecoveryType};
use sqlrustgo_executor::execution::facade::ExecutionFacade;
use sqlrustgo_executor::execution::recovery::{
    ExecutionReplayEngine, RecoveryPlanner, SafeExecutionController,
};
// The recovery-side enum `ExecutionResult` lives at `execution::recovery::ExecutionResult`.
// `execution::result::ExecutionResult` is a different (struct) type — easy to confuse.
use sqlrustgo_executor::execution::recovery::ExecutionResult as RecoveryExecResult;
use sqlrustgo_executor::execution::result::ExecutionResult;
use sqlrustgo_executor::execution::{ExecutionEngine, QueryContext};
use sqlrustgo_executor::mutation_compiler::{
    canonicalize_expr, Assignment, CanonicalExpr, MutationCompiler, RowMutation,
};
use sqlrustgo_executor::predicate_compiler::{compile, compile_optional, PredicateCompiler};
use sqlrustgo_executor::trigger_eval::resolver::resolve_column;
use sqlrustgo_executor::trigger_eval::{EvalContext, TriggerContext};
use sqlrustgo_planner::{Column, Expr, Operator};
use sqlrustgo_storage::engine::Record;
use sqlrustgo_storage::Value;
use sqlrustgo_types::SqlError;

// ===========================================================================
// Mock ExecutionEngine for facade tests
// ===========================================================================

struct MockEngine {
    last_sql: Option<String>,
    last_params: Vec<Value>,
    next_result: ExecutionResult,
}

impl MockEngine {
    fn new(next: ExecutionResult) -> Self {
        Self {
            last_sql: None,
            last_params: Vec::new(),
            next_result: next,
        }
    }
}

impl ExecutionEngine for MockEngine {
    fn execute(&mut self, ctx: &mut QueryContext) -> Result<ExecutionResult, SqlError> {
        self.last_sql = Some(ctx.sql.clone());
        self.last_params = ctx.params.clone();
        Ok(self.next_result.clone())
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

fn mock_with(result: ExecutionResult) -> ExecutionFacade<MockEngine> {
    ExecutionFacade::new(MockEngine::new(result))
}

// ===========================================================================
// execution/facade.rs
// ===========================================================================

#[test]
fn test_facade_execute_sql_propagates_to_engine() {
    let mut facade = mock_with(ExecutionResult::ok(7));
    let r = facade.execute_sql("SELECT 1".to_string()).unwrap();
    assert_eq!(r.affected_rows, 7);
}

#[test]
fn test_facade_execute_with_params_runs_engine() {
    let mut facade = mock_with(ExecutionResult::ok(0));
    let r = facade
        .execute_with_params(
            "SELECT ?".to_string(),
            vec![Value::Integer(7), Value::Text("x".to_string())],
        )
        .unwrap();
    assert_eq!(r.affected_rows, 0);
}

// ===========================================================================
// execution/result.rs — the STRUCT ExecutionResult (constructors)
// ===========================================================================

#[test]
fn test_result_struct_ok() {
    let r = ExecutionResult::ok(5);
    assert_eq!(r.affected_rows, 5);
    assert!(r.last_insert_id.is_none());
    assert!(r.payload.is_none());
}

#[test]
fn test_result_struct_with_payload() {
    let r = ExecutionResult::with_payload(vec![Value::Integer(1), Value::Integer(2)]);
    assert_eq!(r.affected_rows, 0);
    let payload = r.payload.expect("payload");
    assert_eq!(payload.len(), 2);
}

#[test]
fn test_result_struct_with_insert_id() {
    let r = ExecutionResult::ok(0).with_insert_id(42);
    assert_eq!(r.last_insert_id, Some(42));
}

// ===========================================================================
// execution/recovery.rs (uses RecoveryExecResult enum)
// ===========================================================================

fn violation(t: DriftViolationType, s: DriftSeverity, id: &str) -> DriftViolation {
    DriftViolation {
        violation_id: id.to_string(),
        trace_id: "trace-test".to_string(),
        violation_type: t,
        severity: s,
        description: "test".to_string(),
        detected_at: 0,
    }
}

#[test]
fn test_recovery_planner_wal_drift_full_severity_matrix() {
    let p = RecoveryPlanner::new("t1".to_string());
    for sev in [
        DriftSeverity::Critical,
        DriftSeverity::Medium,
        DriftSeverity::Low,
    ] {
        let v = violation(DriftViolationType::WalDrift, sev, "v-wal");
        let plan = p.create_plan(&v).expect("plan");
        let expected = match sev {
            DriftSeverity::Critical => RecoveryType::Rollback,
            DriftSeverity::Medium => RecoveryType::Patch,
            DriftSeverity::Low => RecoveryType::Ignore,
        };
        assert_eq!(plan.recovery_type, expected);
        assert!(!plan.steps.is_empty());
    }
}

#[test]
fn test_recovery_planner_txn_drift_full_severity_matrix() {
    let p = RecoveryPlanner::new("t2".to_string());
    for sev in [
        DriftSeverity::Critical,
        DriftSeverity::Medium,
        DriftSeverity::Low,
    ] {
        let v = violation(DriftViolationType::TxnDrift, sev, "v-txn");
        let plan = p.create_plan(&v).expect("plan");
        let expected = match sev {
            DriftSeverity::Critical => RecoveryType::Rollback,
            DriftSeverity::Medium => RecoveryType::Patch,
            DriftSeverity::Low => RecoveryType::Ignore,
        };
        assert_eq!(plan.recovery_type, expected);
    }
}

#[test]
fn test_recovery_planner_graph_drift_full_severity_matrix() {
    let p = RecoveryPlanner::new("t3".to_string());
    for sev in [
        DriftSeverity::Critical,
        DriftSeverity::Medium,
        DriftSeverity::Low,
    ] {
        let v = violation(DriftViolationType::GraphDrift, sev, "v-graph");
        let plan = p.create_plan(&v).expect("plan");
        let expected = match sev {
            DriftSeverity::Critical | DriftSeverity::Medium => RecoveryType::Rewire,
            DriftSeverity::Low => RecoveryType::Ignore,
        };
        assert_eq!(plan.recovery_type, expected);
    }
}

#[test]
fn test_replay_engine_detects_mutation_outside_wal_segment() {
    let mut e = ExecutionReplayEngine::new("t-rep".to_string());
    e.add_event("StorageMutation".to_string());
    let r = e.replay();
    assert!(!r.valid);
    assert_eq!(r.divergences.len(), 1);
    assert!(r.divergences[0].contains("without open WAL"));
}

#[test]
fn test_replay_engine_accepts_wal_begin_then_commit() {
    let mut e = ExecutionReplayEngine::new("t-rep-ok".to_string());
    e.add_event("WalBegin".to_string());
    e.add_event("StorageMutation".to_string());
    e.add_event("WalCommit".to_string());
    let r = e.replay();
    assert!(r.valid, "divergences: {:?}", r.divergences);
}

#[test]
fn test_safe_controller_blocks_rollback_high_confidence() {
    let c = SafeExecutionController::new();
    let plan = RecoveryPlan::new(
        "t".to_string(),
        "v".to_string(),
        RecoveryType::Rollback,
        RecoveryConfidence::High,
        vec!["step".to_string()],
    );
    let r = c.execute_plan(&plan);
    match r {
        RecoveryExecResult::Blocked { reason, .. } => assert!(reason.contains("Critical")),
        _ => panic!("expected Blocked"),
    }
}

#[test]
fn test_safe_controller_auto_repairs_high_confidence_patch() {
    let c = SafeExecutionController::new();
    let plan = RecoveryPlan::new(
        "t".to_string(),
        "v".to_string(),
        RecoveryType::Patch,
        RecoveryConfidence::High,
        vec!["s1".to_string(), "s2".to_string()],
    );
    match c.execute_plan(&plan) {
        RecoveryExecResult::AutoRepaired { steps_executed, .. } => {
            assert_eq!(steps_executed, 2)
        }
        _ => panic!("expected AutoRepaired"),
    }
}

#[test]
fn test_safe_controller_suggests_when_not_blocked_or_auto_repair() {
    let c = SafeExecutionController::new();
    let plan = RecoveryPlan::new(
        "t".to_string(),
        "v".to_string(),
        RecoveryType::Ignore,
        RecoveryConfidence::Low,
        vec!["advisory".to_string()],
    );
    match c.execute_plan(&plan) {
        RecoveryExecResult::Suggested { steps, .. } => {
            assert_eq!(steps, vec!["advisory".to_string()])
        }
        _ => panic!("expected Suggested"),
    }
}

// ===========================================================================
// mutation_compiler.rs (CanonicalExpr, Assignment, RowMutation, MutationCompiler)
// ===========================================================================

fn lit(v: Value) -> Expr {
    Expr::Literal(v)
}

fn col(name: &str) -> Expr {
    Expr::Column(Column::new(name.to_string()))
}

fn binop(l: Expr, op: Operator, r: Expr) -> Expr {
    Expr::BinaryExpr {
        left: Box::new(l),
        op,
        right: Box::new(r),
    }
}

#[test]
fn test_canonicalize_expr_column_literal_add() {
    let e = binop(col("a"), Operator::Plus, lit(Value::Integer(1)));
    let c = canonicalize_expr(&e);
    match c {
        CanonicalExpr::Compound { op, args } => {
            assert_eq!(op, "+");
            assert_eq!(args.len(), 2);
        }
        _ => panic!("expected Compound +"),
    }
}

#[test]
fn test_canonicalize_expr_minus_is_not_commutative() {
    let e1 = binop(col("a"), Operator::Minus, lit(Value::Integer(1)));
    let e2 = binop(lit(Value::Integer(1)), Operator::Minus, col("a"));
    let c1 = canonicalize_expr(&e1);
    let c2 = canonicalize_expr(&e2);
    // Minus normalizes to Sub(left, right) preserving order; two swapped
    // operands must produce distinct canonical forms.
    assert!(matches!(c1, CanonicalExpr::Sub(_, _)));
    assert!(matches!(c2, CanonicalExpr::Sub(_, _)));
    assert_ne!(format!("{:?}", c1), format!("{:?}", c2));
}

#[test]
fn test_canonicalize_expr_unsupported_returns_null_const() {
    let e = Expr::Wildcard;
    assert!(matches!(
        canonicalize_expr(&e),
        CanonicalExpr::Const(Value::Null)
    ));
}

#[test]
fn test_mutation_compiler_produces_hash_and_assignments() {
    let assigns = vec![
        Assignment {
            column: "name".to_string(),
            expr: lit(Value::Text("alice".to_string())),
        },
        Assignment {
            column: "age".to_string(),
            expr: lit(Value::Integer(30)),
        },
    ];
    let m = MutationCompiler::compile(assigns);
    assert_eq!(m.assignments().len(), 2);
    assert_eq!(m.assignments()[0].column, "name");
    assert_ne!(m.mutation_hash(), 0);
}

#[test]
fn test_mutation_compiler_same_inputs_same_hash() {
    let mk = || {
        MutationCompiler::compile(vec![Assignment {
            column: "x".to_string(),
            expr: lit(Value::Integer(1)),
        }])
    };
    assert_eq!(mk().mutation_hash(), mk().mutation_hash());
}

#[test]
fn test_mutation_compiler_different_inputs_different_hash() {
    // compute_mutation_hash hashes the *canonical* expressions only — not the
    // column name. So changing the column name alone keeps the hash equal.
    // What *does* change the hash is a change in the expression itself.
    let a = MutationCompiler::compile(vec![Assignment {
        column: "x".to_string(),
        expr: lit(Value::Integer(1)),
    }]);
    let b = MutationCompiler::compile(vec![Assignment {
        column: "x".to_string(),
        expr: lit(Value::Integer(2)),
    }]);
    assert_ne!(a.mutation_hash(), b.mutation_hash());
}

#[test]
fn test_row_mutation_accessor_returns_passthrough() {
    let m = RowMutation::new(
        vec![Assignment {
            column: "x".to_string(),
            expr: lit(Value::Integer(1)),
        }],
        42,
    );
    assert_eq!(m.assignments().len(), 1);
    assert_eq!(m.mutation_hash(), 42);
}

// ===========================================================================
// predicate_compiler.rs
#[test]
fn test_predicate_compiler_column_true() {
    let f = compile(&col("active"));
    let row: Record = vec![Value::Boolean(true), Value::Integer(0)];
    assert!(f(&row));
}

#[test]
fn test_predicate_compiler_column_false() {
    let f = compile(&col("active"));
    let row: Record = vec![Value::Boolean(false), Value::Integer(0)];
    assert!(!f(&row));
}

#[test]
fn test_predicate_compiler_literal_matches_any_row() {
    let f = compile(&lit(Value::Integer(42)));
    let row: Record = vec![Value::Boolean(false), Value::Integer(0)];
    assert!(f(&row));
}

#[test]
fn test_predicate_compiler_binary_and() {
    let e = binop(col("active"), Operator::And, col("active"));
    let f = compile(&e);
    let row_a: Record = vec![Value::Boolean(true), Value::Integer(0)];
    let row_b: Record = vec![Value::Boolean(false), Value::Integer(0)];
    assert!(f(&row_a));
    assert!(!f(&row_b));
}

#[test]
fn test_predicate_compiler_unary_not() {
    let e = Expr::UnaryExpr {
        op: Operator::Not,
        expr: Box::new(col("active")),
    };
    let f = compile(&e);
    let row_a: Record = vec![Value::Boolean(true), Value::Integer(0)];
    let row_b: Record = vec![Value::Boolean(false), Value::Integer(0)];
    assert!(!f(&row_a));
    assert!(f(&row_b));
}

#[test]
fn test_predicate_compiler_wildcard_always_true() {
    let f = compile(&Expr::Wildcard);
    let row: Record = vec![Value::Boolean(false), Value::Integer(0)];
    assert!(f(&row));
}

#[test]
fn test_predicate_compiler_optional_none() {
    let opt: Option<&Expr> = None;
    assert!(compile_optional(opt).is_none());
}

#[test]
fn test_predicate_compiler_optional_some() {
    let opt: Option<&Expr> = Some(&lit(Value::Integer(1)));
    let f = compile_optional(opt).expect("filter");
    let row: Record = vec![Value::Boolean(false), Value::Integer(0)];
    assert!(f(&row));
}

// ===========================================================================
// trigger_eval/resolver.rs
// ===========================================================================

#[test]
fn test_resolve_column_returns_null() {
    // resolve_column is currently a stub that always returns Null. This test
    // pins the contract: future implementations must either keep this
    // behavior (default Null) or update this test.
    let trigger_ctx = TriggerContext::new(None, None);
    let ctx = EvalContext::new(&trigger_ctx, None);
    let v = resolve_column("anything", &ctx);
    assert!(matches!(v, Value::Null));
}

// ===========================================================================
// ast_adapter.rs (AstAdapter::to_update_plan)
//
// The AstAdapter accepts ParserUpdateStatement. We construct one directly
// without going through the parser to keep this test focused on the adapter.
// ===========================================================================

#[test]
fn test_ast_adapter_to_update_plan_uses_all_when_no_where() {
    use sqlrustgo_parser::parser::UpdateStatement as ParserUpdateStatement;
    use sqlrustgo_parser::Expression;

    let stmt = ParserUpdateStatement {
        tables: vec![sqlrustgo_parser::TableRef {
            name: "users".to_string(),
            schema: None,
            alias: None,
        }],
        set_clauses: vec![(
            "active".to_string(),
            Expression::Literal("TRUE".to_string()),
        )],
        where_clause: None,
    };
    let info = sqlrustgo_storage::TableInfo {
        name: "users".to_string(),
        columns: vec![sqlrustgo_storage::ColumnDefinition {
            name: "active".to_string(),
            data_type: "BOOLEAN".to_string(),
            nullable: false,
            primary_key: false,
            char_max_length: None,
            ..Default::default()
        }],
        foreign_keys: vec![],
        unique_constraints: vec![],
        check_constraints: vec![],
        compression: None,
        // V312-26 / #4077: collations field added by d48b0a1a71;
        // this initializer was missed by the propagation PR #4140.
        collations: std::collections::HashMap::new(),
        partition_info: None,
    };

    let plan =
        sqlrustgo_executor::ast_adapter::AstAdapter::to_update_plan(&stmt, &info).expect("plan");
    assert_eq!(plan.table, "users");
    assert!(matches!(
        plan.predicate(),
        sqlrustgo_storage::vtu_ir::PredicateIR::All
    ));
    assert_eq!(plan.mutation().assignments().len(), 1);
}

#[test]
fn test_ast_adapter_to_update_plan_errors_on_unknown_column() {
    use sqlrustgo_parser::parser::UpdateStatement as ParserUpdateStatement;
    use sqlrustgo_parser::Expression;

    let stmt = ParserUpdateStatement {
        tables: vec![sqlrustgo_parser::TableRef {
            name: "users".to_string(),
            schema: None,
            alias: None,
        }],
        set_clauses: vec![("missing".to_string(), Expression::Literal("x".to_string()))],
        where_clause: None,
    };
    let info = sqlrustgo_storage::TableInfo {
        name: "users".to_string(),
        columns: vec![sqlrustgo_storage::ColumnDefinition {
            name: "id".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: false,
            primary_key: false,
            char_max_length: None,
            ..Default::default()
        }],
        foreign_keys: vec![],
        unique_constraints: vec![],
        check_constraints: vec![],
        compression: None,
        // V312-26 / #4077: collations field added by d48b0a1a71;
        // this initializer was missed by the propagation PR #4140.
        collations: std::collections::HashMap::new(),
        partition_info: None,
    };
    let err = sqlrustgo_executor::ast_adapter::AstAdapter::to_update_plan(&stmt, &info)
        .expect_err("should fail");
    assert!(format!("{:?}", err).contains("column not found"));
}
