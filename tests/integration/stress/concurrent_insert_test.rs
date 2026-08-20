//! Integration test: Concurrent INSERT under high concurrency
//!
//! Regression test for PERF-5 (Issue #3434), discovered during v3.10.0 168h SOAK.
//! Tests that:
//! - INSERT IGNORE properly skips duplicates (no parse error)
//! - INSERT ... ON DUPLICATE KEY UPDATE works correctly
//! - REPLACE INTO works correctly
//! - High-concurrency inserts don't cause "Lost connection" errors
//!
//! This test uses the wired `sqlrustgo-mysql-server repl` (via the helper from
//! `tests/integration/ddl/alter_table_test.rs`) for true e2e verification.

use std::io::{Read, Write};
use std::process::{Command, Stdio};

/// Locate the mysql-server binary (cargo test sets CARGO_BIN_EXE_<name>).
fn bin_path() -> String {
    std::env::var("CARGO_BIN_EXE_sqlrustgo-mysql-server")
        .ok()
        .or_else(|| std::env::var("SQLRUSTGO_BIN").ok())
        .unwrap_or_else(|| {
            for candidate in [
                "target/release/sqlrustgo-mysql-server",
                "target/debug/sqlrustgo-mysql-server",
                "../target/release/sqlrustgo-mysql-server",
                "../target/debug/sqlrustgo-mysql-server",
            ] {
                if std::path::Path::new(candidate).exists() {
                    return candidate.to_string();
                }
            }
            "sqlrustgo-mysql-server".to_string()
        })
}

/// Run a multi-statement REPL script; return (stdout, stderr, exit_code).
fn run_repl(script: &str) -> (String, String, i32) {
    let mut child = Command::new(bin_path())
        .arg("repl")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn REPL");
    let mut stdin = child.stdin.take().expect("Failed to get stdin");
    let stdout = child.stdout.take().expect("Failed to get stdout");
    let stderr = child.stderr.take().expect("Failed to get stderr");
    let stdout_thread = std::thread::spawn(move || {
        let mut buf = String::new();
        let mut h = stdout;
        let _ = h.read_to_string(&mut buf);
        buf
    });
    let stderr_thread = std::thread::spawn(move || {
        let mut buf = String::new();
        let mut h = stderr;
        let _ = h.read_to_string(&mut buf);
        buf
    });
    stdin
        .write_all(script.as_bytes())
        .expect("Failed to write to stdin");
    std::thread::spawn(move || {
        drop(stdin);
    });
    let status = child.wait().expect("wait failed");
    let stdout = stdout_thread.join().expect("stdout thread");
    let stderr = stderr_thread.join().expect("stderr thread");
    let code = status.code().unwrap_or(-1);
    (stdout, stderr, code)
}

/// Run a script N times in separate processes (for concurrency simulation).
/// Returns number of successful runs (0 errors).
fn run_concurrent(num_processes: usize, script: &str) -> usize {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;

    let success = Arc::new(AtomicUsize::new(0));
    let handles: Vec<_> = (0..num_processes)
        .map(|_i| {
            let script = script.to_string();
            let success = success.clone();
            thread::spawn(move || {
                let (_out, _err, code) = run_repl(&script);
                if code == 0 {
                    success.fetch_add(1, Ordering::SeqCst);
                }
            })
        })
        .collect();

    for h in handles {
        h.join().ok();
    }
    success.load(Ordering::SeqCst)
}

// =============================================================================
// V311-23: PERF-5 Concurrent INSERT regression tests
// =============================================================================

#[test]
fn test_insert_ignore_parses() {
    // V311-23: Verify INSERT IGNORE syntax is accepted (was a parse error in v3.10.0)
    let (out, _err, _code) = run_repl(
        "CREATE TABLE ci_test1 (id INT PRIMARY KEY, val TEXT);\n\
         INSERT IGNORE INTO ci_test1 VALUES (1, 'a');\n\
         INSERT IGNORE INTO ci_test1 VALUES (1, 'b');\n\
         SELECT * FROM ci_test1;\n\
         .exit\n",
    );
    // First INSERT succeeds, second INSERT IGNORE skips duplicate
    assert!(
        out.contains("a"),
        "First INSERT should succeed with 'a', got:\n{}",
        out
    );
    assert!(
        !out.contains("b"),
        "INSERT IGNORE should skip duplicate, got:\n{}",
        out
    );
    assert!(
        !out.contains("Parse error"),
        "INSERT IGNORE should not produce parse error, got:\n{}",
        out
    );
}

