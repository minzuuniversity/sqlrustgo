//! TableEngine trait - per-table storage abstraction for concurrent table-level locking
//!
//! V311-09: Table-level lock architecture
//!
//! Design: Each table has its own lock, allowing concurrent transactions on different tables.
//! The global storage lock is replaced with per-table locks.

use crate::engine::{Record, SqlResult, TableInfo, Value};
use std::collections::HashSet;

/// Trait for per-table storage engines.
/// Each table can be stored with its own implementation (FileTable, MemoryTable, etc.)
pub trait TableEngine: Send + Sync {
    /// Get table info
    fn get_table_info(&self) -> &TableInfo;

    /// Insert records into the table
    fn insert(&mut self, records: Vec<Record>) -> SqlResult<()>;

    /// Scan all records in the table
    fn scan(&self) -> SqlResult<Vec<Record>>;

    /// Delete records matching filters
    fn delete(&mut self, filters: &[Value]) -> SqlResult<usize>;

    /// Update records matching filters
    fn update(&mut self, filters: &[Value], updates: &[(usize, Value)]) -> SqlResult<usize>;

    /// Flush table to disk
    fn flush(&mut self) -> SqlResult<()>;

    /// Get dirty tables (for flush optimization)
    fn dirty_tables(&self) -> &HashSet<String>;

    /// Mark a table as dirty
    fn mark_dirty(&mut self, table: &str);

    /// Get table name
    fn table_name(&self) -> &str;
}
