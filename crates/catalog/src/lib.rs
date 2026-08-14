//! SQLRustGo Catalog Module
//!
//! Provides database metadata management including:
//! - Schema and table definitions
//! - Column and data type definitions
//! - Index information
//! - Stored procedures
//! - Authentication and authorization
//!
//! # Architecture
//!
//! Catalog -> Database -> Schema -> Table
//! Catalog -> StoredProcedure

pub mod auth;
pub mod catalog;
pub mod column;
pub mod data_type;
pub mod database;
pub mod error;
pub mod index;
pub mod rebuild;
pub mod row_level_security;
pub mod schema;
pub mod stored_proc;
pub mod system_tables;
pub mod table;

pub use auth::{
    AuthError, AuthErrorCode, AuthManager, ObjectRef, ObjectType, PasswordPolicy,
    PasswordRotationManager, Privilege, User, UserIdentity, DEFAULT_PASSWORD_HISTORY_SIZE,
    DEFAULT_PASSWORD_LIFETIME_DAYS,
};
pub use catalog::Catalog;
pub use column::ColumnDefinition;
pub use data_type::DataType;
pub use database::Database;
pub use error::{CatalogError, CatalogResult};
pub use row_level_security::{Policy, PolicyCatalog, PolicyCommand};
pub use stored_proc::{
    HandlerCondition, ParamMode, StoredProcParam, StoredProcStatement, StoredProcedure,
};
pub use table::{ForeignKeyAction, ForeignKeyRef, Table};