#[test]
fn test_insert_ignore_concurrent() {
    // V311-23: Verify INSERT IGNORE works correctly under high concurrency
    // 4 processes, each inserting with overlapping ID ranges
    let script = "\
        CREATE TABLE IF NOT EXISTS ci_test2 (id INT PRIMARY KEY, val TEXT);\n\
        DELETE FROM ci_test2;\n\
        INSERT IGNORE INTO ci_test2 VALUES (1, 'a');\n\
        INSERT IGNORE INTO ci_test2 VALUES (1, 'b');\n\
        INSERT IGNORE INTO ci_test2 VALUES (2, 'c');\n\
        .exit\n";
    // Run 4 concurrent processes
    let successes = run_concurrent(4, script);
    assert_eq!(
        successes, 4,
        "All 4 concurrent processes should succeed (0 parse errors)"
    );
}

#[test]
fn test_insert_ignore_concurrent_10x() {
    // V311-23: Stress test with 10 concurrent processes
    let script = "\
        CREATE TABLE IF NOT EXISTS ci_test3 (id INT PRIMARY KEY, val TEXT);\n\
        DELETE FROM ci_test3;\n\
        INSERT IGNORE INTO ci_test3 VALUES (1, 'a');\n\
        .exit\n";
    let successes = run_concurrent(10, script);
    assert_eq!(successes, 10, "All 10 concurrent processes should succeed");
}

#[test]
fn test_on_duplicate_key_update_works() {
    // V311-23: Verify INSERT ... ON DUPLICATE KEY UPDATE still works
    let (out, _err, _code) = run_repl(
        "CREATE TABLE ci_test4 (id INT PRIMARY KEY, val TEXT);\n\
         INSERT INTO ci_test4 VALUES (1, 'first') ON DUPLICATE KEY UPDATE val='updated';\n\
         INSERT INTO ci_test4 VALUES (1, 'second') ON DUPLICATE KEY UPDATE val='updated';\n\
         SELECT * FROM ci_test4;\n\
         .exit\n",
    );
    // First INSERT inserted 'first'; second ON DUPLICATE KEY UPDATE overwrites with 'updated'
    // Only 'updated' should be in the output after both INSERTs
    assert!(
        out.contains("updated"),
        "Value should be 'updated' after ODUK, got:\n{}",
        out
    );
    assert!(
        !out.contains("first"),
        "Value 'first' should be overwritten by ODUK, got:\n{}",
        out
    );
}

#[test]
fn test_replace_into_works() {
    // V311-23: Verify REPLACE INTO still works
    let (out, _err, _code) = run_repl(
        "CREATE TABLE ci_test5 (id INT PRIMARY KEY, val TEXT);\n\
         INSERT INTO ci_test5 VALUES (1, 'original');\n\
         REPLACE INTO ci_test5 VALUES (1, 'replaced');\n\
         SELECT * FROM ci_test5;\n\
         .exit\n",
    );
    assert!(
        out.contains("replaced"),
        "REPLACE should replace value, got:\n{}",
        out
    );
    assert!(
        !out.contains("original"),
        "REPLACE should remove original value, got:\n{}",
        out
    );
}

#[test]
fn test_plain_insert_duplicate_errors() {
    // V311-23: Verify plain INSERT still errors on duplicate (no IGNORE)
    let (out, _err, _code) = run_repl(
        "CREATE TABLE ci_test6 (id INT PRIMARY KEY, val TEXT);\n\
         INSERT INTO ci_test6 VALUES (1, 'a');\n\
         INSERT INTO ci_test6 VALUES (1, 'b');\n\
         .exit\n",
    );
    let combined = format!("{}{}", out, _err);
    assert!(
        combined.contains("Duplicate entry") || combined.contains("duplicate"),
        "Plain INSERT on duplicate should error, got:\nstdout: {}\nstderr: {}",
        out,
        _err
    );
}
