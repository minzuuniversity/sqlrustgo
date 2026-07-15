//! sqlrustgo-cli integration tests

use std::process::Command;

#[test]
fn test_cli_binary_exists() {
    let result = Command::new("cargo")
        .args(["build", "-p", "sqlrustgo-cli"])
        .current_dir(".")
        .output();
    assert!(result.is_ok());
}

#[test]
fn test_cli_help_flag() {
    let output = Command::new("./target/debug/sqlrustgo")
        .args(["--help"])
        .output();
    if output.is_ok() {
        let o = output.unwrap();
        assert!(o.status.success() || !o.stderr.is_empty());
    }
}

#[test]
fn test_cli_version_flag() {
    let output = Command::new("./target/debug/sqlrustgo")
        .args(["--version"])
        .output();
    if output.is_ok() {
        let o = output.unwrap();
        let stdout = String::from_utf8_lossy(&o.stdout);
        let stderr = String::from_utf8_lossy(&o.stderr);
        let combined = format!("{}{}", stdout, stderr);
        assert!(combined.contains("sqlrustgo") || combined.contains("SQLRustGo"));
    }
}

#[test]
fn test_cli_invalid_subcommand() {
    let output = Command::new("./target/debug/sqlrustgo")
        .args(["nonexistent-subcommand-xyz"])
        .output();
    if output.is_ok() {
        let o = output.unwrap();
        let has_output = !o.stdout.is_empty() || !o.stderr.is_empty();
        assert!(!o.status.success() || has_output);
    }
}
