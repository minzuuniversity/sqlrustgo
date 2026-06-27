//! SQLRustGo Canonical CLI Entry Point (Published Binary)
//!
//! Delegates to the sqlrustgo-cli crate.

fn main() {
    std::process::exit(sqlrustgo_cli::run());
}
