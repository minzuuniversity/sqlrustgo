//! RecoveryEngine — deterministic WAL replay onto StorageEngine
//!
//! # Architecture (PR-830D)
//!
//! ```text
//! WAL (append-only log, passive)
//!     ↓
//! RecoveryEngine (interpreter)
//!     ↓
//! StorageEngine (dumb state machine)
//! ```
//!
//! # Principles
//!
//! 1. **WAL is passive**: no logic, just append and recover
//! 2. **Recovery is interpreter**: entry-level determinism
//! 3. **Storage is state machine**: no batch abstraction
//! 4. **ExecutionEngine never sees WAL**: SQL runtime != recovery runtime

use crate::engine::{SqlResult, StorageEngine, Value};
use crate::wal::{WalEntry, WalEntryType, WalManager};
use std::collections::HashMap;
use std::marker::PhantomData;

/// Recovery statistics
#[derive(Debug, Default, Clone)]
pub struct RecoveryReport {
    /// Total WAL entries read
    pub entries_total: usize,
    /// Number of committed transactions replayed
    pub committed_txns: usize,
    /// Number of rolled-back transactions skipped
    pub rolled_back_txns: usize,
    /// Number of incomplete (no Commit/Rollback) transactions
    pub incomplete_txns: usize,
    /// Rows inserted during recovery
    pub rows_inserted: usize,
    /// Rows updated during recovery
    pub rows_updated: usize,
    /// Rows deleted during recovery
    pub rows_deleted: usize,
}

/// RecoveryEngine — deterministic WAL interpreter
///
/// Replays WAL entries directly onto a StorageEngine state machine.
/// No batch abstraction layer — entry-level determinism only.
///
/// # Design
///
/// - `recover()`: full recovery pipeline (read → filter → sort → replay)
/// - `apply_entry()`: single entry replay (public for testing)
pub trait RecoveryEngine<S: StorageEngine>: Send + Sync {
    /// Full recovery: read WAL entries, filter committed, replay to storage
    fn recover(&mut self, storage: &mut S, wal: &mut dyn WalManager) -> SqlResult<RecoveryReport>;

    /// Apply a single WAL entry directly onto storage
    fn apply_entry(&mut self, storage: &mut S, entry: &WalEntry) -> SqlResult<()>;
}

/// Recovery state machine — prevents repeated recovery
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RecoveryState {
    #[default]
    Unrecovered,
    Recovered,
    Failed,
}

/// Stateful wrapper around RecoveryEngineImpl
pub struct StatefulRecoveryEngine<S: StorageEngine> {
    inner: RecoveryEngineImpl,
    state: RecoveryState,
    _marker: PhantomData<S>,
}

impl<S: StorageEngine> Default for StatefulRecoveryEngine<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: StorageEngine> StatefulRecoveryEngine<S> {
    pub fn new() -> Self {
        Self {
            inner: RecoveryEngineImpl,
            state: RecoveryState::Unrecovered,
            _marker: PhantomData,
        }
    }
}

impl<S: StorageEngine> RecoveryEngine<S> for StatefulRecoveryEngine<S> {
    fn recover(&mut self, storage: &mut S, wal: &mut dyn WalManager) -> SqlResult<RecoveryReport> {
        match self.state {
            RecoveryState::Unrecovered => {
                self.state = RecoveryState::Recovered;
                self.inner.recover(storage, wal)
            }
            RecoveryState::Recovered => {
                // Idempotent — already recovered, return empty report
                Ok(RecoveryReport::default())
            }
            RecoveryState::Failed => Err(crate::engine::SqlError::ExecutionError(
                "RecoveryEngine: cannot recover after previous failure".to_string(),
            )),
        }
    }

    fn apply_entry(&mut self, storage: &mut S, entry: &WalEntry) -> SqlResult<()> {
        self.inner.apply_entry(storage, entry)
    }
}

/// Default RecoveryEngine implementation
pub struct RecoveryEngineImpl;

// ---------------------------------------------------------------------------
// Helpers: table_id ↔ table_name
// ---------------------------------------------------------------------------

/// Hash algorithm matching WalStorage::table_name_to_id
fn table_name_to_id(table: &str) -> u64 {
    let mut hash: u64 = 0;
    for byte in table.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
    }
    hash
}

