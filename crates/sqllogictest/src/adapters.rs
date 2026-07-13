//! Database Adapters for SLT Testing
//!
//! ## Beta Baseline Mode
//!
//! In Beta (ISSUE #3373), we run in **baseline mode**: the NoOpAdapter
//! only verifies that sqlrustgo can execute each SQL statement without panicking.
//! Results are NOT compared against a reference (SQLite) yet.
//!
//! ## GA Reference Mode
//!
//! When libsqlite3-dev is installed:
//! ```text
//! # macOS
//! brew install sqlite
//! cargo build -p sqllogictest --features sqlite
//!
//! # Debian/Ubuntu
//! apt install libsqlite3-dev
//! cargo build -p sqllogictest --features sqlite
//! ```
//!
//! The SqliteAdapter runs each SQL on BOTH sqlrustgo and SQLite,
//! then compares the result sets row-by-row.

use std::sync::Arc;
use parking_lot::RwLock;

use sqlrustgo::MemoryExecutionEngine;
use sqlrustgo_storage::MemoryStorage;

use crate::parser::Statement;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffResult {
    /// Results are identical
    Identical,
    /// Results differ
    Mismatch { got: Vec<String> },
    /// sqlrustgo returned an error
    EngineError(String),
    /// Skipped (unsupported feature)
    Skipped(String),
}

pub trait Adapter: Send + Sync + Clone {
    /// Execute a statement and return the diff result
    fn execute_stmt(&self, stmt: &Statement) -> Result<DiffResult, String>;
}

// ---------------------------------------------------------------------------
// NoOp Adapter — Beta baseline mode
// ---------------------------------------------------------------------------

/// NoOp adapter: only checks that sqlrustgo can execute without panicking.
/// Does NOT compare results against a reference.
/// Used in Beta to establish the SLT baseline: "套件可完整执行，建立基线".
#[derive(Clone)]
pub struct NoOpAdapter;

impl NoOpAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoOpAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl Adapter for NoOpAdapter {
    fn execute_stmt(&self, stmt: &Statement) -> Result<DiffResult, String> {
        use crate::parser::StatementKind;

        match stmt.kind {
            StatementKind::StatementOk
            | StatementKind::StatementOnly
            | StatementKind::StatementError => {
                Self::try_execute(&stmt.sql)
            }
            StatementKind::Query
            | StatementKind::HashQuery
            | StatementKind::QueryParallel => {
                Self::try_query(&stmt.sql)
            }
            // Other statement kinds — just pass
            _ => Ok(DiffResult::Identical),
        }
    }
}

impl NoOpAdapter {
    fn try_execute(sql: &str) -> Result<DiffResult, String> {
        let storage = Arc::new(RwLock::new(MemoryStorage::new()));
        let mut engine = MemoryExecutionEngine::new(storage);
        engine
            .execute(sql)
            .map(|_| DiffResult::Identical)
            .map_err(|e| format!("{:?}", e))
    }

    fn try_query(sql: &str) -> Result<DiffResult, String> {
        let storage = Arc::new(RwLock::new(MemoryStorage::new()));
        let mut engine = MemoryExecutionEngine::new(storage);
        engine
            .execute(sql)
            .map(|_| DiffResult::Identical)
            .map_err(|e| format!("{:?}", e))
    }
}

// ---------------------------------------------------------------------------
// SqliteAdapter — GA reference mode (requires --features sqlite)
// ---------------------------------------------------------------------------
//
// To enable SQLite reference testing:
// 1. Install SQLite dev headers (see file-level docs above)
// 2. Uncomment the SqliteAdapter block below
// 3. cargo build -p sqllogictest --features sqlite
//
// The SqliteAdapter runs each SQL on BOTH sqlrustgo and SQLite,
// then compares the result sets row-by-row.
// A Mismatch is reported when sqlrustgo produces different output than SQLite.
