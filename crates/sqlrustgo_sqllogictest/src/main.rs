//! sqlrustgo-sqllogictest binary
//!
//! Uses the `sqllogictest` crate (risinglightdb/sqllogictest-rs) as SLT parser/runner.
//! Implements the `DB` trait for sqlrustgo's MemoryExecutionEngine.
//!
//! ISSUE: #3373 — Beta Testing System
//!
//! Usage:
//!   cargo run -p sqlrustgo_sqllogictest -- --help
//!   cargo run -p sqlrustgo_sqllogictest -- --test-dir crates/sqlrustgo_sqllogictest/testdata

use parking_lot::RwLock;
use sqllogictest::{DBOutput, DefaultColumnType, Runner, DB};
use sqlrustgo::MemoryExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tokio::runtime::Runtime;

#[derive(Error, Debug)]
pub enum SltError {
    #[error("execution error: {0}")]
    Execution(String),
}

impl PartialEq for SltError {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (SltError::Execution(_), SltError::Execution(_))
        )
    }
}

/// Simple SltDb - each instance has its own fresh storage.
/// A fresh instance is created per file to ensure complete isolation.
pub struct SltDb {
    engine: MemoryExecutionEngine,
}

impl SltDb {
    pub fn new() -> Self {
        let storage = Arc::new(RwLock::new(MemoryStorage::new()));
        let engine = MemoryExecutionEngine::new(storage);
        Self { engine }
    }
}

impl Default for SltDb {
    fn default() -> Self {
        Self::new()
    }
}

impl DB for SltDb {
    type Error = SltError;
    type ColumnType = DefaultColumnType;

    fn run(&mut self, sql: &str) -> Result<DBOutput<Self::ColumnType>, Self::Error> {
        match self.engine.execute(sql) {
            Ok(result) => {
                if result.rows.is_empty() {
                    Ok(DBOutput::StatementComplete(result.affected_rows as u64))
                } else {
                    let rows: Vec<Vec<String>> = result
                        .rows
                        .iter()
                        .map(|row| row.iter().map(|v| v.to_sql_string()).collect())
                        .collect();
                    let types: Vec<DefaultColumnType> = if let Some(first) = result.rows.first() {
                        first
                            .iter()
                            .map(|v| match v {
                                sqlrustgo::Value::Integer(_) => DefaultColumnType::Integer,
                                sqlrustgo::Value::Float(_) => DefaultColumnType::FloatingPoint,
                                _ => DefaultColumnType::Text,
                            })
                            .collect()
                    } else {
                        vec![]
                    };
                    Ok(DBOutput::Rows { types, rows })
                }
            }
            Err(e) => Err(SltError::Execution(format!("{}", e))),
        }
    }

    fn engine_name(&self) -> &str {
        "sqlrustgo"
    }

    fn shutdown(&mut self) {
        // Nothing needed - each file gets a fresh SltDb via fresh Runner
    }
}

/// Strips sqlrustgo debug formatting from values.
///
/// DuckDB outputs: `1`, `hello`  
/// sqlrustgo Value::Integer(1) via Debug: `Integer(1)`, `Text("hello")`
#[allow(clippy::ptr_arg)]
fn strip_debug_format(s: &String) -> String {
    let s = s.trim();
    // Integer(42) -> 42
    if let Some(rest) = s.strip_prefix("Integer(") {
        if let Some(n) = rest.strip_suffix(')') {
            return n.to_string();
        }
    }
    // Float(3.14) -> 3.14
    if let Some(rest) = s.strip_prefix("Float(") {
        if let Some(n) = rest.strip_suffix(')') {
            return n.to_string();
        }
    }
    // Text("hello") -> hello  (strips outer quotes)
    if let Some(rest) = s.strip_prefix("Text(") {
        if let Some(inner) = rest.strip_suffix(')') {
            let inner = inner.trim();
            // Strip outer double-quotes if present
            if inner.starts_with('"') && inner.ends_with('"') && inner.len() >= 2 {
                return inner[1..inner.len() - 1].to_string();
            }
            return inner.to_string();
        }
    }
    // Boolean(true) -> true
    if let Some(rest) = s.strip_prefix("Boolean(") {
        if let Some(b) = rest.strip_suffix(')') {
            return b.to_string();
        }
    }
    // Null -> empty
    if s == "Null" {
        return String::new();
    }
    s.to_string()
}

