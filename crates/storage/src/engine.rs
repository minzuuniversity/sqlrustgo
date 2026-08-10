//! Storage Engine trait - abstraction for storage backends
//! Supports multiple storage implementations (File, Memory, etc.)

use serde::{Deserialize, Serialize};
pub use sqlrustgo_types::{SqlError, SqlResult, Value};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Referential action for foreign key constraints
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ForeignKeyAction {
    Cascade,
    SetNull,
    Restrict,
    NoAction,
}

/// Foreign key constraint definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignKeyConstraint {
    pub name: Option<String>,
    pub columns: Vec<String>,
    pub referenced_table: String,
    pub referenced_columns: Vec<String>,
    pub on_delete: Option<ForeignKeyAction>,
    pub on_update: Option<ForeignKeyAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniqueConstraint {
    pub name: Option<String>,
    pub columns: Vec<String>,
}

/// Check constraint definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckConstraint {
    pub name: Option<String>,
    pub expression: String,
}

/// Evaluate a CHECK constraint expression against a record
/// The expression is stored as a string like "age >= 0" or "name IS NOT NULL"
/// Returns Ok(true) if constraint is satisfied, Ok(false) if not, Err on parse error
pub fn evaluate_check_constraint(
    constraint: &CheckConstraint,
    columns: &[String],
    record: &[Value],
) -> SqlResult<bool> {
    evaluate_sql_expression(&constraint.expression, columns, record)
}

/// Evaluate a SQL expression against a record
/// Supports: comparisons (=, !=, <, >, <=, >=), boolean ops (AND, OR, NOT), IS NULL/IS NOT NULL
fn evaluate_sql_expression(expr: &str, columns: &[String], record: &[Value]) -> SqlResult<bool> {
    let expr = expr.trim();

    // Handle AND/OR
    if let Some(idx) = find_top_level_op(expr, "AND") {
        let left = &expr[..idx];
        let right = &expr[idx + 3..];
        return Ok(evaluate_sql_expression(left.trim(), columns, record)?
            && evaluate_sql_expression(right.trim(), columns, record)?);
    }
    if let Some(idx) = find_top_level_op(expr, "OR") {
        let left = &expr[..idx];
        let right = &expr[idx + 2..];
        return Ok(evaluate_sql_expression(left.trim(), columns, record)?
            || evaluate_sql_expression(right.trim(), columns, record)?);
    }

    // Handle NOT
    if expr.to_uppercase().starts_with("NOT ") {
        let inner = &expr[4..].trim();
        return Ok(!evaluate_sql_expression(inner, columns, record)?);
    }

    // Handle IS NULL / IS NOT NULL
    if let Some(idx) = expr.to_uppercase().find(" IS NULL") {
        let col_name = expr[..idx].trim();
        if let Some(val) = get_column_value(col_name, columns, record) {
            return Ok(matches!(val, Value::Null));
        }
        return Ok(true); // column not found, assume OK
    }
    if let Some(idx) = expr.to_uppercase().find(" IS NOT NULL") {
        let col_name = expr[..idx].trim();
        if let Some(val) = get_column_value(col_name, columns, record) {
            return Ok(!matches!(val, Value::Null));
        }
        return Ok(false);
    }

    // Handle comparisons: column op value
    for (op, check) in &[
        (">=", "gte"),
        ("<=", "lte"),
        ("!=", "neq"),
        ("<>", "neq"),
        ("=", "eq"),
        ("==", "eq"),
        (">", "gt"),
        ("<", "lt"),
    ] {
        if let Some(idx) = expr.find(op) {
            let col_name = expr[..idx].trim();
            let value_str = expr[idx + op.len()..].trim();

            if let Some(col_val) = get_column_value(col_name, columns, record) {
                return compare_values(col_val, value_str, check);
            }
            break;
        }
    }

    // If no comparison found, try to evaluate as a literal boolean or column existence check
    let upper = expr.to_uppercase();
    if upper == "TRUE" || upper == "1" {
        return Ok(true);
    }
    if upper == "FALSE" || upper == "0" {
        return Ok(false);
    }

    // Treat as column name - check if not null
    if let Some(val) = get_column_value(expr, columns, record) {
        return Ok(!matches!(val, Value::Null) && !is_zero_or_empty(val));
    }

    Err(format!("Cannot evaluate CHECK expression: {}", expr).into())
}

/// Find top-level operator (not inside quotes or parentheses)
fn find_top_level_op(expr: &str, op: &str) -> Option<usize> {
    let upper = expr.to_uppercase();
    let op_upper = op.to_uppercase();
    let mut depth = 0;
    let mut in_string = false;

    for (i, c) in expr.char_indices() {
        match c {
            '(' => {
                depth += 1;
            }
            ')' => {
                depth -= 1;
            }
            '\'' => {
                in_string = !in_string;
            }
            _ if !in_string && depth == 0 && upper[i..].starts_with(&op_upper) => {
                return Some(i);
            }
            _ => {}
        }
    }
    None
}

/// Get column value by name (case-insensitive)
fn get_column_value<'a>(
    name: &str,
    columns: &'a [String],
    record: &'a [Value],
) -> Option<&'a Value> {
    // Remove quotes if present
    let name = name.trim().trim_matches(|c| c == '\'' || c == '"');

    for (i, col) in columns.iter().enumerate() {
        if col.eq_ignore_ascii_case(name) {
            return record.get(i);
        }
    }
    None
}

/// Compare column value with string representation
fn compare_values(col_val: &Value, compare_with: &str, op: &str) -> SqlResult<bool> {
    let cmp_str = compare_with.trim().trim_matches(|c| c == '\'' || c == '"');

    match op {
        "eq" => match col_val {
            Value::Null => Ok(false),
            Value::Integer(i) => {
                if let Ok(cmp) = cmp_str.parse::<i64>() {
                    Ok(*i == cmp)
                } else {
                    Ok(false)
                }
            }
            Value::Float(f) => {
                if let Ok(cmp) = cmp_str.parse::<f64>() {
                    Ok(*f == cmp)
                } else {
                    Ok(false)
                }
            }
            Value::Text(s) => Ok(s == cmp_str),
            Value::Boolean(b) => {
                let cmp_bool = cmp_str.eq_ignore_ascii_case("true") || cmp_str == "1";
                Ok(*b == cmp_bool)
            }
            Value::Blob(_) => Ok(false),
            Value::Point(_, _) => Ok(false),
        },
        "neq" => Ok(!compare_values(col_val, compare_with, "eq")?),
        "gt" | "gte" | "lt" | "lte" => {
            match col_val {
                Value::Integer(i) => {
                    if let Ok(cmp) = cmp_str.parse::<i64>() {
                        return Ok(match op {
                            "gt" => *i > cmp,
                            "gte" => *i >= cmp,
                            "lt" => *i < cmp,
                            "lte" => *i <= cmp,
                            _ => false,
                        });
                    }
                }
                Value::Float(f) => {
                    if let Ok(cmp) = cmp_str.parse::<f64>() {
                        return Ok(match op {
                            "gt" => *f > cmp,
                            "gte" => *f >= cmp,
                            "lt" => *f < cmp,
                            "lte" => *f <= cmp,
                            _ => false,
                        });
                    }
                }
                Value::Text(s) => {
                    return Ok(match op {
                        "gt" => s.as_str() > cmp_str,
                        "gte" => s.as_str() >= cmp_str,
                        "lt" => s.as_str() < cmp_str,
                        "lte" => s.as_str() <= cmp_str,
                        _ => false,
                    });
                }
                _ => {}
            }
            Err(format!("Cannot compare {} with {}", col_val, cmp_str).into())
        }
        _ => Err(format!("Unknown operator: {}", op).into()),
    }
}

fn is_zero_or_empty(val: &Value) -> bool {
    match val {
        Value::Integer(i) => *i == 0,
        Value::Float(f) => *f == 0.0,
        Value::Text(s) => s.is_empty(),
        Value::Boolean(b) => !*b,
        Value::Null => true,
        Value::Blob(_) => false,
        Value::Point(_, _) => false,
    }
}

/// Trigger timing: BEFORE or AFTER
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriggerTiming {
    Before,
    After,
}

/// Trigger event: INSERT, UPDATE, or DELETE
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriggerEvent {
    Insert,
    Update,
    Delete,
}

/// Trigger definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerInfo {
    pub name: String,
    pub table_name: String,
    pub timing: TriggerTiming,
    pub event: TriggerEvent,
    pub body: String,
}

/// Sequence definition (F-30)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceInfo {
    pub name: String,
    pub start_with: i64,
    pub increment_by: i64,
    pub minvalue: i64,
    pub maxvalue: i64,
    pub cache: i64,
    pub cycle: bool,
    pub current_value: i64,
}

impl Default for SequenceInfo {
    fn default() -> Self {
        Self {
            name: String::new(),
            start_with: 1,
            increment_by: 1,
            minvalue: 1,
            maxvalue: i64::MAX,
            cache: 1,
            cycle: false,
            current_value: 0,
        }
    }
}

/// Partition type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PartitionType {
    Range,
    List,
    Hash,
}

/// Partition definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionInfo {
    pub partition_type: PartitionType,
    pub column: String,
    pub boundaries: Vec<Value>,
}

