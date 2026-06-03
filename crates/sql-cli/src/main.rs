//! sqlrustgo-sql-cli — **DEPRECATED** since v3.8.0.
//!
//! All CLI / REPL features now live in the canonical binary
//! `sqlrustgo-mysql-server` (`repl` and `exec` subcommands).
//! This binary remains so existing scripts that invoke
//! `cargo run --bin sqlrustgo-sql-cli` keep working, but it
//! prints a deprecation notice and exits 0. Remove the
//! invocation in your script and switch to
//! `sqlrustgo-mysql-server repl` (interactive) or
//! `sqlrustgo-mysql-server exec "<sql>"` (one-shot).

fn main() {
    eprintln!(
        "sqlrustgo-sql-cli: DEPRECATED since v3.8.0 — \
         use `sqlrustgo-mysql-server repl` (interactive) or \
         `sqlrustgo-mysql-server exec \"<sql>\"` (one-shot) instead"
    );
}
