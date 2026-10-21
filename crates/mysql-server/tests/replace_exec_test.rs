//! Integration test: REPLACE INTO through the CLI `exec` subcommand.
//!
//! Why this file lives in `crates/mysql-server/tests/`
//! -----------------------------------------------------------
//! Cargo only sets `CARGO_BIN_EXE_<name>` for integration tests of the
//! package that *owns* the binary. The server binary belongs to
//! `sqlrustgo-mysql-server`, so from the root package the variable is
//! absent.
//!
//! The previous version of this test lived in the root package at
//! `tests/integration/dml/replace_test.rs` and worked around that by
//! shelling out to `cargo run` and locating the workspace through the
//! `SQLRUSTGO_REPO_ROOT` environment variable. Nothing in the
//! repository or in CI ever set that variable — `.gitea/workflows/ci.yml`
//! runs `cargo test --all-features --no-fail-fast` with no such export
//! — so `run_sql` returned `Err` on every CI run.
//!
//! The failure was then swallowed: `.unwrap_or_default()` turned the
//! error into an empty string, and the assertion then reported
//! "should have apple: " instead of the real cause ("env var must be
//! set"). That is issue #5431 — a test-layer defect, not an engine
//! defect: `REPLACE INTO` itself was correct all along.
//!
//! `env!` makes the binary location a compile-time guarantee: no
//! environment variable, no `cargo run` nested inside `cargo test`, no
//! workspace-root guesswork, and a spawn failure is now a loud panic
//! with the real cause instead of a silent empty result.
//!
//! The `exec` subcommand shares one in-memory engine across the
//! `;`-separated statements of a single invocation (`exec_multi` in
//! `crates/mysql-server/src/main.rs`), so a later SELECT observes the
//! earlier writes.

use std::io::Read;
use std::process::{Command, Stdio};

/// Run `sqlrustgo-mysql-server exec <sql>` and return its stdout.
///
/// `sql` may contain multiple `;`-separated statements; they share one
/// in-process engine for the duration of this invocation.
///
/// A non-zero exit status is an error carrying stdout **and** stderr:
/// the CLI prints `Error: {e}` on stderr and exits 1 when a statement
/// fails, so swallowing that would hide a genuine REPLACE INTO defect.
fn run_exec(sql: &str) -> Result<String, String> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_sqlrustgo-mysql-server"))
        .args(["exec", sql])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn sqlrustgo-mysql-server: {}", e))?;

    let mut stdout = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut stdout)
        .map_err(|e| format!("read stdout: {}", e))?;

    let mut stderr = String::new();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .map_err(|e| format!("read stderr: {}", e))?;

    let status = child.wait().map_err(|e| format!("wait failed: {}", e))?;

    if !status.success() {
        return Err(format!(
            "sqlrustgo-mysql-server exec exited with {:?}\nstdout: {}\nstderr: {}",
            status, stdout, stderr
        ));
    }
    Ok(stdout)
}

/// Run `sql` and return its stdout, failing the test loudly on error.
///
/// #5431: the previous helper returned `Err` and the callers turned
/// that into `unwrap_or_default()` — an empty string that made every
/// assertion fail with a message that named the symptom ("should have
/// apple") instead of the cause. A test that cannot run must say so.
fn exec_ok(sql: &str) -> String {
    match run_exec(sql) {
        Ok(out) => out,
        Err(e) => panic!("REPLACE INTO exec failed:\n{}", e),
    }
}

/// Assert the exec output carries the given row in the CLI's row format.
///
/// `exec` prints each row as the `Debug` rendering of its cells joined
/// with " | " (see `exec_with_engine_and_options` in
/// `crates/mysql-server/src/main.rs`), so a row is matched exactly
/// rather than by a loose substring. A substring check would accept
/// `Integer(100)` as evidence for a `Integer(1000)` row.
fn assert_has_row(out: &str, row: &str) {
    assert!(
        out.lines().any(|l| l.trim() == row),
        "expected a row exactly equal to {:?}; got:\n{}",
        row,
        out
    );
}

