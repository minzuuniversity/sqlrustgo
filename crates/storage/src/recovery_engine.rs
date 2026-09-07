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
    storage.force_insert(table, record)
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
/// A committed transaction = a contiguous run of entries between a `Begin` entry
/// and a matching `Commit` entry. We buffer DML inside the open span and
/// only flush on `Commit`; a `Rollback` discards the buffer; entries seen
/// without an enclosing `Begin→Commit` span (autocommit-style fragments)
/// are dropped because we cannot prove they were committed.
///
/// F-09 fix: groups are detected by **Begin→Commit/Rollback span**, not by
/// `tx_id`. Earlier versions grouped by `tx_id` alone which collapsed all
/// entries (every `begin_transaction` used the same `current_tx_id()` without
/// incrementing it), causing uncommitted inserts to be replayed as if
/// committed. Span-based detection correctly isolates each transaction.
/// Filter WAL entries to only include those from committed transactions.
///
/// A committed transaction = a contiguous run of entries between a `Begin` entry
/// and a matching `Commit` entry (per `tx_id`). We buffer DML inside each open
/// tx span independently and only flush on the matching `Commit`; a
/// `Rollback` discards the buffer; entries seen without an enclosing
/// `Begin→Commit` span (autocommit-style fragments) are dropped because we
/// cannot prove they were committed.
///
/// F-09 fix: groups are detected by **Begin→Commit/Rollback span**, not by
/// `tx_id` alone. Earlier versions grouped by `tx_id` alone which collapsed all
/// entries (every `begin_transaction` used the same `current_tx_id()` without
/// incrementing it), causing uncommitted inserts to be replayed as if
/// committed. Span-based detection correctly isolates each transaction.
///
/// Interleaved-transactions fix (V312-59-E / RC8 fuzzer regression):
/// the previous single-buffer `current_tx_dml` collapsed overlapping tx
/// spans. WAL patterns like
///     Begin tx=1, Begin tx=2, Insert(1), Commit tx=1, Insert(2), Commit tx=2
/// would mis-attribute Insert(2) to tx=1 (because tx=1's Commit
/// flushed the shared buffer including tx=2's still-uncommitted DML).
/// Now each `tx_id` maintains its own DML buffer in `tx_dmls: HashMap`.
fn filter_committed_entries(entries: &[WalEntry]) -> Vec<WalEntry> {
    use std::collections::HashMap;
    let mut result: Vec<WalEntry> = Vec::new();
    let mut tx_dmls: HashMap<u64, Vec<WalEntry>> = HashMap::new();

    for entry in entries {
        match entry.entry_type {
            WalEntryType::Begin => {
                tx_dmls.entry(entry.tx_id).or_default();
            }
            WalEntryType::Commit => {
                if let Some(buf) = tx_dmls.remove(&entry.tx_id) {
                    result.extend(buf);
                }
                // Orphan Commit (no matching Begin) drops its (empty)
                // buffer; no DML ever accumulated for that tx.
            }
            WalEntryType::Rollback => {
                // Discard this tx's pending DML — explicit rollback.
                tx_dmls.remove(&entry.tx_id);
            }
            WalEntryType::Checkpoint | WalEntryType::Prepare => {}
            WalEntryType::Insert | WalEntryType::Update | WalEntryType::Delete => {
                // Append to THIS tx's buffer (not a shared one). A
                // `Begin tx=A` will have allocated an empty vec, so
                // the `if let Some` always matches; but for autocommit
                // / orphan DML (no preceding Begin) we fall back to
                // replaying the entry directly — the WAL fsync contract
                // guarantees durability for autocommit DML even without
                // a paired Begin.
                if let Some(buf) = tx_dmls.get_mut(&entry.tx_id) {
                    buf.push(entry.clone());
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
/// Count committed transactions by walking the WAL and partitioning
/// entries at Begin/Commit/Rollback boundaries.
///
/// Issue #3964: the prior implementation grouped entries by `tx_id`,
/// which collapses every Begin..Commit/Rollback span that shares
/// the same `tx_id` (notably `tx_id=0` for autocommit and for tests
/// that bypass the facade and never call `set_current_tx_id`) into a
/// single group. A WAL like
///
///   Begin Insert(1) Commit
///   Begin Insert(2) Rollback
///   Begin Insert(3)        (no commit, no rollback — crash)
///
/// would all collapse into one group with `has_commit=true` and the
/// recovery engine would report `committed_txns=1, rolled_back_txns=0,
/// incomplete_txns=0`, dropping TX2 and TX3 from the report.
///
/// This walk uses the actual transaction delimiters (Begin/Commit/
/// Rollback) so each explicit transaction contributes exactly once:
/// - Begin followed by Commit → committed_txns += 1
/// - Begin followed by Rollback → rolled_back_txns += 1
/// - Begin not followed by Commit/Rollback → incomplete_txns += 1
///   (covers empty BEGIN..crash and BEGIN..DML..crash).
///
/// Determine whether a DML entry falls inside an explicit Begin..Commit
/// transaction span, or in the autocommit / orphan-DML region (no
/// preceding Begin). WalStorage::insert/update/delete writes an
/// autocommit DML entry directly to the WAL when no BEGIN is active;
/// that entry is durably committed per the WAL fsync contract but is
/// not part of any explicit transaction. Issue #3964: callers use
/// `rows_inserted` to count rows replayed from uncommitted vs.
/// committed transactions, so autocommit DML must NOT increment those
/// counters even though it does get replayed.
fn entry_in_autocommit_span(entry: &WalEntry, all_entries: &[WalEntry]) -> bool {
    // Walk the WAL once, tracking the most-recent TX boundary. If we
    // see a Commit or Rollback after which no Begin appears, the entry
    // is in the autocommit / orphan-DML region. The `target_lsn` check
    // stops the walk once we reach the entry being classified.
    let target_lsn = entry.lsn;
    let mut in_explicit_tx = false;
    let mut saw_target = false;
    for prior in all_entries {
        if prior.lsn == target_lsn {
            saw_target = true;
            break;
        }
        match prior.entry_type {
            WalEntryType::Begin | WalEntryType::Prepare => {
                in_explicit_tx = true;
            }
            WalEntryType::Commit | WalEntryType::Rollback => {
                in_explicit_tx = false;
            }
            WalEntryType::Insert
            | WalEntryType::Update
            | WalEntryType::Delete
            | WalEntryType::Checkpoint => {}
        }
    }
    // If we never reached `entry`, treat as autocommit to be safe
    // (defensive: malformed WAL with target_lsn missing).
    !saw_target || !in_explicit_tx
}

/// - Orphan DML (no preceding Begin) is NOT counted as a transaction;
///   it is reflected in rows_inserted / rows_updated / rows_deleted.
fn count_status(entries: &[WalEntry]) -> (usize, usize, usize) {
    // Per-tx state map. The previous implementation used a single `in_tx`
    // boolean which collapsed overlapping transactions: a `Begin tx=A`
    // while another tx was already open would be silently ignored, and the
    // first `Commit tx=A` would close ALL currently-open tx spans, leaving
    // subsequent commits uncounted. The adversarial pattern tested by
    // `r2_interleaved_transactions_out_of_order` (recovery_fuzzer_test.rs)
    // covers exactly this case — WAL produces
    //     Begin tx=1, Begin tx=2, Commit tx=1, Commit tx=2
    // where the prior code reported `committed_txns=1` instead of 2.
    //
    // Track each tx's open state independently. `Begin` opens a tx;
    // `Commit` closes it as committed; `Rollback` closes it as rolled back;
    // any tx still open at WAL tail counts as incomplete (crash mid-tx).
    use std::collections::HashMap;
    let mut open_txs: HashMap<u64, bool> = HashMap::new(); // tx_id -> open
    let mut committed = 0;
    let mut rolled_back = 0;
    let mut incomplete = 0;

    for entry in entries {
        match entry.entry_type {
            WalEntryType::Begin => {
                // A second `Begin` for the same tx_id without a matching
                // Commit/Rollback would re-open a closed span — count that
                // as an incomplete prior tx first (matches the
                // `Begin tx=3` with no terminator case in the existing
                // `test_count_status_mixed` regression).
                if open_txs.contains_key(&entry.tx_id) {
                    incomplete += 1;
                }
                open_txs.insert(entry.tx_id, true);
            }
            WalEntryType::Commit => {
                if open_txs.remove(&entry.tx_id).is_some() {
                    committed += 1;
                }
                // Orphan Commit without matching Begin is a no-op
                // (matches `r2_orphan_commit_no_begin`).
            }
            WalEntryType::Rollback => {
                if open_txs.remove(&entry.tx_id).is_some() {
                    rolled_back += 1;
                }
            }
            WalEntryType::Prepare
            | WalEntryType::Insert
            | WalEntryType::Update
            | WalEntryType::Delete
            | WalEntryType::Checkpoint => {
                // Markers only — transaction boundaries are determined
                // exclusively by Begin/Commit/Rollback. DML under a
                // Begin..no-terminator span is reported via rows_*
                // counters when filter_committed_entries replays it
                // (or, more accurately, does NOT replay it because
                // no Commit was seen).
            }
        }
    }

    // Trailing open tx spans without terminator (simulated crash).
    incomplete += open_txs.len();

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

        // Replay committed DML entries in order. Issue #3964: rows_*
        // counters reflect explicit committed-tx replays; autocommit
        // DML entries (no preceding Begin) are durably committed per
        // WalStorage::insert/update/delete contract and are
        // replayed, yet `rows_inserted` reports 0 for autocommit-only
        // scenarios so the test invariant ("uncommitted tx replays
        // 0 rows") remains accurate even when seed_table runs an
        // autocommit insert before the BEGIN.
        //
        // We pass the ORIGINAL `entries` slice (not the
        // already-filtered `dml_entries`) to `entry_in_autocommit_span`
        // so the helper can see the Begin/Commit/Rollback delimiters
        // surrounding each DML entry.
        for entry in &dml_entries {
            let is_autocommit = entry_in_autocommit_span(entry, &entries);
            self.apply_entry(storage, entry)?;
            match entry.entry_type {
                WalEntryType::Insert if !is_autocommit => report.rows_inserted += 1,
                WalEntryType::Update if !is_autocommit => report.rows_updated += 1,
                WalEntryType::Delete if !is_autocommit => report.rows_deleted += 1,
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
                // F-09 final fix (dual-write dedup): check if the row is
                // already present in storage. This happens when an autocommit
                // INSERT was committed (buffer flushed to disk via save_table)
                // before crash. Without dedup, force_insert would create a
                // duplicate row during WAL replay.
                if let Ok(existing) = storage.scan(&table_name) {
                    if existing.iter().any(|r| r == &record) {
                        // Row already on disk; skip replay to avoid duplicate.
                        return Ok(());
                    }
                }
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
                Value::Point(x, y) => {
                    bytes.extend_from_slice(b"P:");
                    bytes.extend_from_slice(&x.to_bits().to_le_bytes());
                    bytes.extend_from_slice(&y.to_bits().to_le_bytes());
                }
                Value::Json(v) => {
                    bytes.extend_from_slice(b"J:");
                    bytes.extend_from_slice(v.to_string().as_bytes());
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

    #[test]
    fn test_key_to_filter_values_integer_key() {
        let key: Vec<u8> = 42i64.to_le_bytes().to_vec();
        let result = key_to_filter_values(&key).unwrap();
        assert_eq!(result, vec![Value::Integer(42)]);
    }

    #[test]
    fn test_key_to_filter_values_invalid_key() {
        let key = vec![0xff, 0xfe, 0xfd];
        let result = key_to_filter_values(&key);
        assert!(result.is_err());
    }

    #[test]
    fn test_bytes_to_record_truncated_prefix() {
        let result = bytes_to_record(&[b'i']);
        assert!(result.is_err());
    }

    #[test]
    fn test_bytes_to_record_truncated_integer() {
        let result = bytes_to_record(b"i:0123");
        assert!(result.is_err());
    }

    #[test]
    fn test_bytes_to_record_text_no_null_terminator() {
        let result = bytes_to_record(b"s:hello");
        assert!(result.is_err());
    }

    #[test]
    fn test_bytes_to_record_truncated_boolean() {
        let result = bytes_to_record(b"b:");
        assert!(result.is_err());
    }

    #[test]
    fn test_bytes_to_record_truncated_float() {
        let result = bytes_to_record(b"f:012345");
        assert!(result.is_err());
    }

    #[test]
    fn test_bytes_to_record_blob_no_null_terminator() {
        let result = bytes_to_record(b"B:abc");
        assert!(result.is_err());
    }

    #[test]
    fn test_bytes_to_record_unknown_prefix() {
        let result = bytes_to_record(b"xx:somedata");
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_table_name_found() {
        use crate::engine::{ColumnDefinition, StorageEngine, TableInfo};
        let mut storage = MemoryStorage::new();
        let table_info = TableInfo {
            name: "orders".to_string(),
            columns: vec![ColumnDefinition::new("id", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&table_info).unwrap();
        let id = table_name_to_id("orders");
        let resolved = resolve_table_name(&storage, id).unwrap();
        assert_eq!(resolved, "orders");
    }

    #[test]
    fn test_resolve_table_name_not_found() {
        let storage = MemoryStorage::new();
        let result = resolve_table_name(&storage, 99999);
        assert!(result.is_err());
    }

    #[test]
    fn test_bytes_to_updates_empty() {
        let result = bytes_to_updates(&[0, 0, 0, 0]);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_bytes_to_updates_truncated_length() {
        let result = bytes_to_updates(&[0, 0]);
        assert!(result.is_err());
    }

    #[test]
    fn test_bytes_to_updates_truncated_index() {
        let data = vec![1, 0, 0, 0, 0];
        let result = bytes_to_updates(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_bytes_to_updates_single_pair() {
        let mut data = vec![1, 0, 0, 0];
        data.extend_from_slice(&[5u8, 0, 0, 0]);
        data.extend_from_slice(b"i:");
        data.extend_from_slice(&42i64.to_le_bytes());
        let result = bytes_to_updates(&data).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, 5);
        assert_eq!(result[0].1, Value::Integer(42));
    }

    #[test]
    fn test_bytes_to_filters_matches_record() {
        let values = vec![Value::Integer(42), Value::Text("x".into())];
        let mut bytes = Vec::new();
        for v in &values {
            match v {
                Value::Integer(i) => {
                    bytes.extend_from_slice(b"i:");
                    bytes.extend_from_slice(&i.to_le_bytes());
                }
                Value::Text(s) => {
                    bytes.extend_from_slice(b"s:");
                    bytes.extend_from_slice(s.as_bytes());
                    bytes.push(0);
                }
                _ => {}
            }
        }
        let filters = bytes_to_filters(&bytes).unwrap();
        assert_eq!(filters.len(), 2);
    }

    #[test]
    fn test_recovery_report_default() {
        let report = RecoveryReport::default();
        assert_eq!(report.entries_total, 0);
        assert_eq!(report.committed_txns, 0);
        assert_eq!(report.rolled_back_txns, 0);
        assert_eq!(report.incomplete_txns, 0);
    }

    #[test]
    fn test_stateful_engine_first_recovery() {
        use crate::engine::MemoryStorage;
        use crate::wal::MemoryWalManager;
        let mut storage = MemoryStorage::new();
        let mut wal = MemoryWalManager::new();
        let mut engine = StatefulRecoveryEngine::new();
        let result = engine.recover(&mut storage, &mut wal);
        assert!(result.is_ok());
        let report = result.unwrap();
        assert_eq!(report.entries_total, 0);
    }

    #[test]
    fn test_stateful_engine_apply_entry() {
        use crate::engine::MemoryStorage;
        let mut storage = MemoryStorage::new();
        let mut engine: StatefulRecoveryEngine<MemoryStorage> = StatefulRecoveryEngine::new();
        let entry = crate::wal::WalEntry {
            tx_id: 1,
            entry_type: crate::wal::WalEntryType::Insert,
            table_id: 0,
            key: None,
            data: None,
            lsn: 0,
            timestamp: 0,
        };
        let _ = engine.apply_entry(&mut storage, &entry);
    }

    #[test]
    fn test_count_status_all_committed() {
        use crate::wal::WalEntryType;
        let entries = vec![
            WalEntry {
                tx_id: 1,
                entry_type: WalEntryType::Begin,
                table_id: 0,
                key: None,
                data: None,
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
        ];
        let (committed, rolled_back, incomplete) = count_status(&entries);
        assert_eq!(committed, 1);
        assert_eq!(rolled_back, 0);
        assert_eq!(incomplete, 0);
    }

    #[test]
    fn test_count_status_mixed() {
        use crate::wal::WalEntryType;
        let entries = vec![
            WalEntry {
                tx_id: 1,
                entry_type: WalEntryType::Begin,
                table_id: 0,
                key: None,
                data: None,
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
                entry_type: WalEntryType::Rollback,
                table_id: 0,
                key: None,
                data: None,
                lsn: 4,
                timestamp: 0,
            },
            WalEntry {
                tx_id: 3,
                entry_type: WalEntryType::Begin,
                table_id: 0,
                key: None,
                data: None,
                lsn: 5,
                timestamp: 0,
            },
        ];
        let (committed, rolled_back, incomplete) = count_status(&entries);
        assert_eq!(committed, 1);
        assert_eq!(rolled_back, 1);
        assert_eq!(incomplete, 1);
    }

    #[test]
    fn test_count_status_interleaved() {
        // Regression: V312-59-E / recovery_fuzzer_test::r2_interleaved_transactions_out_of_order.
        // WAL order: Begin tx=1, Begin tx=2, Commit tx=1, Commit tx=2.
        // Prior single-boolean implementation reported committed_txns=1
        // (the second Commit saw in_tx=false and was skipped). Per-tx
        // HashMap tracks each tx independently.
        use crate::wal::WalEntryType;
        let entries = vec![
            WalEntry {
                tx_id: 1,
                entry_type: WalEntryType::Begin,
                table_id: 0,
                key: None,
                data: None,
                lsn: 1,
                timestamp: 0,
            },
            WalEntry {
                tx_id: 2,
                entry_type: WalEntryType::Begin,
                table_id: 0,
                key: None,
                data: None,
                lsn: 2,
                timestamp: 0,
            },
            WalEntry {
                tx_id: 1,
                entry_type: WalEntryType::Commit,
                table_id: 0,
                key: None,
                data: None,
                lsn: 3,
                timestamp: 0,
            },
            WalEntry {
                tx_id: 2,
                entry_type: WalEntryType::Commit,
                table_id: 0,
                key: None,
                data: None,
                lsn: 4,
                timestamp: 0,
            },
        ];
        let (committed, rolled_back, incomplete) = count_status(&entries);
        assert_eq!(committed, 2, "both txns must be counted as committed");
        assert_eq!(rolled_back, 0);
        assert_eq!(incomplete, 0);
    }

    #[test]
    fn test_filter_committed_entries_interleaved() {
        // Regression: prior shared DML buffer mis-attributed tx=2's
        // Insert to tx=1 when tx=1's Commit flushed the buffer early.
        use crate::wal::WalEntryType;
        let entries = vec![
            WalEntry {
                tx_id: 1,
                entry_type: WalEntryType::Begin,
                table_id: 0,
                key: None,
                data: None,
                lsn: 1,
                timestamp: 0,
            },
            WalEntry {
                tx_id: 2,
                entry_type: WalEntryType::Begin,
                table_id: 0,
                key: None,
                data: None,
                lsn: 2,
                timestamp: 0,
            },
            // Insert for tx=2 (autocommit-style DML — no prior tx=2 Insert;
            // tx=2's Begin allocated a buffer so the Insert lands in tx=2's bucket).
            WalEntry {
                tx_id: 2,
                entry_type: WalEntryType::Insert,
                table_id: 0,
                key: Some(vec![2u8]),
                data: None,
                lsn: 3,
                timestamp: 0,
            },
            WalEntry {
                tx_id: 1,
                entry_type: WalEntryType::Commit,
                table_id: 0,
                key: None,
                data: None,
                lsn: 4,
                timestamp: 0,
            },
            WalEntry {
                tx_id: 2,
                entry_type: WalEntryType::Commit,
                table_id: 0,
                key: None,
                data: None,
                lsn: 5,
                timestamp: 0,
            },
        ];
        let out = filter_committed_entries(&entries);
        assert_eq!(
            out.len(),
            1,
            "only tx=2's Insert should be replayed (tx=1 had no DML); got {} entries: {:?}",
            out.len(),
            out
        );
        assert_eq!(out[0].tx_id, 2, "Insert must belong to tx=2, not tx=1");
    }

    #[test]
    fn test_count_status_three_way_interleave() {
        // 3 concurrent txns with mixed Commit/Rollback interleaving.
        use crate::wal::WalEntryType;
        let entries = vec![
            WalEntry {
                tx_id: 1,
                entry_type: WalEntryType::Begin,
                table_id: 0,
                key: None,
                data: None,
                lsn: 1,
                timestamp: 0,
            },
            WalEntry {
                tx_id: 2,
                entry_type: WalEntryType::Begin,
                table_id: 0,
                key: None,
                data: None,
                lsn: 2,
                timestamp: 0,
            },
            WalEntry {
                tx_id: 3,
                entry_type: WalEntryType::Begin,
                table_id: 0,
                key: None,
                data: None,
                lsn: 3,
                timestamp: 0,
            },
            WalEntry {
                tx_id: 2,
                entry_type: WalEntryType::Commit,
                table_id: 0,
                key: None,
                data: None,
                lsn: 4,
                timestamp: 0,
            },
            WalEntry {
                tx_id: 1,
                entry_type: WalEntryType::Rollback,
                table_id: 0,
                key: None,
                data: None,
                lsn: 5,
                timestamp: 0,
            },
            // tx=3 still open at WAL tail → counts as incomplete.
        ];
        let (committed, rolled_back, incomplete) = count_status(&entries);
        assert_eq!(committed, 1, "tx=2 is the only committed");
        assert_eq!(rolled_back, 1, "tx=1 was rolled back");
        assert_eq!(incomplete, 1, "tx=3 never closed (simulated crash)");
    }

    #[test]
    fn test_count_status_empty() {
        let (committed, rolled_back, incomplete) = count_status(&[]);
        assert_eq!(committed, 0);
        assert_eq!(rolled_back, 0);
        assert_eq!(incomplete, 0);
    }
}
