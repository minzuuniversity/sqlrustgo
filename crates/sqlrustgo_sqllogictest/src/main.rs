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
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::LazyLock;
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

/// SltDb backed by shared storage.
///
/// Each named connection gets its own SltDb, but all share the same MemoryStorage
/// via Arc. This allows different connections to see each other's uncommitted
/// changes (transaction isolation is handled by the storage layer).
pub struct SltDb {
    engine: MemoryExecutionEngine,
}

impl SltDb {
    pub fn new() -> Self {
        let storage = Arc::new(RwLock::new(MemoryStorage::new()));
        let engine = MemoryExecutionEngine::new(storage);
        Self { engine }
    }

    /// Create with shared storage (used for multi-connection tests).
    pub fn with_storage(storage: Arc<RwLock<MemoryStorage>>) -> Self {
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

    fn shutdown(&mut self) {}
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

// =============================================================================
// Pre-processor: handles set variable, include, foreach/endloop, inline connection
// =============================================================================

/// Regex matching inline connection suffix on statement/query lines.
///
/// Matches: "statement ok con1", "query I con2", "query III con_default"
/// Group 1: "statement" or "query"
/// Group 2: the column type part (e.g., "ok", "I", "III")
/// Group 3: whitespace
/// Group 4: connection name (e.g., "con1", "con2")
static CONN_SUFFIX_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"^(statement|query)(\s+[^\s]+)(\s+)([a-zA-Z_][a-zA-Z0-9_]*)$").unwrap()
});

/// Substitute $(varname) or ${varname} patterns with stored values
fn substitute_variables(s: &str, variables: &HashMap<String, String>) -> String {
    let mut result = s.to_string();
    for (name, value) in variables {
        let pattern_dollar = format!("$({})", name);
        let pattern_brace = format!("${{{}}}", name);
        result = result.replace(&pattern_dollar, value);
        result = result.replace(&pattern_brace, value);
    }
    result
}

/// Pre-process a sqllogictest file, expanding include/set/foreach directives
/// and substituting variables.
fn preprocess_test_file(path: &Path) -> Result<String, String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("failed to read {:?}: {}", path, e))?;
    preprocess_content(&raw, path.parent().unwrap_or(Path::new(".")))
}

