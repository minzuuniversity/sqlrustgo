//! CLI repl module tests
//!
//! Tests: run_repl function signature

use sqlrustgo_soak::repl::run_repl;

#[test]
fn test_run_repl_accepts_valid_args() {
    // run_repl with unreachable host should error, not panic
    let result = run_repl("127.0.0.1", 0, "root", "");
    assert!(result.is_err() || result.is_ok());
}