#[test]
fn test_replace_into_new_row() {
    let out = exec_ok(
        "CREATE TABLE rep1 (id INT PRIMARY KEY, name VARCHAR(50), value INT);\
         REPLACE INTO rep1 VALUES (1, 'apple', 100);\
         SELECT * FROM rep1 WHERE id = 1;",
    );
    assert!(
        !out.contains("Error"),
        "REPLACE INTO should succeed, got: {}",
        out
    );
    assert_has_row(&out, "Integer(1) | Text(\"apple\") | Integer(100)");
}

#[test]
fn test_replace_into_existing_row() {
    let out = exec_ok(
        "CREATE TABLE rep2 (id INT PRIMARY KEY, name VARCHAR(50), value INT);\
         INSERT INTO rep2 VALUES (1, 'apple', 100);\
         REPLACE INTO rep2 VALUES (1, 'banana', 200);\
         SELECT * FROM rep2 WHERE id = 1;",
    );
    assert!(
        !out.contains("Error"),
        "REPLACE INTO should succeed, got: {}",
        out
    );
    // REPLACE is delete + insert: the old row must be gone entirely,
    // not merged with the new one.
    assert_has_row(&out, "Integer(1) | Text(\"banana\") | Integer(200)");
    assert!(
        !out.contains("apple"),
        "the replaced row must be gone, got: {}",
        out
    );
}

#[test]
fn test_replace_into_with_autoincrement() {
    let out = exec_ok(
        "CREATE TABLE rep3 (id INT AUTO_INCREMENT PRIMARY KEY, name VARCHAR(50));\
         INSERT INTO rep3 (name) VALUES ('first');\
         INSERT INTO rep3 (name) VALUES ('second');\
         INSERT INTO rep3 (name) VALUES ('third');\
         REPLACE INTO rep3 (id, name) VALUES (2, 'second_updated');\
         SELECT name FROM rep3 WHERE id = 2;\
         SELECT name FROM rep3 WHERE id = 1;\
         SELECT name FROM rep3 WHERE id = 3;",
    );
    assert!(
        !out.contains("Error"),
        "REPLACE/SELECT chain should succeed, got: {}",
        out
    );
    assert_has_row(&out, "Text(\"second_updated\")");
    assert_has_row(&out, "Text(\"first\")");
    assert_has_row(&out, "Text(\"third\")");
}

#[test]
fn test_replace_into_affects_rows() {
    let out = exec_ok(
        "CREATE TABLE rep4 (id INT PRIMARY KEY, value INT);\
         INSERT INTO rep4 VALUES (1, 100);\
         REPLACE INTO rep4 VALUES (1, 999);\
         SELECT value FROM rep4 WHERE id = 1;\
         SELECT COUNT(*) FROM rep4;",
    );
    assert!(
        !out.contains("Error"),
        "replace/select chain should succeed: {}",
        out
    );
    assert_has_row(&out, "Integer(999)");
    // COUNT(*) must report exactly 1 — REPLACE is delete+insert, so the
    // row count is unchanged. Matching the whole row keeps the check
    // from accepting `Integer(10)` for a table that holds 10 rows.
    assert_has_row(&out, "Integer(1)");
}

/// #5431 regression guard: the `exec` subcommand must run every
/// `;`-separated statement against ONE engine, so a SELECT placed after
/// a REPLACE INTO observes the replaced row.
///
/// The previous version of this test asserted the same thing indirectly
/// by shelling out to `cargo run`; this variant keeps the assertion but
/// drops the external dependency that made the test fail on CI.
#[test]
fn test_exec_shares_one_engine_across_statements() {
    let out = exec_ok(
        "CREATE TABLE rep5 (id INT PRIMARY KEY, value INT);\
         INSERT INTO rep5 VALUES (1, 10);\
         REPLACE INTO rep5 VALUES (1, 20);\
         SELECT value FROM rep5 WHERE id = 1;",
    );
    assert_has_row(&out, "Integer(20)");
    assert!(
        !out.contains("Integer(10)"),
        "the pre-REPLACE value must not survive, got: {}",
        out
    );
}