/// Pre-process test content string, expanding directives
fn preprocess_content(content: &str, base_dir: &Path) -> Result<String, String> {
    let mut variables: HashMap<String, String> = HashMap::new();
    let mut output = String::new();
    let mut lines = content.lines().peekable();
    let mut line_number = 0;

    while let Some(line) = lines.next() {
        line_number += 1;
        let trimmed = line.trim();

        // Handle: set variable <name> <value>
        if let Some(rest) = trimmed.strip_prefix("set variable ") {
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            if parts.len() == 2 {
                variables.insert(parts[0].to_string(), parts[1].to_string());
            } else {
                eprintln!(
                    "warning: line {}: invalid set variable syntax: {}",
                    line_number, line
                );
            }
            continue;
        }

        // Handle: include <path>
        if let Some(include_path) = trimmed.strip_prefix("include ") {
            let include_path = include_path.trim();
            // Try relative to base_dir first, then as absolute
            let full_path = if Path::new(include_path).is_absolute() {
                PathBuf::from(include_path)
            } else {
                base_dir.join(include_path)
            };
            // Skip missing include files (non-fatal for DuckDB compatibility tests)
            if full_path.exists() {
                match preprocess_test_file(&full_path) {
                    Ok(included) => {
                        output.push_str(&included);
                        output.push('\n');
                    }
                    Err(e) => {
                        eprintln!("warning: include failed for {:?}: {}", full_path, e);
                    }
                }
            } else {
                eprintln!("warning: include file not found: {:?}", full_path);
            }
            continue;
        }

        // Handle: foreach <var> <value1> <value2> ...
        if let Some(foreach_spec) = trimmed.strip_prefix("foreach ") {
            let parts: Vec<&str> = foreach_spec.split_whitespace().collect();
            if parts.len() < 2 {
                eprintln!(
                    "warning: line {}: foreach needs variable and values: {}",
                    line_number, line
                );
                continue;
            }
            let var_name = parts[0];
            let values: Vec<&str> = parts[1..].to_vec();

            // Collect body until endloop
            let mut loop_body = Vec::new();
            let mut foreach_depth = 0;
            while let Some(body_line) = lines.next() {
                line_number += 1;
                let body_trimmed = body_line.trim();
                if body_trimmed == "endloop" && foreach_depth == 0 {
                    break;
                }
                // Track nested foreach
                if body_trimmed.starts_with("foreach ") {
                    foreach_depth += 1;
                }
                if body_trimmed == "endloop" && foreach_depth > 0 {
                    foreach_depth -= 1;
                }
                loop_body.push(body_line);
            }

            // Expand loop for each value
            for value in values {
                for body_line in &loop_body {
                    // First substitute the loop variable
                    let expanded = body_line.replace(&format!("${{{}}}", var_name), value);
                    // Then substitute any other variables (e.g. from set variable)
                    let expanded = substitute_variables(&expanded, &variables);
                    output.push_str(&expanded);
                    output.push('\n');
                }
            }
            continue;
        }

        // Handle inline connection suffix: "statement ok con1" -> "connection con1\nstatement ok"
        if let Some(caps) = CONN_SUFFIX_REGEX.captures(trimmed) {
            let conn_name = &caps[4];
            // Emit connection directive BEFORE the line
            output.push_str("connection ");
            output.push_str(conn_name);
            output.push('\n');

            // Emit the stripped line (keyword + type, no connection name)
            let prefix = format!("{}{}", &caps[1], &caps[2]);
            let expanded = substitute_variables(&prefix, &variables);
            output.push_str(&expanded);
            output.push('\n');
            continue;
        }

        // Substitute variables in this line
        let expanded = substitute_variables(line, &variables);
        output.push_str(&expanded);
        output.push('\n');
    }

    Ok(output)
}

// =============================================================================
// Temp file handling for pre-processed content
// =============================================================================

static TEMP_DIR: LazyLock<tempfile::TempDir> =
    LazyLock::new(|| tempfile::tempdir().expect("failed to create temp dir"));

fn write_temp_file(content: &str, suffix: &str) -> PathBuf {
    let temp_dir = &*TEMP_DIR;
    let safe_suffix = suffix.replace(['/', ' ', '#', ':'], "_");
    let mut path = temp_dir.path().join(format!("slt_{}", safe_suffix));
    path.set_extension("test");
    let mut file = File::create(&path).expect("failed to create temp file");
    file.write_all(content.as_bytes())
        .expect("failed to write temp file");
    path
}

// =============================================================================
// Main
// =============================================================================

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

        // Pre-process the file: handle set variable, include, foreach/endloop, inline connection
        let processed = match preprocess_test_file(path) {
            Ok(content) => content,
            Err(e) => {
                files_fail += 1;
                eprintln!("PREPROCESS FAIL [{}] {}", filename, e);
                continue;
            }
        };

        // Write processed content to a temp file for the runner
        let temp_path = write_temp_file(&processed, filename);

        // Shared storage for multi-connection support.
        // All named connections share the same storage so they can see each other's
        // uncommitted changes (for testing transaction isolation).
        let shared_storage = Arc::new(RwLock::new(MemoryStorage::new()));

        // Create a fresh Runner for each file. The Runner creates a fresh SltDb
        // for each named connection, but they all share the same storage.
        // RefCell allows FnMut closure to clone the Arc on each call.
        let storage_cell = RefCell::new(shared_storage);
        let mut tester = Runner::new(move || {
            let storage = storage_cell.borrow().clone();
            async move { Ok(SltDb::with_storage(storage)) }
        });
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
        match tester.run_file(&temp_path) {
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
        // Round-9 (V312-24): exit non-zero so the gate script can distinguish
        // a successful baseline run from a real failure. Previously `main`
        // returned 0 even with `files_fail > 0`, masking regressions from the
        // gate logic. See plan: /home/openclaw/.claude/plans/pure-hugging-sifakis.md
        std::process::exit(1);
    }
}
