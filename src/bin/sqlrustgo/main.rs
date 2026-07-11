//! SQLRustGo Canonical CLI Entry Point (Published Binary)
//!
//! DEPRECATED: this single-binary name is being phased out in favor
//! of the dedicated `sqlrustgo-soak` (CLI REPL/exec/soak) and
//! `sqlrustgo-mysql-server` (server) binaries. Use those instead.

use std::io::Write;

fn main() {
    // Print a deprecation warning to stderr (not stdout, so it
    // doesn't interfere with REPL/exec output). The shim continues
    // to delegate to sqlrustgo_cli::run() for backward compat.
    let mut stderr = std::io::stderr().lock();
    let _ = writeln!(
        stderr,
        "DEPRECATED: 'sqlrustgo' binary is being phased out. \
         Use 'sqlrustgo-soak' (client) and 'sqlrustgo-mysql-server' (server) instead."
    );
    drop(stderr);
    std::process::exit(sqlrustgo_cli::run());
}
