//! CLI-01 REPL 集成测试 (v3.8.0-rc1)
//!
//! **Issue**: CLI-01 Client CLI 补全
//! **Date**: 2026-06-04
//!
//! 验证:
//! 1. 7 个新 dot commands (.tables, .schema, .databases, .version, .timing, .headers, .clear)
//! 2. 列名显示 (.headers on)
//! 3. 执行时间显示 (.timing on)
//! 4. 错误处理一致 (未知 command → error message)
//! 5. 帮助信息更新

use std::io::Write;
use std::process::{Command, Stdio};

/// Run REPL with stdin script
fn run_repl_script(script: &str) -> (String, String, i32) {
    let bin = std::env::var("CARGO_BIN_EXE_sqlrustgo-mysql-server")
        .ok()
        .or_else(|| std::env::var("SQLRUSTGO_BIN").ok())
        .unwrap_or_else(|| {
            let path = std::path::Path::new("target/release/sqlrustgo-mysql-server");
            if path.exists() {
                path.to_string_lossy().to_string()
            } else {
                "../cli1/target/release/sqlrustgo-mysql-server".to_string()
            }
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
fn cli01_help_shows_new_commands() {
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
fn cli01_version_command() {
    let (stdout, _stderr, _code) = run_repl_script(".version\n.exit\n");
    assert!(stdout.contains("v3.8.0"), ".version should show v3.8.0");
    assert!(
        stdout.contains("Strong Beta"),
        ".version should mention Strong Beta"
    );
}

#[test]
fn cli01_timing_on_off() {
    let (stdout, _stderr, _code) = run_repl_script(".timing on\nSELECT 1;\n.timing off\n.exit\n");
    assert!(stdout.contains("Timing enabled"));
    assert!(stdout.contains("Timing disabled"));
}

#[test]
fn cli01_headers_on_off() {
    let (stdout, _stderr, _code) = run_repl_script(".headers on\n.headers off\n.exit\n");
    assert!(stdout.contains("Headers enabled"));
    assert!(stdout.contains("Headers disabled"));
}

#[test]
fn cli01_tables_command_runs() {
    // .tables calls SHOW TABLES - verify it executes (no error)
    let (stdout, stderr, _code) = run_repl_script(".tables\n.exit\n");
    // Should either show tables or empty result
    assert!(
        !stdout.contains("unknown command") && !stderr.contains("unknown command"),
        ".tables should be a valid command"
    );
}

#[test]
fn cli01_schema_requires_table() {
    let (stdout, stderr, _code) = run_repl_script(".schema\n.exit\n");
    assert!(
        stdout.contains("requires a table name") || stderr.contains("requires a table name"),
        ".schema without table should error"
    );
}

#[test]
fn cli01_databases_command_runs() {
    let (stdout, stderr, _code) = run_repl_script(".databases\n.exit\n");
    assert!(
        !stdout.contains("unknown command") && !stderr.contains("unknown command"),
        ".databases should be a valid command"
    );
}

#[test]
fn cli01_clear_command() {
    let (_stdout, _stderr, code) = run_repl_script(".clear\n.exit\n");
    assert!(code == 0, ".clear should exit cleanly");
}

#[test]
fn cli01_unknown_command_errors() {
    let (stdout, stderr, _code) = run_repl_script(".unknown_xyz\n.exit\n");
    assert!(
        stdout.contains("unknown command") || stderr.contains("unknown command"),
        "Unknown command should error"
    );
}

#[test]
fn cli01_timing_shows_time() {
    let (stdout, _stderr, _code) = run_repl_script(".timing on\nSELECT 1;\n.exit\n");
    assert!(
        stdout.contains("Time:") || stdout.contains("ms"),
        "Timing enabled should show execution time"
    );
}
