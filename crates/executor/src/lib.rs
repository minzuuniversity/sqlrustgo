// SQLRustGo executor module

pub mod ast_adapter;
pub mod execution;
pub mod executor;
pub mod executor_metrics;
pub mod local_executor_dml;
pub mod mutation_compiler;
pub mod predicate_compiler;
pub mod query_cache;
pub mod query_cache_config;
pub mod sql_executor;
pub mod stored_proc;
pub mod trigger;
pub mod trigger_eval;
pub mod update_compiler;
pub mod vec_simd;
pub mod window_executor;

pub use execution::trace::ExecutionTrace;
pub use executor::{Executor, ExecutorResult, VolcanoExecutor};
pub use executor_metrics::ExecutorMetrics;
pub use sql_executor::{ExecutionResult, SqlExecutor};
