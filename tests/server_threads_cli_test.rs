//! CLI validation tests: --server-threads parameter range/type validation
//!
//! Refs: docs/superpowers/specs/2026-06-26-soak-1h-server-threads-design.md §6.2

use std::process::{Command, Stdio};
use std::time::Duration;

fn get_binary_path() -> String {
    std::env::var("CARGO_BIN_EXE_sqlrustgo-mysql-server")
        .ok()
        .or_else(|| std::env::var("SQLRUSTGO_BIN").ok())
        .unwrap_or_else(|| {
            let p = std::path::Path::new("target/release/sqlrustgo-mysql-server");
            if p.exists() {
                p.to_string_lossy().to_string()
            } else {
                "target/debug/sqlrustgo-mysql-server".to_string()
            }
        })
}

#[test]
fn server_threads_help_shows_flag() {
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
        combined.contains("--server-threads"),
        "serve --help should mention --server-threads; got:\n{combined}"
    );
}

#[test]
fn server_threads_default_is_16() {
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
        combined.contains("16") && combined.contains("server-threads"),
        "default should be 16; got:\n{combined}"
    );
}

#[test]
fn server_threads_rejects_81() {
    let bin = get_binary_path();
    let output = Command::new(&bin)
        .arg("serve")
        .arg("--server-threads")
        .arg("81")
        .output()
        .expect("Failed to invoke binary");
    assert!(
        !output.status.success(),
        "--server-threads 81 should fail with exit != 0"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("80") || stderr.contains("invalid"),
        "stderr should mention the limit; got: {stderr}"
    );
}

#[test]
fn server_threads_rejects_non_integer() {
    let bin = get_binary_path();
    let output = Command::new(&bin)
        .arg("serve")
        .arg("--server-threads")
        .arg("abc")
        .output()
        .expect("Failed to invoke binary");
    assert!(
        !output.status.success(),
        "--server-threads abc should fail"
    );
}

#[test]
fn server_threads_accepts_80() {
    let bin = get_binary_path();
    let mut child = Command::new(&bin)
        .arg("serve")
        .arg("--server-threads")
        .arg("80")
        .arg("--port")
        .arg("13399")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to spawn");
    std::thread::sleep(Duration::from_millis(300));
    let _ = child.kill();
    let _ = child.wait();
}
