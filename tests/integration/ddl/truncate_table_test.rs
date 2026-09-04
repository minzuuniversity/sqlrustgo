//! Integration test: TRUNCATE TABLE
//!
//! Uses the wired `sqlrustgo-mysql-server repl` over stdin (true
//! e2e). `exec` only accepts a single statement per process and
//! starts a fresh MemoryStorage each invocation, so multi-statement
//! DDL testing requires the REPL where state persists across
//! statements.

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

#[test]
fn test_truncate_empty_table() {
    let (out, _err, _code) = run_repl(
        "CREATE TABLE tr1 (id INT PRIMARY KEY, name VARCHAR(50));\n\
         TRUNCATE TABLE tr1;\n\
         .exit\n",
    );
    // TRUNCATE on an empty table should not error. The REPL prints
    // (0 rows) for empty results and an "Error:" prefix for failures.
    assert!(
        !_err.contains("Error") && !out.contains("Error"),
        "TRUNCATE on empty table should succeed; got stdout:\n{}\nstderr:\n{}",
        out,
        _err
    );
}

#[test]
fn test_truncate_table_with_data() {
    let (out, err, _code) = run_repl(
        "CREATE TABLE tr2 (id INT PRIMARY KEY, value INT);\n\
         INSERT INTO tr2 VALUES (1, 100),(2, 200),(3, 300);\n\
         SELECT COUNT(*) FROM tr2;\n\
         TRUNCATE TABLE tr2;\n\
         SELECT COUNT(*) FROM tr2;\n\
         .exit\n",
    );
    let combined = format!("{}{}", out, err);
    // First COUNT must show 3 rows; after TRUNCATE, the second
    // COUNT must show 0 rows. If the parser breaks on TRUNCATE the
    // second COUNT never runs and the output will be missing the
    // expected pattern.
    assert!(
        combined.contains("Integer(3)") || combined.contains("3\n"),
        "Expected pre-TRUNCATE count of 3, got stdout:\n{}\nstderr:\n{}",
        out,
        err
    );
    assert!(
        combined.contains("Integer(0)") || combined.contains("0\n"),
        "Expected post-TRUNCATE count of 0, got stdout:\n{}\nstderr:\n{}",
        out,
        err
    );
}

#[test]
fn test_truncate_and_reinsert() {
    let (out, err, _code) = run_repl(
        "CREATE TABLE tr3 (id INT PRIMARY KEY, name VARCHAR(50));\n\
         INSERT INTO tr3 VALUES (1, 'alice'),(2, 'bob');\n\
         TRUNCATE TABLE tr3;\n\
         INSERT INTO tr3 VALUES (10, 'charlie'),(20, 'david');\n\
         SELECT name FROM tr3 ORDER BY id;\n\
         .exit\n",
    );
    let combined = format!("{}{}", out, err);
    assert!(
        combined.contains("charlie") && combined.contains("david"),
        "After TRUNCATE+re-INSERT, SELECT must show charlie and david; got stdout:\n{}\nstderr:\n{}",
        out,
        err
    );
    // alice/bob should be GONE (truncated).
    assert!(
        !combined.contains("alice") && !combined.contains("bob"),
        "Truncated rows (alice/bob) must not appear; got stdout:\n{}\nstderr:\n{}",
        out,
        err
    );
}

// ============================================================================
// V312-64h / Issue #4762 — TRUNCATE CASCADE / RESTRICT / omitted TABLE keyword
// ============================================================================

#[test]
fn test_truncate_table_cascade() {
    // MySQL/SQLite accept `TRUNCATE TABLE t CASCADE`. sqlrustgo must
    // parse and execute it identically to bare TRUNCATE.
    let (out, err, _code) = run_repl(
        "CREATE TABLE tr_cas (id INT PRIMARY KEY, v INT);\n\
         INSERT INTO tr_cas VALUES (1, 10),(2, 20),(3, 30);\n\
         TRUNCATE TABLE tr_cas CASCADE;\n\
         SELECT COUNT(*) FROM tr_cas;\n\
         .exit\n",
    );
    let combined = format!("{}{}", out, err);
    assert!(
        !combined.contains("Error"),
        "TRUNCATE TABLE tr_cas CASCADE must succeed; got stdout:\n{}\nstderr:\n{}",
        out,
        err
    );
    assert!(
        combined.contains("Integer(0)") || combined.contains("0\n"),
        "post-CASCADE count must be 0; got stdout:\n{}\nstderr:\n{}",
        out,
        err
    );
}

#[test]
fn test_truncate_table_restrict() {
    // MySQL/SQLite accept `TRUNCATE TABLE t RESTRICT`. sqlrustgo
    // must parse it (RESTRICT is recorded but does not gate the
    // truncate yet since FK constraints are not enforced).
    let (out, err, _code) = run_repl(
        "CREATE TABLE tr_res (id INT PRIMARY KEY, v INT);\n\
         INSERT INTO tr_res VALUES (1, 10),(2, 20);\n\
         TRUNCATE TABLE tr_res RESTRICT;\n\
         SELECT COUNT(*) FROM tr_res;\n\
         .exit\n",
    );
    let combined = format!("{}{}", out, err);
    assert!(
        !combined.contains("Error"),
        "TRUNCATE TABLE tr_res RESTRICT must succeed; got stdout:\n{}\nstderr:\n{}",
        out,
        err
    );
    assert!(
        combined.contains("Integer(0)") || combined.contains("0\n"),
        "post-RESTRICT count must be 0; got stdout:\n{}\nstderr:\n{}",
        out,
        err
    );
}

#[test]
fn test_truncate_without_table_keyword() {
    // MySQL accepts `TRUNCATE t` (omitting TABLE keyword). Verify
    // sqlrustgo now parses this form.
    let (out, err, _code) = run_repl(
        "CREATE TABLE tr_notbl (id INT PRIMARY KEY, v INT);\n\
         INSERT INTO tr_notbl VALUES (1, 10),(2, 20),(3, 30);\n\
         TRUNCATE tr_notbl;\n\
         SELECT COUNT(*) FROM tr_notbl;\n\
         .exit\n",
    );
    let combined = format!("{}{}", out, err);
    assert!(
        !combined.contains("Error"),
        "TRUNCATE tr_notbl (no TABLE keyword) must succeed; got stdout:\n{}\nstderr:\n{}",
        out,
        err
    );
    assert!(
        combined.contains("Integer(0)") || combined.contains("0\n"),
        "post-TRUNCATE count must be 0; got stdout:\n{}\nstderr:\n{}",
        out,
        err
    );
}

#[test]
fn test_truncate_without_table_keyword_cascade() {
    // Combined: omit TABLE keyword + add CASCADE.
    let (out, err, _code) = run_repl(
        "CREATE TABLE tr_ntc (id INT PRIMARY KEY, v INT);\n\
         INSERT INTO tr_ntc VALUES (1, 10);\n\
         TRUNCATE tr_ntc CASCADE;\n\
         SELECT COUNT(*) FROM tr_ntc;\n\
         .exit\n",
    );
    let combined = format!("{}{}", out, err);
    assert!(
        !combined.contains("Error"),
        "TRUNCATE tr_ntc CASCADE must succeed; got stdout:\n{}\nstderr:\n{}",
        out,
        err
    );
    assert!(
        combined.contains("Integer(0)") || combined.contains("0\n"),
        "post-TRUNCATE count must be 0; got stdout:\n{}\nstderr:\n{}",
        out,
        err
    );
}
