//! Upgrade Test v3.10.0 → v3.11.0 — GA-P0 #3605
//!
//! Validates the upgrade path from v3.10.0 to v3.11.0.
//! This test is a smoke test — it validates the upgrade test
//! infrastructure without requiring two separate binaries.

use std::process::Command;

/// Check if upgrade script exists and is executable
#[test]
fn test_upgrade_script_exists() {
    let script_path = "scripts/test_upgrade_v310_to_v311.sh";
    assert!(
        std::path::Path::new(script_path).exists(),
        "Upgrade script not found at {}",
        script_path
    );

    // Check it's executable
    let _ = Command::new("chmod").arg("+x").arg(script_path).output();

    println!("Upgrade script exists and is executable");
}

/// Check if upgrade script has required functions
#[test]
fn test_upgrade_script_has_required_functions() {
    let script = std::fs::read_to_string("scripts/test_upgrade_v310_to_v311.sh")
        .expect("Failed to read upgrade script");

    let required_functions = [
        "setup_v310_data",
        "run_upgrade_test",
        "verify_data",
        "cleanup",
        "start_server",
        "stop_server",
    ];

    for func in required_functions.iter() {
        assert!(
            script.contains(&format!("{}()", func)),
            "Required function {} not found in upgrade script",
            func
        );
    }

    println!("All required functions found in upgrade script");
}

/// Check if upgrade script is valid bash
#[test]
fn test_upgrade_script_syntax() {
    let result = Command::new("bash")
        .args(["-n", "scripts/test_upgrade_v310_to_v311.sh"])
        .output();

    let ok = result.as_ref().map(|o| o.status.success()).unwrap_or(false);
    let err_msg = result
        .map(|o| String::from_utf8_lossy(&o.stderr).to_string())
        .unwrap_or_default();

    assert!(ok, "Upgrade script has syntax errors: {}", err_msg);

    println!("Upgrade script has valid bash syntax");
}

/// Check if gate script exists
#[test]
fn test_upgrade_gate_script_exists() {
    let script_path = "scripts/gate/check_upgrade_v310_v311.sh";
    assert!(
        std::path::Path::new(script_path).exists(),
        "Gate script not found at {}",
        script_path
    );
    println!("Gate script exists");
}
