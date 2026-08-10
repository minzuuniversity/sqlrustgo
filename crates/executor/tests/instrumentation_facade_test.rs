//! Tests for instrumentation hooks and execution facade

use sqlrustgo_executor::execution::facade::ExecutionFacade;
use sqlrustgo_executor::execution::{ExecutionEngine, ExecutionResult};
use sqlrustgo_executor::instrumentation::{
    CountingInstrumentationHook, InstrumentationHook, NoopInstrumentationHook,
};
use sqlrustgo_types::{SqlError, SqlResult, Value};
use std::sync::atomic::{AtomicBool, Ordering};

/// Mock execution engine for testing ExecutionFacade
struct MockExecutionEngine {
    called: AtomicBool,
}

impl MockExecutionEngine {
    fn new() -> Self {
        Self {
            called: AtomicBool::new(false),
        }
    }
}

impl ExecutionEngine for MockExecutionEngine {
    fn execute(
        &mut self,
        _ctx: &mut sqlrustgo_executor::execution::QueryContext,
    ) -> SqlResult<ExecutionResult> {
        self.called.store(true, Ordering::SeqCst);
        Ok(ExecutionResult::with_payload(vec![
            Value::Integer(1),
            Value::Text("test".to_string()),
        ]))
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
fn test_execution_facade_new() {
    let engine = MockExecutionEngine::new();
    let _facade = ExecutionFacade::new(engine);
}

#[test]
fn test_execution_facade_execute_sql() {
    let engine = MockExecutionEngine::new();
    let mut facade = ExecutionFacade::new(engine);
    let result = facade.execute_sql("SELECT * FROM t".to_string());
    assert!(result.is_ok());
}

#[test]
fn test_execution_facade_execute_with_params() {
    let engine = MockExecutionEngine::new();
    let mut facade = ExecutionFacade::new(engine);
    let params = vec![Value::Integer(42), Value::Text("hello".to_string())];
    let result = facade.execute_with_params("SELECT * FROM t WHERE id = ?".to_string(), params);
    assert!(result.is_ok());
}

#[test]
fn test_noop_instrumentation_hook_all_methods() {
    let hook = NoopInstrumentationHook;
    hook.on_seq_scan_start("t");
    hook.on_filter_start("t", 100);
    hook.on_filter_end("t", 50);
    hook.on_project_start("t", 100);
    hook.on_project_end("t", 50);
    hook.on_hash_join_build("left");
    hook.on_hash_join_probe(100);
    hook.on_aggregate_start(5);
    hook.on_sort_start(100);
    hook.on_query_complete(1000);
    // All methods should be no-op and not panic
}

#[test]
fn test_counting_instrumentation_hook_all_counters() {
    let hook = CountingInstrumentationHook::new();

    // Trigger all events multiple times
    for _ in 0..3 {
        hook.on_seq_scan_start("t1");
        hook.on_seq_scan_start("t2");
    }

    hook.on_filter_start("t", 100);
    hook.on_filter_end("t", 80);
    hook.on_project_start("t", 100);
    hook.on_project_end("t", 90);
    hook.on_hash_join_build("left");
    hook.on_hash_join_build("right");
    hook.on_hash_join_probe(50);
    hook.on_aggregate_start(5);
    hook.on_sort_start(200);
    hook.on_query_complete(1500);

    assert_eq!(hook.seq_scan_count(), 6);
    assert_eq!(hook.filter_count(), 1);
    assert_eq!(hook.filter_rows_in_total(), 100);
    assert_eq!(hook.filter_rows_out_total(), 80);
    assert_eq!(hook.project_count(), 1);
    assert_eq!(hook.project_rows_in_total(), 100);
    assert_eq!(hook.project_rows_out_total(), 90);
    assert_eq!(hook.hash_join_build_count(), 2);
    assert_eq!(hook.hash_join_probe_count(), 1);
    assert_eq!(hook.hash_join_probe_rows_total(), 50);
    assert_eq!(hook.aggregate_count(), 1);
    assert_eq!(hook.sort_count(), 1);
    assert_eq!(hook.query_complete_count(), 1);
}

#[test]
fn test_counting_instrumentation_default() {
    let hook = CountingInstrumentationHook::default();
    assert_eq!(hook.seq_scan_count(), 0);
    assert_eq!(hook.filter_count(), 0);
}

#[test]
fn test_counting_instrumentation_concurrent() {
    use std::thread;
    let hook = std::sync::Arc::new(CountingInstrumentationHook::new());
    let mut handles = vec![];

    for _ in 0..10 {
        let h = std::sync::Arc::clone(&hook);
        handles.push(thread::spawn(move || {
            for _ in 0..100 {
                h.on_filter_start("t", 10);
                h.on_filter_end("t", 5);
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    assert_eq!(hook.filter_count(), 1000);
    assert_eq!(hook.filter_rows_in_total(), 10000);
    assert_eq!(hook.filter_rows_out_total(), 5000);
}
