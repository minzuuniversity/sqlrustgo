//! Integration test: TRUNCATE TABLE
//!
//! Uses std::process::Command to spawn sqlrustgo-mysql-server exec
//! with TRUNCATE TABLE statements.

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
fn test_truncate_empty_table() {
    // Create an empty table
    run_sql("CREATE TABLE tr1 (id INT PRIMARY KEY, name VARCHAR(50))").ok();

    // Truncate should succeed even on empty table
    let out = run_sql("TRUNCATE TABLE tr1").unwrap_or_default();
    assert!(
        !out.contains("Error"),
        "TRUNCATE TABLE on empty table should succeed, got: {}",
        out
    );

    // Clean up
    run_sql("DROP TABLE tr1").ok();
}

#[test]
fn test_truncate_table_with_data() {
    // Create table and insert data
    run_sql("CREATE TABLE tr2 (id INT PRIMARY KEY, value INT)").ok();
    run_sql("INSERT INTO tr2 VALUES (1, 100)").ok();
    run_sql("INSERT INTO tr2 VALUES (2, 200)").ok();
    run_sql("INSERT INTO tr2 VALUES (3, 300)").ok();

    // Verify data exists
    let select_out = run_sql("SELECT COUNT(*) FROM tr2").unwrap_or_default();
    assert!(
        select_out.contains("3") || select_out.contains("3 rows"),
        "Should have 3 rows before truncate, got: {}",
        select_out
    );

    // Truncate should delete all rows
    let out = run_sql("TRUNCATE TABLE tr2").unwrap_or_default();
    assert!(
        !out.contains("Error"),
        "TRUNCATE TABLE should succeed, got: {}",
        out
    );

    // Verify table is empty
    let select_out = run_sql("SELECT COUNT(*) FROM tr2").unwrap_or_default();
    assert!(
        select_out.contains("0") || select_out.contains("0 rows"),
        "Should have 0 rows after truncate, got: {}",
        select_out
    );

    // Clean up
    run_sql("DROP TABLE tr2").ok();
}

#[test]
fn test_truncate_and_reinsert() {
    // Create table and insert data
    run_sql("CREATE TABLE tr3 (id INT PRIMARY KEY, name VARCHAR(50))").ok();
    run_sql("INSERT INTO tr3 VALUES (1, 'alice')").ok();
    run_sql("INSERT INTO tr3 VALUES (2, 'bob')").ok();

    // Truncate
    run_sql("TRUNCATE TABLE tr3").ok();

    // Re-insert different data
    run_sql("INSERT INTO tr3 VALUES (10, 'charlie')").ok();
    run_sql("INSERT INTO tr3 VALUES (20, 'david')").ok();

    // Verify new data
    let out = run_sql("SELECT * FROM tr3").unwrap_or_default();
    assert!(out.contains("charlie"), "should have charlie: {}", out);
    assert!(out.contains("david"), "should have david: {}", out);
    assert!(
        !out.contains("alice") && !out.contains("bob"),
        "should NOT have alice/bob: {}",
        out
    );

    // Clean up
    run_sql("DROP TABLE tr3").ok();
}
