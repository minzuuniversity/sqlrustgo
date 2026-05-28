//! PostgreSQL Compatibility Layer (P0)
//!
//! This module provides basic PostgreSQL wire protocol compatibility
//! for SQLRustGo, enabling standard PostgreSQL clients and BI tools
//! to connect to SQLRustGo.
//!
//! ## Features
//!
//! - Type mapping between PostgreSQL OIDs and SQLRustGo Value types
//! - Cleartext password authentication handler
//!
//! ## Module Structure
//!
//! - [`types`] - PostgreSQL OID to SQLRustGo type mapping
//! - [`auth`] - Authentication handlers

pub mod auth;
pub mod types;

pub use auth::AuthHandler;
pub use types::{PgOid, PgType, PgTypeMap};
