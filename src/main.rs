//! SQLRustGo root binary — **DEPRECATED** since v3.8.0.
//!
//! All execution paths now live in `sqlrustgo-mysql-server`. This
//! binary remains so existing scripts and tests that invoke
//! `cargo run --bin sqlrustgo` keep working, but it prints a
//! deprecation notice and exits successfully (the REPL itself is
//! available as `sqlrustgo-mysql-server repl`).

fn main() {
    eprintln!(
        "sqlrustgo: DEPRECATED since v3.8.0 — use `sqlrustgo-mysql-server repl` instead"
    );
    println!("SQLRustGo v3.8.0 (canonical entry: sqlrustgo-mysql-server)");
}
