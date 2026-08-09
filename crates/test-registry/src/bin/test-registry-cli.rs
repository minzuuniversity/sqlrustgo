//! Test Registry CLI
//!
//! Command-line interface for managing and running registered tests.

use std::path::Path;
use test_registry::TestRegistry;

fn main() {
    println!("Test Registry CLI");
    println!("=================");
    println!();

    // Load or create a test registry
    let registry_path = Path::new("test-registry.toml");
    let registry = if registry_path.exists() {
        match TestRegistry::from_toml(registry_path) {
            Ok(r) => {
                println!("Loaded {} managed tests from {}",
                    r.managed_len(), registry_path.display());
                r
            }
            Err(e) => {
                eprintln!("Error loading registry: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        println!("No registry file found, creating default...");
        TestRegistry::default()
    };

    // Display registry contents
    println!("\nRegistered managed tests:");
    for test in registry.managed() {
        println!("  - {}: {} (timeout: {}ms, priority: {:?})",
            test.name, test.binary, test.timeout_ms, test.priority);
    }

    // Show summary
    let managed_count = registry.managed_len();
    if managed_count > 0 {
        println!("\nTotal: {} managed test(s)", managed_count);
    } else {
        println!("\nNo managed tests registered.");
        println!("Add tests to test-registry.toml");
    }

    println!("\nUsage:");
    println!("  test-registry-cli list          - List all registered tests");
    println!("  test-registry-cli run <name>    - Run a specific test");
    println!("  test-registry-cli run-all       - Run all registered tests");
}
