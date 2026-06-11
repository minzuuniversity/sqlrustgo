//! Integration test: INSERT ... ON DUPLICATE KEY UPDATE
//!
//! Uses the wired `sqlrustgo-mysql-server repl` over stdin (true e2e).
//! `exec` only accepts a single statement per process and starts a
//! fresh MemoryStorage each invocation, so multi-statement ODKU
//! testing requires the REPL where state persists across statements.

use std::io::{Read, Write};
use std::process::{Command, Stdio};

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
fn test_insert_odku_new_row() {
    let (out, err, _code) = run_repl(
        "CREATE TABLE odku1 (id INT PRIMARY KEY, name VARCHAR(50), value INT);\n\
         INSERT INTO odku1 VALUES (1, 'apple', 100);\n\
         SELECT * FROM odku1 WHERE id = 1;\n\
         .exit\n",
    );
    let combined = format!("{}{}", out, err);
    assert!(
        combined.contains("apple"),
        "should have apple; got:\n{}",
        combined
    );
    assert!(
        combined.contains("100") || combined.contains("Integer(100)"),
        "should have value 100; got:\n{}",
        combined
    );
}

#[test]
fn test_insert_odku_duplicate_key_update() {
    let (out, err, _code) = run_repl(
        "CREATE TABLE odku2 (id INT PRIMARY KEY, name VARCHAR(50), value INT);\n\
         INSERT INTO odku2 VALUES (1, 'apple', 100);\n\
         INSERT INTO odku2 VALUES (1, 'apple', 100) ON DUPLICATE KEY UPDATE value = 200;\n\
         SELECT * FROM odku2 WHERE id = 1;\n\
         .exit\n",
    );
    let combined = format!("{}{}", out, err);
    assert!(
        combined.contains("200") || combined.contains("Integer(200)"),
        "value should be 200 after ODKU update; got:\n{}",
        combined
    );
    // Old value 100 should NOT appear (replaced by 200).
    assert!(
        !combined.contains("Integer(100)"),
        "old value 100 must be replaced; got:\n{}",
        combined
    );
}

#[test]
fn test_insert_odku_multiple_rows() {
    let (out, err, _code) = run_repl(
        "CREATE TABLE odku3 (id INT PRIMARY KEY, name VARCHAR(50), value INT);\n\
         INSERT INTO odku3 VALUES (1, 'one', 1), (2, 'two', 2), (3, 'three', 3);\n\
         INSERT INTO odku3 VALUES (2, 'two', 999) ON DUPLICATE KEY UPDATE value = 999;\n\
         SELECT name FROM odku3 ORDER BY id;\n\
         .exit\n",
    );
    let combined = format!("{}{}", out, err);
    for name in &["one", "two", "three"] {
        assert!(
            combined.contains(name),
            "should have {}; got:\n{}",
            name,
            combined
        );
    }
    assert!(
        combined.contains("999") || combined.contains("Integer(999)"),
        "should have 999 for two; got:\n{}",
        combined
    );
}

#[test]
fn test_insert_odku_affects_rows() {
    let (out, err, _code) = run_repl(
        "CREATE TABLE odku4 (id INT PRIMARY KEY, value INT);\n\
         INSERT INTO odku4 VALUES (1, 100), (2, 200);\n\
         SELECT COUNT(*) FROM odku4;\n\
         INSERT INTO odku4 VALUES (1, 999) ON DUPLICATE KEY UPDATE value = 999;\n\
         SELECT COUNT(*) FROM odku4;\n\
         .exit\n",
    );
    let combined = format!("{}{}", out, err);
    // Both COUNT must show 2 (ODKU must not insert new row on duplicate).
    assert!(
        combined.matches("Integer(2)").count() >= 2,
        "Both COUNT must show Integer(2); got:\n{}",
        combined
    );
}
