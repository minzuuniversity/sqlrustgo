//! SQLRustGo Canonical CLI Entry Point
//!
//! Thin binary wrapper around sqlrustgo_cli crate.

fn main() -> std::process::ExitCode {
    std::process::ExitCode::from(
        sqlrustgo_cli::run() as u8
    )
}
