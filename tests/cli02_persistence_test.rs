//! CLI-01 Stage 2 集成测试 (v3.8.0-rc1)
//!
//! **Issue**: CLI-01 REPL 持久化 (Stage 2)
//! **Date**: 2026-06-04
//!
//! 验证:
//! 1. CREATE → INSERT → SELECT 在同一 REPL session 共享 state
//! 2. .help 显示全部 13 个 dot commands (Stage 1 修复)
//! 3. .version 在 persistence REPL 中仍可工作
//! 4. .timing on 在 persistence REPL 中仍生效
//! 5. 列名显示 (col_0) 在 persistence REPL 中仍生效
//! 6. 多次 SELECT 持续共享 state
//! 7. Session 间不泄漏 (新 session 新 engine)

use std::io::Write;
use std::process::{Command, Stdio};

fn run_repl_script(script: &str) -> (String, String, i32) {
    // cargo test sets CARGO_BIN_EXE_<name> for integration tests; use it
    // first so tests find the binary regardless of cwd.
    let bin = std::env::var("CARGO_BIN_EXE_sqlrustgo-mysql-server")
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
        });

    let mut child = Command::new(&bin)
        .arg("repl")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn REPL");

    {
        let stdin = child.stdin.as_mut().expect("Failed to get stdin");
        stdin
            .write_all(script.as_bytes())
            .expect("Failed to write to stdin");
    }
    let output = child.wait_with_output().expect("Failed to read output");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(-1);
    (stdout, stderr, code)
}

#[test]
fn cli02_persistence_create_insert_select() {
    // Stage 2 KEY: CREATE → INSERT → SELECT must share state
    let (stdout, _stderr, _code) = run_repl_script(
        "CREATE TABLE cli02_p (id INT);\nINSERT INTO cli02_p VALUES (1);\nSELECT * FROM cli02_p;\n.exit\n",
    );
    assert!(
        stdout.contains("(1 rows)"),
        "SELECT after INSERT should return 1 row (got: {:?})",
        &stdout[..stdout.len().min(500)]
    );
    assert!(stdout.contains("Integer(1)"), "SELECT should show value 1");
}

#[test]
fn cli02_persistence_multiple_inserts() {
    let (stdout, _stderr, _code) = run_repl_script(
        "CREATE TABLE cli02_m (n INT);\nINSERT INTO cli02_m VALUES (10);\nINSERT INTO cli02_m VALUES (20);\nINSERT INTO cli02_m VALUES (30);\nSELECT * FROM cli02_m;\n.exit\n",
    );
    assert!(
        stdout.contains("(3 rows)"),
        "should have 3 rows after 3 inserts"
    );
    assert!(stdout.contains("Integer(10)"));
    assert!(stdout.contains("Integer(20)"));
    assert!(stdout.contains("Integer(30)"));
}

#[test]
fn cli02_persistence_select_twice() {
    let (stdout, _stderr, _code) = run_repl_script(
        "CREATE TABLE cli02_s (x TEXT);\nINSERT INTO cli02_s VALUES ('a');\nSELECT * FROM cli02_s;\nSELECT * FROM cli02_s;\n.exit\n",
    );
    let select_count = stdout.matches("(1 rows)").count();
    assert!(
        select_count >= 2,
        "Should have at least 2 SELECTs with (1 rows), got {}",
        select_count
    );
}

#[test]
fn cli02_persistence_columns_displayed() {
    let (stdout, _stderr, _code) = run_repl_script(
        "CREATE TABLE cli02_c (a INT, b TEXT);\nINSERT INTO cli02_c VALUES (1, 'hi');\nSELECT * FROM cli02_c;\n.exit\n",
    );
    assert!(stdout.contains("col_0"), "headers should be col_0");
    assert!(stdout.contains("Integer(1)"));
    assert!(stdout.contains("Text(\"hi\")"));
}

#[test]
fn cli02_help_shows_all_dot_commands() {
    // Stage 2 fix: .help now shows all 13 dot commands
    let (stdout, _stderr, _code) = run_repl_script(".help\n.exit\n");
    assert!(stdout.contains(".tables"), ".help should show .tables");
    assert!(stdout.contains(".schema"), ".help should show .schema");
    assert!(
        stdout.contains(".databases"),
        ".help should show .databases"
    );
    assert!(stdout.contains(".version"), ".help should show .version");
    assert!(stdout.contains(".timing"), ".help should show .timing");
    assert!(stdout.contains(".headers"), ".help should show .headers");
    assert!(stdout.contains(".clear"), ".help should show .clear");
}

#[test]
fn cli02_persistence_timing_still_works() {
    let (stdout, _stderr, _code) =
        run_repl_script(".timing on\nCREATE TABLE cli02_t (id INT);\n.exit\n");
    assert!(
        stdout.contains("Timing enabled"),
        ".timing on should print Timing enabled"
    );
}

#[test]
fn cli02_persistence_no_state_leak_between_sessions() {
    // Two REPL sessions should NOT share state
    let (stdout1, _, _) = run_repl_script(
        "CREATE TABLE cli02_iso (x INT);\nINSERT INTO cli02_iso VALUES (99);\nSELECT * FROM cli02_iso;\n.exit\n",
    );
    assert!(stdout1.contains("(1 rows)"), "session 1 should have 1 row");
    // Session 2: same table name should not exist (new engine = empty)
    let (stdout2, _stderr, _code) = run_repl_script("SELECT * FROM cli02_iso;\n.exit\n");
    assert!(
        !stdout2.contains("Integer(99)"),
        "Session 2 should NOT see session 1's data"
    );
}
