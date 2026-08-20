//! Chaos SOAK Test — GA-P0 #3604
//!
//! Chaos engineering validation for SOAK testing.
//! Verifies system recovery under fault injection:
//! - I/O latency (tc qdisc)
//! - Memory pressure (stress-ng)
//! - Process kill (kill -9)
//!
//! This test is a smoke test — it validates the chaos injection
//! infrastructure without requiring a live server. Full chaos
//! validation happens in the SOAK driver integration.

use std::process::Command;

/// Check if running on Linux (required for tc qdisc and stress-ng)
fn is_linux() -> bool {
    std::env::consts::OS == "linux"
}

/// Check if a command exists in PATH
fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if running as root (needed for tc qdisc)
fn is_root() -> bool {
    Command::new("id")
        .arg("-u")
        .output()
        .map(|o| o.status.success() && String::from_utf8_lossy(&o.stdout).trim() == "0")
        .unwrap_or(false)
}

/// Chaos experiment result
#[derive(Debug)]
struct ChaosExperimentResult {
    name: String,
    success: bool,
    message: String,
}

/// Test 1: I/O Latency Recovery
///
/// Verifies that I/O latency injection via tc qdisc can be applied
/// and cleaned up correctly. On non-Linux or non-root, this is a no-op.
#[test]
fn test_chaos_io_latency_injectable() {
    let result = if !is_linux() {
        ChaosExperimentResult {
            name: "io_latency".into(),
            success: true,
            message: "skipped: Linux only".into(),
        }
    } else if !is_root() {
        ChaosExperimentResult {
            name: "io_latency".into(),
            success: true,
            message: "skipped: requires root".into(),
        }
    } else if !command_exists("tc") {
        ChaosExperimentResult {
            name: "io_latency".into(),
            success: true,
            message: "skipped: tc not installed".into(),
        }
    } else {
        // Try to apply and immediately remove latency on loopback
        let add_result = Command::new("sudo")
            .args([
                "tc", "qdisc", "add", "dev", "lo", "root", "netem", "delay", "100ms",
            ])
            .output();

        let cleanup_result = Command::new("sudo")
            .args(["tc", "qdisc", "del", "dev", "lo", "root"])
            .output();

        match (add_result, cleanup_result) {
            (Ok(add), Ok(cleanup)) if add.status.success() && cleanup.status.success() => {
                ChaosExperimentResult {
                    name: "io_latency".into(),
                    success: true,
                    message: "100ms delay applied and cleaned up".into(),
                }
            }
            (Ok(add), _) => ChaosExperimentResult {
                name: "io_latency".into(),
                success: false,
                message: format!("tc add failed: {}", String::from_utf8_lossy(&add.stderr)),
            },
            _ => ChaosExperimentResult {
                name: "io_latency".into(),
                success: false,
                message: "tc command failed".into(),
            },
        }
    };

    println!(
        "I/O Latency Experiment: {} — {}",
        result.name, result.message
    );
    assert!(
        result.success,
        "I/O latency experiment failed: {}",
        result.message
    );
}

/// Test 2: Memory Pressure — verify stress-ng is available
///
/// On Linux with stress-ng installed, this validates the memory pressure
/// injection capability. On other platforms, it's a no-op.
#[test]
fn test_chaos_memory_pressure_available() {
    let result = if !is_linux() {
        ChaosExperimentResult {
            name: "memory_pressure".into(),
            success: true,
            message: "skipped: Linux only".into(),
        }
    } else if !command_exists("stress-ng") {
        ChaosExperimentResult {
            name: "memory_pressure".into(),
            success: true,
            message: "skipped: stress-ng not installed (install with: sudo apt install stress-ng)"
                .into(),
        }
    } else {
        // Verify stress-ng can be invoked (just version check)
        let version_result = Command::new("stress-ng").arg("--version").output();

        match version_result {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout);
                ChaosExperimentResult {
                    name: "memory_pressure".into(),
                    success: true,
                    message: format!(
                        "stress-ng available: {}",
                        version.lines().next().unwrap_or("unknown")
                    ),
                }
            }
            Ok(output) => ChaosExperimentResult {
                name: "memory_pressure".into(),
                success: false,
                message: format!(
                    "stress-ng --version failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
            },
            Err(e) => ChaosExperimentResult {
                name: "memory_pressure".into(),
                success: false,
                message: format!("failed to run stress-ng: {}", e),
            },
        }
    };

    println!(
        "Memory Pressure Experiment: {} — {}",
        result.name, result.message
    );
    assert!(
        result.success,
        "Memory pressure experiment failed: {}",
        result.message
    );
}

/// Test 3: Chaos Python Controller exists and is valid
///
/// Validates that the chaos_inject.py script exists and contains
/// the required classes and methods.
#[test]
fn test_chaos_controller_script_valid() {
    let script_path = "scripts/soak/chaos_inject.py";
    let script = std::fs::read_to_string(script_path).expect("chaos_inject.py not found");

    // Verify required classes and methods exist
    assert!(
        script.contains("class ChaosController"),
        "ChaosController class not found"
    );
    assert!(
        script.contains("def inject_io_latency"),
        "inject_io_latency method not found"
    );
    assert!(
        script.contains("def inject_memory_pressure"),
        "inject_memory_pressure method not found"
    );
    assert!(
        script.contains("def kill_server_process"),
        "kill_server_process method not found"
    );
    assert!(
        script.contains("def verify_recovery"),
        "verify_recovery method not found"
    );
    assert!(script.contains("def cleanup"), "cleanup method not found");

    println!("Chaos Controller script is valid");
}

/// Test 4: Verify chaos_inject.py is syntactically valid Python
#[test]
fn test_chaos_controller_python_syntax() {
    let result = Command::new("python3")
        .args(["-m", "py_compile", "scripts/soak/chaos_inject.py"])
        .output();

    assert!(
        result.map(|o| o.status.success()).unwrap_or(false),
        "chaos_inject.py has syntax errors"
    );

    println!("chaos_inject.py is valid Python");
}
