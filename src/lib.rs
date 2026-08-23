//! SQLRustGo Database System Library
//!
//! A Rust implementation of a SQL-92 compliant database system.
//! This crate re-exports functionality from the modular crates/ workspace.

#![allow(
    unused_mut,
    clippy::unused_enumerate_index,
    clippy::needless_borrow,
    renamed_and_removed_lints
)]

pub mod cbo_estimator;
pub mod engine_builder;
pub mod engine_collation;
pub mod engine_create;
pub mod engine_cte;
pub mod engine_ddl;
pub mod engine_dml;
pub mod engine_helpers;
pub mod engine_select;
pub mod engine_setops;
pub mod engine_utils;
pub mod execution_engine;
pub mod expr_utils;

#[cfg(test)]
mod execution_engine_tests;

pub use sqlrustgo_executor::parallel_executor::{
    ParallelExecutor, ParallelVolcanoExecutor, PARALLEL_MIN_ROWS,
};
pub use sqlrustgo_executor::{Executor, ExecutorResult};
pub use sqlrustgo_optimizer::Optimizer as QueryOptimizer;
pub use sqlrustgo_parser::lexer::tokenize;
pub use sqlrustgo_parser::{parse, Lexer, Statement, Token};
pub use sqlrustgo_planner::{
    DataType, Expr, Field, LogicalPlan, Optimizer, PhysicalPlan, Planner, Schema, SeqScanExec,
};
pub use sqlrustgo_storage::{
    BPlusTree, BufferPool, FileStorage, MemoryStorage, Page, StorageEngine,
};
pub use sqlrustgo_types::{SqlError, SqlResult, Value};

pub use execution_engine::{ExecutionEngine, MemoryExecutionEngine};

// V312-58 Sprint 3 Phase 1 diagnostic re-exports. Used by
// `tests/integration/oracle/diag_q17_sprint3_path.rs` to verify which
// path TPC-H Q17 actually takes through the correlated-subquery pre-eval
// pipeline. See comments at the call sites in `engine_select.rs`.
pub use engine_select::{dump_v312_58_sprint3_diag, reset_v312_58_sprint3_diag};

/// Initialize the database system
pub fn init() {
    println!("SQLRustGo Database System initialized");
}
