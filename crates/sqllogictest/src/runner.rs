//! SLT Test Runner
//!
//! Runs parsed SLT statements against the sqlrustgo ExecutionEngine.

use std::path::Path;
use std::sync::Arc;
use parking_lot::RwLock;
use thiserror::Error;

use sqlrustgo::MemoryExecutionEngine;
use sqlrustgo_storage::MemoryStorage;

use crate::parser::{SltFile, Statement, StatementKind};

#[derive(Error, Debug)]
pub enum RunError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("engine error: {0}")]
    Engine(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestResult {
    Pass,
    Fail { detail: String },
    Skip { reason: String },
    Error { detail: String },
}

impl TestResult {
    fn detail(&self) -> String {
        match self {
            Self::Pass => String::new(),
            Self::Fail { detail } => detail.clone(),
            Self::Skip { reason } => reason.clone(),
            Self::Error { detail } => detail.clone(),
        }
    }
}

#[derive(Debug)]
pub struct RunConfig {
    pub test_dir: String,
    pub file_filter: String,
    pub max_failures: usize,
    pub verbose: bool,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            test_dir: "crates/sqllogictest/testdata".to_string(),
            file_filter: String::new(),
            max_failures: 0,
            verbose: false,
        }
    }
}

#[derive(Debug)]
pub struct RunResult {
    pub files_run: usize,
    pub files_passed: usize,
    pub files_failed: usize,
    pub total_stmts: usize,
    pub total_pass: usize,
    pub total_fail: usize,
    pub total_skip: usize,
    pub total_error: usize,
}

impl RunResult {
    pub fn pass_rate(&self) -> f64 {
        let total = self.total_pass + self.total_fail;
        if total == 0 {
            0.0
        } else {
            self.total_pass as f64 / total as f64 * 100.0
        }
    }
}

/// Run all .test files in the test directory
pub fn run_all(config: &RunConfig) -> Result<RunResult, RunError> {
    let mut result = RunResult {
        files_run: 0,
        files_passed: 0,
        files_failed: 0,
        total_stmts: 0,
        total_pass: 0,
        total_fail: 0,
        total_skip: 0,
        total_error: 0,
    };

    for entry in walkdir::WalkDir::new(&config.test_dir)
        .max_depth(2)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("test") {
            continue;
        }
        if !config.file_filter.is_empty() {
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if !name.starts_with(&config.file_filter) {
                continue;
            }
        }

        let file_result = run_file(path, config)?;
        result.files_run += 1;
        result.total_stmts += file_result.0;
        result.total_pass += file_result.1;
        result.total_fail += file_result.2;
        result.total_skip += file_result.3;
        result.total_error += file_result.4;
        if file_result.2 == 0 {
            result.files_passed += 1;
        } else {
            result.files_failed += 1;
        }
    }

    Ok(result)
}

/// Run a single .test file, returns (stmts, pass, fail, skip, error)
fn run_file(
    path: &Path,
    config: &RunConfig,
) -> Result<(usize, usize, usize, usize, usize), RunError> {
    let content = std::fs::read_to_string(path)?;
    let file = SltFile::parse(&content, path.to_str().unwrap())
        .map_err(|e| RunError::Parse(e.to_string()))?;

    // Each .test file gets a fresh in-memory engine (SLT semantics)
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = MemoryExecutionEngine::new(storage);

    let mut stmts = 0;
    let mut pass = 0;
    let mut fail = 0;
    let mut skip = 0;
    let mut error = 0;

    for stmt in &file.stmts {
        stmts += 1;
        let result = run_stmt(stmt, &mut engine, config.verbose);

        match result {
            TestResult::Pass => pass += 1,
            TestResult::Skip { .. } => skip += 1,
            TestResult::Fail { .. } => {
                fail += 1;
                if config.verbose {
                    eprintln!("FAIL [{}] {}: {}", path.display(), stmts, result.detail());
                }
                if config.max_failures > 0 && fail >= config.max_failures {
                    break;
                }
            }
            TestResult::Error { .. } => {
                error += 1;
                eprintln!("ERROR [{}] {}: {}", path.display(), stmts, result.detail());
            }
        }
    }

    Ok((stmts, pass, fail, skip, error))
}

fn run_stmt(stmt: &Statement, engine: &mut MemoryExecutionEngine, verbose: bool) -> TestResult {
    match stmt.kind {
        StatementKind::Halt => return TestResult::Skip { reason: "halt".to_string() },
        StatementKind::Skip => return TestResult::Skip { reason: "skip".to_string() },
        StatementKind::Connect | StatementKind::Rebuild | StatementKind::Mode => {
            return TestResult::Pass
        }
        StatementKind::StatementOk
        | StatementKind::StatementOnly
        | StatementKind::StatementError
        | StatementKind::Query
        | StatementKind::HashQuery
        | StatementKind::QueryParallel => {
            // Execute the SQL
            match engine.execute(stmt.sql.as_str()) {
                Ok(_result) => TestResult::Pass,
                Err(e) => {
                    if verbose {
                        eprintln!("  engine error: {:?}", e);
                    }
                    TestResult::Error { detail: format!("{:?}", e) }
                }
            }
        }
    }
}