impl PartitionInfo {
    pub fn new_range(column: &str, boundaries: Vec<Value>) -> Self {
        Self {
            partition_type: PartitionType::Range,
            column: column.to_string(),
            boundaries,
        }
    }

    pub fn new_list(column: &str, values: Vec<Value>) -> Self {
        Self {
            partition_type: PartitionType::List,
            column: column.to_string(),
            boundaries: values,
        }
    }

    pub fn new_hash(column: &str, num_partitions: u32) -> Self {
        Self {
            partition_type: PartitionType::Hash,
            column: column.to_string(),
            boundaries: vec![Value::Integer(num_partitions as i64)],
        }
    }

    pub fn get_partition_index(&self, value: &Value) -> Option<usize> {
        match self.partition_type {
            PartitionType::Range => self.get_range_partition(value),
            PartitionType::List => self.get_list_partition(value),
            PartitionType::Hash => self.get_hash_partition(value),
        }
    }

    fn get_range_partition(&self, value: &Value) -> Option<usize> {
        if let Value::Integer(n) = value {
            for (i, boundary) in self.boundaries.iter().enumerate() {
                if let Value::Integer(b) = boundary {
                    if n < b {
                        return Some(i);
                    }
                }
            }
            Some(self.boundaries.len())
        } else {
            None
        }
    }

    fn get_list_partition(&self, value: &Value) -> Option<usize> {
        for (i, boundary) in self.boundaries.iter().enumerate() {
            if value == boundary {
                return Some(i);
            }
        }
        None
    }

    fn get_hash_partition(&self, value: &Value) -> Option<usize> {
        if let Value::Integer(n) = value {
            let num_partitions = self.boundaries.first()?.as_integer()? as u64;
            let hash = n.unsigned_abs() % num_partitions;
            Some(hash as usize)
        } else if let Value::Text(s) = value {
            let num_partitions = self.boundaries.first()?.as_integer()? as u32;
            let hash = calculate_hash(s.as_bytes()) % num_partitions;
            Some(hash as usize)
        } else {
            None
        }
    }
}

fn calculate_hash(data: &[u8]) -> u32 {
    data.iter()
        .fold(0u32, |acc, &b| acc.wrapping_add(b as u32).wrapping_mul(31))
}

/// Table metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TableInfo {
    pub name: String,
    pub columns: Vec<ColumnDefinition>,
    #[serde(default)]
    pub foreign_keys: Vec<ForeignKeyConstraint>,
    #[serde(default)]
    pub unique_constraints: Vec<UniqueConstraint>,
    #[serde(default)]
    pub check_constraints: Vec<CheckConstraint>,
    #[serde(skip)]
    pub partition_info: Option<PartitionInfo>,
    /// V311-12 F-27: table compression specifier (algorithm: "LZ4", "ZSTD", "ZLIB")
    #[serde(default)]
    pub compression: Option<String>,
}

/// Column definition for table schema
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ColumnDefinition {
    pub name: String,
    pub data_type: String,
    #[serde(default)]
    pub nullable: bool,
    #[serde(default)]
    pub primary_key: bool,
    /// SQL CHAR(N) / VARCHAR(N) declared max length. See
    /// `sqlrustgo-catalog::column::ColumnDefinition::char_max_length` for
    /// semantics. `None` (default) means no length cap.
    #[serde(default)]
    pub char_max_length: Option<usize>,
}

impl ColumnDefinition {
    pub fn new(name: &str, data_type: &str) -> Self {
        Self {
            name: name.to_string(),
            data_type: data_type.to_string(),
            nullable: false,
            primary_key: false,
            char_max_length: None,
        }
    }
}

/// Table data - combines metadata and rows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableData {
    pub info: TableInfo,
    pub rows: Vec<Record>,
}

/// Record type - a single row of values
pub type Record = Vec<Value>;

/// Row mutation with assignments and metadata
#[derive(Debug, Clone)]
pub struct RowMutation {
    assignments: Vec<(usize, Value)>,
    mutation_hash: u64,
}

impl RowMutation {
    pub fn new(assignments: Vec<(usize, Value)>, mutation_hash: u64) -> Self {
        Self {
            assignments,
            mutation_hash,
        }
    }
    pub fn assignments(&self) -> &[(usize, Value)] {
        &self.assignments
    }
    pub fn mutation_hash(&self) -> u64 {
        self.mutation_hash
    }
}

/// Filter function type for row-level filtering
pub type RowFilter = Box<dyn Fn(&Record) -> bool + Send + Sync>;

/// StorageEngine trait - abstraction for table storage
/// Enables multiple storage backends (FileStorage, MemoryStorage, etc.)
pub trait StorageEngine: Send + Sync {
    /// Scan all rows from a table
    fn scan(&self, table: &str) -> SqlResult<Vec<Record>>;
    /// Parallel scan - returns partitions for parallel processing
    ///
    /// v3.10.0 Issue #3703 Phase 2: Storage-layer parallelization.
    /// Each partition is an iterator to avoid loading all data at once.
    ///
    /// Default implementation returns Err (engines must opt-in).
    fn parallel_scan(
        &self,
        table: &str,
        num_partitions: usize,
    ) -> SqlResult<Vec<Box<dyn Iterator<Item = Record> + Send>>> {
        let _ = (table, num_partitions);
        Err(SqlError::ExecutionError(
            "parallel_scan not supported by this storage engine".to_string(),
        ))
    }

    /// Insert rows into a table
    fn insert(&mut self, table: &str, records: Vec<Record>) -> SqlResult<()>;

    /// Force-insert a row bypassing any deferred-write buffer.
    ///
    /// Default implementation just calls `insert`. Storage engines that buffer
    /// inserts (e.g. FileStorage) MUST override this to write directly to
    /// `data.rows` so subsequent scan/delete in the same call stack see the
    /// row. Used by WAL recovery to apply replayed entries deterministically.
    fn force_insert(&mut self, table: &str, record: Vec<Value>) -> SqlResult<()> {
        self.insert(table, vec![record])
    }

    /// Delete rows matching a filter
    fn delete(&mut self, table: &str, _filters: &[Value]) -> SqlResult<usize>;
    fn delete_if(&mut self, table: &str, filter: &RowFilter) -> SqlResult<usize>;
    fn update(
        &mut self,
        table: &str,
        _filters: &[Value],
        _updates: &[(usize, Value)],
    ) -> SqlResult<usize>;
    fn update_if(
        &mut self,
        table: &str,
        filter: &RowFilter,
        mutation: &RowMutation,
    ) -> SqlResult<usize>;

    /// Create a new database (directory). No-op for in-memory engines.
    fn create_database(&mut self, _db_name: &str) -> SqlResult<()> {
        Err(SqlError::ExecutionError(
            "create_database not supported by this storage engine".to_string(),
        ))
    }

    /// Drop a database (directory). No-op for in-memory engines.
    fn drop_database(&mut self, _db_name: &str) -> SqlResult<()> {
        Err(SqlError::ExecutionError(
            "drop_database not supported by this storage engine".to_string(),
        ))
    }

    /// Create a table
    fn create_table(&mut self, info: &TableInfo) -> SqlResult<()>;

    /// Drop a table
    fn drop_table(&mut self, table: &str) -> SqlResult<()>;

    /// Get table metadata
    fn get_table_info(&self, table: &str) -> SqlResult<TableInfo>;

    /// Check if table exists
    fn has_table(&self, table: &str) -> bool;

    /// List all tables
    fn list_tables(&self) -> Vec<String>;

    /// Create an index on a table
    fn create_index(&mut self, table: &str, column: &str, column_index: usize) -> SqlResult<()>;

    /// Drop an index from a table
    fn drop_index(&mut self, table: &str, column: &str) -> SqlResult<()>;

    /// Add a column to an existing table
    fn add_column(&mut self, table: &str, column: ColumnDefinition) -> SqlResult<()>;

    /// Rename a table
    fn rename_table(&mut self, table: &str, new_name: &str) -> SqlResult<()>;

    /// Drop a column from a table
    fn drop_column(&mut self, _table: &str, _column: &str) -> SqlResult<()> {
        Err(SqlError::ExecutionError(
            "drop_column not supported by this storage engine".to_string(),
        ))
    }
    /// Rename a column in a table
    fn rename_column(&mut self, _table: &str, _old_name: &str, _new_name: &str) -> SqlResult<()> {
        Err(SqlError::ExecutionError(
            "rename_column not supported by this storage engine".to_string(),
        ))
    }

    /// Modify a column definition
    fn modify_column(
        &mut self,
        _table: &str,
        _column: &str,
        _new_def: ColumnDefinition,
    ) -> SqlResult<()> {
        Err(SqlError::ExecutionError(
            "modify_column not supported by this storage engine".to_string(),
        ))
    }

    /// Create a trigger on a table
    fn create_trigger(&mut self, info: TriggerInfo) -> SqlResult<()>;

    /// Drop a trigger by name
    fn drop_trigger(&mut self, name: &str) -> SqlResult<()>;

    /// Get a trigger by name
    fn get_trigger(&self, name: &str) -> Option<TriggerInfo>;

    /// List all triggers for a table
    fn list_triggers(&self, table: &str) -> Vec<TriggerInfo>;

    /// List all indexes for a table, returns Vec of (column_name, index_name)
    fn list_indexes(&self, table: &str) -> Vec<(String, String)>;

