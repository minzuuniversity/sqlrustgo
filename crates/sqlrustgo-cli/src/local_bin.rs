//! V312-57 `sqlrustgo` binary — sqlite3-style local mode CLI entry point.
//!
//! This binary exposes the sqlite3-like local DB mode of `sqlrustgo-cli`
//! (BustubX-EDU teaching CLI). It does NOT depend on a MySQL server.
//!
//! Usage mirrors sqlite3:
//!   sqlrustgo edu.db                 interactive stdin batch
//!   sqlrustgo edu.db < script.sql   batch from stdin
//!   sqlrustgo edu.db --continue-on-error   keep going after errors

fn main() -> std::process::ExitCode {
    std::process::ExitCode::from(sqlrustgo_cli::run() as u8)
}
