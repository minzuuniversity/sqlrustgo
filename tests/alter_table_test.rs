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
    // v3.10.0+ added MemoryStorage::drop_column support (see
    // crates/storage/src/engine.rs:961). This test was originally
    // written when DROP COLUMN was a known gap; the assertion that
    // it would surface an error is stale. We now assert the opposite:
    // DROP COLUMN succeeds, and the column is gone from DESC output.
    //
    // The previous "known gap" comment block is preserved below for
    // historical reference; if a future refactor reintroduces the gap
    // (e.g. by removing the MemoryStorage::drop_column impl), this
    // test will start failing with a clearer "expected 'age' to be
    // gone but still in DESC" message instead of a generic panic.
    let (out, _err, _code) = run_repl(
        "CREATE TABLE t2 (id INT PRIMARY KEY, name VARCHAR(50), age INT, email VARCHAR(100));\n\
         ALTER TABLE t2 DROP COLUMN age;\n\
         DESC t2;\n\
         .exit\n",
    );
    // 'age' must be gone from DESC t2 output.
    // Note: DESC output uses Text("col_name") format, so we check
    // for the absence of the literal "age" string. The other 3
    // columns are "id", "name", "email" — distinct from "age".
    assert!(
        !out.contains("age"),
        "DROP COLUMN should remove 'age' from DESC t2, but it's still present:\n{}",
        out
    );
    // The other 3 columns must still be there.
    for col in &["id", "name", "email"] {
        assert!(
            out.contains(col),
            "DESC t2 should still contain '{}' after DROP COLUMN, got:\n{}",
            col,
            out
        );
    }
}