    /// Check if a view exists
    fn has_view(&self, name: &str) -> bool;

    /// Begin a transaction, returns a transaction ID
    fn begin_transaction(&mut self) -> SqlResult<u64> {
        Err(SqlError::ExecutionError(
            "Transactions not supported by this storage engine".to_string(),
        ))
    }

    /// Commit the current transaction
    fn commit_transaction(&mut self) -> SqlResult<()> {
        Err(SqlError::ExecutionError(
            "Transactions not supported by this storage engine".to_string(),
        ))
    }

    /// Rollback the current transaction
    fn rollback_transaction(&mut self) -> SqlResult<()> {
        Err(SqlError::ExecutionError(
            "Transactions not supported by this storage engine".to_string(),
        ))
    }

    /// Release all gap locks held by a transaction
    ///
    /// Called during transaction commit/rollback to release gap locks.
    /// No-op if gap locking is not enabled.
    fn release_all_gap_locks(&mut self, _tx_id: u64) {
        // Default: no-op
    }

    /// Check if a transaction is in progress
    fn in_transaction(&self) -> bool {
        false
    }

    /// Get the current transaction ID
    fn current_tx_id(&self) -> u64 {
        0
    }

    /// Set the current transaction ID (used by WAL integration)
    fn set_current_tx_id(&mut self, _id: u64) {}

    /// Flush any buffered data to durable storage
    fn flush(&mut self) -> SqlResult<()> {
        Ok(())
    }

    /// Flush with parallel table writes (V311-09)
    /// Default implementation falls back to sequential flush
    fn flush_parallel(&mut self) -> SqlResult<()> {
        self.flush()
    }
    fn is_wal_enabled(&self) -> bool {
        false
    }

    // === Sequence support (F-30) ===

    /// Create a new sequence
    fn create_sequence(&mut self, _seq: SequenceInfo) -> SqlResult<()> {
        Err(SqlError::ExecutionError(
            "Sequences not supported by this storage engine".to_string(),
        ))
    }

    /// Drop a sequence
    fn drop_sequence(&mut self, _name: &str) -> SqlResult<()> {
        Err(SqlError::ExecutionError(
            "Sequences not supported by this storage engine".to_string(),
        ))
    }

    /// Get next value from a sequence
    fn next_sequence_value(&mut self, _name: &str) -> SqlResult<i64> {
        Err(SqlError::ExecutionError(
            "Sequences not supported by this storage engine".to_string(),
        ))
    }

    /// Get current value from a sequence (without advancing)
    fn current_sequence_value(&self, _name: &str) -> SqlResult<i64> {
        Err(SqlError::ExecutionError(
            "Sequences not supported by this storage engine".to_string(),
        ))
    }

    /// Check if a sequence exists
    fn has_sequence(&self, _name: &str) -> bool {
        false
    }

    /// List all sequences
    fn list_sequences(&self) -> Vec<String> {
        Vec::new()
    }

    /// Get a sequence by name
    fn get_sequence(&self, _name: &str) -> Option<SequenceInfo> {
        None
    }
}

/// In-memory storage implementation for testing and caching
pub struct MemoryStorage {
    tables: HashMap<String, Vec<Record>>,
    table_infos: HashMap<String, TableInfo>,
    triggers: HashMap<String, TriggerInfo>,
    views: HashSet<String>,
    /// Sequence definitions (F-30)
    sequences: HashMap<String, SequenceInfo>,
    /// 内存中的数据库集合 (CREATE DATABASE 注册的, in-memory 模式)
    databases: HashSet<String>,
    /// Tracks the current transaction ID for VtuGuard::assert_dml_safe.
    /// VtuGuard checks S::in_transaction() which returns `current_tx_id != 0`.
    current_tx_id: u64,
    next_tx_id: u64,
    /// `Some(log)` between matching `begin`/`commit` (or `begin`/`rollback`);
    /// `None` outside a transaction.
    tx_log: Option<TxLog>,
}

