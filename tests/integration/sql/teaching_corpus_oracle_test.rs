//! V312-56B / ISSUE #4252 — Teaching corpus SQLite-vs-SQLRustGo row oracle.
//!
//! Runs every `oracle_mode: rowset` corpus file through a fresh
//! `ExecutionEngine<MemoryStorage>` and compares the resulting row set
//! against the SQLite-generated golden file under
//! `docs/releases/v3.12.0/evidence/teaching_corpus/golden/<category>/<file>.txt`.
//!
//! On mismatch, a per-file unified diff is written to
//! `docs/releases/v3.12.0/evidence/teaching_corpus/oracle_diffs/` so a
//! reviewer can inspect the divergence without re-running the test.
//!
//! Out of scope: `oracle_mode: plan_shape` (EXPLAIN plan-shape comparison,
//! deferred to #4218) and `oracle_mode: dialect_only` (PREPARE/EXECUTE,
//! MySQL-only syntax). Those entries are skipped here.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::fs;
use std::path::Path;
use std::sync::Arc;

const CORPUS_DIR: &str = "tests/compat/teaching_sql_v3_12";
const MANIFEST_PATH: &str = "tests/compat/teaching_sql_v3_12/manifest.yml";
const GOLDEN_DIR: &str = "docs/releases/v3.12.0/evidence/teaching_corpus/golden";
const DIFF_DIR: &str = "docs/releases/v3.12.0/evidence/teaching_corpus/oracle_diffs";

#[derive(Debug, Default)]
struct Entry {
    path: String,
    oracle_mode: String,
    order_sensitive: bool,
}

#[derive(Debug)]
struct OracleDiff {
    path: String,
    expected: String,
    actual: String,
}

impl std::fmt::Display for OracleDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "--- oracle diff for {}", self.path)?;
        writeln!(f, "--- expected (SQLite golden)")?;
        for line in self.expected.lines() {
            if line.is_empty() {
                writeln!(f)?;
            } else {
                writeln!(f, "- {line}")?;
            }
        }
        writeln!(f, "+++ actual (SQLRustGo rows)")?;
        for line in self.actual.lines() {
            if line.is_empty() {
                writeln!(f)?;
            } else {
                writeln!(f, "+ {line}")?;
            }
        }
        Ok(())
    }
}

