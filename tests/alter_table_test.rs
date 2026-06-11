//! Integration test: ALTER TABLE ADD/DROP COLUMN
//!
//! Uses the wired `sqlrustgo-mysql-server repl` over stdin (true
//! e2e). `exec` only accepts a single statement per process and
//! starts a fresh MemoryStorage each invocation, so multi-statement
//! DDL testing requires the REPL where state persists across
//! statements.

use std::io::Write;
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
    use std::io::Read;
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
    // Write the whole script in one call so the OS hands it to the
    // child atomically (avoids the REPL seeing EOF before it has
    // finished reading the last SQL line).
    stdin
        .write_all(script.as_bytes())
        .expect("Failed to write to stdin");
    // Shut down the write side so the REPL reads EOF and exits
    // gracefully. We do this on a separate thread because closing
    // stdin while the child is still writing output can race.
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
fn test_alter_table_add_column() {
    let (out, _err, _code) = run_repl(
        "CREATE TABLE t1 (id INT PRIMARY KEY, name VARCHAR(50));\n\
         ALTER TABLE t1 ADD COLUMN age INT;\n\
         DESC t1;\n\
         .exit\n",
    );
    assert!(
        out.contains("age"),
        "DESC t1 should contain 'age' column after ADD COLUMN, got:\n{}",
        out
    );
    assert!(out.contains("id"), "DESC should still show id");
    assert!(out.contains("name"), "DESC should still show name");
}

#[test]
fn test_alter_table_add_multiple_columns() {
    let (out, _err, _code) = run_repl(
        "CREATE TABLE t3 (id INT PRIMARY KEY);\n\
         ALTER TABLE t3 ADD COLUMN col1 INT;\n\
         ALTER TABLE t3 ADD COLUMN col2 VARCHAR(50);\n\
         ALTER TABLE t3 ADD COLUMN col3 DECIMAL(10,2);\n\
         DESC t3;\n\
         .exit\n",
    );
    for col in &["col1", "col2", "col3"] {
        assert!(
            out.contains(col),
            "DESC t3 should contain '{}' after ADD COLUMN, got:\n{}",
            col,
            out
        );
    }
}

#[test]
fn test_alter_table_drop_column() {
    // Known gap: MemoryStorage's `drop_column` returns
    //   "drop_column not supported by this storage engine"
    // (crates/storage/src/engine.rs:511-515). The default
    // StorageEngine impl is `Err(...)`, so no storage backend
    // currently supports DROP COLUMN. Tracked as a v3.9.0 GA
    // blocker for full DDL coverage. The wired REPL must surface
    // this as a SQL error.
    let (out, err, _code) = run_repl(
        "CREATE TABLE t2 (id INT PRIMARY KEY, name VARCHAR(50), age INT, email VARCHAR(100));\n\
         ALTER TABLE t2 DROP COLUMN age;\n\
         .exit\n",
    );
    // REPL must surface the storage error (not silently succeed).
    let combined = format!("{}{}", out, err);
    assert!(
        combined.contains("Error")
            || combined.contains("error")
            || combined.contains("not supported"),
        "DROP COLUMN should surface a storage error in REPL, got stdout:\n{}\nstderr:\n{}",
        out,
        err
    );
}
