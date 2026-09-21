//! Integration test: REPLACE INTO
//!
//! Each test issues a single `exec` call containing all SQL statements
//! (CREATE, INSERT/REPLACE, SELECT, DROP) separated by `;`. The CLI
//! `exec` subcommand shares one in-memory engine across these statements
//! (see `exec_multi` in `crates/mysql-server/src/main.rs`), so subsequent
//! SELECTs observe earlier writes. Earlier versions of this file spawned
//! one `run_sql` per statement, which hit a fresh MemoryStorage per
//! spawn and the SELECTs always saw an empty catalog.

use std::io::Read;
use std::process::{Command, Stdio};

/// Run sqlrustgo-mysql-server exec with the given SQL and return stdout.
/// `sql` may contain multiple `;`-separated statements; they share one
/// in-process engine for the duration of this invocation.
///
/// `cargo run` is invoked from `SQLRUSTGO_REPO_ROOT` (the workspace
/// root). We pass `-p sqlrustgo-mysql-server` because the binary lives
/// in a workspace member (`crates/mysql-server`); a bare `--bin` from
/// the workspace root resolves to "no bin target in default-run packages"
/// and the spawned process exits silently, leaving the SELECT assertions
/// with empty output.
fn run_sql(sql: &str) -> Result<String, String> {
    let mut child = Command::new("cargo")
        .args([
            "run",
            "-p",
            "sqlrustgo-mysql-server",
            "--bin",
            "sqlrustgo-mysql-server",
            "--",
            "exec",
            sql,
        ])
        .current_dir(
            std::env::var("SQLRUSTGO_REPO_ROOT")
                .map_err(|_| "SQLRUSTGO_REPO_ROOT env var must be set to repo root".to_string())?,
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn: {}", e))?;

    let mut stdout = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut stdout)
        .map_err(|e| e.to_string())?;

    let mut stderr = String::new();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .map_err(|e| e.to_string())?;

    let status = child.wait().map_err(|e| format!("wait failed: {}", e))?;

    if !status.success() {
        return Err(format!("exit {:?}: {}\nstdout: {}", status, stderr, stdout));
    }
    Ok(stdout)
}

#[test]
fn test_replace_into_new_row() {
    let out = run_sql(
        "CREATE TABLE rep1 (id INT PRIMARY KEY, name VARCHAR(50), value INT);\
         REPLACE INTO rep1 VALUES (1, 'apple', 100);\
         SELECT * FROM rep1 WHERE id = 1;",
    )
    .unwrap_or_default();
    assert!(
        !out.contains("Error"),
        "REPLACE INTO should succeed, got: {}",
        out
    );
    assert!(out.contains("apple"), "should have apple: {}", out);
    assert!(out.contains("100"), "should have value 100: {}", out);
}

#[test]
fn test_replace_into_existing_row() {
    let out = run_sql(
        "CREATE TABLE rep2 (id INT PRIMARY KEY, name VARCHAR(50), value INT);\
         INSERT INTO rep2 VALUES (1, 'apple', 100);\
         REPLACE INTO rep2 VALUES (1, 'banana', 200);\
         SELECT * FROM rep2 WHERE id = 1;",
    )
    .unwrap_or_default();
    assert!(
        !out.contains("Error"),
        "REPLACE INTO should succeed, got: {}",
        out
    );
    assert!(
        out.contains("banana") && !out.contains("apple"),
        "should have banana, not apple: {}",
        out
    );
    assert!(out.contains("200"), "should have value 200: {}", out);
}

#[test]
fn test_replace_into_with_autoincrement() {
    let out = run_sql(
        "CREATE TABLE rep3 (id INT AUTO_INCREMENT PRIMARY KEY, name VARCHAR(50));\
         INSERT INTO rep3 (name) VALUES ('first');\
         INSERT INTO rep3 (name) VALUES ('second');\
         INSERT INTO rep3 (name) VALUES ('third');\
         REPLACE INTO rep3 (id, name) VALUES (2, 'second_updated');\
         SELECT name FROM rep3 WHERE id = 2;\
         SELECT name FROM rep3 WHERE id = 1;\
         SELECT name FROM rep3 WHERE id = 3;",
    )
    .unwrap_or_default();
    assert!(
        !out.contains("Error"),
        "REPLACE/SELECT chain should succeed, got: {}",
        out
    );
    assert!(
        out.contains("second_updated"),
        "id=2 should be updated: {}",
        out
    );
    assert!(out.contains("first"), "id=1 unchanged: {}", out);
    assert!(out.contains("third"), "id=3 unchanged: {}", out);
}

#[test]
fn test_replace_into_affects_rows() {
    let out = run_sql(
        "CREATE TABLE rep4 (id INT PRIMARY KEY, value INT);\
         INSERT INTO rep4 VALUES (1, 100);\
         REPLACE INTO rep4 VALUES (1, 999);\
         SELECT value FROM rep4 WHERE id = 1;\
         SELECT COUNT(*) FROM rep4;",
    )
    .unwrap_or_default();
    assert!(
        !out.contains("Error"),
        "replace/select chain should succeed: {}",
        out
    );
    assert!(
        out.contains("999"),
        "value should be 999 after REPLACE: {}",
        out
    );
    // COUNT(*) must report exactly 1 — REPLACE is delete+insert, so the
    // row count is unchanged. The exact format depends on the value
    // formatter; check for a digit 1 surrounded by boundaries that
    // distinguish it from `id = 1` substrings.
    assert!(
        out.contains(" 1\n")
            || out.contains(" 1 |")
            || out.contains("(1 row")
            || out.contains("= 1\n"),
        "expected COUNT(*) = 1 in output: {}",
        out
    );
}