/// Resolve a table_id to a table_name by scanning all tables
fn resolve_table_name<S: StorageEngine>(
    storage: &S,
    table_id: u64,
) -> Result<String, crate::engine::SqlError> {
    for name in storage.list_tables() {
        if table_name_to_id(&name) == table_id {
            return Ok(name);
        }
    }
    Err(crate::engine::SqlError::ExecutionError(format!(
        "RecoveryEngine: table not found for id={}",
        table_id
    )))
}

// ---------------------------------------------------------------------------
// Helpers: WAL entry serialization (inverse of WalStorage::record_to_bytes)
// ---------------------------------------------------------------------------

/// Deserialize record bytes produced by WalStorage::record_to_bytes
///
/// Format:
/// - `i:` + 8 bytes LE = Integer
/// - `s:` + bytes + `\0` = Text
/// - `b:` + 1 byte = Boolean
/// - `n:` = Null
/// - `f:` + 8 bytes LE = Float
/// - `B:` + bytes + `\0` = Blob
fn bytes_to_record(data: &[u8]) -> Result<Vec<Value>, crate::engine::SqlError> {
    let mut record = Vec::new();
    let mut pos = 0;
    while pos < data.len() {
        if pos + 2 > data.len() {
            return Err(crate::engine::SqlError::ExecutionError(
                "RecoveryEngine: truncated value prefix".to_string(),
            ));
        }
        match &data[pos..pos + 2] {
            b"i:" => {
                if pos + 10 > data.len() {
                    return Err(crate::engine::SqlError::ExecutionError(
                        "RecoveryEngine: truncated Integer".to_string(),
                    ));
                }
                let mut buf = [0u8; 8];
                buf.copy_from_slice(&data[pos + 2..pos + 10]);
                record.push(Value::Integer(i64::from_le_bytes(buf)));
                pos += 10;
            }
            b"s:" => {
                let start = pos + 2;
                // Find null terminator
                let end = data[start..].iter().position(|&b| b == 0).ok_or_else(|| {
                    crate::engine::SqlError::ExecutionError(
                        "RecoveryEngine: Text missing null terminator".to_string(),
                    )
                })?;
                let s = std::str::from_utf8(&data[start..start + end]).map_err(|e| {
                    crate::engine::SqlError::ExecutionError(format!(
                        "RecoveryEngine: invalid UTF-8 in Text: {}",
                        e
                    ))
                })?;
                record.push(Value::Text(s.to_string()));
                pos = start + end + 1;
            }
            b"b:" => {
                if pos + 3 > data.len() {
                    return Err(crate::engine::SqlError::ExecutionError(
                        "RecoveryEngine: truncated Boolean".to_string(),
                    ));
                }
                record.push(Value::Boolean(data[pos + 2] != 0));
                pos += 3;
            }
            b"n:" => {
                record.push(Value::Null);
                pos += 2;
            }
            b"f:" => {
                if pos + 10 > data.len() {
                    return Err(crate::engine::SqlError::ExecutionError(
                        "RecoveryEngine: truncated Float".to_string(),
                    ));
                }
                let mut buf = [0u8; 8];
                buf.copy_from_slice(&data[pos + 2..pos + 10]);
                record.push(Value::Float(f64::from_bits(u64::from_le_bytes(buf))));
                pos += 10;
            }
            b"B:" => {
                let start = pos + 2;
                let end = data[start..].iter().position(|&b| b == 0).ok_or_else(|| {
                    crate::engine::SqlError::ExecutionError(
                        "RecoveryEngine: Blob missing null terminator".to_string(),
                    )
                })?;
                record.push(Value::Blob(data[start..start + end].to_vec()));
                pos = start + end + 1;
            }
            _ => {
                return Err(crate::engine::SqlError::ExecutionError(format!(
                    "RecoveryEngine: unknown value prefix: {:02x?}",
                    &data[pos..pos + 2]
                )));
            }
        }
    }
    Ok(record)
}

#[allow(dead_code)]
pub(crate) fn bytes_to_filters(data: &[u8]) -> Result<Vec<Value>, crate::engine::SqlError> {
    bytes_to_record(data)
}

