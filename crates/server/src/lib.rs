//! SQLRustGo legacy server module
//!
//! ⚠️ **DEPRECATED since v3.10.0** — production wire-protocol path is
//! `crates/sqlrustgo-mysql-server` (see `crates/mysql-server/`). This crate
//! remains in the workspace **only** for ~13 integration tests that still
//! reference its symbols; production code (sqlrustgo-cli, sqlrustgo-server,
//! sqlrustgo-mysql-server) does NOT depend on it.
//!
//! **v3.11.0 migration plan**:
//! 1. Switch the 13 integration tests in `tests/integration/*` and
//!    `tests/{server01,server_health}_test.rs` to import from
//!    `sqlrustgo_mysql_server::...` instead of `sqlrustgo_server::...`.
//! 2. Remove this crate from `Cargo.toml` `[workspace] members`.
//! 3. Delete `crates/server/`.
//!
//! See `ISOLATED_MODULES.md` §4 and `docs/governance/debt/debt-registry.yaml`
//! `extension_crates` section for tracking.

pub mod health;
pub mod hybrid_endpoints;
pub mod hybrid_rerank;
pub mod metrics_endpoint;
pub mod scheduler;
pub mod security_integration;