#[derive(Default)]
struct TxLog {
    inserted: Vec<(String, Record)>,
    deleted: Vec<(String, Record)>,
    updated: Vec<(String, Record, Record)>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            tables: HashMap::new(),
            table_infos: HashMap::new(),
            triggers: HashMap::new(),
            views: HashSet::new(),
            sequences: HashMap::new(),
            databases: HashSet::new(),
            current_tx_id: 0,
            next_tx_id: 1,
            tx_log: None,
        }
    }

    /// v3.10.0 Issue #3703: returns pre-partitioned chunks so the caller
    /// (typically `engine_select::filter_partitions_parallel`) can
    /// process each chunk on a separate rayon worker, fusing scan
    /// + filter into one parallel pipeline.
    ///
    /// Returns `Vec<Vec<Record>>` of `num_partitions` chunks. Falls
    /// back to a single-chunk vector when `num_partitions <= 1` or
    /// `rows.len() < PARALLEL_SCAN_MIN_ROWS`.
    pub fn partition_rows(&self, table: &str, num_partitions: usize) -> Vec<Vec<Record>> {
        const PARALLEL_SCAN_MIN_ROWS: usize = 500_000;
        let n_partitions = num_partitions.max(1);
        let Some(all_rows) = self.tables.get(table) else {
            return vec![Vec::new()];
        };
        if all_rows.len() < PARALLEL_SCAN_MIN_ROWS || n_partitions <= 1 {
            return vec![all_rows.clone()];
        }
        let total = all_rows.len();
        let base = total / n_partitions;
        let rem = total % n_partitions;
        let mut out: Vec<Vec<Record>> = Vec::with_capacity(n_partitions);
        let mut cur = 0usize;
        for i in 0..n_partitions {
            let size = base + if i < rem { 1 } else { 0 };
            out.push(all_rows[cur..cur + size].to_vec());
            cur += size;
        }
        out
    }

    /// Load data from a TPC-H `.tbl` file (pipe-delimited format).
    /// Parses each line according to the table's column definitions and
    /// inserts records in batches. Returns the number of rows loaded.
    pub fn bulk_load_tbl_file(&mut self, table_name: &str, path: &str) -> SqlResult<usize> {
        use std::fs;
        let content = fs::read_to_string(path).map_err(|e| SqlError::IoError(e.to_string()))?;
        let lines: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();
        if lines.is_empty() {
            return Ok(0);
        }
        let info = self.get_table_info(table_name)?;
        let cols: Vec<&str> = info.columns.iter().map(|c| c.data_type.as_str()).collect();
        let mut batch: Vec<Record> = Vec::with_capacity(1024);
        let mut total = 0usize;
        for line in &lines {
            let parts: Vec<&str> = line.trim_end_matches('|').split('|').collect();
            let mut row = Vec::with_capacity(parts.len());
            for (i, field) in parts.iter().enumerate() {
                if field.is_empty() || *field == "NULL" || *field == "null" {
                    row.push(Value::Null);
                    continue;
                }
                let dtype = cols.get(i).copied().unwrap_or("TEXT");
                let val = match dtype.to_uppercase().as_str() {
                    "INTEGER" | "INT" | "BIGINT" | "SMALLINT" | "TINYINT" => {
                        Value::Integer(field.parse::<i64>().map_err(|e| {
                            SqlError::ExecutionError(format!("parse int at col {i}: {e}"))
                        })?)
                    }
                    "REAL" | "FLOAT" | "DOUBLE" | "DECIMAL" | "NUMERIC" => {
                        Value::Float(field.parse::<f64>().map_err(|e| {
                            SqlError::ExecutionError(format!("parse float at col {i}: {e}"))
                        })?)
                    }
                    _ => Value::Text(field.to_string()),
                };
                row.push(val);
            }
            batch.push(row);
            if batch.len() >= 1024 {
                total += batch.len();
                self.insert(table_name, batch)?;
                batch = Vec::with_capacity(1024);
            }
        }
        if !batch.is_empty() {
            total += batch.len();
            self.insert(table_name, batch)?;
        }
        Ok(total)
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl StorageEngine for MemoryStorage {
    fn scan(&self, table: &str) -> SqlResult<Vec<Record>> {
        Ok(self.tables.get(&table.to_lowercase()).cloned().unwrap_or_default())
    }

    fn begin_transaction(&mut self) -> SqlResult<u64> {
        if self.tx_log.is_some() {
            return Err(SqlError::ExecutionError(
                "Nested transactions are not supported on MemoryStorage".to_string(),
            ));
        }
        let tx_id = self.next_tx_id;
        self.next_tx_id += 1;
        self.current_tx_id = tx_id;
        self.tx_log = Some(TxLog::default());
        Ok(tx_id)
    }

    fn commit_transaction(&mut self) -> SqlResult<()> {
        self.tx_log = None;
        self.current_tx_id = 0;
        Ok(())
    }

    fn rollback_transaction(&mut self) -> SqlResult<()> {
        if let Some(log) = self.tx_log.take() {
            for (table, row) in log.deleted.into_iter().rev() {
                self.tables.entry(table).or_default().push(row);
            }
            for (table, row) in log.inserted.into_iter().rev() {
                if let Some(records) = self.tables.get_mut(&table) {
                    records.retain(|r| r != &row);
                }
            }
            for (table, prior, _new) in log.updated.into_iter().rev() {
                if let Some(records) = self.tables.get_mut(&table) {
                    for record in records.iter_mut() {
                        if *record == _new {
                            *record = prior.clone();
                        }
                    }
                }
            }
        }
        self.current_tx_id = 0;
        Ok(())
    }

    fn insert(&mut self, table: &str, records: Vec<Record>) -> SqlResult<()> {
        let table_key = table.to_lowercase();
        if let Some(log) = self.tx_log.as_mut() {
            for row in &records {
                log.inserted.push((table_key.clone(), row.clone()));
            }
        }
        self.tables
            .entry(table_key)
            .or_default()
            .extend(records);
        Ok(())
    }

    fn delete(&mut self, table: &str, filters: &[Value]) -> SqlResult<usize> {
        let Some(records) = self.tables.get_mut(table) else {
            return Ok(0);
        };
        if filters.is_empty() {
            if let Some(log) = self.tx_log.as_mut() {
                for row in records.iter() {
                    log.deleted.push((table.to_string(), row.clone()));
                }
            }
            let count = records.len();
            records.clear();
            return Ok(count);
        }
        let original_len = records.len();
        records.retain(|r| {
            let keep = !filters.iter().enumerate().all(|(i, v)| r.get(i) == Some(v));
            if !keep {
                if let Some(log) = self.tx_log.as_mut() {
                    log.deleted.push((table.to_string(), r.clone()));
                }
            }
            keep
        });
        Ok(original_len - records.len())
    }

    fn delete_if(&mut self, table: &str, filter: &RowFilter) -> SqlResult<usize> {
        let Some(records) = self.tables.get_mut(table) else {
            return Ok(0);
        };
        let original_len = records.len();
        records.retain(|r| {
            let keep = !filter(r);
            if !keep {
                if let Some(log) = self.tx_log.as_mut() {
                    log.deleted.push((table.to_string(), r.clone()));
                }
            }
            keep
        });
        Ok(original_len - records.len())
    }

    fn update(
        &mut self,
        table: &str,
        filters: &[Value],
        updates: &[(usize, Value)],
    ) -> SqlResult<usize> {
        let Some(records) = self.tables.get_mut(table) else {
            return Ok(0);
        };

        let mut count = 0;

        if filters.is_empty() {
            for record in records.iter_mut() {
                let prior = record.clone();
                for &(col_idx, ref new_val) in updates {
                    if col_idx < record.len() {
                        record[col_idx] = new_val.clone();
                    }
                }
                if let Some(log) = self.tx_log.as_mut() {
                    log.updated.push((table.to_string(), prior, record.clone()));
                }
                count += 1;
            }
        } else if let Some(filter_val) = filters.first() {
            for record in records.iter_mut() {
                let matches = record.first().map(|v| v == filter_val).unwrap_or(false);
                if matches {
                    let prior = record.clone();
                    for &(col_idx, ref new_val) in updates {
                        if col_idx < record.len() {
                            record[col_idx] = new_val.clone();
                        }
                    }
                    if let Some(log) = self.tx_log.as_mut() {
                        log.updated.push((table.to_string(), prior, record.clone()));
                    }
                    count += 1;
                }
            }
        }

        Ok(count)
    }

    fn update_if(
        &mut self,
        table: &str,
        filter: &RowFilter,
        mutation: &RowMutation,
    ) -> SqlResult<usize> {
        let Some(records) = self.tables.get_mut(table) else {
            return Ok(0);
        };

        let mut count = 0;
        let assignments = mutation.assignments();

        for record in records.iter_mut() {
            if filter(record) {
                let prior = record.clone();
                for &(col_idx, ref new_val) in assignments {
                    if col_idx < record.len() {
                        record[col_idx] = new_val.clone();
                    }
                }
                if let Some(log) = self.tx_log.as_mut() {
                    log.updated.push((table.to_string(), prior, record.clone()));
                }
                count += 1;
            }
        }

        Ok(count)
    }
    fn create_table(&mut self, info: &TableInfo) -> SqlResult<()> {
        self.table_infos
            .insert(info.name.to_lowercase(), info.clone());
        self.tables
            .entry(info.name.to_lowercase())
            .or_default();
        Ok(())
    }

    fn create_database(&mut self, db_name: &str) -> SqlResult<()> {
        // 内存模式: 仅记录数据库名
        self.databases.insert(db_name.to_string());
        Ok(())
    }

    fn drop_database(&mut self, db_name: &str) -> SqlResult<()> {
        self.databases.remove(db_name);
        Ok(())
    }

    fn drop_table(&mut self, table: &str) -> SqlResult<()> {
        self.tables.remove(&table.to_lowercase());
        self.table_infos.remove(table);
        Ok(())
    }

    fn get_table_info(&self, table: &str) -> SqlResult<TableInfo> {
        self.table_infos
            .get(table)
            .cloned()
            .ok_or_else(|| SqlError::ExecutionError(format!("Table not found: {}", table)))
    }

    fn has_table(&self, table: &str) -> bool {
        self.table_infos.contains_key(&table.to_lowercase())
    }

    fn list_tables(&self) -> Vec<String> {
        self.table_infos.keys().cloned().collect()
    }

    fn create_index(&mut self, _table: &str, _column: &str, _column_index: usize) -> SqlResult<()> {
        Ok(())
    }

    fn drop_index(&mut self, _table: &str, _column: &str) -> SqlResult<()> {
        Ok(())
    }

    fn add_column(&mut self, table: &str, column: ColumnDefinition) -> SqlResult<()> {
        if let Some(info) = self.table_infos.get_mut(&table.to_lowercase()) {
            info.columns.push(column);
            Ok(())
        } else {
            Err(SqlError::ExecutionError(format!(
                "Cannot add column: table {} not found",
                table
            )))
        }
    }

    fn rename_table(&mut self, table: &str, new_name: &str) -> SqlResult<()> {
        let table_key = table.to_lowercase();
        let info = self.table_infos.remove(&table_key);
        let records = self.tables.remove(&table_key);
        if let (Some(info), Some(records)) = (info, records) {
            let mut new_info = info;
            new_info.name = new_name.to_lowercase();
            self.table_infos.insert(new_name.to_lowercase(), new_info);
            self.tables.insert(new_name.to_lowercase(), records);
            Ok(())
        } else {
            Err(SqlError::ExecutionError(format!(
                "Cannot rename table: table {} not found",
                table
            )))
        }
    }

    fn create_trigger(&mut self, info: TriggerInfo) -> SqlResult<()> {
        self.triggers.insert(info.name.clone(), info);
        Ok(())
    }

    fn drop_trigger(&mut self, name: &str) -> SqlResult<()> {
        self.triggers
            .remove(name)
            .map(|_| ())
            .ok_or_else(|| SqlError::ExecutionError(format!("Trigger not found: {}", name)))
    }

    fn get_trigger(&self, name: &str) -> Option<TriggerInfo> {
        self.triggers.get(name).cloned()
    }

    fn list_triggers(&self, table: &str) -> Vec<TriggerInfo> {
        self.triggers
            .values()
            .filter(|t| t.table_name == table)
            .cloned()
            .collect()
    }

    fn has_view(&self, name: &str) -> bool {
        self.views.contains(name)
    }

    // === Sequence support (F-30) ===

    fn create_sequence(&mut self, seq: SequenceInfo) -> SqlResult<()> {
        if self.sequences.contains_key(&seq.name) {
            return Err(SqlError::ExecutionError(format!(
                "Sequence '{}' already exists",
                seq.name
            )));
        }
        self.sequences.insert(seq.name.clone(), seq);
        Ok(())
    }

    fn drop_sequence(&mut self, name: &str) -> SqlResult<()> {
        self.sequences
            .remove(name)
            .ok_or_else(|| SqlError::ExecutionError(format!("Sequence '{}' not found", name)))?;
        Ok(())
    }

    fn next_sequence_value(&mut self, name: &str) -> SqlResult<i64> {
        let seq = self
            .sequences
            .get_mut(name)
            .ok_or_else(|| SqlError::ExecutionError(format!("Sequence '{}' not found", name)))?;
        let next = seq.current_value + seq.increment_by;
        if next > seq.maxvalue {
            if seq.cycle {
                seq.current_value = seq.minvalue;
                return Ok(seq.minvalue);
            } else {
                return Err(SqlError::ExecutionError(format!(
                    "Sequence '{}' exhausted: next value {} exceeds MAXVALUE {}",
                    name, next, seq.maxvalue
                )));
            }
        }
        if next < seq.minvalue {
            if seq.cycle {
                seq.current_value = seq.maxvalue;
                return Ok(seq.maxvalue);
            } else {
                return Err(SqlError::ExecutionError(format!(
                    "Sequence '{}' exhausted: next value {} below MINVALUE {}",
                    name, next, seq.minvalue
                )));
            }
        }
        seq.current_value = next;
        Ok(next)
    }

    fn current_sequence_value(&self, name: &str) -> SqlResult<i64> {
        let seq = self
            .sequences
            .get(name)
            .ok_or_else(|| SqlError::ExecutionError(format!("Sequence '{}' not found", name)))?;
        Ok(seq.current_value)
    }

    fn has_sequence(&self, name: &str) -> bool {
        self.sequences.contains_key(name)
    }

    fn list_sequences(&self) -> Vec<String> {
        self.sequences.keys().cloned().collect()
    }

    fn get_sequence(&self, name: &str) -> Option<SequenceInfo> {
        self.sequences.get(name).cloned()
    }
    fn list_indexes(&self, _table: &str) -> Vec<(String, String)> {
        Vec::new()
    }

    fn is_wal_enabled(&self) -> bool {
        true
    }

    fn in_transaction(&self) -> bool {
        self.current_tx_id != 0
    }

    fn current_tx_id(&self) -> u64 {
        self.current_tx_id
    }

    fn set_current_tx_id(&mut self, id: u64) {
        self.current_tx_id = id;
    }

    fn drop_column(&mut self, table: &str, column: &str) -> SqlResult<()> {
        let table_key = table.to_lowercase();
        let info = self
            .table_infos
            .get_mut(&table_key)
            .ok_or_else(|| SqlError::ExecutionError(format!("Table not found: {}", table)))?;
        let col_idx = info
            .columns
            .iter()
            .position(|c| c.name.to_lowercase() == column.to_lowercase())
            .ok_or_else(|| SqlError::ExecutionError(format!("Column not found: {}", column)))?;
        info.columns.remove(col_idx);
        if let Some(records) = self.tables.get_mut(&table_key) {
            for record in records.iter_mut() {
                if col_idx < record.len() {
                    record.remove(col_idx);
                }
            }
        }
        Ok(())
    }

    fn modify_column(
        &mut self,
        table: &str,
        column: &str,
        new_def: ColumnDefinition,
    ) -> SqlResult<()> {
        let info = self
            .table_infos
            .get_mut(&table.to_lowercase())
            .ok_or_else(|| SqlError::ExecutionError(format!("Table not found: {}", table)))?;
        let col_idx = info
            .columns
            .iter()
            .position(|c| c.name.to_lowercase() == column.to_lowercase())
            .ok_or_else(|| SqlError::ExecutionError(format!("Column not found: {}", column)))?;
        info.columns[col_idx] = new_def;
        Ok(())
    }

    fn rename_column(&mut self, table: &str, old_name: &str, new_name: &str) -> SqlResult<()> {
        let info = self
            .table_infos
            .get_mut(&table.to_lowercase())
            .ok_or_else(|| SqlError::ExecutionError(format!("Table not found: {}", table)))?;
        let col = info
            .columns
            .iter_mut()
            .find(|c| c.name.to_lowercase() == old_name.to_lowercase())
            .ok_or_else(|| SqlError::ExecutionError(format!("Column not found: {}", old_name)))?;
        col.name = new_name.to_string();
        Ok(())
    }
    fn parallel_scan(
        &self,
        table: &str,
        num_partitions: usize,
    ) -> SqlResult<Vec<Box<dyn Iterator<Item = Record> + Send>>> {
        let data = self
            .tables
            .get(&table.to_lowercase())
            .ok_or_else(|| SqlError::ExecutionError(format!("Table not found: {}", table)))?;
        let total = data.len();
        if total == 0 || num_partitions == 0 {
            return Ok(vec![]);
        }
        let num_partitions = num_partitions.min(total);
        let base = total / num_partitions;
        let rem = total % num_partitions;
        let mut partitions: Vec<Box<dyn Iterator<Item = Record> + Send>> =
            Vec::with_capacity(num_partitions);
        let mut cur = 0;
        // v3.10.0 Issue #3776 / F-36: Arc-shared, no per-partition Vec clone
        let shared: Arc<Vec<Record>> = Arc::new((*data).clone());
        for i in 0..num_partitions {
            let size = if i < rem { base + 1 } else { base };
            if size > 0 {
                let part = Arc::clone(&shared);
                partitions.push(Box::new(SharedSliceIter::new(part, cur, cur + size)));
            }
            cur += size;
        }
        Ok(partitions)
    }
}

/// Zero-copy iterator over a contiguous slice of an `Arc<Vec<Record>>`.
/// Each `next()` returns a row clone but no per-partition Vec deep copy.
#[derive(Debug, Clone)]
pub struct SharedSliceIter {
    data: Arc<Vec<Record>>,
    pos: usize,
    end: usize,
}

impl SharedSliceIter {
    pub fn new(data: Arc<Vec<Record>>, start: usize, end: usize) -> Self {
        debug_assert!(start <= end);
        debug_assert!(end <= data.len());
        Self {
            data,
            pos: start,
            end,
        }
    }

    pub fn remaining(&self) -> usize {
        self.end.saturating_sub(self.pos)
    }

    pub fn strong_count(&self) -> usize {
        Arc::strong_count(&self.data)
    }
}

impl Iterator for SharedSliceIter {
    type Item = Record;
    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < self.end {
            let r = self.data[self.pos].clone();
            self.pos += 1;
            Some(r)
        } else {
            None
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let rem = self.remaining();
        (rem, Some(rem))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that StorageEngine trait is defined correctly
    #[test]
    fn test_storage_engine_trait_exists() {
        fn _check_trait(_engine: &dyn StorageEngine) {}
    }

    #[test]
    fn test_memory_storage_new() {
        let storage = MemoryStorage::new();
        assert!(storage.list_tables().is_empty());
    }

    #[test]
    fn test_memory_storage_has_table() {
        let storage = MemoryStorage::new();
        assert!(!storage.has_table("users"));
    }

    #[test]
    fn test_memory_storage_create_and_drop() {
        let mut storage = MemoryStorage::new();
        let info = TableInfo {
            name: "users".to_string(),
            columns: vec![],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            partition_info: None,
            compression: None,
        };
        storage.create_table(&info).unwrap();
        let tables = storage.list_tables();
        assert!(tables.contains(&"users".to_string()));
    }

    #[test]
    fn test_memory_storage_scan_empty() {
        let mut storage = MemoryStorage::new();
        storage.tables.insert("users".to_string(), vec![]);
        let result = storage.scan("users").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_memory_storage_insert_and_scan() {
        let mut storage = MemoryStorage::new();
        storage.tables.insert(
            "users".to_string(),
            vec![
                vec![Value::Integer(1), Value::Text("Alice".to_string())],
                vec![Value::Integer(2), Value::Text("Bob".to_string())],
            ],
        );
        let result = storage.scan("users").unwrap();
        assert_eq!(result.len(), 2);
    }

    /// Regression test for Issue #3276 (Sprint 1.5 cell-level diff).
    /// Ensures the public `insert()` API preserves the Value type
    /// (Integer vs Float vs Text) for round-trip retrieval. This is the
    /// storage-layer contract that the SUM(REAL)=0 fix depends on:
    /// a `Value::Float(100.5)` written via insert() MUST come back as
    /// `Value::Float(100.5)` (not coerced to Integer or Null).
    #[test]
    fn test_memory_storage_insert_scan_preserves_real_type() {
        let mut storage = MemoryStorage::new();
        storage.tables.insert(
            "lineitem".to_string(),
            vec![
                vec![Value::Integer(10), Value::Float(100.5), Value::Float(0.05)],
                vec![Value::Integer(20), Value::Float(200.5), Value::Float(0.10)],
                vec![Value::Integer(30), Value::Float(300.5), Value::Float(0.05)],
            ],
        );
        let result = storage.scan("lineitem").unwrap();
        assert_eq!(result.len(), 3);
        for (i, row) in result.iter().enumerate() {
            assert!(
                matches!(row[0], Value::Integer(_)),
                "row[{}] col 0 (q INTEGER) must remain Integer, got {:?}",
                i,
                row[0]
            );
            assert!(
                matches!(row[1], Value::Float(_)),
                "row[{}] col 1 (p REAL) must remain Float, got {:?}",
                i,
                row[1]
            );
            assert!(
                matches!(row[2], Value::Float(_)),
                "row[{}] col 2 (d REAL) must remain Float, got {:?}",
                i,
                row[2]
            );
        }
        assert_eq!(result[0][1], Value::Float(100.5));
        assert_eq!(result[2][1], Value::Float(300.5));
    }

    #[test]
    fn test_storage_engine_send_sync() {
        fn _check<T: Send + Sync>() {}
        _check::<MemoryStorage>();
    }

    #[test]
    fn test_memory_storage_is_wal_enabled() {
        let storage = MemoryStorage::new();
        assert!(storage.is_wal_enabled());
    }

    #[test]
    fn test_storage_engine_create_and_drop_table() {
        let mut storage = MemoryStorage::new();
        let info = TableInfo {
            name: "users".to_string(),
            columns: vec![],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            partition_info: None,
            compression: None,
        };

        storage.create_table(&info).unwrap();
        assert!(storage.has_table("users"));
        assert_eq!(storage.list_tables(), vec!["users"]);

        storage.drop_table("users").unwrap();
        assert!(!storage.has_table("users"));
    }

    #[test]
    fn test_storage_engine_get_table_info() {
        let mut storage = MemoryStorage::new();
        let info = TableInfo {
            name: "users".to_string(),
            columns: vec![ColumnDefinition {
                name: "id".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: true,
                char_max_length: None,
            }],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            partition_info: None,
            compression: None,
        };

        storage.create_table(&info).unwrap();
        let retrieved = storage.get_table_info("users").unwrap();
        assert_eq!(retrieved.name, "users");
        assert_eq!(retrieved.columns.len(), 1);
    }

    #[test]
    fn test_storage_engine_insert_records() {
        let mut storage = MemoryStorage::new();
        storage.tables.insert("users".to_string(), vec![]);

        storage
            .insert("users", vec![vec![Value::Integer(1)]])
            .unwrap();
        let records = storage.scan("users").unwrap();
        assert_eq!(records.len(), 1);
    }

    #[test]
    fn test_storage_engine_delete_all() {
        let mut storage = MemoryStorage::new();
        storage.tables.insert(
            "users".to_string(),
            vec![vec![Value::Integer(1)], vec![Value::Integer(2)]],
        );

        let deleted = storage.delete("users", &[]).unwrap();
        assert_eq!(deleted, 2);
    }

    #[test]
    fn test_storage_engine_update_values() {
        let mut storage = MemoryStorage::new();
        storage.tables.insert(
            "users".to_string(),
            vec![vec![Value::Integer(1), Value::Text("Alice".to_string())]],
        );

        let updated = storage
            .update("users", &[], &[(1, Value::Text("Bob".to_string()))][..])
            .unwrap();
        assert_eq!(updated, 1);
    }

    #[test]
    fn test_storage_engine_table_operations() {
        let mut storage = MemoryStorage::new();
        let info1 = TableInfo {
            name: "users".to_string(),
            columns: vec![],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            partition_info: None,
            compression: None,
        };
        let info2 = TableInfo {
            name: "orders".to_string(),
            columns: vec![],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            partition_info: None,
            compression: None,
        };
        storage.create_table(&info1).unwrap();
        storage.create_table(&info2).unwrap();

        let tables = storage.list_tables();
        assert_eq!(tables.len(), 2);
        assert!(tables.contains(&"users".to_string()));
        assert!(tables.contains(&"orders".to_string()));
    }

    #[test]
    fn test_storage_engine_has_table_check() {
        let mut storage = MemoryStorage::new();
        assert!(!storage.has_table("users"));

        let info = TableInfo {
            name: "users".to_string(),
            columns: vec![],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            partition_info: None,
            compression: None,
        };
        storage.create_table(&info).unwrap();
        assert!(storage.has_table("users"));
    }

    #[test]
    fn test_storage_engine_table_not_found() {
        let storage = MemoryStorage::new();
        let result = storage.get_table_info("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_evaluate_sql_expression_integer_comparison() {
        let columns = vec!["age".to_string(), "name".to_string()];
        let record = vec![Value::Integer(25), Value::Text("Alice".to_string())];

        // age > 18
        let result = evaluate_sql_expression("age > 18", &columns, &record);
        assert!(result.is_ok());
        assert!(result.unwrap());

        // age >= 25
        let result = evaluate_sql_expression("age >= 25", &columns, &record);
        assert!(result.is_ok());
        assert!(result.unwrap());

        // age < 18
        let result = evaluate_sql_expression("age < 18", &columns, &record);
        assert!(result.is_ok());
        assert!(!result.unwrap());

        // age = 25
        let result = evaluate_sql_expression("age = 25", &columns, &record);
        assert!(result.is_ok());
        assert!(result.unwrap());

        // age <> 30
        let result = evaluate_sql_expression("age <> 30", &columns, &record);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_evaluate_sql_expression_boolean_and_logical_ops() {
        let columns = vec!["age".to_string(), "active".to_string()];
        let record = vec![Value::Integer(25), Value::Boolean(true)];

        // age > 18 AND active = true
        let result = evaluate_sql_expression("age > 18 AND active = true", &columns, &record);
        assert!(result.is_ok());
        assert!(result.unwrap());

        // age < 18 OR active = true
        let result = evaluate_sql_expression("age < 18 OR active = true", &columns, &record);
        assert!(result.is_ok());
        assert!(result.unwrap());

        // NOT active = false
        let result = evaluate_sql_expression("NOT active = false", &columns, &record);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_evaluate_sql_expression_null_handling() {
        let columns = vec!["age".to_string(), "email".to_string()];
        let record = vec![Value::Null, Value::Text("test@test.com".to_string())];

        // age IS NOT NULL should be false
        let result = evaluate_sql_expression("age IS NOT NULL", &columns, &record);
        assert!(result.is_ok());
        assert!(!result.unwrap());

        // email IS NOT NULL should be true
        let result = evaluate_sql_expression("email IS NOT NULL", &columns, &record);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_evaluate_sql_expression_text_comparison() {
        let columns = vec!["name".to_string()];
        let record = vec![Value::Text("Alice".to_string())];

        // name = 'Alice'
        let result = evaluate_sql_expression("name = 'Alice'", &columns, &record);
        assert!(result.is_ok());
        assert!(result.unwrap());

        // name <> 'Bob'
        let result = evaluate_sql_expression("name <> 'Bob'", &columns, &record);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_storage_update_if_signature() {
        let mut storage = MemoryStorage::new();
        let info = TableInfo {
            name: "users".to_string(),
            columns: vec![ColumnDefinition {
                name: "id".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: true,
                char_max_length: None,
            }],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            partition_info: None,
            compression: None,
        };
        storage.create_table(&info).unwrap();
        storage
            .insert("users", vec![vec![Value::Integer(1)]])
            .unwrap();

        let filter: RowFilter = Box::new(|row| row[0] == Value::Integer(1));
        let mutation = RowMutation::new(vec![(0, Value::Integer(99))], 0x1234);

        let affected = storage.update_if("users", &filter, &mutation).unwrap();
        assert_eq!(affected, 1);

        let records = storage.scan("users").unwrap();
        assert_eq!(records[0][0], Value::Integer(99));
    }

    // v3.10.0 Issue #3703: partition_rows

    #[test]
    fn test_partition_rows_below_threshold_returns_single_chunk() {
        let mut storage = MemoryStorage::new();
        storage.tables.insert(
            "t".to_string(),
            (0..100_000_i64).map(|i| vec![Value::Integer(i)]).collect(),
        );
        let parts = storage.partition_rows("t", 8);
        assert_eq!(parts.len(), 1, "below threshold should return 1 chunk");
        assert_eq!(parts[0].len(), 100_000);
    }

    #[test]
    fn test_partition_rows_above_threshold_splits_evenly() {
        let mut storage = MemoryStorage::new();
        storage.tables.insert(
            "t".to_string(),
            (0..600_000_i64).map(|i| vec![Value::Integer(i)]).collect(),
        );
        let parts = storage.partition_rows("t", 4);
        assert_eq!(parts.len(), 4);
        let total: usize = parts.iter().map(|p| p.len()).sum();
        assert_eq!(total, 600_000);
        for p in &parts {
            assert_eq!(p.len(), 150_000);
        }
    }

    #[test]
    fn test_partition_rows_uneven_remainder() {
        let mut storage = MemoryStorage::new();
        storage.tables.insert(
            "t".to_string(),
            (0..503_003_i64).map(|i| vec![Value::Integer(i)]).collect(),
        );
        let parts = storage.partition_rows("t", 4);
        assert_eq!(parts.len(), 4);
        let total: usize = parts.iter().map(|p| p.len()).sum();
        assert_eq!(total, 503_003);
        assert_eq!(parts[0].len(), 125_751);
        assert_eq!(parts[1].len(), 125_751);
        assert_eq!(parts[2].len(), 125_751);
        assert_eq!(parts[3].len(), 125_750);
    }

    #[test]
    fn test_partition_rows_missing_table() {
        let storage = MemoryStorage::new();
        let parts = storage.partition_rows("does_not_exist", 4);
        assert_eq!(parts.len(), 1);
        assert!(parts[0].is_empty());
        let info = PartitionInfo::new_range("id", vec![Value::Integer(10), Value::Integer(20)]);
        assert_eq!(info.partition_type, PartitionType::Range);
        assert_eq!(info.column, "id");
    }

    #[test]
    fn test_partition_info_new_list() {
        let info = PartitionInfo::new_list(
            "region",
            vec![Value::Text("US".into()), Value::Text("EU".into())],
        );
        assert_eq!(info.partition_type, PartitionType::List);
    }

    #[test]
    fn test_partition_info_new_hash() {
        let info = PartitionInfo::new_hash("id", 4);
        assert_eq!(info.partition_type, PartitionType::Hash);
        assert_eq!(info.boundaries, vec![Value::Integer(4)]);
    }

    #[test]
    fn test_partition_range_index() {
        let info = PartitionInfo::new_range("id", vec![Value::Integer(10), Value::Integer(20)]);
        assert_eq!(info.get_partition_index(&Value::Integer(5)), Some(0));
        assert_eq!(info.get_partition_index(&Value::Integer(15)), Some(1));
        assert_eq!(info.get_partition_index(&Value::Integer(25)), Some(2));
    }

    #[test]
    fn test_partition_range_invalid_value() {
        let info = PartitionInfo::new_range("id", vec![Value::Integer(10)]);
        assert_eq!(info.get_partition_index(&Value::Text("abc".into())), None);
    }

    #[test]
    fn test_partition_list_match() {
        let info = PartitionInfo::new_list(
            "region",
            vec![Value::Text("US".into()), Value::Text("EU".into())],
        );
        assert_eq!(info.get_partition_index(&Value::Text("US".into())), Some(0));
        assert_eq!(info.get_partition_index(&Value::Text("EU".into())), Some(1));
        assert_eq!(info.get_partition_index(&Value::Text("AS".into())), None);
    }

    #[test]
    fn test_partition_hash_integer() {
        let info = PartitionInfo::new_hash("id", 4);
        let idx = info.get_partition_index(&Value::Integer(42));
        assert!(idx.is_some());
        assert!(idx.unwrap() < 4);
    }

    #[test]
    fn test_partition_hash_text() {
        let info = PartitionInfo::new_hash("name", 3);
        let idx = info.get_partition_index(&Value::Text("test".into()));
        assert!(idx.is_some());
        assert!(idx.unwrap() < 3);
    }

    #[test]
    fn test_partition_hash_invalid() {
        let info = PartitionInfo::new_hash("id", 4);
        assert_eq!(info.get_partition_index(&Value::Null), None);
        assert_eq!(info.get_partition_index(&Value::Boolean(true)), None);
    }

    #[test]
    fn test_evaluate_check_constraint_eq() {
        let constraint = CheckConstraint {
            name: Some("c1".into()),
            expression: "x = 5".into(),
        };
        let cols = vec!["x".to_string()];
        assert!(evaluate_check_constraint(&constraint, &cols, &vec![Value::Integer(5)]).unwrap());
        assert!(!evaluate_check_constraint(&constraint, &cols, &vec![Value::Integer(10)]).unwrap());
    }

    #[test]
    fn test_evaluate_check_constraint_and() {
        let constraint = CheckConstraint {
            name: Some("c1".into()),
            expression: "x > 0 AND y < 100".into(),
        };
        let cols = vec!["x".into(), "y".into()];
        assert!(evaluate_check_constraint(
            &constraint,
            &cols,
            &vec![Value::Integer(5), Value::Integer(50)]
        )
        .unwrap());
        assert!(!evaluate_check_constraint(
            &constraint,
            &cols,
            &vec![Value::Integer(-1), Value::Integer(50)]
        )
        .unwrap());
        assert!(!evaluate_check_constraint(
            &constraint,
            &cols,
            &vec![Value::Integer(5), Value::Integer(150)]
        )
        .unwrap());
    }

    #[test]
    fn test_evaluate_check_constraint_or() {
        let constraint = CheckConstraint {
            name: Some("c1".into()),
            expression: "x = 0 OR y = 0".into(),
        };
        let cols = vec!["x".into(), "y".into()];
        assert!(evaluate_check_constraint(
            &constraint,
            &cols,
            &vec![Value::Integer(0), Value::Integer(50)]
        )
        .unwrap());
        assert!(evaluate_check_constraint(
            &constraint,
            &cols,
            &vec![Value::Integer(5), Value::Integer(0)]
        )
        .unwrap());
        assert!(!evaluate_check_constraint(
            &constraint,
            &cols,
            &vec![Value::Integer(5), Value::Integer(50)]
        )
        .unwrap());
    }

    #[test]
    fn test_evaluate_check_constraint_not() {
        let constraint = CheckConstraint {
            name: Some("c1".into()),
            expression: "NOT x = 5".into(),
        };
        let cols = vec!["x".to_string()];
        assert!(evaluate_check_constraint(&constraint, &cols, &vec![Value::Integer(10)]).unwrap());
        assert!(!evaluate_check_constraint(&constraint, &cols, &vec![Value::Integer(5)]).unwrap());
    }

    #[test]
    fn test_evaluate_check_constraint_is_null() {
        let constraint = CheckConstraint {
            name: Some("c1".into()),
            expression: "x IS NULL".into(),
        };
        let cols = vec!["x".to_string()];
        assert!(evaluate_check_constraint(&constraint, &cols, &vec![Value::Null]).unwrap());
        assert!(!evaluate_check_constraint(&constraint, &cols, &vec![Value::Integer(0)]).unwrap());
    }

    #[test]
    fn test_evaluate_check_constraint_column_missing() {
        let constraint = CheckConstraint {
            name: Some("c1".into()),
            expression: "x IS NULL".into(),
        };
        let cols = vec![];
        let rec = vec![];
        assert!(evaluate_check_constraint(&constraint, &cols, &rec).is_ok());
    }

    #[test]
    fn test_calculate_hash_simple() {
        let h = calculate_hash(b"hello");
        let h2 = calculate_hash(b"hello");
        assert_eq!(h, h2);
        let h3 = calculate_hash(b"world");
        assert_ne!(h, h3);
    }

    #[test]
    fn test_calculate_hash_empty() {
        assert_eq!(calculate_hash(b""), 0);
    }

    #[test]
    fn test_partition_type_eq() {
        assert_eq!(PartitionType::Range, PartitionType::Range);
        assert_ne!(PartitionType::Range, PartitionType::List);
    }

    #[test]
    fn test_foreign_key_struct() {
        let fk = ForeignKeyConstraint {
            name: Some("fk1".into()),
            columns: vec!["a".into()],
            referenced_table: "b".into(),
            referenced_columns: vec!["id".into()],
            on_delete: None,
            on_update: None,
        };
        assert_eq!(fk.referenced_table, "b");
    }

    #[test]
    fn test_unique_constraint_struct() {
        let uc = UniqueConstraint {
            name: Some("uq1".into()),
            columns: vec!["x".into(), "y".into()],
        };
        assert_eq!(uc.columns.len(), 2);
    }

    #[test]
    fn test_check_constraint_struct() {
        let cc = CheckConstraint {
            name: Some("ck1".into()),
            expression: "x > 0".into(),
        };
        assert_eq!(cc.name, Some("ck1".into()));
        assert_eq!(cc.expression, "x > 0");
    }

    #[test]
    fn test_column_definition_new() {
        let cd = ColumnDefinition::new("age", "INTEGER");
        assert_eq!(cd.name, "age");
        assert_eq!(cd.data_type, "INTEGER");
        assert!(!cd.nullable);
        assert!(!cd.primary_key);
    }

    #[test]
    fn test_row_mutation_accessors() {
        let m = RowMutation::new(vec![(0, Value::Integer(1))], 42);
        assert_eq!(m.assignments().len(), 1);
        assert_eq!(m.mutation_hash(), 42);
    }

    #[test]
    fn test_trigger_info_struct() {
        let ti = TriggerInfo {
            name: "trig1".into(),
            table_name: "users".into(),
            timing: TriggerTiming::Before,
            event: TriggerEvent::Insert,
            body: "BEGIN UPDATE stats SET count = count + 1; END".into(),
        };
        assert_eq!(ti.name, "trig1");
        assert_eq!(ti.table_name, "users");
    }

    #[test]
    fn test_table_data_default() {
        let mut s = MemoryStorage::new();
        let info = TableInfo {
            name: "t".into(),
            ..Default::default()
        };
        s.create_table(&info).unwrap();
        assert!(s.has_table("t"));
    }

    #[test]
    fn test_storage_engine_force_insert() {
        let mut s = MemoryStorage::new();
        s.force_insert("t", vec![Value::Integer(1)]).unwrap();
        let rows = s.scan("t").unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn test_storage_engine_modify_column_unsupported() {
        let mut s = MemoryStorage::new();
        let col = ColumnDefinition::new("a", "INTEGER");
        let result = s.modify_column("t", "a", col);
        assert!(result.is_err());
    }

    #[test]
    fn test_storage_engine_get_trigger_none() {
        let s = MemoryStorage::new();
        assert!(s.get_trigger("nonexistent").is_none());
    }

    #[test]
    fn test_storage_engine_list_triggers_empty() {
        let s = MemoryStorage::new();
        assert!(s.list_triggers("t").is_empty());
    }

    #[test]
    fn test_storage_engine_list_indexes_empty() {
        let s = MemoryStorage::new();
        assert!(s.list_indexes("t").is_empty());
    }

    #[test]
    fn test_storage_engine_has_view_false() {
        let s = MemoryStorage::new();
        assert!(!s.has_view("v"));
    }

    #[test]
    fn test_storage_engine_transaction() {
        let mut s = MemoryStorage::new();
        assert!(!s.in_transaction());
        assert_eq!(s.current_tx_id(), 0);
        let id = s.begin_transaction().unwrap();
        assert_eq!(id, 1);
        assert!(s.in_transaction());
        s.commit_transaction().unwrap();
        assert!(!s.in_transaction());
    }

    #[test]
    fn test_storage_engine_begin_rollback() {
        let mut s = MemoryStorage::new();
        let id = s.begin_transaction().unwrap();
        s.rollback_transaction().unwrap();
        assert_eq!(id, 1);
        assert!(!s.in_transaction());
    }

    #[test]
    fn test_storage_engine_set_current_tx_id() {
        let mut s = MemoryStorage::new();
        s.set_current_tx_id(42);
    }

    #[test]
    fn test_storage_engine_flush() {
        let mut s = MemoryStorage::new();
        s.flush().unwrap();
    }

    #[test]
    fn test_partition_rows_num_partitions_zero() {
        let mut storage = MemoryStorage::new();
        storage.tables.insert(
            "t".to_string(),
            (0..600_000_i64).map(|i| vec![Value::Integer(i)]).collect(),
        );
        let parts = storage.partition_rows("t", 0);
        assert_eq!(parts.len(), 1, "n_partitions should be clamped to 1");
        assert_eq!(parts[0].len(), 600_000);
    }

    #[test]
    fn test_partition_rows_num_partitions_one_large() {
        let mut storage = MemoryStorage::new();
        storage.tables.insert(
            "t".to_string(),
            (0..600_000_i64).map(|i| vec![Value::Integer(i)]).collect(),
        );
        let parts = storage.partition_rows("t", 1);
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].len(), 600_000);
    }

    #[test]
    fn test_partition_rows_above_threshold_2_partitions() {
        let mut storage = MemoryStorage::new();
        storage.tables.insert(
            "t".to_string(),
            (0..500_100_i64).map(|i| vec![Value::Integer(i)]).collect(),
        );
        let parts = storage.partition_rows("t", 2);
        assert_eq!(parts.len(), 2);
        let total: usize = parts.iter().map(|p| p.len()).sum();
        assert_eq!(total, 500_100);
    }

    #[test]
    fn test_nested_transaction_returns_error() {
        let mut s = MemoryStorage::new();
        s.begin_transaction().unwrap();
        let result = s.begin_transaction();
        assert!(result.is_err());
    }

    #[test]
    fn test_update_with_log_no_filter() {
        let mut s = MemoryStorage::new();
        s.tables.insert(
            "t".to_string(),
            vec![vec![Value::Integer(1)], vec![Value::Integer(2)]],
        );
        s.current_tx_id = 1;
        s.tx_log = Some(TxLog::default());
        let count = s.update("t", &[], &[(0, Value::Integer(99))]).unwrap();
        assert_eq!(count, 2);
        assert_eq!(s.tables["t"][0][0], Value::Integer(99));
    }

    #[test]
    fn test_update_with_log_with_filter() {
        let mut s = MemoryStorage::new();
        s.tables.insert(
            "t".to_string(),
            vec![vec![Value::Integer(1)], vec![Value::Integer(2)]],
        );
        s.current_tx_id = 1;
        s.tx_log = Some(TxLog::default());
        let count = s
            .update("t", &[Value::Integer(1)], &[(0, Value::Integer(99))])
            .unwrap();
        assert_eq!(count, 1);
        assert_eq!(s.tables["t"][0][0], Value::Integer(99));
        assert_eq!(s.tables["t"][1][0], Value::Integer(2));
    }

    #[test]
    fn test_update_nonexistent_table() {
        let mut s = MemoryStorage::new();
        let count = s
            .update(
                "nonexistent",
                &[Value::Integer(1)],
                &[(0, Value::Integer(99))],
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_update_if_with_log() {
        let mut s = MemoryStorage::new();
        s.tables.insert(
            "t".to_string(),
            vec![vec![Value::Integer(1)], vec![Value::Integer(2)]],
        );
        s.current_tx_id = 1;
        s.tx_log = Some(TxLog::default());
        let filter: RowFilter = Box::new(|row: &Record| row[0] == Value::Integer(2));
        let mutation = RowMutation::new(vec![(0, Value::Integer(88))], 0);
        let count = s.update_if("t", &filter, &mutation).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_update_if_nonexistent_table() {
        let mut s = MemoryStorage::new();
        let filter: RowFilter = Box::new(|_: &Record| true);
        let mutation = RowMutation::new(vec![(0, Value::Integer(0))], 0);
        let count = s.update_if("nonexistent", &filter, &mutation).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_insert_dedup_logic() {
        let mut s = MemoryStorage::new();
        s.tables
            .insert("t".to_string(), vec![vec![Value::Integer(1)]]);
        s.current_tx_id = 1;
        s.tx_log = Some(TxLog::default());
        s.tables.get_mut("t").unwrap().push(vec![Value::Integer(2)]);
    }

    #[test]
    fn test_rollback_removes_inserted_rows() {
        let mut s = MemoryStorage::new();
        s.tables
            .insert("t".to_string(), vec![vec![Value::Integer(1)]]);
        s.begin_transaction().unwrap();
        s.insert("t", vec![vec![Value::Integer(2)]]).unwrap();
        s.rollback_transaction().unwrap();
        let rows = s.scan("t").unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn test_rollback_restores_deleted_rows() {
        let mut s = MemoryStorage::new();
        s.tables.insert(
            "t".to_string(),
            vec![vec![Value::Integer(1)], vec![Value::Integer(2)]],
        );
        s.begin_transaction().unwrap();
        s.delete("t", &[Value::Integer(1)]).unwrap();
        s.rollback_transaction().unwrap();
        let rows = s.scan("t").unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_create_table_with_table_info() {
        let mut s = MemoryStorage::new();
        let info = TableInfo {
            name: "users".to_string(),
            columns: vec![ColumnDefinition::new("id", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            partition_info: None,
            compression: None,
        };
        s.create_table(&info).unwrap();
        assert!(s.has_table("users"));
    }

    #[test]
    fn test_drop_table_clears_infos() {
        let mut s = MemoryStorage::new();
        let info = TableInfo {
            name: "t".to_string(),
            columns: vec![],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            partition_info: None,
            compression: None,
        };
        s.create_table(&info).unwrap();
        s.drop_table("t").unwrap();
        assert!(!s.has_table("t"));
    }

    #[test]
    fn test_get_table_info_returns_clone() {
        let mut s = MemoryStorage::new();
        let info = TableInfo {
            name: "t".to_string(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            partition_info: None,
            compression: None,
        };
        s.create_table(&info).unwrap();
        let got = s.get_table_info("t").unwrap();
        assert_eq!(got.name, "t");
        assert_eq!(got.columns.len(), 1);
    }

    #[test]
    fn test_get_table_info_not_found_err() {
        let s = MemoryStorage::new();
        let result = s.get_table_info("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_list_tables_returns_names() {
        let mut s = MemoryStorage::new();
        for name in ["a", "b", "c"] {
            let info = TableInfo {
                name: name.to_string(),
                columns: vec![],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                partition_info: None,
                compression: None,
            };
            s.create_table(&info).unwrap();
        }
        let tables = s.list_tables();
        assert_eq!(tables.len(), 3);
    }

    #[test]
    fn test_create_drop_database_memory() {
        let mut s = MemoryStorage::new();
        s.create_database("db1").unwrap();
        s.drop_database("db1").unwrap();
    }

    #[test]
    fn test_add_column_to_existing_table() {
        let mut s = MemoryStorage::new();
        let info = TableInfo {
            name: "t".to_string(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            partition_info: None,
            compression: None,
        };
        s.create_table(&info).unwrap();
        s.add_column("t", ColumnDefinition::new("b", "TEXT"))
            .unwrap();
        let got = s.get_table_info("t").unwrap();
        assert_eq!(got.columns.len(), 2);
    }

    #[test]
    fn test_add_column_to_nonexistent_table_errs() {
        let mut s = MemoryStorage::new();
        let result = s.add_column("nonexistent", ColumnDefinition::new("a", "INTEGER"));
        assert!(result.is_err());
    }

    #[test]
    fn test_rename_table_success() {
        let mut s = MemoryStorage::new();
        let info = TableInfo {
            name: "old".to_string(),
            columns: vec![],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            partition_info: None,
            compression: None,
        };
        s.create_table(&info).unwrap();
        s.insert("old", vec![vec![Value::Integer(1)]]).unwrap();
        s.rename_table("old", "new").unwrap();
        assert!(!s.has_table("old"));
        assert!(s.has_table("new"));
        let rows = s.scan("new").unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn test_rename_table_not_found_errs() {
        let mut s = MemoryStorage::new();
        let result = s.rename_table("nonexistent", "new");
        assert!(result.is_err());
    }

    #[test]
    fn test_create_drop_trigger_memory() {
        let mut s = MemoryStorage::new();
        let trigger = TriggerInfo {
            name: "trig".to_string(),
            table_name: "t".to_string(),
            timing: crate::engine::TriggerTiming::Before,
            event: crate::engine::TriggerEvent::Insert,
            body: "".to_string(),
        };
        s.create_trigger(trigger).unwrap();
        assert!(s.get_trigger("trig").is_some());
        s.drop_trigger("trig").unwrap();
        assert!(s.get_trigger("trig").is_none());
    }

    #[test]
    fn test_drop_trigger_not_found_errs() {
        let mut s = MemoryStorage::new();
        let result = s.drop_trigger("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_has_view_default_false() {
        let s = MemoryStorage::new();
        assert!(!s.has_view("v"));
    }

    #[test]
    fn test_list_triggers_filters_by_table() {
        let mut s = MemoryStorage::new();
        let t1 = TriggerInfo {
            name: "a".to_string(),
            table_name: "users".to_string(),
            timing: crate::engine::TriggerTiming::Before,
            event: crate::engine::TriggerEvent::Insert,
            body: "".to_string(),
        };
        let t2 = TriggerInfo {
            name: "b".to_string(),
            table_name: "orders".to_string(),
            timing: crate::engine::TriggerTiming::After,
            event: crate::engine::TriggerEvent::Update,
            body: "".to_string(),
        };
        s.create_trigger(t1).unwrap();
        s.create_trigger(t2).unwrap();
        let users_triggers = s.list_triggers("users");
        let orders_triggers = s.list_triggers("orders");
        assert_eq!(users_triggers.len(), 1);
        assert_eq!(orders_triggers.len(), 1);
    }

    #[test]
    fn test_storage_engine_default_impl_trait_methods() {
        let mut s = MemoryStorage::new();
        s.flush().unwrap();
        assert!(s.is_wal_enabled());
        s.begin_transaction().unwrap();
        s.rollback_transaction().unwrap();
        s.begin_transaction().unwrap();
        s.commit_transaction().unwrap();
    }
}