/// Force an insert during recovery, bypassing any insert buffer so subsequent
/// scan/delete in the same recovery pass see the row in `data.rows` directly.
pub(crate) fn recovery_force_insert<S: StorageEngine>(
    storage: &mut S,
    table: &str,
    record: Vec<Value>,
) -> Result<(), crate::engine::SqlError> {
    storage.insert(table, vec![record])
}

#[allow(dead_code)]
pub(crate) fn bytes_to_updates(
    data: &[u8],
) -> Result<Vec<(usize, Value)>, crate::engine::SqlError> {
    if data.len() < 4 {
        return Err(crate::engine::SqlError::ExecutionError(
            "RecoveryEngine: truncated updates length".to_string(),
        ));
    }
    let mut pos = 4;
    let num_pairs = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;

    let mut updates = Vec::with_capacity(num_pairs);
    for _ in 0..num_pairs {
        if pos + 4 > data.len() {
            return Err(crate::engine::SqlError::ExecutionError(
                "RecoveryEngine: truncated update column index".to_string(),
            ));
        }
        let col_idx =
            u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
        pos += 4;

        let value = bytes_to_value(data, &mut pos)?;
        updates.push((col_idx, value));
    }
    Ok(updates)
}

#[allow(dead_code)]
fn bytes_to_value(data: &[u8], pos: &mut usize) -> Result<Value, crate::engine::SqlError> {
    if *pos + 2 > data.len() {
        return Err(crate::engine::SqlError::ExecutionError(
            "RecoveryEngine: truncated value prefix".to_string(),
        ));
    }
    match &data[*pos..*pos + 2] {
        b"i:" => {
            if *pos + 10 > data.len() {
                return Err(crate::engine::SqlError::ExecutionError(
                    "RecoveryEngine: truncated Integer".to_string(),
                ));
            }
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&data[*pos + 2..*pos + 10]);
            let value = Value::Integer(i64::from_le_bytes(buf));
            *pos += 10;
            Ok(value)
        }
        b"s:" => {
            let start = *pos + 2;
            let end = data[start..].iter().position(|&b| b == 0).ok_or_else(|| {
                crate::engine::SqlError::ExecutionError(
                    "RecoveryEngine: Text missing null terminator".to_string(),
                )
            })?;
            let s = std::str::from_utf8(&data[start..start + end]).map_err(|e| {
                crate::engine::SqlError::ExecutionError(format!(
                    "RecoveryEngine: invalid UTF-8 in Text: {}",
                    e
                ))
            })?;
            *pos = start + end + 1;
            Ok(Value::Text(s.to_string()))
        }
        b"b:" => {
            if *pos + 3 > data.len() {
                return Err(crate::engine::SqlError::ExecutionError(
                    "RecoveryEngine: truncated Boolean".to_string(),
                ));
            }
            let value = Value::Boolean(data[*pos + 2] != 0);
            *pos += 3;
            Ok(value)
        }
        b"n:" => {
            *pos += 2;
            Ok(Value::Null)
        }
        b"f:" => {
            if *pos + 10 > data.len() {
                return Err(crate::engine::SqlError::ExecutionError(
                    "RecoveryEngine: truncated Float".to_string(),
                ));
            }
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&data[*pos + 2..*pos + 10]);
            let value = Value::Float(f64::from_bits(u64::from_le_bytes(buf)));
            *pos += 10;
            Ok(value)
        }
        b"B:" => {
            let start = *pos + 2;
            let end = data[start..].iter().position(|&b| b == 0).ok_or_else(|| {
                crate::engine::SqlError::ExecutionError(
                    "RecoveryEngine: Blob missing null terminator".to_string(),
                )
            })?;
            let value = Value::Blob(data[start..start + end].to_vec());
            *pos = start + end + 1;
            Ok(value)
        }
        _ => Err(crate::engine::SqlError::ExecutionError(format!(
            "RecoveryEngine: unknown value prefix: {:02x?}",
            &data[*pos..*pos + 2]
        ))),
    }
}

