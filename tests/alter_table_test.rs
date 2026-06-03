//! Integration test: ALTER TABLE ADD/DROP COLUMN
//!
//! Uses std::process::Command to spawn sqlrustgo-mysql-server exec
//! with ALTER TABLE statements.

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
fn test_alter_table_add_column() {
    // Create table first
    run_sql("CREATE TABLE t1 (id INT PRIMARY KEY, name VARCHAR(50))").ok();

    // Add a new column
    let out = run_sql("ALTER TABLE t1 ADD COLUMN age INT").unwrap_or_default();
    // Should succeed (empty output on success, or rows affected message)
    assert!(
        !out.contains("Error"),
        "ALTER TABLE ADD COLUMN should succeed, got: {}",
        out
    );

    // Verify the column exists by describing the table
    let desc = run_sql("DESC t1").unwrap_or_default();
    assert!(
        desc.contains("age"),
        "DESC t1 should contain 'age' column, got: {}",
        desc
    );

    // Clean up
    run_sql("DROP TABLE t1").ok();
}

#[test]
fn test_alter_table_drop_column() {
    // Create table with multiple columns
    run_sql("CREATE TABLE t2 (id INT PRIMARY KEY, name VARCHAR(50), age INT, email VARCHAR(100))")
        .ok();

    // Drop the 'age' column
    let out = run_sql("ALTER TABLE t2 DROP COLUMN age").unwrap_or_default();
    assert!(
        !out.contains("Error"),
        "ALTER TABLE DROP COLUMN should succeed, got: {}",
        out
    );

    // Verify the column is gone
    let desc = run_sql("DESC t2").unwrap_or_default();
    assert!(
        !desc.contains("age"),
        "DESC t2 should NOT contain 'age' column after DROP, got: {}",
        desc
    );

    // Clean up
    run_sql("DROP TABLE t2").ok();
}

#[test]
fn test_alter_table_add_multiple_columns() {
    // Create table
    run_sql("CREATE TABLE t3 (id INT PRIMARY KEY)").ok();

    // Add multiple columns
    run_sql("ALTER TABLE t3 ADD COLUMN col1 INT").ok();
    run_sql("ALTER TABLE t3 ADD COLUMN col2 VARCHAR(50)").ok();
    run_sql("ALTER TABLE t3 ADD COLUMN col3 DECIMAL(10,2)").ok();

    let desc = run_sql("DESC t3").unwrap_or_default();
    assert!(desc.contains("col1"), "should have col1: {}", desc);
    assert!(desc.contains("col2"), "should have col2: {}", desc);
    assert!(desc.contains("col3"), "should have col3: {}", desc);

    // Clean up
    run_sql("DROP TABLE t3").ok();
}