fn main() {
    let rt = Runtime::new().expect("failed to create tokio runtime");
    rt.block_on(async_main());
}

async fn async_main() {
    let mut args: Vec<String> = std::env::args().collect();
    args.remove(0);

    let mut test_dir = PathBuf::from("crates/sqlrustgo_sqllogictest/testdata");
    let mut filter = String::new();
    let mut max_fail = 0usize;
    let mut show_help = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--test-dir" => {
                i += 1;
                if i < args.len() {
                    test_dir = PathBuf::from(&args[i]);
                }
            }
            "--filter" => {
                i += 1;
                if i < args.len() {
                    filter = args[i].clone();
                }
            }
            "--max-fail" => {
                i += 1;
                if i < args.len() {
                    max_fail = args[i].parse().unwrap_or(0);
                }
            }
            "--help" | "-h" => {
                show_help = true;
            }
            _ => {}
        }
        i += 1;
    }

    if show_help {
        println!("sqlrustgo-sqllogictest — SQLLogicTest Runner for sqlrustgo");
        println!();
        println!("USAGE:  cargo run -p sqlrustgo_sqllogictest [OPTIONS]");
        println!("  --test-dir <DIR>  Test directory (default: testdata/)");
        println!("  --filter <PREFIX> Only .test files matching prefix");
        println!("  --max-fail <N>   Stop after N failures");
        println!("  --help, -h        Show this help");
        return;
    }

    println!("=== sqlrustgo SQLLogicTest Runner ===");
    println!("test_dir: {}", test_dir.display());
    if !filter.is_empty() {
        println!("filter: {}", filter);
    }
    println!();
    let mut files_run = 0usize;
    let mut files_pass = 0usize;
    let mut files_fail = 0usize;

    for entry in walkdir::WalkDir::new(&test_dir)
        .max_depth(3)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        // Skip files in _unsupported subdirectories
        if path.to_string_lossy().contains("_unsupported") {
            continue;
        }
        if !matches!(
            path.extension().and_then(|s| s.to_str()),
            Some("test") | Some("slt")
        ) {
            continue;
        }
        let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if !filter.is_empty() && !filename.starts_with(&filter) {
            continue;
        }

        // Create a fresh Runner (and SltDb) for each file.
        // This ensures complete isolation: each file sees an empty database.
        let mut tester = Runner::new(|| async { Ok(SltDb::new()) });
        tester.with_normalizer(strip_debug_format);
        tester.with_validator(|norm, actual, expected| {
            let expected_results: Vec<String> = expected
                .iter()
                .map(|e| {
                    let normalized = norm(e);
                    normalized.split_whitespace().collect::<Vec<_>>().join(" ")
                })
                .collect();
            let normalized_rows: Vec<String> = actual
                .iter()
                .map(|row| row.iter().map(norm).collect::<Vec<_>>().join(" "))
                .collect();
            normalized_rows == expected_results
        });

        files_run += 1;
        match tester.run_file(path) {
            Ok(_) => {
                files_pass += 1;
                println!("PASS [{}]", filename);
            }
            Err(e) => {
                files_fail += 1;
                eprintln!("FAIL [{}] {}", filename, e);
                if max_fail > 0 && files_fail >= max_fail {
                    eprintln!("\nMax failures ({}) reached.", max_fail);
                    break;
                }
            }
        }
    }

    println!("\n=== Summary ===");
    println!("files:    {}/{} (pass/fail)", files_pass, files_fail);
    let pct = if files_run > 0 {
        files_pass as f64 / files_run as f64 * 100.0
    } else {
        0.0
    };
    println!("pass rate: {:.1}%", pct);

    if files_fail > 0 {
        println!(
            "\nNOTE: Baseline established — fail count decreases as sqlrustgo SQL coverage improves."
        );
    }
}