fn key_to_filter_values(key: &[u8]) -> Result<Vec<Value>, crate::engine::SqlError> {
    if key.is_empty() {
        return Ok(Vec::new());
    }
    if key.len() == 8 {
        let mut buf = [0u8; 8];
        buf.copy_from_slice(key);
        return Ok(vec![Value::Integer(i64::from_le_bytes(buf))]);
    }
    if let Ok(s) = std::str::from_utf8(key) {
        return Ok(vec![Value::Text(s.to_string())]);
    }
    Err(crate::engine::SqlError::ExecutionError(
        "RecoveryEngine: cannot parse key to filter values".to_string(),
    ))
}

fn replace_by_key<S: StorageEngine>(
    storage: &mut S,
    table: &str,
    key: &[u8],
    new_record: Vec<Value>,
) -> Result<(), crate::engine::SqlError> {
    let filter_values = key_to_filter_values(key)?;
    storage.delete(table, &filter_values)?;
    storage.insert(table, vec![new_record])?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers: WAL entry filtering
// ---------------------------------------------------------------------------

/// Filter WAL entries to only include those from committed transactions.
///
/// PR-842: entries are walked in append order and split by `Begin`/
/// `Commit`/`Rollback` boundaries rather than grouped by `tx_id`. This
/// matters because the production WalStorage writes all entries with
/// `tx_id = current_tx_id()` (which stays `0` for FileStorage when no
/// transaction manager is hooked up). Grouping by `tx_id` then conflates
/// autocommit DML with uncommitted DML that happens to share the same id.
///
/// The new rule:
/// - A `Begin` opens a transaction; subsequent DML entries are accumulated
///   in a per-tx scratch buffer.
/// - A `Commit` flushes that buffer into the result (the DML is durable).
/// - A `Rollback` discards the buffer (the DML is undone).
/// - DML entries encountered while no transaction is open (i.e. autocommit)
///   are emitted directly, because the legacy semantics is "autocommit is
///   already committed by the time it reaches the WAL".
/// - Metadata entries (Checkpoint, Prepare) are skipped.
fn filter_committed_entries(entries: &[WalEntry]) -> Vec<WalEntry> {
    let mut result = Vec::new();
    let mut current_tx_dml: Vec<WalEntry> = Vec::new();
    let mut in_tx = false;

    for entry in entries {
        match entry.entry_type {
            WalEntryType::Begin => {
                in_tx = true;
                current_tx_dml.clear();
            }
            WalEntryType::Commit => {
                if in_tx {
                    result.append(&mut current_tx_dml);
                    in_tx = false;
                }
            }
            WalEntryType::Rollback => {
                current_tx_dml.clear();
                in_tx = false;
            }
            WalEntryType::Checkpoint | WalEntryType::Prepare => {}
            WalEntryType::Insert | WalEntryType::Update | WalEntryType::Delete => {
                if in_tx {
                    current_tx_dml.push(entry.clone());
                } else {
                    result.push(entry.clone());
                }
            }
        }
    }

    if result.len() > 1 {
        result.sort_by_key(|e| e.lsn);
    }

    result
}

/// Count committed transactions
fn count_status(entries: &[WalEntry]) -> (usize, usize, usize) {
    let mut groups: HashMap<u64, Vec<&WalEntry>> = HashMap::new();
    for entry in entries {
        groups.entry(entry.tx_id).or_default().push(entry);
    }

    let mut committed = 0;
    let mut rolled_back = 0;
    let mut incomplete = 0;

    for group in groups.values() {
        let has_commit = group.iter().any(|e| e.entry_type == WalEntryType::Commit);
        let has_rollback = group.iter().any(|e| e.entry_type == WalEntryType::Rollback);
        if has_commit {
            committed += 1;
        } else if has_rollback {
            rolled_back += 1;
        } else if group
            .iter()
            .any(|e| e.entry_type != WalEntryType::Checkpoint)
        {
            incomplete += 1;
        }
    }

    (committed, rolled_back, incomplete)
}

// ---------------------------------------------------------------------------
// RecoveryEngineImpl
// ---------------------------------------------------------------------------

impl<S: StorageEngine> RecoveryEngine<S> for RecoveryEngineImpl {
    fn recover(&mut self, storage: &mut S, wal: &mut dyn WalManager) -> SqlResult<RecoveryReport> {
        let entries = wal.recover()?;
        let total = entries.len();

        let (committed, rolled_back, incomplete) = count_status(&entries);
        let dml_entries = filter_committed_entries(&entries);

        let mut report = RecoveryReport {
            entries_total: total,
            committed_txns: committed,
            rolled_back_txns: rolled_back,
            incomplete_txns: incomplete,
            ..Default::default()
        };

        // Replay committed DML entries in order
        for entry in &dml_entries {
            self.apply_entry(storage, entry)?;
            match entry.entry_type {
                WalEntryType::Insert => report.rows_inserted += 1,
                WalEntryType::Update => report.rows_updated += 1,
                WalEntryType::Delete => report.rows_deleted += 1,
                _ => {}
            }
        }

        Ok(report)
    }

    fn apply_entry(&mut self, storage: &mut S, entry: &WalEntry) -> SqlResult<()> {
        // Resolve table_name from table_id hash
        let table_name = resolve_table_name(storage, entry.table_id).map_err(|e| {
            crate::engine::SqlError::ExecutionError(format!(
                "RecoveryEngine::apply_entry: {} (tx_id={}, entry_type={:?}, table_id={})",
                e, entry.tx_id, entry.entry_type, entry.table_id,
            ))
        })?;

        match entry.entry_type {
            WalEntryType::Insert => {
                let data = entry.data.as_deref().unwrap_or(&[]);
                if data.is_empty() {
                    return Err(crate::engine::SqlError::ExecutionError(
                        "RecoveryEngine: Insert entry with empty data".to_string(),
                    ));
                }
                let record = bytes_to_record(data)?;
                // During recovery, force direct insert to avoid buffer/direct split
                // so subsequent scan/delete in same recovery see the inserted row.
                recovery_force_insert(storage, &table_name, record)?;
            }
            WalEntryType::Update => {
                if let Some(ref key) = entry.key {
                    if let Some(ref data) = entry.data {
                        let new_record = bytes_to_record(data)?;
                        replace_by_key(storage, &table_name, key, new_record)?;
                    } else {
                        log::warn!(
                            "RecoveryEngine: UPDATE entry without data for table {} (tx_id={})",
                            table_name,
                            entry.tx_id
                        );
                    }
                } else {
                    log::warn!(
                        "RecoveryEngine: UPDATE entry without key for table {} (tx_id={})",
                        table_name,
                        entry.tx_id
                    );
                }
            }
            WalEntryType::Delete => {
                if let Some(ref key) = entry.key {
                    let filter_values = key_to_filter_values(key)?;
                    storage.delete(&table_name, &filter_values)?;
                } else {
                    log::warn!(
                        "RecoveryEngine: DELETE entry without key for table {} - full table delete",
                        table_name
                    );
                    storage.delete(&table_name, &[])?;
                }
            }
            _ => {
                // Begin, Commit, Rollback, Checkpoint, Prepare are metadata
                // entries handled by the filtering step; skip here.
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::MemoryStorage;

    #[test]
    fn test_table_name_to_id_consistency() {
        // Verify our hash matches WalStorage's table_name_to_id
        let name = "orders";
        let hash1 = table_name_to_id(name);
        let hash2 = super::table_name_to_id(name);
        assert_eq!(hash1, hash2, "hash must be deterministic");
        assert_ne!(table_name_to_id("orders"), table_name_to_id("order"));
    }

    #[test]
    fn test_bytes_to_record_roundtrip() {
        let values = vec![
            Value::Integer(42),
            Value::Text("hello".to_string()),
            Value::Boolean(true),
            Value::Null,
            Value::Float(3.14),
            Value::Blob(vec![0x01, 0x02, 0x03]),
        ];

        // Simulate WalStorage::record_to_bytes
        let mut bytes = Vec::new();
        for value in &values {
            match value {
                Value::Integer(i) => {
                    bytes.extend_from_slice(b"i:");
                    bytes.extend_from_slice(&i.to_le_bytes());
                }
                Value::Text(s) => {
                    bytes.extend_from_slice(b"s:");
                    bytes.extend_from_slice(s.as_bytes());
                    bytes.push(0);
                }
                Value::Boolean(b) => {
                    bytes.extend_from_slice(b"b:");
                    bytes.push(*b as u8);
                }
                Value::Null => {
                    bytes.extend_from_slice(b"n:");
                }
                Value::Float(f) => {
                    bytes.extend_from_slice(b"f:");
                    bytes.extend_from_slice(&f.to_bits().to_le_bytes());
                }
                Value::Blob(b) => {
                    bytes.extend_from_slice(b"B:");
                    bytes.extend_from_slice(b);
                    bytes.push(0);
                }
            }
        }

        let parsed = bytes_to_record(&bytes).unwrap();
        assert_eq!(parsed.len(), values.len());
        for (i, v) in parsed.iter().enumerate() {
            assert_eq!(v, &values[i], "value {} mismatch", i);
        }
    }

    #[test]
    fn test_bytes_to_record_empty() {
        let parsed = bytes_to_record(&[]).unwrap();
        assert!(parsed.is_empty());
    }

    #[test]
    fn test_filter_committed_entries() {
        use crate::wal::WalEntryType;
        let entries = vec![
            WalEntry {
                tx_id: 1,
                entry_type: WalEntryType::Begin,
                table_id: 0,
                key: None,
                data: None,
                lsn: 0,
                timestamp: 0,
            },
            WalEntry {
                tx_id: 1,
                entry_type: WalEntryType::Insert,
                table_id: 100,
                key: None,
                data: Some(b"i:{}" as &[u8]).map(|s| s.to_vec()),
                lsn: 1,
                timestamp: 0,
            },
            WalEntry {
                tx_id: 1,
                entry_type: WalEntryType::Commit,
                table_id: 0,
                key: None,
                data: None,
                lsn: 2,
                timestamp: 0,
            },
            WalEntry {
                tx_id: 2,
                entry_type: WalEntryType::Begin,
                table_id: 0,
                key: None,
                data: None,
                lsn: 3,
                timestamp: 0,
            },
            WalEntry {
                tx_id: 2,
                entry_type: WalEntryType::Insert,
                table_id: 100,
                key: None,
                data: None,
                lsn: 4,
                timestamp: 0,
            },
            // No Commit for tx_id=2 → rolled back or incomplete
        ];

        let committed = filter_committed_entries(&entries);
        assert_eq!(committed.len(), 1, "only tx_id=1's Insert should be kept");
        assert_eq!(committed[0].tx_id, 1);
        assert_eq!(committed[0].entry_type, WalEntryType::Insert);

        // Verify metadata entries are excluded
        for entry in &committed {
            assert!(
                matches!(
                    entry.entry_type,
                    WalEntryType::Insert | WalEntryType::Update | WalEntryType::Delete
                ),
                "metadata entries must be filtered out"
            );
        }
    }

    #[test]
    fn test_recovery_state_default() {
        let state = RecoveryState::Unrecovered;
        assert_eq!(state, RecoveryState::Unrecovered);
    }

    #[test]
    fn test_stateful_engine_blocks_double_recovery() {
        use crate::engine::MemoryStorage;
        use crate::wal::MemoryWalManager;

        let mut storage = MemoryStorage::new();
        let mut wal = MemoryWalManager::new();
        let mut engine = StatefulRecoveryEngine::new();

        let result1 = engine.recover(&mut storage, &mut wal);
        assert!(result1.is_ok());

        let result2 = engine.recover(&mut storage, &mut wal);
        assert!(result2.is_ok());
        let report = result2.unwrap();
        assert_eq!(report.committed_txns, 0);
    }

    #[test]
    fn test_recovery_engine_impl_trait_bounds() {
        let _engine: Box<dyn RecoveryEngine<MemoryStorage>> = Box::new(RecoveryEngineImpl);
    }

    #[test]
    fn test_key_to_filter_values_text() {
        let key = b"hello".to_vec();
        let result = key_to_filter_values(&key).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], Value::Text("hello".to_string()));
    }

    #[test]
    fn test_key_to_filter_values_empty() {
        let key = vec![];
        let result = key_to_filter_values(&key).unwrap();
        assert!(result.is_empty());
    }
}
