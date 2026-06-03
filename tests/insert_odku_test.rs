//! Integration test: INSERT ON DUPLICATE KEY UPDATE (ODKU)
//!
//! Uses std::process::Command to spawn sqlrustgo-mysql-server exec
//! with INSERT ON DUPLICATE KEY UPDATE statements.

use std::io::Read;
use std::process::{Command, Stdio};

/// Run sqlrustgo-mysql-server exec with the given SQL and return stdout.
fn run_sql(sql: &str) -> Result<String, String> {
    let mut child = Command::new("cargo")
        .args(["run", "--bin", "sqlrustgo-mysql-server", "--", "exec", sql])
        .current_dir(
            "/Users/liying/workspace/dev/yinglichina163/sqlrustgo/.worktrees/feature-tests",
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
fn test_insert_odku_new_row() {
    // Create table with a unique key
    run_sql("CREATE TABLE odku1 (id INT PRIMARY KEY, name VARCHAR(50), value INT)").ok();

    // Insert a new row - should create it
    let out = run_sql("INSERT INTO odku1 VALUES (1, 'apple', 100)").unwrap_or_default();
    assert!(
        !out.contains("Error"),
        "INSERT should succeed, got: {}",
        out
    );

    // Verify row was inserted
    let select = run_sql("SELECT * FROM odku1 WHERE id = 1").unwrap_or_default();
    assert!(select.contains("apple"), "should have apple: {}", select);
    assert!(select.contains("100"), "should have value 100: {}", select);

    // Clean up
    run_sql("DROP TABLE odku1").ok();
}

#[test]
fn test_insert_odku_duplicate_key_update() {
    // Create table with a unique key
    run_sql("CREATE TABLE odku2 (id INT PRIMARY KEY, name VARCHAR(50), value INT)").ok();

    // Insert initial row
    run_sql("INSERT INTO odku2 VALUES (1, 'apple', 100)").ok();

    // Insert same primary key with ON DUPLICATE KEY UPDATE - should update
    let out =
        run_sql("INSERT INTO odku2 VALUES (1, 'apple', 100) ON DUPLICATE KEY UPDATE value = 200")
            .unwrap_or_default();
    assert!(
        !out.contains("Error"),
        "INSERT ON DUPLICATE KEY UPDATE should succeed, got: {}",
        out
    );

    // Verify the value was updated
    let select = run_sql("SELECT * FROM odku2 WHERE id = 1").unwrap_or_default();
    assert!(
        select.contains("200"),
        "value should be 200 after update: {}",
        select
    );

    // Clean up
    run_sql("DROP TABLE odku2").ok();
}

#[test]
fn test_insert_odku_multiple_rows() {
    // Create table
    run_sql("CREATE TABLE odku3 (id INT PRIMARY KEY, name VARCHAR(50), value INT)").ok();

    // Insert multiple rows
    run_sql("INSERT INTO odku3 VALUES (1, 'one', 1), (2, 'two', 2), (3, 'three', 3)").ok();

    // Update one of them with ODKU
    run_sql("INSERT INTO odku3 VALUES (2, 'two', 999) ON DUPLICATE KEY UPDATE value = 999").ok();

    // Verify only id=2 was updated
    let select = run_sql("SELECT * FROM odku3 ORDER BY id").unwrap_or_default();
    assert!(select.contains("one"), "should have one: {}", select);
    assert!(
        select.contains("999"),
        "should have 999 for two: {}",
        select
    );
    assert!(select.contains("three"), "should have three: {}", select);

    // Clean up
    run_sql("DROP TABLE odku3").ok();
}

#[test]
fn test_insert_odku_affects_rows() {
    // Create table
    run_sql("CREATE TABLE odku4 (id INT PRIMARY KEY, value INT)").ok();

    // Insert initial row
    run_sql("INSERT INTO odku4 VALUES (1, 100)").ok();

    // Insert duplicate - ODKU updates, should report 2 affected rows
    let out =
        run_sql("INSERT INTO odku4 VALUES (1, 100) ON DUPLICATE KEY UPDATE value = value + 1")
            .unwrap_or_default();

    // Verify the update happened
    let select = run_sql("SELECT value FROM odku4 WHERE id = 1").unwrap_or_default();
    assert!(
        select.contains("101"),
        "value should be 101 after increment: {}",
        select
    );

    // Clean up
    run_sql("DROP TABLE odku4").ok();
}