/// Top-level test: every `oracle_mode: rowset` corpus file must produce
/// the same rows as the SQLite golden file.
#[test]
fn test_teaching_corpus_oracle_rowset_matches_sqlite() {
    let entries = read_manifest();
    let rowset_entries: Vec<&Entry> = entries
        .iter()
        .filter(|e| e.oracle_mode == "rowset")
        .collect();
    assert!(
        !rowset_entries.is_empty(),
        "no rowset entries discovered in manifest (manifest parser broken?)"
    );

    let mut diffs: Vec<OracleDiff> = Vec::new();
    for entry in &rowset_entries {
        match run_oracle(entry) {
            Ok(()) => {}
            Err(diff) => {
                write_diff_report(entry, &diff);
                diffs.push(diff);
            }
        }
    }

    assert!(
        diffs.is_empty(),
        "{} of {} rowset corpus files diverge from SQLite golden:\n{}",
        diffs.len(),
        rowset_entries.len(),
        diffs
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// V312-56E / #4255: Run each `oracle_mode: plan_shape` corpus file
/// through `ExecutionEngine::execute_explain` and compare the AST-walk
/// plan dump against the SQLite `EXPLAIN QUERY PLAN` golden after
/// normalization.
#[test]
fn test_teaching_corpus_oracle_plan_shape_matches_sqlite() {
    let entries = read_manifest();
    let plan_shape_entries: Vec<&Entry> = entries
        .iter()
        .filter(|e| e.oracle_mode == "plan_shape")
        .collect();
    assert!(
        !plan_shape_entries.is_empty(),
        "no plan_shape entries discovered in manifest"
    );

    let mut diffs: Vec<OracleDiff> = Vec::new();
    for entry in &plan_shape_entries {
        match run_plan_shape_oracle(entry) {
            Ok(()) => {}
            Err(diff) => {
                write_diff_report(entry, &diff);
                diffs.push(diff);
            }
        }
    }

    assert!(
        diffs.is_empty(),
        "{} of {} plan_shape corpus files diverge from SQLite EXPLAIN QUERY PLAN:\n{}",
        diffs.len(),
        plan_shape_entries.len(),
        diffs
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    );
}

// -------- plan_shape oracle execution (V312-56E / #4255) --------

fn run_plan_shape_oracle(entry: &Entry) -> Result<(), OracleDiff> {
    let sql_path = format!("{}/{}", CORPUS_DIR, entry.path);
    let raw_sql =
        fs::read_to_string(&sql_path).unwrap_or_else(|e| panic!("read {}: {}", sql_path, e));
    let stmts = split_statements(&strip_comments(&raw_sql));
    assert!(
        !stmts.is_empty(),
        "{}: no executable statements after stripping comments",
        entry.path
    );

    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    let mut last_err: Option<String> = None;
    let mut last_rows: Vec<Vec<Value>> = Vec::new();
    for stmt in &stmts {
        match engine.execute(stmt) {
            Ok(result) => {
                last_rows = result.rows;
                last_err = None;
            }
            Err(err) => {
                last_err = Some(format!("{err:?}"));
            }
        }
    }
    if let Some(err) = last_err {
        return Err(OracleDiff {
            path: entry.path.clone(),
            expected: "<engine should have produced plan rows>".to_string(),
            actual: err,
        });
    }

    let actual_text = normalize_plan_rows(&last_rows);
    let golden_path = format!("{}/{}", GOLDEN_DIR, entry.path.replace(".sql", ".txt"));
    let expected_raw = match fs::read_to_string(&golden_path) {
        Ok(s) => s,
        Err(e) => {
            return Err(OracleDiff {
                path: entry.path.clone(),
                expected: format!("<missing golden: {e}>"),
                actual: actual_text,
            });
        }
    };

    let mut expected_lines = normalize_sqlite_plan(&expected_raw);
    let mut actual_lines = actual_text
        .lines()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    expected_lines.sort();
    actual_lines.sort();

    if expected_lines == actual_lines {
        return Ok(());
    }
    Err(OracleDiff {
        path: entry.path.clone(),
        expected: expected_lines.join("\n"),
        actual: actual_lines.join("\n"),
    })
}

/// Convert SQLRustGo EXPLAIN rows into a normalized multi-line
/// string. Operators SQLite does not emit are dropped so the two
/// sides line up structurally.
fn normalize_plan_rows(rows: &[Vec<Value>]) -> String {
    let mut out = Vec::new();
    for row in rows {
        if let Some(v) = row.first() {
            let line = v.to_sql_string();
            if line.starts_with("Projection ")
                || line == "Distinct"
                || line.starts_with("Sort (TEMP B-TREE FOR")
            {
                continue;
            }
            let normalized = if line.starts_with("Sort ") && !line.contains("TEMP") {
                format!("Sort {}", line.trim_start_matches("Sort "))
            } else {
                line.clone()
            };
            let normalized = if let Some(rest) = normalized.strip_prefix("GroupBy ") {
                if rest.ends_with("keys") {
                    "GroupBy".to_string()
                } else {
                    normalized
                }
            } else {
                normalized
            };
            out.push(normalized);
        }
    }
    out.join("\n")
}

/// Normalize a SQLite `EXPLAIN QUERY PLAN` golden into the canonical
/// operator vocabulary used by SQLRustGo.
fn normalize_sqlite_plan(raw: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed == "QUERY PLAN" {
            continue;
        }
        let unindented = trimmed
            .trim_start_matches("|--")
            .trim_start_matches("`--")
            .trim_start_matches("`")
            .trim_start_matches("|");
        let mapped = if let Some(rest) = unindented.strip_prefix("SCAN ") {
            format!("SeqScan {rest}")
        } else if let Some(rest) = unindented.strip_prefix("SEARCH ") {
            let table = rest.split_whitespace().next().unwrap_or("");
            format!("IndexScan {table}")
        } else if unindented.starts_with("USE TEMP B-TREE FOR GROUP BY") {
            "GroupBy".to_string()
        } else if unindented.starts_with("USE TEMP B-TREE FOR ORDER BY") {
            "Sort".to_string()
        } else {
            unindented.to_string()
        };
        out.push(mapped);
    }
    out
}

// -------- manifest parsing (hand-rolled, mirrors teaching_corpus_test.rs) --------

fn read_manifest() -> Vec<Entry> {
    let raw = fs::read_to_string(MANIFEST_PATH).expect("read manifest");
    let mut in_files = false;
    let mut entries: Vec<Entry> = Vec::new();
    let mut current: Option<Entry> = None;
    for line in raw.lines() {
        if line.starts_with("files:") {
            in_files = true;
            continue;
        }
        if in_files {
            let trimmed = line.trim_start();
            // Skip blank lines and YAML comments inside the files list.
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if trimmed.starts_with("- ") {
                if let Some(prev) = current.take() {
                    entries.push(prev);
                }
                if let Some(rest) = trimmed.trim_start_matches("- ").strip_prefix("path:") {
                    current = Some(Entry {
                        path: unquote(rest.trim()),
                        oracle_mode: String::new(),
                        order_sensitive: false,
                    });
                }
            } else if current.is_some() && line.starts_with(' ') {
                // Indented field of the current entry. Recognized fields
                // are stored; unknown fields (e.g., `description:`,
                // `feature:`) are silently ignored so we don't exit
                // the `files:` section on a foreign field name.
                if let Some(ref mut e) = current {
                    if let Some(v) = trimmed.strip_prefix("oracle_mode:") {
                        e.oracle_mode = unquote(v.trim()).to_string();
                    } else if let Some(v) = trimmed.strip_prefix("order_sensitive:") {
                        e.order_sensitive = matches!(v.trim(), "true");
                    }
                }
            } else if !trimmed.is_empty() {
                // Non-indented, non-`- ` line → top-level structural
                // key (e.g. `ignore_rules:`). Finalize the current
                // entry and exit `files:`.
                if let Some(prev) = current.take() {
                    entries.push(prev);
                }
                in_files = false;
            }
        }
    }
    if let Some(prev) = current.take() {
        entries.push(prev);
    }
    entries
}

fn unquote(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.len() >= 2
        && ((trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\'')))
    {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

// -------- per-file oracle execution --------

fn run_oracle(entry: &Entry) -> Result<(), OracleDiff> {
    let sql_path = format!("{}/{}", CORPUS_DIR, entry.path);
    let raw_sql =
        fs::read_to_string(&sql_path).unwrap_or_else(|e| panic!("read {}: {}", sql_path, e));
    let stmts = split_statements(&strip_comments(&raw_sql));
    assert!(
        !stmts.is_empty(),
        "{}: no executable statements after stripping comments",
        entry.path
    );

    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    let mut last_err: Option<String> = None;
    let mut last_rows: Vec<Vec<Value>> = Vec::new();
    for stmt in &stmts {
        match engine.execute(stmt) {
            Ok(result) => {
                last_rows = result.rows;
                last_err = None;
            }
            Err(err) => {
                last_err = Some(format!("{err:?}"));
            }
        }
    }
    if let Some(err) = last_err {
        return Err(OracleDiff {
            path: entry.path.clone(),
            expected: "<engine should have produced rows>".to_string(),
            actual: err,
        });
    }

    let actual_text = normalize_rows(&last_rows);
    let golden_path = format!("{}/{}", GOLDEN_DIR, entry.path.replace(".sql", ".txt"));
    let expected_raw = match fs::read_to_string(&golden_path) {
        Ok(s) => s,
        Err(e) => {
            return Err(OracleDiff {
                path: entry.path.clone(),
                expected: format!("<missing golden: {e}>"),
                actual: actual_text,
            });
        }
    };

    let mut expected_lines = canonicalize_lines(&expected_raw);
    let mut actual_lines = canonicalize_lines(&actual_text);
    if !entry.order_sensitive {
        expected_lines.sort();
        actual_lines.sort();
    }

    if expected_lines == actual_lines {
        return Ok(());
    }
    Err(OracleDiff {
        path: entry.path.clone(),
        expected: expected_lines.join("\n"),
        actual: actual_lines.join("\n"),
    })
}

fn normalize_rows(rows: &[Vec<Value>]) -> String {
    rows.iter()
        .map(|row| {
            row.iter()
                .map(|v| canonicalize_token(&v.to_sql_string()))
                .collect::<Vec<_>>()
                .join("|")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Strip the trailing ".0" from any numeric token so SQLite's
/// `150.0` matches the engine's `150` (Rust's `f64` Display
/// drops the trailing zero). True non-integer floats like
/// `3.14` are unchanged.
fn canonicalize_token(s: &str) -> String {
    match s.strip_suffix(".0") {
        Some(prefix) => prefix.to_string(),
        None => s.to_string(),
    }
}

fn canonicalize_lines(s: &str) -> Vec<String> {
    s.lines()
        .map(|line| {
            line.split('|')
                .map(canonicalize_token)
                .collect::<Vec<_>>()
                .join("|")
        })
        .collect()
}

// -------- SQL preprocessing --------

fn strip_comments(s: &str) -> String {
    s.lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Split SQL on `;` outside of single-quoted strings. The teaching
/// corpus uses no escaped quotes and no semicolons inside literals.
fn split_statements(s: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut buf = String::new();
    let mut in_str = false;
    for c in s.chars() {
        match c {
            '\'' => {
                in_str = !in_str;
                buf.push(c);
            }
            ';' if !in_str => {
                let trimmed = buf.trim();
                if !trimmed.is_empty() {
                    out.push(trimmed.to_string());
                }
                buf.clear();
            }
            _ => buf.push(c),
        }
    }
    let trimmed = buf.trim();
    if !trimmed.is_empty() {
        out.push(trimmed.to_string());
    }
    out
}

// -------- diff report --------

fn write_diff_report(entry: &Entry, diff: &OracleDiff) {
    if !Path::new(DIFF_DIR).exists() {
        let _ = fs::create_dir_all(DIFF_DIR);
    }
    let name = entry.path.replace('/', "_").replace(".sql", ".diff");
    let body = format!(
        "# Oracle diff for {}\n# generated_at={}\n--- expected (SQLite golden)\n+++ actual (SQLRustGo rows)\n{}\n",
        entry.path,
        chrono_like_now(),
        unified_diff_text(&diff.expected, &diff.actual)
    );
    let _ = fs::write(format!("{DIFF_DIR}/{name}"), body);
}

/// Tiny RFC3339-ish stamp without pulling in the `chrono` crate.
fn chrono_like_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("unix={secs}")
}

fn unified_diff_text(expected: &str, actual: &str) -> String {
    let mut s = String::new();
    for line in expected.lines() {
        s.push('-');
        s.push(' ');
        s.push_str(line);
        s.push('\n');
    }
    for line in actual.lines() {
        s.push('+');
        s.push(' ');
        s.push_str(line);
        s.push('\n');
    }
    s
}
