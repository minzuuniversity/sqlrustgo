//! Integration test: qualified names (`db.table`) — INSERT 1064 + SHOW CREATE TABLE 1105.
//!
//! Regression tests for two parser defects (found during #5250-#5305 series):
//! - `INSERT INTO db.t` → ERROR 1064 (parse_insert only ate a single Identifier)
//! - `SHOW CREATE TABLE db.t` → ERROR 1105 (parse_show CreateTable branch
//!   treated `db.t` as a whole table name, looked it up in session_db)
//!
//! Uses the wired `sqlrustgo-mysql-server repl` (via the helper pattern from
//! `concurrent_insert_test.rs`) for true e2e verification.

use std::io::{Read, Write};
use std::process::{Command, Stdio};

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_sqlrustgo-mysql-server")
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
    let mut stdout = child.stdout.take().expect("Failed to get stdout");
    let mut stderr = child.stderr.take().expect("Failed to get stderr");
    let stdout_thread = std::thread::spawn(move || {
        let mut buf = String::new();
        let _ = stdout.read_to_string(&mut buf);
        buf
    });
    let stderr_thread = std::thread::spawn(move || {
        let mut buf = String::new();
        let _ = stderr.read_to_string(&mut buf);
        buf
    });
    stdin
        .write_all(script.as_bytes())
        .expect("Failed to write to stdin");
    std::thread::spawn(move || drop(stdin));
    let status = child.wait().expect("Failed to wait for REPL");
    let out = stdout_thread.join().unwrap_or_default();
    let err = stderr_thread.join().unwrap_or_default();
    (out, err, status.code().unwrap_or(-1))
}

#[test]
fn qualified_insert_and_show_create_table_parse_and_execute() {
    let script = "CREATE DATABASE qn_test;\n\
USE qn_test;\n\
CREATE TABLE t(id INT PRIMARY KEY, v TEXT);\n\
INSERT INTO qn_test.t VALUES(1,'a');\n\
SELECT * FROM qn_test.t;\n\
SHOW CREATE TABLE qn_test.t;\n\
.exit\n";
    let (out, err, _code) = run_repl(script);
    let combined = format!("{}\n{}", out, err);
    assert!(
        !combined.contains("1064"),
        "qualified INSERT must not produce 1064: {}",
        combined
    );
    assert!(
        !combined.contains("1105"),
        "qualified SHOW CREATE TABLE must not produce 1105: {}",
        combined
    );
    assert!(
        !combined.contains("does not exist"),
        "qualified SHOW CREATE TABLE must not report table does not exist: {}",
        combined
    );
}

#[test]
fn unqualified_insert_still_works() {
    let script = "CREATE DATABASE qn_unqual;\n\
USE qn_unqual;\n\
CREATE TABLE t(id INT PRIMARY KEY, v TEXT);\n\
INSERT INTO t VALUES(1,'a');\n\
SELECT * FROM t;\n\
.exit\n";
    let (out, err, _code) = run_repl(script);
    let combined = format!("{}\n{}", out, err);
    assert!(
        !combined.contains("1064"),
        "unqualified INSERT must not produce 1064: {}",
        combined
    );
}
