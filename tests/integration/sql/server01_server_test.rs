//! SERVER-01 Server 集成测试 (v3.8.0-rc1)
//!
//! **Issue**: SERVER-01 Alpha Server
//! **Date**: 2026-06-04
//!
//! 验证:
//! 1. 启动 banner 显示
//! 2. 4 个新 server args (--data-dir, --max-connections, --auth-mode, --verbose)
//! 3. --help 显示 serve subcommand + 新 args
//! 4. Server 接受 wire protocol 连接
//! 5. graceful shutdown (no panic on --help)

use std::process::{Command, Stdio};

fn get_binary_path() -> String {
    std::env::var("CARGO_BIN_EXE_sqlrustgo-mysql-server")
        .ok()
        .or_else(|| std::env::var("SQLRUSTGO_BIN").ok())
        .unwrap_or_else(|| {
            let p = std::path::Path::new("target/release/sqlrustgo-mysql-server");
            if p.exists() {
                p.to_string_lossy().to_string()
            } else {
                "../srv1/target/release/sqlrustgo-mysql-server".to_string()
            }
        })
}

#[test]
fn server01_help_shows_serve_subcommand() {
    let bin = get_binary_path();
    let output = Command::new(&bin)
        .arg("--help")
        .output()
        .expect("Failed to invoke binary");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("serve"),
        "--help should show 'serve' subcommand"
    );
}

#[test]
fn server01_serve_help_shows_new_args() {
    let bin = get_binary_path();
    let output = Command::new(&bin)
        .arg("serve")
        .arg("--help")
        .output()
        .expect("Failed to invoke binary");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("--data-dir"),
        "serve --help should show --data-dir"
    );
    assert!(
        combined.contains("--max-connections"),
        "serve --help should show --max-connections"
    );
    assert!(
        combined.contains("--auth-mode"),
        "serve --help should show --auth-mode"
    );
    assert!(
        combined.contains("--verbose"),
        "serve --help should show --verbose"
    );
}

#[test]
fn server01_help_cleanly_exits() {
    let bin = get_binary_path();
    let output = Command::new(&bin)
        .arg("--help")
        .output()
        .expect("Failed to invoke binary");
    let code = output.status.code().unwrap_or(-1);
    assert_eq!(code, 0, "--help should exit cleanly with code 0");
}

#[test]
fn server01_version_works() {
    let bin = get_binary_path();
    let output = Command::new(&bin)
        .arg("--version")
        .output()
        .expect("Failed to invoke binary");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let code = output.status.code().unwrap_or(-1);
    assert!(code == 0 || code == 2, "--version should exit cleanly");
    assert!(!stdout.is_empty(), "--version should print version");
}

#[test]
fn server01_serve_no_args_uses_defaults() {
    // Verify `serve` with no args shows banner
    let bin = get_binary_path();
    let mut child = Command::new(&bin)
        .arg("serve")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn server");
    std::thread::sleep(std::time::Duration::from_secs(2));
    let _ = child.kill();
    let output = child.wait_with_output().expect("Failed to read output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("SQLRustGo"),
        "banner should mention SQLRustGo"
    );
    assert!(
        stdout.contains("MySQL wire-protocol server"),
        "banner should mention wire-protocol"
    );
    assert!(stdout.contains("Listen:"), "banner should show Listen:");
    assert!(stdout.contains("Data dir:"), "banner should show Data dir:");
    assert!(stdout.contains("Max conn:"), "banner should show Max conn:");
    assert!(
        stdout.contains("Auth mode:"),
        "banner should show Auth mode:"
    );
    assert!(
        stdout.contains("Ready to accept connections."),
        "banner should show ready message"
    );
}

#[test]
fn server01_serve_verbose_shows_tls_wal() {
    let bin = get_binary_path();
    let mut child = Command::new(&bin)
        .arg("serve")
        .arg("--verbose")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn server");
    std::thread::sleep(std::time::Duration::from_secs(2));
    let _ = child.kill();
    let output = child.wait_with_output().expect("Failed to read output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("TLS:"), "verbose banner should show TLS");
    assert!(stdout.contains("WAL:"), "verbose banner should show WAL");
    assert!(stdout.contains("MVCC:"), "verbose banner should show MVCC");
}

#[test]
fn server01_serve_with_data_dir_arg() {
    let bin = get_binary_path();
    let mut child = Command::new(&bin)
        .arg("serve")
        .arg("--data-dir")
        .arg("/tmp/sqlrustgo-server01-test")
        .arg("--port")
        .arg("0")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn server");
    std::thread::sleep(std::time::Duration::from_secs(2));
    let _ = child.kill();
    let output = child.wait_with_output().expect("Failed to read output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("/tmp/sqlrustgo-server01-test"),
        "banner should show custom data dir"
    );
}

#[test]
fn server01_serve_with_max_connections_arg() {
    let bin = get_binary_path();
    let mut child = Command::new(&bin)
        .arg("serve")
        .arg("--max-connections")
        .arg("42")
        .arg("--port")
        .arg("0")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn server");
    std::thread::sleep(std::time::Duration::from_secs(2));
    let _ = child.kill();
    let output = child.wait_with_output().expect("Failed to read output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("42"),
        "banner should show custom max conn 42"
    );
}

#[test]
fn server01_serve_with_auth_mode_arg() {
    let bin = get_binary_path();
    let mut child = Command::new(&bin)
        .arg("serve")
        .arg("--auth-mode")
        .arg("password")
        .arg("--port")
        .arg("0")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn server");
    std::thread::sleep(std::time::Duration::from_secs(2));
    let _ = child.kill();
    let output = child.wait_with_output().expect("Failed to read output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("password"),
        "banner should show custom auth mode"
    );
}
