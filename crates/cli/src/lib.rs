//! sqlrustgo-cli — CLI client for SQLRustGo.
//!
//! Subcommands:
//! - `exec`  — execute a one-off SQL query and print results
//! - `repl`  — interactive REPL shell
//! - `soak`  — real SQL SOAK test runner (continuous query workload)

pub mod client;
pub mod exec;
pub mod repl;
pub mod soak;
