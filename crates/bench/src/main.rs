//! sqlrustgo-bench — **DEPRECATED** since v3.8.0.
//!
//! The benchmark runner lives behind the `bench` subcommand of
//! the canonical binary `sqlrustgo-mysql-server`. Full feature
//! parity (workload selection, scale factor, threads, etc.)
//! migrates in a follow-up; the canonical binary prints a
//! helpful message today. This binary remains so existing
//! `cargo run --bin sqlrustgo-bench` invocations keep working
//! during the migration window.

fn main() {
    eprintln!(
        "sqlrustgo-bench: DEPRECATED since v3.8.0 — \
         use `sqlrustgo-mysql-server bench` instead (full feature \
         parity migrates in a follow-up)"
    );
}
