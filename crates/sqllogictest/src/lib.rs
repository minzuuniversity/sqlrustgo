//! SQLLogicTest Runner for sqlrustgo
//!
//! ISSUE: #3373 — Beta Testing System
//!
//! This crate runs SQLite's official SQLLogicTest (.test) files against
//! sqlrustgo, using SQLite as the reference implementation for result
//! comparison.
//!
//! ## SLT Format Reference
//!
//! ```text
//! statement ok
//! CREATE TABLE t1 (a INT, b TEXT)
//!
//! query I T
//! SELECT a, b FROM t1 ORDER BY a
//! ----
//! 1    hello
//! 2    world
//!
//! statement error
//! SELECT * FROM nonexistent
//! ```
//!
//! - `statement ok`     — expect execution to succeed
//! - `statement error`  — expect a specific error code
//! - `query I T`       — expect rows (I=integer, T=text, R=real)
//! - `----`             — separates SQL from expected output
//! - `halt` / `skip`   — skip remaining tests in this file

pub mod parser;
pub mod runner;
pub mod adapters;

pub use parser::{ParseError, Statement, StatementKind, SltFile};
pub use adapters::{Adapter, DiffResult};
pub use runner::{run_all, RunError};
pub use runner::{RunConfig, RunResult, TestResult};
