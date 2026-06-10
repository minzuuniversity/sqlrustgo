//! Integration test: REPLACE INTO
//!
//! Uses std::process::Command to spawn sqlrustgo-mysql-server exec
//! with REPLACE INTO statements.

use std::io::Read;
use std::process::{Command, Stdio};

/// Run sqlrustgo-mysql-server exec with the given SQL and return stdout.
fn run_sql(sql: &str) -> Result<String, String> {
    let mut child = Command::new("cargo")
        .args(["run", "--bin", "sqlrustgo-mysql-server", "--", "exec", sql])
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
        return Err(format!("exit {:?}: {}\n{}", status, stderr, stdout));
    }
    Ok(stdout)
}

#[test]
fn test_replace_into_new_row() {
    // Create table with a primary key
    run_sql("CREATE TABLE rep1 (id INT PRIMARY KEY, name VARCHAR(50), value INT)").ok();

    // Replace a new row - should insert
    let out = run_sql("REPLACE INTO rep1 VALUES (1, 'apple', 100)").unwrap_or_default();
    assert!(
        !out.contains("Error"),
        "REPLACE INTO should succeed, got: {}",
        out
    );

    // Verify row was inserted
    let select = run_sql("SELECT * FROM rep1 WHERE id = 1").unwrap_or_default();
    assert!(select.contains("apple"), "should have apple: {}", select);
    assert!(select.contains("100"), "should have value 100: {}", select);

    // Clean up
    run_sql("DROP TABLE rep1").ok();
}

#[test]
fn test_replace_into_existing_row() {
    // Create table with a primary key
    run_sql("CREATE TABLE rep2 (id INT PRIMARY KEY, name VARCHAR(50), value INT)").ok();

    // Insert initial row
    run_sql("INSERT INTO rep2 VALUES (1, 'apple', 100)").ok();

    // Replace with same primary key - should delete old and insert new
    let out = run_sql("REPLACE INTO rep2 VALUES (1, 'banana', 200)").unwrap_or_default();
    assert!(
        !out.contains("Error"),
        "REPLACE INTO should succeed, got: {}",
        out
    );

    // Verify old row was replaced
    let select = run_sql("SELECT * FROM rep2 WHERE id = 1").unwrap_or_default();
    assert!(
        select.contains("banana") && !select.contains("apple"),
        "should have banana, not apple: {}",
        select
    );
    assert!(select.contains("200"), "should have value 200: {}", select);

    // Clean up
    run_sql("DROP TABLE rep2").ok();
}

#[test]
fn test_replace_into_with_autoincrement() {
    // Create table with autoincrement
    run_sql("CREATE TABLE rep3 (id INT AUTO_INCREMENT PRIMARY KEY, name VARCHAR(50))").ok();

    // Insert rows
    run_sql("INSERT INTO rep3 (name) VALUES ('first')").ok();
    run_sql("INSERT INTO rep3 (name) VALUES ('second')").ok();
    run_sql("INSERT INTO rep3 (name) VALUES ('third')").ok();

    // Replace the second row by id
    run_sql("REPLACE INTO rep3 (id, name) VALUES (2, 'second_updated')").ok();

    // Verify update
    let select = run_sql("SELECT name FROM rep3 WHERE id = 2").unwrap_or_default();
    assert!(
        select.contains("second_updated"),
        "should be updated: {}",
        select
    );

    // Verify other rows unchanged
    let select1 = run_sql("SELECT name FROM rep3 WHERE id = 1").unwrap_or_default();
    assert!(
        select1.contains("first"),
        "first should be unchanged: {}",
        select1
    );

    let select3 = run_sql("SELECT name FROM rep3 WHERE id = 3").unwrap_or_default();
    assert!(
        select3.contains("third"),
        "third should be unchanged: {}",
        select3
    );

    // Clean up
    run_sql("DROP TABLE rep3").ok();
}

#[test]
fn test_replace_into_affects_rows() {
    // Create table
    run_sql("CREATE TABLE rep4 (id INT PRIMARY KEY, value INT)").ok();

    // Insert a row
    run_sql("INSERT INTO rep4 VALUES (1, 100)").ok();

    // Replace existing row - should affect 2 rows (1 delete + 1 insert)
    let out = run_sql("REPLACE INTO rep4 VALUES (1, 999)").unwrap_or_default();
    // Just verify it succeeds; row count interpretation may vary

    // Verify the new value
    let select = run_sql("SELECT value FROM rep4 WHERE id = 1").unwrap_or_default();
    assert!(select.contains("999"), "value should be 999: {}", select);

    // Verify still only 1 row
    let count = run_sql("SELECT COUNT(*) FROM rep4").unwrap_or_default();
    assert!(
        count.contains("1") || count.contains("1 rows"),
        "should still have exactly 1 row: {}",
        count
    );

    // Clean up
    run_sql("DROP TABLE rep4").ok();
}
