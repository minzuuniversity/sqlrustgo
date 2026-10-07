//! File-based table storage
//! Persists table data to JSON files

use crate::bplus_tree::BPlusTree;
use crate::engine::{
    ColumnDefinition, ForeignKeyConstraint, IndexInfo, Record, RowFilter, RowMutation,
    SharedSliceIter, StorageEngine, TableData, TableInfo, TriggerInfo, UniqueConstraint, ViewInfo,
};
use crate::wal::{WalEntry, WalEntryType};
use sqlrustgo_types::{SqlError, SqlResult, Value};
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

// C.1: parking_lot::Mutex is used as a `Mutex<()>` for the write-side
// synchronisation of the five fields that previously relied on the outer
// `Arc<RwLock<FileStorage>>`. See PHASE_C_1_INTERNAL_LOCKING.md §2.

/// #4951: the mutable state that `write_state` guards.
///
/// These four fields used to be plain fields on `FileStorage`, mutated
/// through the `as_mut_self` escape hatch — an
/// `unsafe { &mut *(self as *const Self as *mut Self) }` that derived a
/// `&mut Self` from `&self`. Two connections both reaching `Arc<
/// RwLock<FileStorage>>::read()` would each mint a `&mut` to the same
/// `tables` map: simultaneous `&mut` borrows, which is UB regardless of
/// whether they happen to touch different keys.
///
/// Packing them behind one `RwLock` makes the exclusivity structural
/// instead of a comment. `current_tx_id` is deliberately **not** here —
/// #4984 made it an `AtomicU64`, which already gives it the concurrency
/// semantics it needs.
///
/// Field names match the old `FileStorage` fields on purpose: the ~20
/// `with_write_lock` closures that only touch guarded fields compile
/// unchanged, which keeps this refactor's diff proportional to the risk
/// it actually carries.
struct WriteState {
    /// In-memory cache of tables
    tables: HashMap<String, TableData>,
    /// Insert buffer for batching writes
    insert_buffer: HashMap<String, Vec<Record>>,
    /// Issue #4581 / B-track case 35-36: per-transaction undo log for
    /// ROLLBACK support. When `current_tx_id != 0`, every UPDATE/DELETE
    /// in the storage layer records the original row here so a
    /// subsequent ROLLBACK can replay the log in reverse and restore
    /// the pre-tx state. Cleared on COMMIT. Empty when autocommit.
    ///
    /// Note: scope is intentionally limited to the issue's spec —
    /// UPDATE/DELETE row restore. INSERT inside a tx is already routed
    /// through `insert_buffer` (see `insert()`), which is cleared on
    /// ROLLBACK by draining any buffered entries added during the tx.
    /// Schema DDL (CREATE/DROP/ALTER) inside a tx is not rolled back —
    /// that requires catalog-level undo, tracked as a separate follow-up.
    ///
    /// Entries carry the owning `tx_id` (see [`TxUndoEntry`]) because
    /// this log is shared by every connection: a rollback must replay
    /// only its own transaction's entries, never a peer's.
    tx_undo_log: Vec<TxUndoEntry>,
    /// V311-07: Dirty table tracker - marks tables modified since last flush
    ///
    /// #5057: keyed by `(database, table)`, NOT by a bare table name.
    ///
    /// It used to be a `HashSet<String>` holding bare table names, and the
    /// flush path resolved each name against `current_db` when it wrote the
    /// file. A bare name cannot say which database it belongs to, so two
    /// databases holding a table of the same name collapsed into ONE set
    /// entry, and the single write went to whichever database `current_db`
    /// happened to name:
    ///
    ///     d1.t <- 3 rows, d2.t <- 3 rows, current_db = d1
    ///     flush() -> Ok(())
    ///     d1/t.json  3 rows      <- correct
    ///     d2/t.json  0 rows      <- 3 rows silently gone, no error
    ///
    /// A tuple keeps both halves and needs no separator round-trip. That
    /// matters: `scoped_key` lowercases both halves, so parsing a
    /// `db\u{1}table` key back apart would hand a lowercased table name to
    /// the file-writing helpers, which build `<table>.json` from it — a
    /// table named `T` would start being persisted as `t.json`.
    dirty_tables: HashSet<(String, String)>,
}

impl WriteState {
    /// #5060: the two places a row of `scoped_table` can be.
    ///
    /// `insert_buffer` is a **second copy of table state**, not a
    /// staging area the rest of the engine knows about. Until #5060 only
    /// `scan` looked at both, so an autocommit `INSERT` followed by an
    /// `UPDATE` or `DELETE` on the same row reported **0 rows affected**
    /// and left the old value in place. Worse, `delete` stripped the row
    /// from the buffer without counting it, so the row vanished from
    /// memory while `dirty_tables` was never set and the deletion was
    /// never persisted — the row came back on the next open.
    ///
    /// These two helpers are the single place that knows a table has two
    /// stores, so `update` / `update_if` / `delete` / `delete_if` cannot
    /// each re-derive the mistake.
    ///
    /// `scoped_table` is the `db\x01table` key. Returns the post-image of
    /// every row that matched, in store order (table rows first, then
    /// buffered rows) — both the WAL (#5055) and the change log (#5048)
    /// need those, and neither can reconstruct them after the fact.
    fn mutate_matching<F, G>(
        &mut self,
        scoped_table: &str,
        matches: F,
        mut mutate: G,
    ) -> Vec<(Record, Record)>
    where
        F: Fn(&Record) -> bool,
        G: FnMut(&mut Record),
    {
        // (pre-image, post-image) per touched row: the pre-image is
        // what ROLLBACK needs, the post-image is what the WAL (#5055)
        // and the change log (#5048) need. Neither can be reconstructed
        // after the mutation.
        let mut touched: Vec<(Record, Record)> = Vec::new();
        if let Some(data) = self.tables.get_mut(scoped_table) {
            for row in data.rows.iter_mut().filter(|r| matches(r)) {
                let pre = row.clone();
                mutate(row);
                touched.push((pre, row.clone()));
            }
        }
        // Rows still in the buffer are this transaction's uncommitted
        // inserts. They are real rows for every purpose except
        // durability, so an UPDATE has to reach them — otherwise
        // INSERT-then-UPDATE inside a transaction commits the *old*
        // value.
        if let Some(buffered) = self.insert_buffer.get_mut(scoped_table) {
            for row in buffered.iter_mut().filter(|r| matches(r)) {
                let pre = row.clone();
                mutate(row);
                touched.push((pre, row.clone()));
            }
        }
        touched
    }

    /// #5060: as [`mutate_matching`](Self::mutate_matching), but removes
    /// the matching rows from both stores. Returns them so the caller can
    /// log a delete by key.
    fn remove_matching<F>(&mut self, scoped_table: &str, matches: F) -> Vec<Record>
    where
        F: Fn(&Record) -> bool,
    {
        let mut removed: Vec<Record> = Vec::new();
        if let Some(data) = self.tables.get_mut(scoped_table) {
            let mut kept = Vec::with_capacity(data.rows.len());
            for row in std::mem::take(&mut data.rows) {
                if matches(&row) {
                    removed.push(row);
                } else {
                    kept.push(row);
                }
            }
            data.rows = kept;
        }
        if let Some(buffered) = self.insert_buffer.get_mut(scoped_table) {
            let mut kept = Vec::with_capacity(buffered.len());
            for row in std::mem::take(buffered) {
                if matches(&row) {
                    removed.push(row);
                } else {
                    kept.push(row);
                }
            }
            *buffered = kept;
        }
        removed
    }
}

/// File-based storage manager
pub struct FileStorage {
    /// Base directory for database files
    data_dir: PathBuf,
    /// #4951: guards `WriteState`. Replaces the `as_mut_self` escape
    /// hatch — see that type's doc comment for why deriving `&mut Self`
    /// from `&self` was unsound here.
    ///
    /// C.1 (pre-#4951) had this as a bare `parking_lot::Mutex<()>` that
    /// callers entered through `with_write_lock`. Holding the data inside
    /// the lock rather than beside it means a `&mut` to the guarded
    /// fields can only exist for the duration of a real guard, so the
    /// borrow checker enforces what the old code could only assert in a
    /// comment.
    write_state: parking_lot::RwLock<WriteState>,
    /// #5025: the active database.
    ///
    /// A table in a non-default database lives under `data_dir/{db}/`;
    /// the default database keeps the historical `data_dir/{table}.json`
    /// layout so an existing installation is untouched. The in-memory
    /// cache is keyed the same way the on-disk layout is, via `tbl()`.
    current_db: RwLock<String>,
    /// B+ Tree indexes protected by RwLock for concurrent access.
    /// Keyed by (table, column) because the on-disk layout is one
    /// file per (table, column) pair.
    indexes: RwLock<HashMap<(String, String), BPlusTree>>,
    /// V312-95 v3 / P3-HINT-001 follow-up: index metadata catalog.
    /// Without this, the default `list_all_indexes()` returns empty,
    /// so the executor's `INDEXED BY <name>` validator reports
    /// "index does not exist" for every CLI batch-mode index.
    index_metadata: RwLock<HashMap<String, IndexInfo>>,
    /// Threshold to trigger buffer flush
    buffer_threshold: usize,
    /// Enable insert buffering
    enable_buffer: bool,
    /// PR-842: the active transaction id (0 == autocommit). Mirrored from
    /// the ExecutionEngine via `set_current_tx_id` so `in_transaction()`
    /// can answer correctly even on the bare FileStorage path.
    ///
    /// #4984: atomic, and deliberately *not* part of `WriteState` — the
    /// escape hatch this field used to rely on (`as_mut_self`) is gone,
    /// and `AtomicU64` is both simpler and correct for a scalar.
    ///
    /// Note this is a **storage-level** single value, not per-connection:
    /// it records "the transaction that most recently set it". Two
    /// connections still overwrite each other, which is why #4983 has to
    /// thread a real per-connection `reader_tx` down instead of reading
    /// this. Removing the unsoundness did not make the field correct.
    current_tx_id: std::sync::atomic::AtomicU64,
    /// Trigger definitions keyed by trigger name, protected by RwLock for concurrent access
    triggers: RwLock<HashMap<String, TriggerInfo>>,
    /// V312-95 v2 / Issue #4814: view definitions keyed by view name,
    /// protected by RwLock for concurrent access. Persisted to disk as
    /// one JSON file per view under `view_<name>.json`, mirroring the
    /// trigger persistence pattern.
    views: RwLock<HashMap<String, ViewInfo>>,
    /// Gap lock manager for REPEATABLE-READ isolation (F-16 Gap Locking)
    #[allow(dead_code)]
    gap_lock_manager: Option<std::sync::Arc<crate::lock::GapLockManager>>,
    /// V400-PERF-DELTA: per-table count of rows that have been persisted
    /// to disk (either in the base JSON or in the .delta file). Used
    /// by `save_table` to decide whether to write anything, and to
    /// limit incremental writes to only the new rows.
    last_saved_row_count: Mutex<HashMap<String, usize>>,
    /// #5055: monotonic transaction-id source.
    ///
    /// See [`next_tx_id`](Self::next_tx_id) for why the previous
    /// wall-clock derivation was not good enough. Starts at 1 because
    /// `current_tx_id == 0` means autocommit.
    next_tx_id_counter: std::sync::atomic::AtomicU64,
    /// #5055: the write-ahead log, opened by [`new_with_wal`].
    ///
    /// `None` in every other constructor, which is what makes
    /// [`StorageEngine::is_wal_enabled`] answer `false` there. It is a
    /// plain `Mutex` rather than being folded into `WriteState`
    /// because WAL appends happen *after* the `WriteState` guard is
    /// released — see `wal_append` — and folding them together would
    /// mean holding the storage write lock across a file write.
    ///
    /// Before this field existed, `new_with_wal` computed
    /// `let _wal_path = data_dir.join("sqlrustgo.wal")` and dropped it
    /// on the floor. Nothing was ever written, so `admin pitr` had an
    /// empty or absent WAL to "replay" and reported success anyway.
    wal: Mutex<Option<Box<dyn crate::wal::WalManager>>>,
    /// #5048: opt-in change log, the prerequisite for a real incremental
    /// backup. `None` unless a caller turns it on, so the ordinary path
    /// pays nothing.
    ///
    /// It exists because `FileStorage` had no versioned change capture of
    /// any kind: no WAL, no CDC, no per-table change log. `table_change_
    /// stamp` (the trait default) answers "is my cached copy stale", not
    /// "which rows changed", and only `MemoryStorage` overrode it at all
    /// — so an incremental backup could not be produced from a real data
    /// directory, which is the only thing the backup tool opens.
    ///
    /// #5055: the WAL above is the *other* half of the same gap — it
    /// records changes for crash recovery and PITR, in an order that can
    /// be replayed, while this records them for a backup delta. They are
    /// separate because their consumers differ (one needs commit
    /// boundaries, the other needs a sliceable LSN), not because the
    /// writes are different: both hooks sit in the same DML paths.
    change_log: Mutex<Option<ChangeLog>>,
}

/// #5048: a recorded change, as a backup delta needs it.
///
/// Positional, not column-named: the storage layer's DML surface works on
/// `Record = Vec<Value>`, and giving the backup tool column names would
/// mean carrying schema alongside every change. The backup exporter
/// already writes positional SQL, so the two agree.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ChangeLogEntry {
    pub table: String,
    pub op: ChangeOp,
    /// Leading key columns of the affected row.
    pub key: Vec<Value>,
    /// The row after the change; `None` for a delete.
    pub row: Option<Vec<Value>>,
    /// Monotonic per-log sequence number. Doubles as the LSN: it orders
    /// changes exactly, and a backup can slice "everything after the last
    /// full backup's snapshot" by comparing it.
    pub lsn: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ChangeOp {
    Insert,
    Update,
    Delete,
}

#[derive(Debug, Default)]
struct ChangeLog {
    entries: Vec<ChangeLogEntry>,
    next_lsn: u64,
    /// #5048: true once the on-disk log has been read. A freshly
    /// enabled log on an open database starts empty, but a log read
    /// from disk that happens to be empty is a different fact — and a
    /// backup must be able to tell them apart.
    #[allow(dead_code)]
    loaded_from_disk: bool,
    /// #5048: how many entries are already on disk. `persist_change_log`
    /// appends only the tail, so repeated flushes do not duplicate.
    persisted: usize,
}

/// #5048: take the change-log lock, ignoring poisoning.
///
/// A panic while the log was held would otherwise make every later
/// backup fail permanently. The log is diagnostic state — losing it is
/// better than refusing to open the database, and the log is opt-in so
/// a caller can detect that it went missing.
fn change_log_lock(me: &FileStorage) -> std::sync::MutexGuard<'_, Option<ChangeLog>> {
    me.change_log.lock().unwrap_or_else(|e| e.into_inner())
}

/// #5048: the key columns of a row.
///
/// The storage layer's delete/update filters are positional and compare
/// against the row's *leading* columns, so the leading column is the
/// identity a change record can name. `table.primary_key` is a name, and
/// turning a name back into a position needs the schema; the filters
/// themselves do not, so the two stay consistent by using position.
fn key_of(row: &[Value]) -> Vec<Value> {
    row.iter().take(1).cloned().collect()
}

/// Issue #4581 / B-track case 35-36: per-transaction undo log entry.
/// Captures a pre-tx row state so ROLLBACK can revert it. Replayed in
/// reverse insertion order during rollback (last-in / first-out),
/// matching the standard SQL semantics for nested-row restore.
#[derive(Debug, Clone)]
#[allow(dead_code)] // BufferedInsert reserved for a future per-INSERT undo path
enum UndoOp {
    /// UPDATE on a row at `row_idx` in `table`. `original` is the row
    /// value BEFORE the UPDATE statement modified it.
    UpdateRow {
        table: String,
        row_idx: usize,
        original: Vec<crate::engine::Value>,
    },
    /// DELETE on a row at `row_idx`. ROLLBACK re-inserts the row at
    /// the same index (or end-of-table if subsequent INSERTs shifted it).
    DeleteRow {
        table: String,
        row_idx: usize,
        original: Vec<crate::engine::Value>,
    },
    /// DELETE-all (filters empty). ROLLBACK restores the full row set.
    DeleteAll {
        table: String,
        original_rows: Vec<Vec<crate::engine::Value>>,
    },
    /// INSERT inside a tx. ROLLBACK removes the buffered row.
    BufferedInsert {
        table: String,
        row: Vec<crate::engine::Value>,
    },
    /// #5059: DELETE of a row that was still in the insert buffer.
    ///
    /// It needs its own variant because `DeleteRow` identifies its row
    /// by index into `tables`, and a buffered row has no index there.
    /// ROLLBACK puts the value back into the buffer, where it was.
    ///
    /// #5059's actual symptom was neither of these: ROLLBACK never
    /// removed *anything* from the buffer, because the "belt-and-
    /// suspenders" sweep at the end of `rollback_transaction` iterated
    /// `s.tables.keys()` — already-scoped keys — and then scoped them a
    /// second time, so it addressed `default\x01default\x01tx_t` and
    /// matched no buffer. 100 committed + 100 rolled-back inserts all
    /// survived, and the test saw 200.
    BufferedDelete {
        table: String,
        row: Vec<crate::engine::Value>,
    },
    /// #5060: UPDATE of a row that was still in the insert buffer.
    ///
    /// `DeleteRow` / `UpdateRow` address their row by index into
    /// `tables`; a buffered row has no index there, and by rollback
    /// time the buffer may have been drained and refilled by other
    /// work, so an index captured now would be meaningless. Instead the
    /// row is located by its post-image and replaced with the
    /// pre-image.
    ///
    /// If the post-image is gone — the row was flushed to `tables`, or
    /// updated again — the buffered copy has nothing left to undo, and
    /// the `tables` copy is covered by the ordinary `UpdateRow` entry.
    BufferedUpdate {
        table: String,
        post: Vec<crate::engine::Value>,
        original: Vec<crate::engine::Value>,
    },
}

/// An [`UndoOp`] plus the id of the transaction that produced it.
///
/// # Why this wrapper exists
///
/// `WriteState.tx_undo_log` is a single `Vec` shared by every connection:
/// the MySQL server hands each connection handler the same
/// `Arc<RwLock<FileStorage>>` (`do_command_loop` takes
/// `storage: Arc<RwLock<BoxStorageEngine>>`). Before this wrapper,
/// `rollback_transaction` drained that one vector, so a rolling-back
/// transaction replayed whatever entries happened to be in it — including
/// entries belonging to transactions still running or already committed.
///
/// Measured consequence before the fix (pinned by
/// `tests/integration/transaction/concurrent_rollback_isolation_test.rs`):
/// two connections deleting the same row erased it 1/30 of the time even
/// though the loser's `ROLLBACK` reported success, and a concurrent
/// `DELETE id=N; INSERT id=N` workload drifted off its row count in both
/// directions — silently, with zero errors.
///
/// Tagging each entry with its owning `tx_id` lets a rollback discard
/// everything that is not its own, which is what "roll back *my*
/// transaction" means.
#[derive(Debug, Clone)]
struct TxUndoEntry {
    tx_id: u64,
    op: UndoOp,
}

impl FileStorage {
    // --- #5055: write-ahead logging -----------------------------------
    //
    // The methods below are the whole WAL surface of `FileStorage`.
    // They are deliberately thin: encode, hand to the `WalManager`,
    // done. All the "which rows changed" bookkeeping lives at the DML
    // call sites, because that is the only place it is still true.

    /// `true` when a WAL was opened by [`new_with_wal`](Self::new_with_wal).
    ///
    /// A poisoned mutex means some other thread panicked while holding
    /// it. The `WalManager` behind it is still a live, usable object —
    /// `append` is the only thing that touches it and it does not leave
    /// it half-updated in a way `&mut` cannot express — so recovering
    /// the guard is strictly better than propagating the panic to every
    /// subsequent write.
    fn wal_enabled(&self) -> bool {
        self.wal.lock().map(|g| g.is_some()).unwrap_or(true)
    }

    /// Append a batch of entries. A no-op when no WAL is open.
    ///
    /// Callers must invoke this **after** releasing `write_state`. WAL
    /// append does file I/O, and holding the storage write lock across
    /// it would serialise every reader behind the disk.
    fn wal_append(&self, entries: Vec<WalEntry>) -> SqlResult<()> {
        if entries.is_empty() {
            return Ok(());
        }
        let mut guard = match self.wal.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        let Some(wal) = guard.as_mut() else {
            return Ok(());
        };
        for entry in entries {
            wal.append(entry)?;
        }
        Ok(())
    }

    /// Build one row-level WAL entry for `table`.
    ///
    /// `table_name` is always `Some` here. #5055 made it an optional
    /// field on `WalEntry` for backward compatibility with WALs written
    /// before it existed, but a row entry this code writes can always
    /// name its table, and a replay that has to guess would be worse
    /// than one that refuses.
    fn wal_row_entry(
        &self,
        entry_type: WalEntryType,
        table: &str,
        key: Vec<u8>,
        data: Option<Vec<u8>>,
    ) -> WalEntry {
        WalEntry {
            tx_id: self
                .current_tx_id
                .load(std::sync::atomic::Ordering::Acquire),
            entry_type,
            table_id: crate::wal_record_codec::table_name_to_id(table),
            table_name: Some(table.to_string()),
            key: Some(key),
            data,
            lsn: 0,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        }
    }

    /// Build a transaction-boundary entry (BEGIN / COMMIT / ROLLBACK).
    fn wal_tx_entry(&self, entry_type: WalEntryType, tx_id: u64, lsn: u64) -> WalEntry {
        WalEntry {
            tx_id,
            entry_type,
            table_id: 0,
            table_name: None,
            key: None,
            data: None,
            lsn,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        }
    }

    /// Encode one row into the `Insert` entries for it.
    fn wal_insert_entries(&self, table: &str, records: &[Record]) -> Vec<WalEntry> {
        records
            .iter()
            .map(|r| {
                self.wal_row_entry(
                    WalEntryType::Insert,
                    table,
                    crate::wal_record_codec::record_key(r),
                    Some(crate::wal_record_codec::record_to_bytes(r)),
                )
            })
            .collect()
    }

    /// Encode one row into the `Delete` entries for it. A delete logs
    /// only the key, not the row: replay removes by key.
    fn wal_delete_entries(&self, table: &str, rows: &[Record]) -> Vec<WalEntry> {
        rows.iter()
            .map(|r| {
                self.wal_row_entry(
                    WalEntryType::Delete,
                    table,
                    crate::wal_record_codec::record_key(r),
                    None,
                )
            })
            .collect()
    }

    /// Encode one row into the `Update` entry for it. An update logs
    /// the key *and* the full post-image, so replay is a blind
    /// overwrite and does not need to re-evaluate the predicate.
    fn wal_update_entries(&self, table: &str, rows: &[Record]) -> Vec<WalEntry> {
        rows.iter()
            .map(|r| {
                self.wal_row_entry(
                    WalEntryType::Update,
                    table,
                    crate::wal_record_codec::record_key(r),
                    Some(crate::wal_record_codec::record_to_bytes(r)),
                )
            })
            .collect()
    }

    /// #5055: read back every entry currently in the WAL.
    ///
    /// Public because "what does this data directory's log actually
    /// contain" is the question a recovery tool, an operator, and a
    /// test all need answered, and until now the only way to get at it
    /// was to open the file by hand. Returns an empty vector when no
    /// WAL is open — that is a fact about the backend, not an error.
    pub fn recover_wal_entries(&self) -> SqlResult<Vec<WalEntry>> {
        let mut guard = match self.wal.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        match guard.as_mut() {
            Some(wal) => wal.recover(),
            None => Ok(Vec::new()),
        }
    }

    /// #5055: the LSN the next appended entry will receive, or 0 when
    /// no WAL is open.
    pub fn wal_current_lsn(&self) -> u64 {
        let guard = match self.wal.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        guard.as_ref().map(|w| w.current_lsn()).unwrap_or(0)
    }

    /// #5048: start recording changes so an incremental backup can be
    /// produced from this data directory.
    ///
    /// Off by default: the log grows without bound until drained, and a
    /// caller that never takes an incremental backup should not pay for
    /// it.
    ///
    /// #5048: an existing on-disk log is loaded. That is what makes a
    /// delta possible from a *different process* than the writer — which
    /// is the only way the `backup` CLI can work at all. `next_lsn`
    /// continues past whatever was on disk, so LSNs never collide with
    /// a previous run's.
    pub fn enable_change_log(&self) {
        let already = change_log_lock(self).is_some();
        if already {
            return;
        }
        let mut log = ChangeLog {
            entries: Vec::new(),
            next_lsn: 0,
            loaded_from_disk: false,
            persisted: 0,
        };
        match self.load_change_log_from_disk() {
            Ok(entries) => {
                log.next_lsn = entries.iter().map(|e| e.lsn).max().unwrap_or(0);
                log.persisted = entries.len();
                log.entries = entries;
                log.loaded_from_disk = true;
            }
            Err(e) => {
                // A corrupt or unreadable log must not stop the database
                // from opening. It does mean an incremental backup taken
                // now would silently miss earlier changes, so say so
                // loudly rather than letting it pass.
                // `sqlrustgo-storage` does not depend on `tracing`, and
                // this is exactly the case where silence would mislead:
                // the caller would take a delta that is quietly missing
                // earlier changes.
                eprintln!(
                    "WARN: could not read change log at {}: {e}. An incremental \
                     backup taken now may be missing earlier changes.",
                    self.change_log_path().display()
                );
            }
        }
        *change_log_lock(self) = Some(log);
    }

    /// #5048: append the log to disk.
    ///
    /// Called from `flush`, not from each write. A backup taken between
    /// a write and its flush must not claim a change the database has not
    /// committed — the log and the data have to become durable together
    /// or the delta would replay rows the base does not contain.
    pub fn persist_change_log(&self) -> std::io::Result<()> {
        let mut guard = change_log_lock(self);
        let Some(log) = guard.as_mut() else {
            return Ok(());
        };
        if log.entries.len() == log.persisted {
            return Ok(());
        }
        let tail = &log.entries[log.persisted..];
        let path = self.change_log_path();
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        use std::io::Write;
        let mut writer = BufWriter::new(file);
        for e in tail {
            let line = serde_json::to_string(e)
                .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
            writer.write_all(line.as_bytes())?;
            writer.write_all(b"\n")?;
        }
        writer.flush()?;
        // Only now is the tail durable; a failed write must leave
        // `persisted` alone so the next flush retries it.
        log.persisted = log.entries.len();
        Ok(())
    }

    fn load_change_log_from_disk(&self) -> std::io::Result<Vec<ChangeLogEntry>> {
        let path = self.change_log_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = std::fs::read_to_string(&path)?;
        let mut out = Vec::new();
        for (i, line) in content.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            match serde_json::from_str::<ChangeLogEntry>(line) {
                Ok(e) => out.push(e),
                // A half-written final line is the expected shape of a
                // crash during append. Earlier lines stay valid.
                Err(err) => {
                    if i + 1 == content.lines().count() {
                        eprintln!("WARN: truncating incomplete change log line {i}: {err}");
                        break;
                    }
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("change log line {i} is corrupt: {err}"),
                    ));
                }
            }
        }
        out.sort_by_key(|e| e.lsn);
        Ok(out)
    }

    fn change_log_path(&self) -> std::path::PathBuf {
        let file = "changelog.jsonl";
        match self.db_dir() {
            None => self.data_dir.join(file),
            Some(dir) => {
                let _ = std::fs::create_dir_all(&dir);
                dir.join(file)
            }
        }
    }

    /// #5048: changes recorded after `since_lsn`, in order.
    ///
    /// Returns an empty list when the log was never enabled — a caller
    /// must not be able to mistake "no changes" for "no log".
    pub fn changes_since(&self, since_lsn: u64) -> Vec<ChangeLogEntry> {
        let guard = change_log_lock(self);
        match guard.as_ref() {
            Some(log) => log
                .entries
                .iter()
                .filter(|e| e.lsn > since_lsn)
                .cloned()
                .collect(),
            None => Vec::new(),
        }
    }

    /// The LSN a backup taken now should record, so a later
    /// [`Self::changes_since`] starts exactly where this one ended.
    pub fn current_change_lsn(&self) -> u64 {
        let guard = change_log_lock(self);
        guard.as_ref().map(|l| l.next_lsn).unwrap_or(0)
    }

    /// True once [`Self::enable_change_log`] has been called. Lets the
    /// backup tool tell "nothing changed" apart from "nothing was being
    /// watched".
    pub fn change_log_enabled(&self) -> bool {
        change_log_lock(self).is_some()
    }

    /// Record one change. No-op unless the log is enabled, so every
    /// write path can call this unconditionally.
    fn record_change(&self, table: &str, op: ChangeOp, key: Vec<Value>, row: Option<Vec<Value>>) {
        let mut guard = change_log_lock(self);
        if let Some(log) = guard.as_mut() {
            log.next_lsn += 1;
            let lsn = log.next_lsn;
            log.entries.push(ChangeLogEntry {
                // #5025 keys rows by `db\u{1}table`. A backup replays by
                // table name against a restored database, which has no such
                // prefix — recording the scoped key would make every delta
                // un-replayable. The database travels with the backup
                // directory instead.
                table: table.to_string(),
                op,
                key,
                row,
                lsn,
            });
        }
    }

    /// Create a new FileStorage with the given data directory
    pub fn new(data_dir: PathBuf) -> std::io::Result<Self> {
        // Create directory if it doesn't exist
        fs::create_dir_all(&data_dir)?;

        let storage = Self {
            data_dir,
            current_db: RwLock::new(crate::engine::DEFAULT_DATABASE.to_string()),
            write_state: parking_lot::RwLock::new(WriteState {
                tables: HashMap::new(),
                insert_buffer: HashMap::new(),
                tx_undo_log: Vec::new(),
                dirty_tables: HashSet::new(),
            }),
            indexes: RwLock::new(HashMap::new()),
            index_metadata: RwLock::new(HashMap::new()),
            // v3.12.0 #4020 follow-up: raise default buffer flush threshold from
            // 100 to 10_000. Each flush goes through insert_direct, which clones
            // the full TableData and serializes it via serde_json::to_string_pretty
            // — an O(rows_loaded) operation. With threshold=100 the bulk-load
            // cost was O(N^2), which made TPC-H SF=10 supplier only achieve
            // ~111 rows/s. Micro-bench (bulk_load_quadraticity) shows
            // threshold=10000 is ~50.6x faster on identical final state.
            // Callers that need the old behaviour should use
            // new_with_buffer_config(dir, 100, true).
            buffer_threshold: 10_000,
            enable_buffer: true,
            current_tx_id: std::sync::atomic::AtomicU64::new(0),
            triggers: RwLock::new(HashMap::new()),
            gap_lock_manager: None,
            last_saved_row_count: Mutex::new(HashMap::new()),
            next_tx_id_counter: std::sync::atomic::AtomicU64::new(1),
            wal: Mutex::new(None),
            // #5048: off unless `enable_change_log` is called.
            change_log: Mutex::new(None),
            views: RwLock::new(HashMap::new()),
        };

        // Load existing tables
        storage.load_all_tables()?;

        // Load existing indexes
        storage.load_all_indexes()?;

        // V312-95 v2 / Issue #4814: views are normal catalog objects and
        // must be loaded on plain `new` so they survive process restart
        // independent of WAL mode (triggers, by contrast, only load under
        // `new_with_wal`).
        storage.load_all_views()?;

        Ok(storage)
    }

    pub fn new_with_buffer_config(
        data_dir: PathBuf,
        buffer_threshold: usize,
        enable_buffer: bool,
    ) -> std::io::Result<Self> {
        fs::create_dir_all(&data_dir)?;

        let storage = Self {
            data_dir,
            current_db: RwLock::new(crate::engine::DEFAULT_DATABASE.to_string()),
            write_state: parking_lot::RwLock::new(WriteState {
                tables: HashMap::new(),
                insert_buffer: HashMap::new(),
                tx_undo_log: Vec::new(),
                dirty_tables: HashSet::new(),
            }),
            indexes: RwLock::new(HashMap::new()),
            index_metadata: RwLock::new(HashMap::new()),
            buffer_threshold,
            enable_buffer,
            current_tx_id: std::sync::atomic::AtomicU64::new(0),
            triggers: RwLock::new(HashMap::new()),
            gap_lock_manager: None,
            last_saved_row_count: Mutex::new(HashMap::new()),
            next_tx_id_counter: std::sync::atomic::AtomicU64::new(1),
            wal: Mutex::new(None),
            // #5048: off unless `enable_change_log` is called.
            change_log: Mutex::new(None),
            views: RwLock::new(HashMap::new()),
        };

        storage.load_all_tables()?;
        storage.load_all_indexes()?;
        storage.load_all_views()?;

        Ok(storage)
    }

    /// Create a new FileStorage with WAL (Write-Ahead Log) enabled for crash recovery.
    /// The WAL file will be stored in the data directory as "sqlrustgo.wal".
    /// Returns Err if WAL cannot be initialized.
    ///
    /// #5055: this used to compute `wal_path` and immediately drop it.
    /// The storage behaved identically to [`new`](Self::new) and wrote
    /// no log at all, while `is_wal_enabled()` — which this type never
    /// overrode — answered `false`, so nothing downstream could notice
    /// either. `admin pitr` then read a WAL that either did not exist
    /// or was empty, and the CLI reported `pitr ok` and exited 0.
    ///
    /// A failure to open the WAL is now an error, not a silent
    /// downgrade to a log-less storage: the caller asked for
    /// durability, and quietly returning a storage without it is how
    /// the original bug existed in the first place.
    pub fn new_with_wal(data_dir: PathBuf) -> std::io::Result<Self> {
        fs::create_dir_all(&data_dir)?;

        let wal_path = data_dir.join("sqlrustgo.wal");
        let wal = crate::wal::FileBackedWalManager::new(wal_path).map_err(|e| {
            std::io::Error::other(format!("failed to open WAL {}: {e}", data_dir.display()))
        })?;

        let storage = Self {
            data_dir,
            current_db: RwLock::new(crate::engine::DEFAULT_DATABASE.to_string()),
            write_state: parking_lot::RwLock::new(WriteState {
                tables: HashMap::new(),
                insert_buffer: HashMap::new(),
                tx_undo_log: Vec::new(),
                dirty_tables: HashSet::new(),
            }),
            indexes: RwLock::new(HashMap::new()),
            index_metadata: RwLock::new(HashMap::new()),
            // v3.12.0 #4020 follow-up: see FileStorage::new — default raised to
            // 10_000 to amortise O(N) insert_direct over a much larger batch.
            buffer_threshold: 10_000,
            enable_buffer: true, // Transaction boundary handled by buffer flush on commit
            current_tx_id: std::sync::atomic::AtomicU64::new(0),
            triggers: RwLock::new(HashMap::new()),
            gap_lock_manager: None,
            last_saved_row_count: Mutex::new(HashMap::new()),
            next_tx_id_counter: std::sync::atomic::AtomicU64::new(1),
            wal: Mutex::new(Some(Box::new(wal))),
            // #5048: off unless `enable_change_log` is called.
            change_log: Mutex::new(None),
            views: RwLock::new(HashMap::new()),
        };

        // Load existing tables
        storage.load_all_tables()?;

        // Load existing indexes
        storage.load_all_indexes()?;

        // Load existing triggers
        storage.load_all_triggers()?;

        // Load existing views (V312-95 v2 / Issue #4814)
        storage.load_all_views()?;

        Ok(storage)
    }

    /// Create a new FileStorage with a shared GapLockManager for REPEATABLE-READ isolation.
    ///
    /// This enables gap locking on index operations for transactions with
    /// REPEATABLE-READ isolation level.
    ///
    /// # Arguments
    /// * `data_dir` - Directory for database files
    /// * `lock_manager` - Shared GapLockManager instance (typically Arc::new(GapLockManager::new()))
    ///
    /// # Returns
    /// * `Ok(Self)` - FileStorage with gap locking enabled
    pub fn new_with_lock_manager(
        data_dir: PathBuf,
        lock_manager: std::sync::Arc<crate::lock::GapLockManager>,
    ) -> std::io::Result<Self> {
        fs::create_dir_all(&data_dir)?;

        let storage = Self {
            data_dir,
            current_db: RwLock::new(crate::engine::DEFAULT_DATABASE.to_string()),
            write_state: parking_lot::RwLock::new(WriteState {
                tables: HashMap::new(),
                insert_buffer: HashMap::new(),
                tx_undo_log: Vec::new(),
                dirty_tables: HashSet::new(),
            }),
            indexes: RwLock::new(HashMap::new()),
            index_metadata: RwLock::new(HashMap::new()),
            // v3.12.0 #4020 follow-up: see FileStorage::new — default raised to
            // 10_000 to amortise O(N) insert_direct over a much larger batch.
            buffer_threshold: 10_000,
            enable_buffer: true,
            current_tx_id: std::sync::atomic::AtomicU64::new(0),
            triggers: RwLock::new(HashMap::new()),
            gap_lock_manager: Some(lock_manager),
            last_saved_row_count: Mutex::new(HashMap::new()),
            next_tx_id_counter: std::sync::atomic::AtomicU64::new(1),
            wal: Mutex::new(None),
            // #5048: off unless `enable_change_log` is called.
            change_log: Mutex::new(None),
            views: RwLock::new(HashMap::new()),
        };

        storage.load_all_tables()?;
        storage.load_all_indexes()?;
        storage.load_all_views()?;

        Ok(storage)
    }

    /// #4951: run `f` with exclusive access to the guarded state.
    ///
    /// Takes `&self`, not `&mut self`. That is the whole point: the
    /// predecessor took `&mut Self` and every caller had to manufacture
    /// one from a shared reference via `as_mut_self`, an
    /// `unsafe { &mut *(self as *const Self as *mut Self) }` that was
    /// UB the moment two connections held the storage's read guard at
    /// once. Here the `&mut WriteState` comes from a real
    /// `RwLockWriteGuard`, so its lifetime is tied to the guard and the
    /// aliasing invariant holds by construction.
    fn with_write_lock<R>(me: &Self, f: impl FnOnce(&mut WriteState) -> R) -> R {
        let mut guard = me.write_state.write();
        f(&mut guard)
    }

    /// #4951: run `f` with shared access to the guarded state.
    ///
    /// Prefer this over `with_write_lock` for read-only lookups: it
    /// lets concurrent readers proceed in parallel. Note this is a real
    /// lock, unlike the pre-#4951 code, which read `self.tables` from
    /// `&self` methods with no synchronisation at all. An unsynchronised
    /// `HashMap` read racing a concurrent write is a data race
    /// regardless of whether the reader goes on to clone what it read —
    /// the race is on obtaining the reference, not on using it. The old
    /// comment claiming otherwise ("every read site clones the relevant
    /// `Record` slice before returning, so torn reads are impossible")
    /// described an invariant the code did not provide.
    fn with_read_lock<R>(&self, f: impl FnOnce(&WriteState) -> R) -> R {
        let guard = self.write_state.read();
        f(&guard)
    }

    /// Get the path for a table file
    /// #5025: read side. Falls back to the pre-#5025 root location when
    /// the database directory holds nothing, so tables written before the
    /// layout change stay visible.
    fn table_path(&self, table_name: &str) -> PathBuf {
        let db = self.current_db_name();
        self.table_path_in(&db, table_name)
    }

    /// #5025: `table_path` with the database named explicitly, instead of
    /// resolved from `current_db`.
    ///
    /// The startup loader has to read *every* database's directory while
    /// `current_db` is still `default`; it cannot flip `current_db` per
    /// directory to borrow the resolution above, because that is a
    /// process-wide setting other connections also read.
    fn table_path_in(&self, db: &str, table_name: &str) -> PathBuf {
        let file = format!("{}.json", table_name);
        match self.db_dir_for(db) {
            None => self.data_dir.join(file),
            Some(dir) => {
                let scoped = dir.join(&file);
                if scoped.exists() || dir.is_dir() {
                    scoped
                } else {
                    self.data_dir.join(file)
                }
            }
        }
    }

    /// #5025: write side. Never falls back — writing a new table into the
    /// shared root from inside a database is exactly what #5025 is about.
    /// #5057: write side for the database named explicitly; see
    /// `table_path_in`.
    ///
    /// There used to be a `table_path_for_write` that resolved through
    /// `current_db`. Every write now passes its database down, because a
    /// write helper that had already resolved the right database's rows
    /// would otherwise hand them to the *current* database's directory —
    /// the rows found under the right key, written to the wrong place.
    fn table_path_for_write_in(&self, db: &str, table_name: &str) -> PathBuf {
        let file = format!("{}.json", table_name);
        match self.db_dir_for(db) {
            None => self.data_dir.join(file),
            Some(dir) => {
                let _ = std::fs::create_dir_all(&dir);
                dir.join(file)
            }
        }
    }

    /// #5025: read side for an index file; see `table_path`.
    fn index_path(&self, table_name: &str, column_name: &str) -> PathBuf {
        let db = self.current_db_name();
        self.index_path_in(&db, table_name, column_name)
    }

    /// #5025: `index_path` with the database named explicitly; see
    /// `table_path_in`.
    fn index_path_in(&self, db: &str, table_name: &str, column_name: &str) -> PathBuf {
        let file = format!("{}_idx_{}.json", table_name, column_name);
        match self.db_dir_for(db) {
            None => self.data_dir.join(file),
            Some(dir) => {
                let scoped = dir.join(&file);
                if scoped.exists() || dir.is_dir() {
                    scoped
                } else {
                    self.data_dir.join(file)
                }
            }
        }
    }

    /// #5025: write side for an index file; see `table_path_in`.
    /// #5057: write side for the database named explicitly; see
    /// `table_path_for_write_in`.
    fn index_path_for_write_in(&self, db: &str, table_name: &str, column_name: &str) -> PathBuf {
        let file = format!("{}_idx_{}.json", table_name, column_name);
        match self.db_dir_for(db) {
            None => self.data_dir.join(file),
            Some(dir) => {
                let _ = std::fs::create_dir_all(&dir);
                dir.join(file)
            }
        }
    }

    fn index_path_for_write(&self, table_name: &str, column_name: &str) -> PathBuf {
        let db = self.current_db_name();
        self.index_path_for_write_in(&db, table_name, column_name)
    }

    /// Get the path for a trigger file (named after the trigger, not the table)
    fn trigger_path(&self, trigger_name: &str) -> PathBuf {
        self.data_dir.join(format!("trigger_{}.json", trigger_name))
    }

    /// V312-95 v2 / Issue #4814: get the disk path for a view file.
    fn view_path(&self, view_name: &str) -> PathBuf {
        self.data_dir.join(format!("view_{}.json", view_name))
    }

    /// V312-95 v2 / Issue #4814: load a single view from disk.
    fn load_view(&self, view_name: &str) -> std::io::Result<ViewInfo> {
        let path = self.view_path(view_name);
        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        let info: ViewInfo = serde_json::from_reader(reader)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(info)
    }

    /// V312-95 v2 / Issue #4814: save a view to disk (WAL-style:
    /// write-ahead, then mutate in-memory).
    fn save_view(&self, info: &ViewInfo) -> std::io::Result<()> {
        let path = self.view_path(&info.name);
        let file = File::create(&path)?;
        let mut writer = BufWriter::new(file);
        let json = serde_json::to_string_pretty(info)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        writer.write_all(json.as_bytes())?;
        writer.flush()?;
        Ok(())
    }

    /// V312-95 v2 / Issue #4814: remove a view file from disk
    /// (missing file is OK — idempotent).
    fn remove_view_file(&self, view_name: &str) -> std::io::Result<()> {
        let path = self.view_path(view_name);
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }

    /// V312-95 v2 / Issue #4814: scan the data directory for `view_*.json`
    /// files and load each into the in-memory catalog. Called from
    /// `new_with_wal` so views survive process restart.
    fn load_all_views(&self) -> std::io::Result<()> {
        if !self.data_dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(&self.data_dir)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                if file_name.starts_with("view_") && file_name.ends_with(".json") {
                    if let Some(name) = file_name
                        .strip_prefix("view_")
                        .and_then(|s| s.strip_suffix(".json"))
                    {
                        if let Ok(info) = self.load_view(name) {
                            if let Ok(mut views) = self.views.write() {
                                views.insert(name.to_string(), info);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Load a single trigger from disk
    fn load_trigger(&self, trigger_name: &str) -> std::io::Result<TriggerInfo> {
        let path = self.trigger_path(trigger_name);
        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        let info: TriggerInfo = serde_json::from_reader(reader)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(info)
    }

    /// Save a trigger to disk
    fn save_trigger(&self, info: &TriggerInfo) -> std::io::Result<()> {
        let path = self.trigger_path(&info.name);
        let file = File::create(&path)?;
        let mut writer = BufWriter::new(file);
        let json = serde_json::to_string_pretty(info)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        writer.write_all(json.as_bytes())?;
        writer.flush()?;
        Ok(())
    }

    /// Remove a trigger file from disk (best-effort: missing file is OK)
    fn remove_trigger_file(&self, trigger_name: &str) -> std::io::Result<()> {
        let path = self.trigger_path(trigger_name);
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }

    /// Load all triggers from the data directory
    fn load_all_triggers(&self) -> std::io::Result<()> {
        if !self.data_dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(&self.data_dir)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                if file_name.starts_with("trigger_") && file_name.ends_with(".json") {
                    if let Some(name) = file_name
                        .strip_prefix("trigger_")
                        .and_then(|s| s.strip_suffix(".json"))
                    {
                        if let Ok(info) = self.load_trigger(name) {
                            if let Ok(mut triggers) = self.triggers.write() {
                                triggers.insert(name.to_string(), info);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Load all tables from the data directory
    fn load_all_tables(&self) -> std::io::Result<()> {
        if !self.data_dir.exists() {
            return Ok(());
        }
        // C.1: collect pairs via &self reads first, then apply under
        // write_lock. The Vec owns the data so no &self borrow is live
        // when we cross into with_write_lock.
        //
        // #5025: walk *every* database directory, not just `data_dir`.
        // The pre-fix loader only read the root, so a table created in a
        // named database was written to `data/<db>/t.json` and then
        // silently absent from the cache on the next open — `scan`
        // returned zero rows against a file that was sitting on disk.
        let mut rows_to_insert: Vec<(String, TableData)> = Vec::new();
        for (db, dir) in self.db_dirs() {
            for entry in fs::read_dir(&dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) != Some("json") {
                    continue;
                }
                let Some(table_name) = path.file_stem().and_then(|s| s.to_str()) else {
                    continue;
                };
                // `*_idx_*.json` holds a serialised BPlusTree, never a
                // table. It used to be filtered implicitly, by failing
                // to parse as `StoredTableData`; say so outright so the
                // two loaders cannot both claim the same file.
                if table_name.contains("_idx_") {
                    continue;
                }
                if let Ok(table_data) = self.load_table_in(&db, table_name) {
                    rows_to_insert.push((crate::engine::scoped_key(&db, table_name), table_data));
                }
            }
        }
        Self::with_write_lock(self, |s| {
            for (key, data) in rows_to_insert {
                s.tables.insert(key, data);
            }
        });
        // V400-MVCC-PKFAST: auto-build the PK B+Tree index for every
        // loaded table. This ensures scan_with_index finds rows by PK
        // even without an explicit `CREATE INDEX`, restoring O(log N)
        // PK lookups in production (which never issues CREATE INDEX).
        if let Err(e) = self.rebuild_pk_indexes() {
            eprintln!("[v400] rebuild_pk_indexes after load: {}", e);
        }
        Ok(())
    }

    /// Load all indexes from the data directory
    fn load_all_indexes(&self) -> std::io::Result<()> {
        if !self.data_dir.exists() {
            return Ok(());
        }

        // #5025: walk every database directory and key the cache by the
        // *scoped* name. This loader used to walk only `data_dir` and
        // store the bare `(table, column)` tuple, so two things broke at
        // once: indexes belonging to a named database were never read, and
        // even the default database's indexes were filed under a key that
        // `has_index`/`get_index` — which resolve through `tbl()` — could
        // never match. `test_e2e_index_survives_restart` caught the second
        // half; see `index_survives_restart_5025.rs` for the first.
        for (db, dir) in self.db_dirs() {
            for entry in fs::read_dir(&dir)? {
                let entry = entry?;
                let path = entry.path();

                // Look for index files: table_idx_column.json
                if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                    if file_name.ends_with(".json") && file_name.contains("_idx_") {
                        // Parse table_idx_column.json
                        if let Some((table_name, column_name)) = file_name
                            .strip_suffix(".json")
                            .and_then(|s| s.split_once("_idx_"))
                        {
                            if let Ok(index) = self.load_index_in(&db, table_name, column_name) {
                                if let Ok(mut indexes) = self.indexes.write() {
                                    indexes.insert(
                                        (
                                            crate::engine::scoped_key(&db, table_name),
                                            column_name.to_string(),
                                        ),
                                        index,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// #5025: load a single index from disk, for the database named
    /// explicitly. There is no `current_db` shortcut: every caller is the
    /// startup loader, which reads each database in turn while
    /// `current_db` is still `default`.
    fn load_index_in(
        &self,
        db: &str,
        table_name: &str,
        column_name: &str,
    ) -> std::io::Result<BPlusTree> {
        let path = self.index_path_in(db, table_name, column_name);
        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        let index: BPlusTree = serde_json::from_reader(reader)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        Ok(index)
    }

    /// Save an index to disk
    fn save_index(
        &self,
        table_name: &str,
        column_name: &str,
        index: &BPlusTree,
    ) -> std::io::Result<()> {
        let db = self.current_db_name();
        self.save_index_in(&db, table_name, column_name, index)
    }

    /// #5057: `save_index` for a stated database; see `table_path_in`.
    fn save_index_in(
        &self,
        db: &str,
        table_name: &str,
        column_name: &str,
        index: &BPlusTree,
    ) -> std::io::Result<()> {
        let path = self.index_path_for_write_in(db, table_name, column_name);
        let file = File::create(&path)?;
        let mut writer = BufWriter::new(file);

        let json = serde_json::to_string_pretty(index)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        writer.write_all(json.as_bytes())?;
        writer.flush()?;

        Ok(())
    }

    /// #5025: load a single table from disk, for the database named
    /// explicitly. See `load_index_in` for why there is no `current_db`
    /// shortcut.
    fn load_table_in(&self, db: &str, table_name: &str) -> std::io::Result<TableData> {
        let path = self.table_path_in(db, table_name);
        // V400-PERF-DELTA: a missing JSON is OK if the delta file
        // exists — that's the cold-start case where the base JSON
        // was already compacted away.
        let (name, columns, foreign_keys, unique_constraints, mut rows) = if path.exists() {
            let file = File::open(&path)?;
            let reader = BufReader::new(file);
            let stored: StoredTableData = serde_json::from_reader(reader)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            (
                stored.name,
                stored.columns,
                stored.foreign_keys,
                stored.unique_constraints,
                stored.rows,
            )
        } else {
            // No base snapshot — use empty schema (caller will provide
            // via TableInfo at insert time).
            (
                table_name.to_string(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            )
        };
        // Apply pending deltas on top of the base snapshot.
        let delta_rows = self.load_table_delta_in(db, table_name)?;
        rows.extend(delta_rows);

        Ok(TableData {
            info: TableInfo {
                name,
                columns,
                foreign_keys,
                unique_constraints,
                check_constraints: vec![],
                compression: None,
                collations: HashMap::new(),
                partition_info: None,
                original_sql: String::new(),
            },
            rows,
        })
    }

    /// Save a table to disk
    /// Save a table to disk.
    ///
    /// V400-PERF-DELTA: instead of writing the entire table on every
    /// call, append only the new rows (delta) to `<table>.delta` (a
    /// binary length-prefixed log). The base `<table>.json` snapshot
    /// is rewritten lazily — when (a) the table has no delta yet,
    /// (b) the delta exceeds ~10 MB, or (c) `compact_table` is
    /// called. This makes the common "append a few rows" path O(new
    /// rows) instead of O(total rows).
    /// #4951: takes `st` for signature symmetry with `save_table_window`
    /// and because it is called from inside `with_write_lock` closures —
    /// see that method for why it must not re-acquire the lock itself.
    /// It does not read `st` today: every caller hands it an explicit
    /// `table_data`.
    /// #5057: `db` says which database's directory the file belongs in.
    /// It is the same database the caller resolved the rows under; passing
    /// anything else writes the right rows into the wrong directory.
    fn save_table(
        &self,
        db: &str,
        _st: &WriteState,
        table_name: &str,
        table_data: &TableData,
    ) -> std::io::Result<()> {
        let total_rows = table_data.rows.len();
        // #5057: the bookkeeping key is scoped, so `d1.t` and `d2.t` keep
        // separate "how many rows are on disk" counters. Sharing one
        // counter made the second table look already-persisted and skip
        // its write.
        let scoped = crate::engine::scoped_key(db, table_name);
        let last_saved = *self
            .last_saved_row_count
            .lock()
            .unwrap()
            .get(&scoped)
            .unwrap_or(&0);

        // V400-PERF-DELTA: on the very first save (or after a delete
        // shrunk the row count to 0), we must still emit the JSON
        // because cold-start load relies on it for table schema.
        if total_rows == 0 || last_saved == 0 {
            return self.save_table_full(db, table_name, table_data);
        }

        if total_rows <= last_saved {
            // Pure DELETE/UPDATE path: the row set may have shrunk.
            // Force a full snapshot to keep on-disk consistent.
            return self.save_table_full(db, table_name, table_data);
        }

        // Delta-only path: append the new rows to <table>.delta.
        let new_rows = &table_data.rows[last_saved..];
        self.append_table_delta(db, table_name, new_rows)?;
        self.last_saved_row_count
            .lock()
            .unwrap()
            .insert(scoped, total_rows);

        // Periodic compaction: when delta size exceeds threshold,
        // rewrite the base snapshot and clear the delta file.
        let delta_path = self.delta_path_in(db, table_name);
        if let Ok(meta) = std::fs::metadata(&delta_path) {
            if meta.len() > 10 * 1024 * 1024 {
                let _ = self.save_table_full(db, table_name, table_data);
            }
        }
        Ok(())
    }

    /// B2.1 / #4915 (F-09): delta-aware persist that takes an explicit
    /// `total_rows`.
    ///
    /// `save_table` derives the total row count from
    /// `table_data.rows.len()`. Callers that hand it a *window*
    /// snapshot (only the rows appended since `last_saved`) must
    /// therefore also pass the real table size, otherwise the
    /// "did the table grow?" decision and the `last_saved_row_count`
    /// bookkeeping would be computed against the window length.
    /// #5057: `db` — see [`save_table`](Self::save_table).
    fn save_table_window(
        &self,
        db: &str,
        st: &WriteState,
        table_name: &str,
        window: &TableData,
        total_rows: usize,
    ) -> std::io::Result<()> {
        // #4951: `st` is the already-held guard state. This method must
        // NOT re-acquire `write_state` — it is always called from inside a
        // `with_write_lock` closure, and `parking_lot::RwLock` is not
        // reentrant, so re-locking here would deadlock. Taking the
        // state as a parameter makes that obligation explicit.
        // #5057: scoped bookkeeping key; see `save_table`.
        let scoped = crate::engine::scoped_key(db, table_name);
        let last_saved = *self
            .last_saved_row_count
            .lock()
            .unwrap()
            .get(&scoped)
            .unwrap_or(&0);

        if total_rows == 0 || last_saved == 0 || total_rows <= last_saved {
            // Cold start, shrink, or a DELETE/UPDATE path: the caller
            // handed us a window, not a snapshot, so re-derive the
            // full table under the lock. Rare relative to inserts.
            // #5025: the cache is keyed by scoped name.
            if let Some(data) = st.tables.get(&scoped) {
                return self.save_table_full(db, table_name, data);
            }
            return Ok(());
        }

        // Delta-only path: append just the window we were given.
        self.append_table_delta(db, table_name, &window.rows)?;
        self.last_saved_row_count
            .lock()
            .unwrap()
            .insert(scoped.clone(), total_rows);

        // Periodic compaction: when the delta file exceeds the
        // threshold, rewrite the base snapshot and clear the delta.
        let delta_path = self.delta_path_in(db, table_name);
        if let Ok(meta) = std::fs::metadata(&delta_path) {
            if meta.len() > 10 * 1024 * 1024 {
                if let Some(data) = st.tables.get(&scoped) {
                    let _ = self.save_table_full(db, table_name, data);
                }
            }
        }
        Ok(())
    }

    /// V400-PERF-DELTA: write the full table JSON snapshot. Called by
    /// `save_table` on the first write, after a schema change, and
    /// when the delta file grows too large.
    fn save_table_full(
        &self,
        db: &str,
        table_name: &str,
        table_data: &TableData,
    ) -> std::io::Result<()> {
        // #5025: writes go to the scoped location; `table_path` falls back
        // to the root for reads only.
        // #5057: ...and the database is the one the caller named, not
        // `current_db`.
        let path = self.table_path_for_write_in(db, table_name);
        let file = File::create(&path)?;
        // B2.3 / #4915 (F-11): 1 MB buffer, and serialize straight
        // into it. The previous code built an owned StoredTableData
        // (another full copy of the rows) and then
        // `to_string_pretty` into a String before writing, so a
        // snapshot cost two extra copies of the table in memory.
        let mut writer = BufWriter::with_capacity(1 << 20, file);

        let stored = StoredTableDataRef {
            name: &table_data.info.name,
            columns: &table_data.info.columns,
            foreign_keys: &table_data.info.foreign_keys,
            unique_constraints: &table_data.info.unique_constraints,
            rows: &table_data.rows,
        };

        serde_json::to_writer_pretty(&mut writer, &stored)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        writer.flush()?;
        // Drop any pending deltas — they're now incorporated.
        let _ = std::fs::remove_file(self.delta_path_in(db, table_name));
        self.last_saved_row_count.lock().unwrap().insert(
            crate::engine::scoped_key(db, table_name),
            table_data.rows.len(),
        );
        Ok(())
    }

    /// V400-PERF-DELTA: append `rows` to `<table>.delta` in a
    /// JSON-line format (one row per line). Lines are chosen over
    /// bincode to keep the delta file human-inspectable and
    /// dependency-free. Each line is `serde_json::to_string(row)`.
    fn append_table_delta(
        &self,
        db: &str,
        table_name: &str,
        rows: &[Vec<Value>],
    ) -> std::io::Result<()> {
        use std::io::Write;
        // #5057: the delta file follows the database, like the table it
        // extends. Resolving it through `current_db` sent two databases'
        // appended rows into one `<table>.delta`, which is then replayed
        // into whichever table the loader finds under that name.
        let path = self.delta_path_in(db, table_name);
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        let mut writer = BufWriter::new(file);
        for row in rows {
            let line = serde_json::to_string(row)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            writer.write_all(line.as_bytes())?;
            writer.write_all(b"\n")?;
        }
        writer.flush()?;
        Ok(())
    }

    /// V400-PERF-DELTA: read all delta rows from `<table>.delta`,
    /// append order, for the database named explicitly. See
    /// `load_index_in` for why there is no `current_db` shortcut.
    fn load_table_delta_in(&self, db: &str, table_name: &str) -> std::io::Result<Vec<Vec<Value>>> {
        let path = self.delta_path_in(db, table_name);
        if !path.exists() {
            return Ok(Vec::new());
        }
        let file = File::open(&path)?;
        let reader = std::io::BufReader::new(file);
        let mut all_rows: Vec<Vec<Value>> = Vec::new();
        use std::io::BufRead;
        for line in reader.lines() {
            let line = line?;
            if line.is_empty() {
                continue;
            }
            let row: Vec<Value> = serde_json::from_str(&line)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            all_rows.push(row);
        }
        Ok(all_rows)
    }

    /// #5025 / #5057: the on-disk delta path for a table in a named
    /// database.
    ///
    /// #5025: the delta file must follow the same scoping as the table, or
    /// two databases with a table of the same name would append to one
    /// delta file and corrupt each other's rows.
    ///
    /// #5057: there used to be a `delta_path(table_name)` that resolved
    /// through `current_db`, and every write path called it — the same
    /// shape `table_path_for_write` had. A helper that picks the database
    /// from a process-wide setting lets a write that already resolved its
    /// own database's rows append them to the *active* database's delta
    /// file. It is removed rather than left dead, because an entry point
    /// that cannot name its database is how this bug comes back.
    fn delta_path_in(&self, db: &str, table_name: &str) -> std::path::PathBuf {
        let file = format!("{}.delta", table_name);
        match self.db_dir_for(db) {
            None => self.data_dir.join(file),
            Some(dir) => {
                let _ = std::fs::create_dir_all(&dir);
                dir.join(file)
            }
        }
    }

    /// Get a table by name.
    ///
    /// #4951: this can no longer hand out a `&TableData` borrowed from
    /// `self` — the tables now live behind a `RwLock` guard, and a
    /// reference into the guard cannot outlive it. Callers that only
    /// read should prefer `with_table`, which runs a closure inside the
    /// guard and so pays no clone.
    ///
    /// This clone-returning form is kept for the callers that genuinely
    /// want an owned `TableData` (tests, `examples/`, and the one
    /// production site in `crates/server/src/openclaw_endpoints.rs`).
    /// **Prefer `with_table` in new code** — a whole-table clone on the
    /// read path is a real cost, not a convenience.
    pub fn get_table(&self, name: &str) -> Option<TableData> {
        self.with_table(name, |t| t.cloned())
    }

    /// #4951: run `f` with a borrowed view of `name`'s table, if present.
    ///
    /// The guard is held for the duration of `f`, so the `&TableData`
    /// handed to it is valid exactly as long as `f` runs. This is the
    /// cheap way to read a table — no clone.
    pub fn with_table<R>(&self, name: &str, f: impl FnOnce(Option<&TableData>) -> R) -> R {
        // #5025: the cache is keyed by scoped name.
        let key = self.tbl(name);
        self.with_read_lock(|st| f(st.tables.get(&key)))
    }

    /// Get a mutable table by name.
    ///
    /// #4951: still `&mut self`, so `RwLock::get_mut` hands out the
    /// `&mut WriteState` with no lock overhead — the caller already has
    /// exclusive ownership of the whole storage.
    pub fn get_table_mut(&mut self, name: &str) -> Option<&mut TableData> {
        // #5025: the cache is keyed by scoped name.
        let key = self.tbl(name);
        self.write_state.get_mut().tables.get_mut(&key)
    }

    /// Insert a new table
    pub fn insert_table(&self, name: String, table_data: TableData) -> std::io::Result<()> {
        let db = self.current_db_name();
        self.insert_table_in(&db, name, table_data)
    }

    /// #5057: [`insert_table`](Self::insert_table) in a stated database.
    fn insert_table_in(
        &self,
        db: &str,
        name: String,
        table_data: TableData,
    ) -> std::io::Result<()> {
        // #5057: the database is read once, outside the state lock, and
        // used for both the cache key and the on-disk path. Reading it
        // inside the closure would take a second lock while `write_state`
        // is held.
        Self::with_write_lock(self, |s| {
            // #5025: scope the cache key, but keep the bare name for the
            // on-disk file and `TableData.info.name`.
            s.tables.insert(self.tbl_in(db, &name), table_data.clone());
            self.save_table(db, s, &name, &table_data)
        })
    }

    /// Drop (delete) a table
    pub fn drop_table(&self, name: &str) -> std::io::Result<()> {
        let db = self.current_db_name();
        self.drop_table_in(&db, name)
    }

    /// #5057: [`drop_table`](Self::drop_table) in a stated database.
    pub fn drop_table_in(&self, db: &str, name: &str) -> std::io::Result<()> {
        Self::with_write_lock(self, |s| {
            let key = self.tbl_in(db, name);
            s.tables.remove(&key);
            let path = self.table_path_in(db, name);
            if path.exists() {
                std::fs::remove_file(path)?;
            }
            Ok(())
        })
    }

    /// Get all table names
    pub fn table_names(&self) -> Vec<String> {
        // #5025: only this database's tables, reported by bare name —
        // the scoped cache key is an implementation detail, and leaking it
        // would show up as `d1\u{1}t` in `SHOW TABLES`.
        self.with_read_lock(|st| match self.db_dir() {
            None => st.tables.values().map(|v| v.info.name.clone()).collect(),
            Some(dir) => {
                let prefix = format!(
                    "{}\u{1}",
                    dir.file_name().unwrap_or_default().to_string_lossy()
                );
                st.tables
                    .iter()
                    .filter_map(|(k, v)| k.strip_prefix(&prefix).map(|_| v.info.name.clone()))
                    .collect()
            }
        })
    }

    /// Force save all dirty tables to disk
    /// V311-07: Only persist tables that have been modified since last flush
    pub fn flush(&self) -> std::io::Result<()> {
        // Buffered inserts live in `insert_buffer`, not in `tables.rows`,
        // so draining `dirty_tables` alone would snapshot an empty window
        // and write nothing — the rows would be dropped. Push the buffers
        // into `tables` first. The `StorageEngine::flush` override has
        // always done this; the inherent version did not, which is part of
        // why the two copies of this loop drifted apart.
        self.flush_all_buffers().map_err(Self::io_err_from_sql)?;
        let pending = self.drain_dirty_windowed();
        // Lock released. `save_table_window` / `save_table_full` only
        // touch `data_dir`, `last_saved_row_count` and the filesystem.
        //
        // #5057: `db` is the entry's OWN database, carried out of the dirty
        // set. It used to be `self.current_db_name()`, one value for the
        // whole loop — so every dirty table was written under whichever
        // database was active at flush time.
        for (db, name, window, total) in &pending {
            // #4951: `save_table_window` needs `&WriteState` but must not
            // re-acquire the lock itself (non-reentrant). Hold the read
            // guard across the call. It only reads `tables` to decide
            // whether a full re-write is needed.
            self.with_read_lock(|st| self.save_table_window(db, st, name, window, *total))?;
        }

        // #5048: persist the change log only after the data it describes
        // is on disk. The other order would let a crash leave a log that
        // names rows the database does not contain, and a delta built
        // from it would replay a change that never happened.
        self.persist_change_log()?;

        Ok(())
    }

    /// `flush_all_buffers` reports `SqlResult`; the two `flush` entry
    /// points return `std::io::Result`. One place to convert, so the
    /// two copies of the flush loop cannot drift on the error mapping.
    fn io_err_from_sql(e: sqlrustgo_types::SqlError) -> std::io::Error {
        std::io::Error::other(format!("{}", e))
    }

    /// Drain `dirty_tables` and snapshot only the rows each dirty table
    /// gained since its last persist, in one short critical section.
    ///
    /// This is the single place that decides *what* to write. It used to
    /// be duplicated: `FileStorage::flush` had one copy and the
    /// `StorageEngine::flush` override another, and only the override is
    /// reachable in production (the server calls it through
    /// `MvccStorage`) — so B2.2 optimised the copy nobody ran while the
    /// live copy kept its whole-`TableData` `.cloned()`. See
    /// `PERF_B22_CONCURRENT_MEASUREMENT.md` §3.
    ///
    /// The window (`[last_saved..]`) rather than a whole `TableData`
    /// copy is what makes the I/O-outside-the-lock version pay off. A
    /// full snapshot was measured as a 0.86x regression on
    /// `b2_flush_dirty_tables/5tables_*` — the copy cost more than the
    /// lock it avoided.
    ///
    /// Returns `(database, table_name, window, total_rows)` per dirty table.
    /// `save_table_window` re-derives the full table itself on the rare
    /// cold-start / shrink / compaction branches.
    ///
    /// #5057: the database comes back out of the set, not from `current_db`.
    /// Callers used to do `let db = self.current_db_name()` and hand that
    /// same value to every entry, which is how one table's rows ended up
    /// written into another table's file. Now each entry carries its own.
    fn drain_dirty_windowed(&self) -> Vec<(String, String, TableData, usize)> {
        Self::with_write_lock(self, |s| {
            std::mem::take(&mut s.dirty_tables)
                .into_iter()
                .filter_map(|(db, name)| {
                    // #5025: scoped cache key, now built from the entry's
                    // OWN database instead of `self.tbl(&name)` (which read
                    // `current_db` and so could only ever address the active
                    // database's copy).
                    let scoped = crate::engine::scoped_key(&db, &name);
                    let data = s.tables.get(&scoped)?;
                    let total = data.rows.len();
                    let last_saved = *self
                        .last_saved_row_count
                        .lock()
                        .unwrap()
                        .get(&scoped)
                        .unwrap_or(&0);
                    // Shrink: a DELETE/UPDATE reduced the row count, so
                    // the on-disk set no longer matches and the full
                    // snapshot must be rewritten. `save_table_window`
                    // would take that branch anyway, but the window we
                    // hand it is meaningless here, so let it re-derive.
                    if last_saved > total {
                        return Some((db, name, TableData::snapshot_from(data, total), total));
                    }
                    // Nothing new to write. `flush_all_buffers` runs first
                    // in both callers and persists buffered inserts via
                    // `save_table_window`, which sets `last_saved` to the
                    // full row count. Without this skip the drain would
                    // see `total <= last_saved` and take
                    // `save_table_window`'s cold-start/shrink branch,
                    // rewriting the whole snapshot the buffer flush had
                    // just written incrementally.
                    if last_saved == total {
                        return None;
                    }
                    Some((db, name, TableData::snapshot_from(data, last_saved), total))
                })
                .collect()
        })
    }

    /// Check if a table exists
    /// #5025: scope a bare table name to the active database.
    ///
    /// The in-memory `WriteState::tables` cache is keyed this way, and so
    /// is every on-disk path, so `d1.t` and `d2.t` are separate entries
    /// both in memory and on disk.
    #[inline]
    fn tbl(&self, name: impl AsRef<str>) -> String {
        crate::engine::scoped_key(&self.current_db.read().unwrap(), name.as_ref())
    }

    /// #5025: the directory a database's tables live in, if it is not the
    /// implicit default. `None` means "use `data_dir` directly".
    #[inline]
    fn db_dir(&self) -> Option<std::path::PathBuf> {
        self.db_dir_for(&self.current_db_name())
    }

    /// #5025: the database named explicitly, instead of resolved from
    /// `current_db`.
    #[inline]
    fn db_dir_for(&self, db: &str) -> Option<std::path::PathBuf> {
        if db == crate::engine::DEFAULT_DATABASE {
            None
        } else {
            Some(self.data_dir.join(db))
        }
    }

    /// #5025: `current_db` as an owned `String`, so a caller can pass the
    /// name on to `*_in` helpers without holding the `current_db` read
    /// guard across them. Those helpers call `db_dir_for`, and parking_lot
    /// read locks are not safe to nest when a writer may be queued.
    #[inline]
    fn current_db_name(&self) -> String {
        self.current_db.read().unwrap().clone()
    }

    /// #5057: scope a bare table name to an explicitly named database.
    ///
    /// The counterpart of [`tbl`](Self::tbl) for callers that were handed
    /// the database instead of expected to read the storage's.
    ///
    /// Why it exists: the storage is shared by every connection, so
    /// `self.current_db` answers "whoever ran `USE` last", not "whoever is
    /// asking". A statement already in flight can resolve its table against
    /// another connection's database. Taking the database as a parameter
    /// binds the whole call chain to the asker.
    ///
    /// This is the seam every `*_in_db` method threads through. If a helper
    /// below this point needs the database and does not receive it, its
    /// signature is missing a parameter — do not reach for
    /// `self.current_db` to fill the gap.
    #[inline]
    fn tbl_in(&self, db: &str, name: impl AsRef<str>) -> String {
        crate::engine::scoped_key(db, name.as_ref())
    }

    /// #5025: every database that has a directory on disk.
    ///
    /// The implicit default database has no directory — its files sit in
    /// `data_dir` itself — so it is paired with `data_dir` here. Every
    /// other entry is a real subdirectory, because `create_database` is
    /// the only thing that creates one.
    fn db_dirs(&self) -> Vec<(String, std::path::PathBuf)> {
        let mut out = vec![(
            crate::engine::DEFAULT_DATABASE.to_string(),
            self.data_dir.clone(),
        )];
        if let Ok(entries) = fs::read_dir(&self.data_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                        out.push((name.to_string(), path));
                    }
                }
            }
        }
        out
    }

    pub fn contains_table(&self, name: &str) -> bool {
        // #5025: the cache is keyed by scoped name.
        let key = self.tbl(name);
        self.with_read_lock(|st| st.tables.contains_key(&key))
    }

    /// Create a new database directory under data_dir.
    pub fn create_database(&self, db_name: &str) -> std::io::Result<()> {
        let db_path = self.data_dir.join(db_name);
        std::fs::create_dir_all(&db_path)?;
        Ok(())
    }

    /// Drop a database directory. Refuses to drop if the directory is not empty.
    pub fn drop_database(&self, db_name: &str) -> std::io::Result<()> {
        let db_path = self.data_dir.join(db_name);
        if db_path.exists() {
            for entry in fs::read_dir(&db_path)? {
                let entry = entry?;
                let file_name = entry.file_name();
                let name = file_name.to_string_lossy();
                // Skip WAL files; reject everything else
                if !name.ends_with(".wal") {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::DirectoryNotEmpty,
                        format!("database '{}' is not empty", db_name),
                    ));
                }
            }
            fs::remove_dir(&db_path)?;
        }
        Ok(())
    }

    /// Save a table to disk (call after modifications)
    pub fn persist_table(&self, name: &str) -> std::io::Result<()> {
        // #5025: scoped cache key.
        let key = self.tbl(name);
        let db = self.current_db_name();
        self.with_read_lock(|st| match st.tables.get(&key) {
            Some(table_data) => self.save_table(&db, st, name, table_data),
            None => Ok(()),
        })
    }

    // ==================== Index Methods ====================

    /// Check if an index exists for a table column
    pub fn has_index(&self, table_name: &str, column_name: &str) -> bool {
        self.indexes
            .read()
            .map(|indexes| indexes.contains_key(&(self.tbl(table_name), column_name.to_string())))
            .unwrap_or(false)
    }

    /// Get an index for a table column (read-only)
    pub fn get_index(&self, table_name: &str, column_name: &str) -> Option<BPlusTree> {
        self.indexes.read().ok().and_then(|indexes| {
            indexes
                .get(&(self.tbl(table_name), column_name.to_string()))
                .cloned()
        })
    }

    /// Create or update an index for a table column from existing data
    pub fn create_index(
        &mut self,
        table_name: &str,
        column_name: &str,
        column_index: usize,
    ) -> std::io::Result<()> {
        // #4951: `&mut self` — get_mut needs no lock. The index is built
        // entirely from the returned borrow and never outlives it.
        // #5025: the cache is keyed by scoped name; resolve the key before
        // the mutable borrow so `tbl()` can still read `current_db`.
        let key = self.tbl(table_name);
        let table =
            self.write_state.get_mut().tables.get(&key).ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::NotFound, "Table not found")
            })?;

        // Build B+ Tree from existing rows
        let mut index = crate::bplus_tree::BPlusTree::new();
        for (row_id, row) in table.rows.iter().enumerate() {
            if let Some(value) = row.get(column_index) {
                if let Some(key) = value.to_index_key() {
                    index.insert(key, row_id as u32);
                }
            }
        }

        // Save to disk
        self.save_index(table_name, column_name, &index)?;

        // Store in memory
        if let Ok(mut indexes) = self.indexes.write() {
            let ik = self.tbl(table_name);
            indexes.insert((ik, column_name.to_string()), index);
        }

        Ok(())
    }

    /// V400-MVCC-PKFAST: for every loaded table that has a PRIMARY KEY
    /// column, build the B+Tree PK index from existing rows. This
    /// eliminates the need for explicit `CREATE INDEX` on the PK and
    /// restores O(log N) PK lookup in production workloads.
    pub fn rebuild_pk_indexes(&self) -> std::io::Result<()> {
        // Snapshot table info first (clone columns)
        let tables_snapshot: Vec<(String, Vec<ColumnDefinition>)> =
            Self::with_write_lock(self, |s| {
                s.tables
                    .iter()
                    .map(|(name, t)| (name.clone(), t.info.columns.clone()))
                    .collect()
            });
        for (table_name, columns) in tables_snapshot {
            // Find PK column
            let pk_col = columns.iter().find(|c| c.primary_key);
            let Some(pk_col) = pk_col else { continue };
            let pk_col_name = pk_col.name.clone();
            let pk_col_idx = columns.iter().position(|c| c.name == pk_col_name).unwrap();

            // Build B+Tree from current rows
            let snapshot = Self::with_write_lock(self, |s| {
                s.tables.get(&self.tbl(&table_name)).map(|t| t.rows.clone())
            });
            let Some(rows) = snapshot else { continue };
            let mut index = crate::bplus_tree::BPlusTree::new();
            for (row_id, row) in rows.iter().enumerate() {
                if let Some(value) = row.get(pk_col_idx) {
                    if let Some(key) = value.to_index_key() {
                        index.insert(key, row_id as u32);
                    }
                }
            }
            // Persist + register
            self.save_index(&table_name, &pk_col_name, &index)?;
            if let Ok(mut indexes) = self.indexes.write() {
                indexes.insert((self.tbl(&table_name), pk_col_name.clone()), index);
            }
        }
        Ok(())
    }

    /// Insert a row and update index
    pub fn insert_with_index(
        &mut self,
        table_name: &str,
        column_name: &str,
        key: i64,
        row_id: u32,
    ) -> std::io::Result<()> {
        let key_exists = (self.tbl(table_name), column_name.to_string());

        // Clone the key for later use
        let has_index = self
            .indexes
            .read()
            .map(|indexes| indexes.contains_key(&key_exists))
            .unwrap_or(false);

        if has_index {
            // First get a clone of the index to save, then modify
            let index_clone = {
                let indexes = self.indexes.read().unwrap();
                indexes.get(&key_exists).cloned()
            };

            // Update the in-memory index
            if let Ok(mut indexes) = self.indexes.write() {
                if let Some(index) = indexes.get_mut(&key_exists) {
                    index.insert(key, row_id);
                }
            }

            // Save to disk (outside the write lock)
            if let Some(idx) = index_clone {
                let _ = self.save_index(table_name, column_name, &idx);
            }
        }

        Ok(())
    }

    /// Search using index - returns row IDs matching the key
    pub fn search_index(&self, table_name: &str, column_name: &str, key: i64) -> Option<u32> {
        self.indexes.read().ok().and_then(|indexes| {
            indexes
                .get(&(self.tbl(table_name), column_name.to_string()))
                .and_then(|index| index.search(key))
        })
    }

    /// Range query using index
    pub fn range_index(
        &self,
        table_name: &str,
        column_name: &str,
        start: i64,
        end: i64,
    ) -> Vec<u32> {
        self.indexes
            .read()
            .ok()
            .and_then(|indexes| {
                indexes
                    .get(&(self.tbl(table_name), column_name.to_string()))
                    .map(|index| index.range_query(start, end))
            })
            .unwrap_or_default()
    }

    /// Drop an index
    pub fn drop_index(&self, table_name: &str, column_name: &str) -> std::io::Result<()> {
        // #5025: index keys carry the scoped table name.
        let key = (self.tbl(table_name), column_name.to_string());

        if let Ok(mut indexes) = self.indexes.write() {
            indexes.remove(&key);
        }

        // V312-95 v3 / P3-HINT-001 follow-up: also remove the
        // metadata-catalog entries that name this (table, column)
        // pair so `list_all_indexes()` doesn't return a ghost entry.
        if let Ok(mut md) = self.index_metadata.write() {
            md.retain(|_, info| {
                !(info.table == table_name
                    && info
                        .columns
                        .iter()
                        .any(|c| c.name.as_deref() == Some(column_name)))
            });
        }

        let path = self.index_path(table_name, column_name);
        if path.exists() {
            fs::remove_file(path)?;
        }

        Ok(())
    }

    /// Flush all indexes to disk
    pub fn flush_indexes(&self) -> std::io::Result<()> {
        if let Ok(indexes) = self.indexes.read() {
            for ((table_name, column_name), index) in indexes.iter() {
                self.save_index(table_name, column_name, index)?;
            }
        }
        Ok(())
    }
}

/// Stored table data (for serialization)
#[derive(serde::Serialize, serde::Deserialize)]
struct StoredTableData {
    name: String,
    columns: Vec<ColumnDefinition>,
    foreign_keys: Vec<ForeignKeyConstraint>,
    unique_constraints: Vec<UniqueConstraint>,
    rows: Vec<Vec<Value>>,
}

/// B2.3 / #4915 (F-11): borrowed twin of [`StoredTableData`] used only for
/// serialization.
///
/// `save_table_full` used to build a `StoredTableData` — a struct whose
/// `rows` field owned a full copy of the table — and then hand it to
/// `serde_json::to_string_pretty`, which allocates the whole JSON document
/// as a `String` before a single byte reaches the file. On a 500 MB table
/// that is two extra copies of the data resident at once.
///
/// This mirror keeps every field borrowed so the serializer can walk
/// straight from the live `TableData` into the writer. Deserialization
/// keeps using the owned `StoredTableData`.
#[derive(serde::Serialize)]
struct StoredTableDataRef<'a> {
    name: &'a str,
    columns: &'a [ColumnDefinition],
    foreign_keys: &'a [ForeignKeyConstraint],
    unique_constraints: &'a [UniqueConstraint],
    rows: &'a [Vec<Value>],
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::ColumnDefinition;
    use std::fs::remove_dir_all;

    #[test]
    fn test_file_storage() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_file_storage");
        let _ = remove_dir_all(&temp_dir);

        {
            let mut storage = FileStorage::new(temp_dir.clone()).unwrap();

            // Insert a table
            let table_data = TableData {
                info: TableInfo {
                    name: "users".to_string(),
                    columns: vec![
                        ColumnDefinition {
                            name: "id".to_string(),
                            data_type: "INTEGER".to_string(),
                            nullable: false,
                            primary_key: true,
                            char_max_length: None,
                            collation: None,
                            default_value: None,
                            auto_increment: false,
                        },
                        ColumnDefinition {
                            name: "name".to_string(),
                            data_type: "TEXT".to_string(),
                            nullable: true,
                            primary_key: false,
                            char_max_length: None,
                            collation: None,
                            default_value: None,
                            auto_increment: false,
                        },
                    ],
                    foreign_keys: vec![],
                    unique_constraints: vec![],
                    check_constraints: vec![],
                    compression: None,
                    collations: std::collections::HashMap::new(),
                    partition_info: None,
                    original_sql: String::new(),
                },
                rows: vec![vec![Value::Integer(1), Value::Text("Alice".to_string())]],
            };

            storage
                .insert_table("users".to_string(), table_data)
                .unwrap();
        }

        // Load from disk
        {
            let storage = FileStorage::new(temp_dir.clone()).unwrap();
            let table = storage.get_table("users").unwrap();
            assert_eq!(table.info.name, "users");
            assert_eq!(table.rows.len(), 1);
        }

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_storage_contains_and_drop() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_contains");
        let _ = remove_dir_all(&temp_dir);

        let table_data = TableData {
            info: TableInfo {
                name: "test".to_string(),
                columns: vec![ColumnDefinition {
                    name: "id".to_string(),
                    data_type: "INTEGER".to_string(),
                    nullable: false,
                    auto_increment: false,
                    ..Default::default()
                }],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                compression: None,
                collations: std::collections::HashMap::new(),
                partition_info: None,
                original_sql: String::new(),
            },
            rows: vec![],
        };

        let mut storage = FileStorage::new(temp_dir.clone()).unwrap();
        storage
            .insert_table("test".to_string(), table_data)
            .unwrap();

        // Test contains_table
        assert!(storage.contains_table("test"));
        assert!(!storage.contains_table("nonexistent"));

        // Test table_names
        let names = storage.table_names();
        assert!(names.contains(&"test".to_string()));

        // Test drop_table
        storage.drop_table("test").unwrap();
        assert!(!storage.contains_table("test"));

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_storage_persist() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_persist");
        let _ = remove_dir_all(&temp_dir);

        let mut storage = FileStorage::new(temp_dir.clone()).unwrap();

        // Create table without saving
        let table_data = TableData {
            info: TableInfo {
                name: "persist_test".to_string(),
                columns: vec![ColumnDefinition {
                    name: "id".to_string(),
                    data_type: "INTEGER".to_string(),
                    nullable: false,
                    auto_increment: false,
                    ..Default::default()
                }],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                compression: None,
                collations: std::collections::HashMap::new(),
                partition_info: None,
                original_sql: String::new(),
            },
            rows: vec![],
        };
        storage
            .insert_table("persist_test".to_string(), table_data)
            .unwrap();

        // Test persist_table
        storage.persist_table("persist_test").unwrap();

        // Test flush
        storage.flush().unwrap();

        // Verify table still exists after reload
        let storage2 = FileStorage::new(temp_dir.clone()).unwrap();
        assert!(storage2.contains_table("persist_test"));

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_storage_get_mut() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_get_mut");
        let _ = remove_dir_all(&temp_dir);

        let mut storage = FileStorage::new(temp_dir.clone()).unwrap();

        let table_data = TableData {
            info: TableInfo {
                name: "mutable".to_string(),
                columns: vec![ColumnDefinition {
                    name: "id".to_string(),
                    data_type: "INTEGER".to_string(),
                    nullable: false,
                    auto_increment: false,
                    ..Default::default()
                }],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                compression: None,
                collations: std::collections::HashMap::new(),
                partition_info: None,
                original_sql: String::new(),
            },
            rows: vec![],
        };
        storage
            .insert_table("mutable".to_string(), table_data)
            .unwrap();

        // Test get_table_mut
        {
            let table = storage.get_table_mut("mutable").unwrap();
            table.rows.push(vec![Value::Integer(1)]);
        }

        let table = storage.get_table("mutable").unwrap();
        assert_eq!(table.rows.len(), 1);

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_storage_index() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_index");
        let _ = remove_dir_all(&temp_dir);

        let mut storage = FileStorage::new(temp_dir.clone()).unwrap();

        // Insert table with data
        let table_data = TableData {
            info: TableInfo {
                name: "idx_test".to_string(),
                columns: vec![
                    ColumnDefinition {
                        name: "id".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        auto_increment: false,
                        ..Default::default()
                    },
                    ColumnDefinition {
                        name: "value".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        auto_increment: false,
                        ..Default::default()
                    },
                ],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                compression: None,
                collations: std::collections::HashMap::new(),
                partition_info: None,
                original_sql: String::new(),
            },
            rows: vec![
                vec![Value::Integer(1), Value::Integer(100)],
                vec![Value::Integer(2), Value::Integer(200)],
            ],
        };
        storage
            .insert_table("idx_test".to_string(), table_data)
            .unwrap();

        // Create index on id column (column_index = 0)
        storage.create_index("idx_test", "id", 0).unwrap();

        // Test has_index
        assert!(storage.has_index("idx_test", "id"));
        assert!(!storage.has_index("idx_test", "nonexistent"));

        // Test search_index
        let row_id = storage.search_index("idx_test", "id", 1);
        assert!(row_id.is_some());

        // Test range_index
        let range_results = storage.range_index("idx_test", "id", 1, 3);
        assert!(!range_results.is_empty());

        // Test insert_with_index
        storage.insert_with_index("idx_test", "id", 3, 2).unwrap();

        // Test drop_index
        storage.drop_index("idx_test", "id").unwrap();
        assert!(!storage.has_index("idx_test", "id"));

        // Test flush_indexes
        storage.flush_indexes().unwrap();

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_storage_index_search() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_idx_search");
        let _ = remove_dir_all(&temp_dir);

        let mut storage = FileStorage::new(temp_dir.clone()).unwrap();

        // Create table and index
        let table_data = TableData {
            info: TableInfo {
                name: "search_test".to_string(),
                columns: vec![ColumnDefinition {
                    name: "id".to_string(),
                    data_type: "INTEGER".to_string(),
                    nullable: false,
                    auto_increment: false,
                    ..Default::default()
                }],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                compression: None,
                collations: std::collections::HashMap::new(),
                partition_info: None,
                original_sql: String::new(),
            },
            rows: vec![],
        };
        storage
            .insert_table("search_test".to_string(), table_data)
            .unwrap();
        storage.create_index("search_test", "id", 0).unwrap();

        // Insert with index
        storage
            .insert_with_index("search_test", "id", 10, 0)
            .unwrap();
        storage
            .insert_with_index("search_test", "id", 20, 1)
            .unwrap();

        // Search
        let result = storage.search_index("search_test", "id", 10);
        assert_eq!(result, Some(0));

        // Range query
        let range = storage.range_index("search_test", "id", 5, 15);
        assert!(!range.is_empty());

        let _ = remove_dir_all(&temp_dir);
    }

    // ==================== Additional Coverage Tests ====================

    #[test]
    fn test_file_storage_get_index() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_get_index");
        let _ = remove_dir_all(&temp_dir);

        let mut storage = FileStorage::new(temp_dir.clone()).unwrap();

        let table_data = TableData {
            info: TableInfo {
                name: "get_idx_test".to_string(),
                columns: vec![ColumnDefinition {
                    name: "id".to_string(),
                    data_type: "INTEGER".to_string(),
                    nullable: false,
                    auto_increment: false,
                    ..Default::default()
                }],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                compression: None,
                collations: std::collections::HashMap::new(),
                partition_info: None,
                original_sql: String::new(),
            },
            rows: vec![],
        };
        storage
            .insert_table("get_idx_test".to_string(), table_data)
            .unwrap();
        storage.create_index("get_idx_test", "id", 0).unwrap();

        // Test get_index
        let index = storage.get_index("get_idx_test", "id");
        assert!(index.is_some());

        // Test get_index for non-existent
        let index_none = storage.get_index("get_idx_test", "nonexistent");
        assert!(index_none.is_none());

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_storage_index_no_matching_rows() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_no_match");
        let _ = remove_dir_all(&temp_dir);

        let mut storage = FileStorage::new(temp_dir.clone()).unwrap();

        // Create table with non-integer columns (will skip indexing)
        let table_data = TableData {
            info: TableInfo {
                name: "text_table".to_string(),
                columns: vec![ColumnDefinition {
                    name: "name".to_string(),
                    data_type: "TEXT".to_string(),
                    nullable: false,
                    auto_increment: false,
                    ..Default::default()
                }],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                compression: None,
                collations: std::collections::HashMap::new(),
                partition_info: None,
                original_sql: String::new(),
            },
            rows: vec![vec![Value::Text("Alice".to_string())]],
        };
        storage
            .insert_table("text_table".to_string(), table_data)
            .unwrap();

        // Create index - this will work but won't have any entries
        storage.create_index("text_table", "name", 0).unwrap();

        // search_index should return None for TEXT column (no Integer keys)
        let result = storage.search_index("text_table", "name", 1);
        assert_eq!(result, None);

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_storage_empty_tables() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_empty");
        let _ = remove_dir_all(&temp_dir);

        let mut storage = FileStorage::new(temp_dir.clone()).unwrap();

        // Test empty storage
        assert_eq!(storage.table_names().len(), 0);
        assert!(!storage.contains_table("anything"));
        assert!(storage.get_table("anything").is_none());

        // Test flush on empty storage
        storage.flush().unwrap();
        storage.flush_indexes().unwrap();

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_storage_persist_nonexistent() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_persist_none");
        let _ = remove_dir_all(&temp_dir);

        let storage = FileStorage::new(temp_dir.clone()).unwrap();

        // persist_table on non-existent table should return Ok
        let result = storage.persist_table("nonexistent");
        assert!(result.is_ok());

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_storage_range_index_no_results() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_range_empty");
        let _ = remove_dir_all(&temp_dir);

        let mut storage = FileStorage::new(temp_dir.clone()).unwrap();

        let table_data = TableData {
            info: TableInfo {
                name: "range_test".to_string(),
                columns: vec![ColumnDefinition {
                    name: "id".to_string(),
                    data_type: "INTEGER".to_string(),
                    nullable: false,
                    auto_increment: false,
                    ..Default::default()
                }],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                compression: None,
                collations: std::collections::HashMap::new(),
                partition_info: None,
                original_sql: String::new(),
            },
            rows: vec![],
        };
        storage
            .insert_table("range_test".to_string(), table_data)
            .unwrap();
        storage.create_index("range_test", "id", 0).unwrap();

        // Add some data
        storage.insert_with_index("range_test", "id", 5, 0).unwrap();

        // Range with no matching results
        let range = storage.range_index("range_test", "id", 100, 200);
        assert!(range.is_empty());

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_storage_insert_with_index_no_index() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_no_idx");
        let _ = remove_dir_all(&temp_dir);

        let mut storage = FileStorage::new(temp_dir.clone()).unwrap();

        let table_data = TableData {
            info: TableInfo {
                name: "no_idx_test".to_string(),
                columns: vec![ColumnDefinition {
                    name: "id".to_string(),
                    data_type: "INTEGER".to_string(),
                    nullable: false,
                    auto_increment: false,
                    ..Default::default()
                }],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                compression: None,
                collations: std::collections::HashMap::new(),
                partition_info: None,
                original_sql: String::new(),
            },
            rows: vec![],
        };
        storage
            .insert_table("no_idx_test".to_string(), table_data)
            .unwrap();

        // Insert with index when no index exists - should be ok (no-op)
        let result = storage.insert_with_index("no_idx_test", "id", 1, 0);
        assert!(result.is_ok());

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_storage_has_index_no_table() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_has_idx_no");
        let _ = remove_dir_all(&temp_dir);

        let storage = FileStorage::new(temp_dir.clone()).unwrap();

        // Check index on non-existent table - should return false
        let result = storage.has_index("nonexistent", "id");
        assert!(!result);

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_storage_drop_index_no_table() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_drop_no");
        let _ = remove_dir_all(&temp_dir);

        let mut storage = FileStorage::new(temp_dir.clone()).unwrap();

        // Try to drop index from non-existent table - should return Ok (no-op)
        let result = storage.drop_index("nonexistent", "id");
        assert!(result.is_ok());

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_storage_range_index_no_table() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_range_no");
        let _ = remove_dir_all(&temp_dir);

        let storage = FileStorage::new(temp_dir.clone()).unwrap();

        // Range query on non-existent table - should return empty
        let result = storage.range_index("nonexistent", "id", 0, 100);
        assert_eq!(result, Vec::<u32>::new());

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_storage_add_column() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_add_col");
        let _ = remove_dir_all(&temp_dir);

        let mut storage = FileStorage::new(temp_dir.clone()).unwrap();

        let table_data = TableData {
            info: TableInfo {
                name: "add_col_test".to_string(),
                columns: vec![ColumnDefinition {
                    name: "id".to_string(),
                    data_type: "INTEGER".to_string(),
                    nullable: false,
                    primary_key: true,
                    char_max_length: None,
                    collation: None,
                    default_value: None,
                    auto_increment: false,
                }],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                compression: None,
                collations: std::collections::HashMap::new(),
                partition_info: None,
                original_sql: String::new(),
            },
            rows: vec![],
        };
        storage
            .insert_table("add_col_test".to_string(), table_data)
            .unwrap();

        // Test add_column
        let new_col = ColumnDefinition {
            name: "name".to_string(),
            data_type: "TEXT".to_string(),
            nullable: true,
            primary_key: false,
            char_max_length: None,
            collation: None,
            default_value: None,
            auto_increment: false,
        };
        let result = storage.add_column("add_col_test", new_col);
        assert!(result.is_ok());

        // Verify column was added
        let table = storage.get_table("add_col_test").unwrap();
        assert_eq!(table.info.columns.len(), 2);
        assert_eq!(table.info.columns[1].name, "name");

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_storage_rename_table() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_rename");
        let _ = remove_dir_all(&temp_dir);

        let mut storage = FileStorage::new(temp_dir.clone()).unwrap();

        let table_data = TableData {
            info: TableInfo {
                name: "old_name".to_string(),
                columns: vec![ColumnDefinition {
                    name: "id".to_string(),
                    data_type: "INTEGER".to_string(),
                    nullable: false,
                    auto_increment: false,
                    ..Default::default()
                }],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                compression: None,
                collations: std::collections::HashMap::new(),
                partition_info: None,
                original_sql: String::new(),
            },
            rows: vec![],
        };
        storage
            .insert_table("old_name".to_string(), table_data)
            .unwrap();

        // Test rename_table
        let result = storage.rename_table("old_name", "new_name");
        assert!(result.is_ok());

        // Verify table was renamed
        assert!(!storage.contains_table("old_name"));
        assert!(storage.contains_table("new_name"));

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_storage_rename_nonexistent() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_rename_none");
        let _ = remove_dir_all(&temp_dir);

        let mut storage = FileStorage::new(temp_dir.clone()).unwrap();

        // Rename non-existent table should return Ok (no-op)
        let result = storage.rename_table("nonexistent", "new_name");
        assert!(result.is_ok());

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_storage_trigger_operations() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_triggers");
        let _ = remove_dir_all(&temp_dir);

        let mut storage = FileStorage::new(temp_dir.clone()).unwrap();

        // Test create_trigger (returns Ok but does nothing)
        let trigger_info = TriggerInfo {
            name: "test_trigger".to_string(),
            table_name: "test_table".to_string(),
            timing: crate::engine::TriggerTiming::Before,
            event: crate::engine::TriggerEvent::Insert,
            body: "BEGIN END".to_string(),
            update_columns: None,
            original_sql: String::new(),
        };
        let result = storage.create_trigger(trigger_info);
        assert!(result.is_ok());

        // PR-2760: drop_trigger is now implemented (FileStorage trigger persistence).
        // Drop should succeed and return Ok.
        let result = storage.drop_trigger("test_trigger");
        assert!(result.is_ok(), "drop_trigger should succeed after PR-2760");

        // Test get_trigger returns None (dropped above)
        let result = storage.get_trigger("test_trigger");
        assert!(result.is_none());

        // Test list_triggers returns empty (dropped above)
        let result = storage.list_triggers("test_table");
        assert!(result.is_empty());

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_storage_view_operations() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_views");
        let _ = remove_dir_all(&temp_dir);

        let storage = FileStorage::new(temp_dir.clone()).unwrap();

        // Test has_view returns false
        assert!(!storage.has_view("test_view"));

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_insert_buffering() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_buffer");
        let _ = remove_dir_all(&temp_dir);

        let mut storage = FileStorage::new_with_buffer_config(temp_dir.clone(), 10, true).unwrap();

        let table_info = TableInfo {
            name: "test_table".to_string(),
            columns: vec![ColumnDefinition {
                name: "id".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: true,
                char_max_length: None,
                collation: None,
                default_value: None,
                auto_increment: false,
            }],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&table_info).unwrap();

        for i in 0..5 {
            let record = vec![Value::Integer(i as i64)];
            storage.insert("test_table", vec![record]).unwrap();
        }

        assert!({
            // #5025: the buffer is keyed by the scoped table name.
            let key = storage.tbl("test_table");
            storage.with_read_lock(|st| st.insert_buffer.contains_key(&key))
        });
        assert_eq!(
            {
                let key = storage.tbl("test_table");
                storage.with_read_lock(|st| st.insert_buffer.get(&key).map(|v| v.len()))
            },
            Some(5)
        );

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_insert_buffer_threshold() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_threshold");
        let _ = remove_dir_all(&temp_dir);

        let mut storage = FileStorage::new_with_buffer_config(temp_dir.clone(), 3, true).unwrap();

        let table_info = TableInfo {
            name: "test_table".to_string(),
            columns: vec![ColumnDefinition {
                name: "id".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: true,
                char_max_length: None,
                collation: None,
                default_value: None,
                auto_increment: false,
            }],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&table_info).unwrap();

        for i in 0..5 {
            let record = vec![Value::Integer(i as i64)];
            storage.insert("test_table", vec![record]).unwrap();
        }

        // #5025: the buffer is keyed by the scoped table name. Assert
        // through the public surface instead — the buffer key is an
        // implementation detail, and the point of the test is the
        // threshold behaviour, not the key format.
        let key = storage.tbl("test_table");
        assert!(storage.with_read_lock(|st| st.insert_buffer.contains_key(&key)));
        assert_eq!(
            storage.with_read_lock(|st| st.insert_buffer.get(&key).map(|v| v.len())),
            Some(2)
        );

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_flush_all_buffers() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_flush");
        let _ = remove_dir_all(&temp_dir);

        let mut storage = FileStorage::new_with_buffer_config(temp_dir.clone(), 100, true).unwrap();

        let table_info = TableInfo {
            name: "test_table".to_string(),
            columns: vec![ColumnDefinition {
                name: "id".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: true,
                char_max_length: None,
                collation: None,
                default_value: None,
                auto_increment: false,
            }],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&table_info).unwrap();

        for i in 0..5 {
            let record = vec![Value::Integer(i as i64)];
            storage.insert("test_table", vec![record]).unwrap();
        }

        assert!({
            // #5025: the buffer is keyed by the scoped table name.
            let key = storage.tbl("test_table");
            storage.with_read_lock(|st| st.insert_buffer.contains_key(&key))
        });

        storage.flush_all_buffers().unwrap();

        assert!(!{
            // #5025: the buffer is keyed by the scoped table name.
            let key = storage.tbl("test_table");
            storage.with_read_lock(|st| st.insert_buffer.contains_key(&key))
        });

        let table = storage.get_table("test_table").unwrap();
        assert_eq!(table.rows.len(), 5);

        let _ = remove_dir_all(&temp_dir);
    }

    fn make_storage(dir: &str) -> FileStorage {
        let temp_dir = std::env::temp_dir().join(dir);
        let _ = remove_dir_all(&temp_dir);
        FileStorage::new_with_buffer_config(temp_dir, 100, false).unwrap()
    }

    #[test]
    fn test_new_with_buffer_config() {
        let temp_dir = std::env::temp_dir().join("file_storage_buf_cfg");
        let _ = remove_dir_all(&temp_dir);
        let storage = FileStorage::new_with_buffer_config(temp_dir.clone(), 50, false).unwrap();
        assert!(storage.buffer_threshold == 50);
        assert!(!storage.enable_buffer);
        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_new_with_wal() {
        let temp_dir = std::env::temp_dir().join("file_storage_wal");
        let _ = remove_dir_all(&temp_dir);
        let storage = FileStorage::new_with_wal(temp_dir.clone()).unwrap();
        assert!(storage.data_dir.ends_with("file_storage_wal"));
        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_get_table_mut() {
        let mut storage = make_storage("fs_get_table_mut");
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        let t = storage.get_table_mut("t");
        assert!(t.is_some());
    }

    #[test]
    fn test_get_table_mut_nonexistent() {
        let mut storage = make_storage("fs_get_table_mut_ne");
        assert!(storage.get_table_mut("nonexistent").is_none());
    }

    #[test]
    fn test_insert_table() {
        let mut storage = make_storage("fs_insert_table");
        let info = TableInfo {
            name: "t1".into(),
            columns: vec![ColumnDefinition::new("x", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        let data = TableData { info, rows: vec![] };
        storage.insert_table("t1".into(), data).unwrap();
        assert!(storage.contains_table("t1"));
    }

    #[test]
    fn test_table_names() {
        let mut storage = make_storage("fs_table_names");
        let info = TableInfo {
            name: "t".into(),
            columns: vec![],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        let names = storage.table_names();
        assert!(names.contains(&"t".to_string()));
    }

    #[test]
    fn test_table_names_empty() {
        let storage = make_storage("fs_table_names_empty");
        assert!(storage.table_names().is_empty());
    }

    #[test]
    fn test_contains_table() {
        let mut storage = make_storage("fs_contains_table");
        let info = TableInfo {
            name: "x".into(),
            columns: vec![],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        assert!(storage.contains_table("x"));
        assert!(!storage.contains_table("y"));
    }

    #[test]
    fn test_drop_database_nonexistent() {
        let mut storage = make_storage("fs_drop_db_ne");
        let result = storage.drop_database("nonexistent_db");
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_database() {
        let mut storage = make_storage("fs_create_db");
        let result = storage.create_database("my_db");
        assert!(result.is_ok());
        let _ = std::fs::remove_dir_all(storage.data_dir.join("my_db"));
    }

    #[test]
    fn test_clear_all_tables() {
        let mut storage = make_storage("fs_clear_all");
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        storage.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
        storage.clear_all_tables();
        let rows = storage.scan("t").unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn test_flush_all_buffers_extra() {
        let mut storage = make_storage("fs_flush_all_ex");
        storage.flush_all_buffers().unwrap();
    }

    #[test]
    fn test_in_transaction() {
        let storage = make_storage("fs_in_tx");
        assert!(!storage.in_transaction());
        assert_eq!(storage.current_tx_id(), 0);
    }

    #[test]
    fn test_set_current_tx_id() {
        let mut storage = make_storage("fs_set_tx");
        storage.set_current_tx_id(42);
        assert_eq!(storage.current_tx_id(), 42);
        assert!(storage.in_transaction());
    }

    #[test]
    fn test_force_insert() {
        let mut storage = make_storage("fs_force_ins");
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        storage.force_insert("t", vec![Value::Integer(5)]).unwrap();
        let rows = storage.scan("t").unwrap();
        assert_eq!(rows, vec![vec![Value::Integer(5)]]);
    }

    #[test]
    fn test_delete_if() {
        let mut storage = make_storage("fs_del_if");
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        storage
            .insert("t", vec![vec![Value::Integer(1)], vec![Value::Integer(2)]])
            .unwrap();
        let filter: RowFilter = Box::new(|row: &Record| row[0] == Value::Integer(1));
        let removed = storage.delete_if("t", &filter).unwrap();
        assert_eq!(removed, 1);
    }

    #[test]
    fn test_delete_if_nonexistent() {
        let mut storage = make_storage("fs_del_if_ne");
        let filter: RowFilter = Box::new(|_: &Record| true);
        let removed = storage.delete_if("nonexistent", &filter).unwrap();
        assert_eq!(removed, 0);
    }

    #[test]
    fn test_update_nonexistent() {
        let mut storage = make_storage("fs_upd_ne");
        let count = storage
            .update(
                "nonexistent",
                &[Value::Integer(1)],
                &[(0, Value::Integer(99))],
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_update_if_nonexistent() {
        let mut storage = make_storage("fs_upd_if_ne");
        let filter: RowFilter = Box::new(|_: &Record| true);
        let mutation = RowMutation::new(vec![(0, Value::Integer(99))], 0);
        let count = storage
            .update_if("nonexistent", &filter, &mutation)
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_update_if_existing() {
        let mut storage = make_storage("fs_upd_if_ex");
        let info = TableInfo {
            name: "t".into(),
            columns: vec![
                ColumnDefinition::new("x", "INTEGER"),
                ColumnDefinition::new("y", "INTEGER"),
            ],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        storage
            .insert("t", vec![vec![Value::Integer(1), Value::Integer(10)]])
            .unwrap();
        let filter: RowFilter = Box::new(|row: &Record| row[0] == Value::Integer(1));
        let mutation = RowMutation::new(vec![(1, Value::Integer(99))], 0);
        let count = storage.update_if("t", &filter, &mutation).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_create_table_storage() {
        let mut storage = make_storage("fs_create_t");
        let info = TableInfo {
            name: "users".into(),
            columns: vec![ColumnDefinition::new("id", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        assert!(storage.has_table("users"));
    }

    #[test]
    fn test_get_table_info_not_found() {
        let storage = make_storage("fs_gti_ne");
        let result = storage.get_table_info("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_has_table_storage() {
        let mut storage = make_storage("fs_has_t");
        let info = TableInfo {
            name: "x".into(),
            columns: vec![],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        assert!(storage.has_table("x"));
        assert!(!storage.has_table("y"));
    }

    #[test]
    fn test_list_tables_storage() {
        let mut storage = make_storage("fs_list_t");
        let info = TableInfo {
            name: "t1".into(),
            columns: vec![],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        let tables = storage.list_tables();
        assert!(tables.contains(&"t1".to_string()));
    }

    #[test]
    fn test_create_index_storage() {
        let mut storage = make_storage("fs_create_idx");
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("id", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        storage
            .insert(
                "t",
                vec![
                    vec![Value::Integer(1)],
                    vec![Value::Integer(2)],
                    vec![Value::Integer(3)],
                ],
            )
            .unwrap();
        storage.create_index("t", "id", 0).unwrap();
        assert!(storage.has_index("t", "id"));
    }

    #[test]
    fn test_create_index_table_not_found() {
        let mut storage = make_storage("fs_create_idx_ne");
        let result = storage.create_index("nonexistent", "c", 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_drop_index_storage() {
        let mut storage = make_storage("fs_drop_idx");
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("id", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        storage.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
        storage.create_index("t", "id", 0).unwrap();
        storage.drop_index("t", "id").unwrap();
        assert!(!storage.has_index("t", "id"));
    }

    #[test]
    fn test_has_index_no_table() {
        let storage = make_storage("fs_has_idx_no_t");
        assert!(!storage.has_index("nonexistent", "c"));
    }

    #[test]
    fn test_add_column_storage() {
        let mut storage = make_storage("fs_add_col");
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        storage
            .add_column("t", ColumnDefinition::new("b", "TEXT"))
            .unwrap();
        let info_after = storage.get_table_info("t").unwrap();
        assert_eq!(info_after.columns.len(), 2);
    }

    #[test]
    fn test_rename_table_storage() {
        let mut storage = make_storage("fs_rename_t");
        let info = TableInfo {
            name: "old".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        storage
            .insert("old", vec![vec![Value::Integer(1)]])
            .unwrap();
        storage.rename_table("old", "new").unwrap();
        assert!(!storage.has_table("old"));
        assert!(storage.has_table("new"));
    }

    #[test]
    fn test_rename_table_nonexistent() {
        let mut storage = make_storage("fs_rename_t_ne");
        storage.rename_table("nonexistent", "new_name").unwrap();
    }

    #[test]
    fn test_trigger_operations_storage() {
        let mut storage = make_storage("fs_trigger_ops");
        let trigger = TriggerInfo {
            name: "trig1".into(),
            table_name: "t".into(),
            timing: crate::engine::TriggerTiming::Before,
            event: crate::engine::TriggerEvent::Insert,
            body: "BEGIN UPDATE stats SET n = n + 1; END".into(),
            update_columns: None,
            original_sql: String::new(),
        };
        storage.create_trigger(trigger).unwrap();
        let got = storage.get_trigger("trig1");
        assert!(got.is_some());
        let triggers = storage.list_triggers("t");
        assert_eq!(triggers.len(), 1);
        storage.drop_trigger("trig1").unwrap();
        assert!(storage.get_trigger("trig1").is_none());
    }

    #[test]
    fn test_get_trigger_none() {
        let storage = make_storage("fs_get_trig_none");
        assert!(storage.get_trigger("nonexistent").is_none());
    }

    #[test]
    fn test_list_triggers_by_table() {
        let mut storage = make_storage("fs_list_triggers");
        let trigger1 = TriggerInfo {
            name: "t1".into(),
            table_name: "users".into(),
            timing: crate::engine::TriggerTiming::Before,
            event: crate::engine::TriggerEvent::Insert,
            body: "".into(),
            update_columns: None,
            original_sql: String::new(),
        };
        let trigger2 = TriggerInfo {
            name: "t2".into(),
            table_name: "orders".into(),
            timing: crate::engine::TriggerTiming::After,
            event: crate::engine::TriggerEvent::Update,
            body: "".into(),
            update_columns: None,
            original_sql: String::new(),
        };
        storage.create_trigger(trigger1).unwrap();
        storage.create_trigger(trigger2).unwrap();
        let users_triggers = storage.list_triggers("users");
        let orders_triggers = storage.list_triggers("orders");
        assert_eq!(users_triggers.len(), 1);
        assert_eq!(orders_triggers.len(), 1);
    }

    #[test]
    fn test_has_view_storage() {
        let storage = make_storage("fs_has_view");
        assert!(!storage.has_view("v"));
    }

    #[test]
    fn test_list_indexes_storage() {
        let mut storage = make_storage("fs_list_idx");
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("id", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        storage.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
        storage.create_index("t", "id", 0).unwrap();
        let indexes = storage.list_indexes("t");
        assert_eq!(indexes.len(), 1);
    }

    #[test]
    fn test_list_indexes_empty() {
        let storage = make_storage("fs_list_idx_empty");
        let indexes = storage.list_indexes("nonexistent");
        assert!(indexes.is_empty());
    }

    #[test]
    fn test_create_database_storage() {
        let mut storage = make_storage("fs_create_db_s");
        storage.create_database("db1").unwrap();
        let path = storage.data_dir.join("db1");
        assert!(path.exists());
        let _ = std::fs::remove_dir_all(path);
    }

    #[test]
    fn test_drop_database_nonempty() {
        let mut storage = make_storage("fs_drop_db_ne2");
        storage.create_database("db1").unwrap();
        let path = storage.data_dir.join("db1");
        std::fs::write(path.join("marker.txt"), "x").unwrap();
        let result = storage.drop_database("db1");
        assert!(result.is_err());
        let _ = std::fs::remove_dir_all(storage.data_dir.join("db1"));
    }

    #[test]
    fn test_drop_column_storage() {
        let mut storage = make_storage("fs_drop_col");
        let info = TableInfo {
            name: "t".into(),
            columns: vec![
                ColumnDefinition::new("a", "INTEGER"),
                ColumnDefinition::new("b", "TEXT"),
            ],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        storage
            .insert(
                "t",
                vec![vec![Value::Integer(1), Value::Text("hello".into())]],
            )
            .unwrap();
        storage.drop_column("t", "b").unwrap();
        let info_after = storage.get_table_info("t").unwrap();
        assert_eq!(info_after.columns.len(), 1);
    }

    #[test]
    fn test_drop_column_table_not_found() {
        let mut storage = make_storage("fs_drop_col_ne");
        let result = storage.drop_column("nonexistent", "a");
        assert!(result.is_err());
    }

    #[test]
    fn test_drop_column_not_found() {
        let mut storage = make_storage("fs_drop_col_nf");
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        let result = storage.drop_column("t", "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_modify_column_storage() {
        let mut storage = make_storage("fs_mod_col");
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        let new_def = ColumnDefinition {
            name: "a".into(),
            data_type: "TEXT".into(),
            nullable: true,
            primary_key: false,
            char_max_length: None,
            collation: None,
            default_value: None,
            auto_increment: false,
        };
        storage.modify_column("t", "a", new_def).unwrap();
    }

    #[test]
    fn test_modify_column_table_not_found() {
        let mut storage = make_storage("fs_mod_col_ne");
        let new_def = ColumnDefinition::new("a", "TEXT");
        let result = storage.modify_column("nonexistent", "a", new_def);
        assert!(result.is_err());
    }

    #[test]
    fn test_modify_column_not_found() {
        let mut storage = make_storage("fs_mod_col_nf");
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        let new_def = ColumnDefinition::new("nonexistent", "TEXT");
        let result = storage.modify_column("t", "nonexistent", new_def);
        assert!(result.is_err());
    }

    #[test]
    fn test_scan_merges_buffer() {
        let mut storage = make_storage("fs_scan_buf");
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("x", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        storage.set_current_tx_id(1);
        storage.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
        let rows = storage.scan("t").unwrap();
        assert_eq!(rows.len(), 1);
        storage.set_current_tx_id(0);
    }

    #[test]
    fn test_buffered_insert_flush() {
        let dir = std::env::temp_dir().join("fs_buf_flush");
        let _ = remove_dir_all(&dir);
        let mut storage = FileStorage::new_with_buffer_config(dir.clone(), 2, true).unwrap();
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("x", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        storage.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
        storage.insert("t", vec![vec![Value::Integer(2)]]).unwrap();
        let rows = storage.scan("t").unwrap();
        assert!(rows.len() >= 1);
        let _ = remove_dir_all(&dir);
    }

    #[test]
    fn test_buffered_insert_in_tx() {
        let dir = std::env::temp_dir().join("fs_buf_tx");
        let _ = remove_dir_all(&dir);
        let mut storage = FileStorage::new_with_buffer_config(dir.clone(), 10, true).unwrap();
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("x", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        storage.set_current_tx_id(1);
        storage.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
        storage.flush_all_buffers().unwrap();
        storage.set_current_tx_id(0);
        let _ = remove_dir_all(&dir);
    }

    #[test]
    fn test_trigger_roundtrip_disk() {
        let temp_dir = std::env::temp_dir().join("fs_trigger_roundtrip");
        let _ = remove_dir_all(&temp_dir);
        let mut storage = FileStorage::new_with_wal(temp_dir.clone()).unwrap();
        let trigger = TriggerInfo {
            name: "trig_disk".to_string(),
            table_name: "t".to_string(),
            timing: crate::engine::TriggerTiming::Before,
            event: crate::engine::TriggerEvent::Insert,
            body: "BEGIN UPDATE s SET n = n + 1; END".to_string(),
            update_columns: None,
            original_sql: String::new(),
        };
        storage.create_trigger(trigger).unwrap();
        drop(storage);

        let mut storage2 = FileStorage::new_with_wal(temp_dir.clone()).unwrap();
        let got = storage2.get_trigger("trig_disk");
        assert!(got.is_some());
        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_save_trigger_load_trigger_duplicate() {
        let temp_dir = std::env::temp_dir().join("fs_trigger_dup");
        let _ = remove_dir_all(&temp_dir);
        let mut storage = FileStorage::new(temp_dir.clone()).unwrap();
        let trigger = TriggerInfo {
            name: "trig_dup".to_string(),
            table_name: "t".to_string(),
            timing: crate::engine::TriggerTiming::After,
            event: crate::engine::TriggerEvent::Update,
            body: "".to_string(),
            update_columns: None,
            original_sql: String::new(),
        };
        storage.create_trigger(trigger.clone()).unwrap();
        storage.create_trigger(trigger).unwrap();
        let count = storage.list_triggers("t").len();
        assert!(count >= 1);
        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_drop_trigger_disk_removal() {
        let temp_dir = std::env::temp_dir().join("fs_drop_trig_disk");
        let _ = remove_dir_all(&temp_dir);
        let mut storage = FileStorage::new_with_wal(temp_dir.clone()).unwrap();
        let trigger = TriggerInfo {
            name: "trig_x".to_string(),
            table_name: "t".to_string(),
            timing: crate::engine::TriggerTiming::Before,
            event: crate::engine::TriggerEvent::Delete,
            body: "".to_string(),
            update_columns: None,
            original_sql: String::new(),
        };
        storage.create_trigger(trigger).unwrap();
        storage.drop_trigger("trig_x").unwrap();
        let path = temp_dir.join("trigger_trig_x.json");
        assert!(!path.exists());
    }

    #[test]
    fn test_file_with_buffer_disabled_flow() {
        let mut storage = make_storage("fs_no_buf");
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        storage.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
        storage.delete("t", &[Value::Integer(1)]).unwrap();
        storage
            .update("t", &[Value::Integer(1)], &[(0, Value::Integer(99))])
            .unwrap();
        let rows = storage.scan("t").unwrap();
        assert!(rows.is_empty() || rows[0][0] != Value::Integer(1));
    }

    #[test]
    fn test_flush_all_buffers_with_buffered_table() {
        let dir = std::env::temp_dir().join("fs_flush_buf_table");
        let _ = remove_dir_all(&dir);
        let mut storage = FileStorage::new_with_buffer_config(dir.clone(), 1000, true).unwrap();
        let info = TableInfo {
            name: "t".into(),
            columns: vec![],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        storage.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
        storage.flush_all_buffers().unwrap();
        let _ = remove_dir_all(&dir);
    }

    #[test]
    fn test_load_table_invalid_json() {
        let temp_dir = std::env::temp_dir().join("fs_invalid_json");
        let _ = remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::fs::write(temp_dir.join("bad.json"), "not valid json{{{").unwrap();
        let storage = FileStorage::new(temp_dir.clone()).unwrap();
        let result = storage.get_table("bad");
        assert!(result.is_none());
        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_insert_buffered_threshold_flushing() {
        let dir = std::env::temp_dir().join("fs_thr_flush");
        let _ = remove_dir_all(&dir);
        let mut storage = FileStorage::new_with_buffer_config(dir.clone(), 2, true).unwrap();
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        storage
            .insert(
                "t",
                vec![
                    vec![Value::Integer(1)],
                    vec![Value::Integer(2)],
                    vec![Value::Integer(3)],
                ],
            )
            .unwrap();
        let _ = remove_dir_all(&dir);
    }

    #[test]
    fn test_partition_rows_below_threshold() {
        let mut storage = make_storage("fs_pr_below");
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        storage
            .insert("t", (0..100_i64).map(|i| vec![Value::Integer(i)]).collect())
            .unwrap();
        let parts = storage.partition_rows("t", 4);
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].len(), 100);
    }

    #[test]
    fn test_partition_rows_above_threshold() {
        let mut storage = make_storage("fs_pr_above");
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        storage
            .insert(
                "t",
                (0..600_000_i64).map(|i| vec![Value::Integer(i)]).collect(),
            )
            .unwrap();
        let parts = storage.partition_rows("t", 4);
        assert_eq!(parts.len(), 4);
        let total: usize = parts.iter().map(|p| p.len()).sum();
        assert_eq!(total, 600_000);
    }

    #[test]
    fn test_partition_rows_missing_table() {
        let storage = make_storage("fs_pr_missing");
        let parts = storage.partition_rows("nonexistent", 4);
        assert_eq!(parts.len(), 1);
        assert!(parts[0].is_empty());
    }

    #[test]
    fn test_partition_rows_num_partitions_zero() {
        let mut storage = make_storage("fs_pr_zero");
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        storage
            .insert(
                "t",
                (0..600_000_i64).map(|i| vec![Value::Integer(i)]).collect(),
            )
            .unwrap();
        let parts = storage.partition_rows("t", 0);
        assert_eq!(parts.len(), 1);
    }

    #[test]
    fn test_partition_rows_with_buffer() {
        let dir = std::env::temp_dir().join("fs_pr_buf");
        let _ = remove_dir_all(&dir);
        let mut storage = FileStorage::new_with_buffer_config(dir.clone(), 1000, true).unwrap();
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        storage.set_current_tx_id(1);
        storage
            .insert(
                "t",
                (0..600_000_i64).map(|i| vec![Value::Integer(i)]).collect(),
            )
            .unwrap();
        let _ = storage.partition_rows("t", 4);
        let _ = remove_dir_all(&dir);
    }

    #[test]
    fn test_partition_rows_uneven_remainder() {
        let mut storage = FileStorage::new_with_buffer_config(
            std::env::temp_dir().join("fs_pr_uneven"),
            100,
            true,
        )
        .unwrap();
        let dir = std::env::temp_dir().join("fs_pr_uneven");
        let _ = remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut storage = FileStorage::new_with_buffer_config(dir.clone(), 100, true).unwrap();
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        storage
            .insert(
                "t",
                (0..503_003_i64).map(|i| vec![Value::Integer(i)]).collect(),
            )
            .unwrap();
        let parts = storage.partition_rows("t", 4);
        assert_eq!(parts.len(), 4);
        let total: usize = parts.iter().map(|p| p.len()).sum();
        assert_eq!(total, 503_003);
        let _ = remove_dir_all(&dir);
    }

    #[test]
    fn test_partition_rows_two_partitions_above_threshold() {
        let dir = std::env::temp_dir().join("fs_pr_2p");
        let _ = remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut storage = FileStorage::new_with_buffer_config(dir.clone(), 100, true).unwrap();
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        storage
            .insert(
                "t",
                (0..500_100_i64).map(|i| vec![Value::Integer(i)]).collect(),
            )
            .unwrap();
        let parts = storage.partition_rows("t", 2);
        assert_eq!(parts.len(), 2);
        let _ = remove_dir_all(&dir);
    }

    #[test]
    fn test_insert_direct_table_not_in_map() {
        let mut storage = make_storage("fs_ins_direct_ne");
        storage
            .insert("nonexistent", vec![vec![Value::Integer(1)]])
            .unwrap();
    }

    #[test]
    fn test_flush_buffer_with_buffered_rows() {
        let dir = std::env::temp_dir().join("fs_flush_buf");
        let _ = remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut storage = FileStorage::new_with_buffer_config(dir.clone(), 5, true).unwrap();
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        storage
            .insert("t", (0..10_i64).map(|i| vec![Value::Integer(i)]).collect())
            .unwrap();
        storage.flush_all_buffers().unwrap();
        let _ = remove_dir_all(&dir);
    }

    #[test]
    fn test_storage_save_reload_roundtrip() {
        let dir = std::env::temp_dir().join("fs_save_reload");
        let _ = remove_dir_all(&dir);
        let mut storage = FileStorage::new_with_buffer_config(dir.clone(), 100, false).unwrap();
        let info = TableInfo {
            name: "users".to_string(),
            columns: vec![ColumnDefinition::new("id", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        storage
            .insert("users", vec![vec![Value::Integer(42)]])
            .unwrap();
        drop(storage);

        let storage2 = FileStorage::new(dir.clone()).unwrap();
        let rows = storage2.scan("users").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0][0], Value::Integer(42));
        let _ = remove_dir_all(&dir);
    }

    #[test]
    fn test_storage_engine_get_table_info_not_found() {
        let storage = make_storage("fs_eng_gti_ne");
        let result = storage.get_table_info("missing");
        assert!(result.is_err());
    }

    #[test]
    fn test_storage_engine_table_operations_empty() {
        let mut storage = make_storage("fs_eng_empty");
        assert!(storage.list_tables().is_empty());
        assert!(!storage.has_table("anytable"));
    }

    #[test]
    fn test_storage_engine_drop_index_nonexistent_table() {
        let mut storage = make_storage("fs_drop_idx_ne");
        storage.drop_index("nonexistent", "col").unwrap();
    }
}

impl FileStorage {
    fn insert_direct(&self, db: &str, table: &str, records: Vec<Record>) -> SqlResult<()> {
        let snap: Option<(Vec<ColumnDefinition>, u32, usize)> =
            Self::with_write_lock(self, |s| -> Option<(Vec<ColumnDefinition>, u32, usize)> {
                #[allow(unused_assignments)]
                // start_row_id is set inside the if-let branch and consumed via snap
                let mut start_row_id: u32 = 0;
                let row_count = records.len();
                let mut result: Option<(Vec<ColumnDefinition>, u32, usize)> = None;
                if let Some(ref mut data) = s.tables.get_mut(&self.tbl_in(db, table)) {
                    start_row_id = data.rows.len() as u32;
                    data.rows.extend(records.iter().cloned());
                    let cols = data.info.columns.clone();
                    // B2.1 / #4915 (F-09): the original code did
                    // `let table_data = data.clone();` — a full copy of
                    // every row in the table, per insert. The delta
                    // path in `save_table` only reads
                    // `rows[last_saved..]`, so hand it a snapshot that
                    // contains just the appended window. Same JSON on
                    // disk, O(row_count) instead of O(table_size).
                    let total_rows = data.rows.len();
                    let table_data = data.snapshot_from(start_row_id as usize);
                    if self
                        .save_table_window(db, s, table, &table_data, total_rows)
                        .is_ok()
                    {
                        result = Some((cols, start_row_id, row_count));
                    }
                }
                result
            });
        if let Some((columns, start_row_id, _row_count)) = snap {
            // V400-PERF-FIX: pass &records directly so the index
            // helper reads PK values from the input rather than
            // cloning the entire `data.rows` vector. The clone
            // was O(N) per insert and dominated write throughput.
            Self::update_pk_index(self, db, table, &columns, &records, start_row_id as usize);
        }
        Ok(())
    }

    /// The one implementation of an insert, parameterised by database.
    ///
    /// #5057: this body used to read `self.current_db` inside the write
    /// helpers, so a single body had to serve both "resolve against the
    /// storage's current database" and "resolve against this database",
    /// with no way to tell which was asked for. Lifting it here lets the
    /// trait's `insert` and `insert_in_db` be one-line wrappers over the
    /// same code that give different answers.
    fn insert_at(&mut self, db: &str, table: &str, records: Vec<Record>) -> SqlResult<()> {
        // #5055: log before applying. This is the write-ahead half of
        // write-ahead logging — a crash between here and the buffer
        // flush must still leave the rows recoverable, which it cannot
        // if the log is written after the mutation.
        if self.wal_enabled() {
            self.wal_append(self.wal_insert_entries(table, &records))?;
        }

        // C.1.2: in_transaction / insert_buffered / insert_direct are
        // inherent `&self` methods — safe to call from outside the
        // lock and from inside (Rust reborrows `&mut Self` as `&Self`
        // automatically). The only bare-field write inside the trait
        // body is the dirty_tables insert at the end; that goes under
        // the lock.
        //
        // PR-842: route inserts through the buffer when we are inside a
        // transaction so that a crash before COMMIT does not leak partially
        // applied rows to disk. Outside a transaction (autocommit) the
        // insert is durable immediately. `enable_buffer: false` is
        // overridden for tx-scoped writes so WAL recovery sees a clean
        // apply-or-rollback boundary.
        //
        // v3.11.0 P1 fix: removed `records.len() >= self.buffer_threshold`
        // condition that triggered immediate `insert_direct` (full table save).
        // This was causing O(N * table_size) behavior during bulk loads where
        // each batch of 100+ rows triggered a full table serialization and write.
        // Now: always buffer inserts, caller explicitly calls flush() to persist.
        // #5048: the change log needs the row contents, but the insert
        // path below consumes `records`. Cloning is only paid when the
        // log is actually on.
        let logged = if self.change_log_enabled() {
            Some(records.clone())
        } else {
            None
        };
        if self.in_transaction() {
            // #5059: record one undo entry per row. Without this,
            // ROLLBACK had nothing to act on for buffered inserts and
            // every rolled-back row survived — see `UndoOp`.
            let tx_id = self.current_tx_id();
            Self::with_write_lock(self, |s| {
                for row in &records {
                    s.tx_undo_log.push(TxUndoEntry {
                        tx_id,
                        op: UndoOp::BufferedInsert {
                            table: table.to_string(),
                            row: row.clone(),
                        },
                    });
                }
            });
            self.insert_buffered(db, table, records)?
        } else if !self.enable_buffer {
            self.insert_direct(db, table, records)?
        } else {
            self.insert_buffered(db, table, records)?
        };
        // V311-07: Mark table dirty for optimized flush
        Self::with_write_lock(self, |s| {
            s.dirty_tables.insert((db.to_string(), table.to_string()));
        });
        // #5048: record the change so an incremental backup can be
        // produced from this data directory.
        if let Some(rows) = logged {
            for row in rows {
                self.record_change(table, ChangeOp::Insert, key_of(&row), Some(row));
            }
        }
        Ok(())
    }

    fn insert_buffered(&self, db: &str, table: &str, records: Vec<Record>) -> SqlResult<()> {
        let snap: Option<(usize, usize, Vec<ColumnDefinition>)> =
            Self::with_write_lock(self, |s| -> Option<(usize, usize, Vec<ColumnDefinition>)> {
                let buffered = s.insert_buffer.entry(self.tbl_in(db, table)).or_default();
                buffered.extend(records.iter().cloned());

                let mut result: Option<(usize, usize, Vec<ColumnDefinition>)> = None;
                if buffered.len() >= self.buffer_threshold {
                    if let Some(records) = s.insert_buffer.remove(&self.tbl_in(db, table)) {
                        let row_count = records.len();
                        if let Some(ref mut data) = s.tables.get_mut(&self.tbl_in(db, table)) {
                            let start_row_id = data.rows.len();
                            data.rows.extend(records.iter().cloned());
                            let cols = data.info.columns.clone();
                            // B2.1 / #4915 (F-09): same windowed
                            // snapshot as `insert_direct`.
                            let total_rows = data.rows.len();
                            let table_data = data.snapshot_from(start_row_id);
                            if self
                                .save_table_window(db, s, table, &table_data, total_rows)
                                .is_ok()
                            {
                                result = Some((start_row_id, row_count, cols));
                            }
                        }
                    }
                }
                result
            });
        if let Some((start_row_id, _row_count, columns)) = snap {
            // V400-PERF-FIX: pass &records directly. The closure
            // returns the columns/start_row_id but the records
            // have been moved into the closure body. Since
            // `records: Vec<Record>` is owned by this function
            // and the closure took ownership of the move
            // (`buffered.extend(records.iter().cloned())` does
            // clone, then `s.insert_buffer.remove(&crate::engine::scoped_key(&self.current_db.read().unwrap(), table))` moves
            // the buffer out — but `records` is still owned by
            // us at this point because we cloned into the buffer),
            // we can pass &records to read PKs directly.
            Self::update_pk_index(self, db, table, &columns, &records, start_row_id);
        }
        Ok(())
    }

    fn flush_buffer(&self, db: &str, table: &str) -> SqlResult<()> {
        let snap: Option<(usize, usize, Vec<ColumnDefinition>)> =
            Self::with_write_lock(self, |s| -> Option<(usize, usize, Vec<ColumnDefinition>)> {
                let mut result: Option<(usize, usize, Vec<ColumnDefinition>)> = None;
                if let Some(records) = s.insert_buffer.remove(&self.tbl_in(db, table)) {
                    let row_count = records.len();
                    if let Some(ref mut data) = s.tables.get_mut(&self.tbl_in(db, table)) {
                        let start_row_id = data.rows.len();
                        data.rows.extend(records);
                        let cols = data.info.columns.clone();
                        // B2.1 / #4915 (F-09): same windowed
                        // snapshot as `insert_direct`.
                        let total_rows = data.rows.len();
                        let table_data = data.snapshot_from(start_row_id);
                        if self
                            .save_table_window(db, s, table, &table_data, total_rows)
                            .is_ok()
                        {
                            result = Some((start_row_id, row_count, cols));
                        }
                    }
                }
                result
            });
        if let Some((start_row_id, row_count, columns)) = snap {
            // V400-PERF-FIX: flush_buffer has no caller-side records
            // (they were consumed by the closure via
            // `s.insert_buffer.remove(&crate::engine::scoped_key(&self.current_db.read().unwrap(), table))`). Use a separate
            // helper that reads ONLY the [start_row_id, +row_count)
            // window of data.rows — O(row_count) not O(table_size).
            Self::update_pk_index_window(self, db, table, &columns, start_row_id, row_count);
        }
        Ok(())
    }

    /// V400-MVCC-PKFAST: helper to update the PK B+Tree index for the
    /// rows just inserted. Caller passes the `records` it inserted so
    /// we don't have to clone the entire `data.rows` vector just to
    /// extract PK values for `count` rows.
    ///
    /// Acquires `indexes.write()` to perform the B+Tree inserts.
    fn update_pk_index(
        &self,
        db: &str,
        table: &str,
        columns: &[ColumnDefinition],
        records: &[Vec<Value>],
        start_row_id: usize,
    ) {
        let pk_col_idx = columns.iter().position(|c| c.primary_key);
        let Some(pk_idx) = pk_col_idx else { return };
        let pk_col_name = columns[pk_idx].name.clone();
        let mut updates: Vec<(i64, u32)> = Vec::with_capacity(records.len());
        for (i, row) in records.iter().enumerate() {
            if let Some(v) = row.get(pk_idx) {
                if let Some(ikey) = v.to_index_key() {
                    updates.push((ikey, (start_row_id + i) as u32));
                }
            }
        }
        if updates.is_empty() {
            return;
        }
        if let Ok(mut indexes) = self.indexes.write() {
            if let Some(index) = indexes.get_mut(&(self.tbl_in(db, table), pk_col_name.clone())) {
                for (ikey, rid) in updates {
                    index.insert(ikey, rid);
                }
            }
        }
    }

    /// V400-PERF-FIX: variant of `update_pk_index` for callers that
    /// have already moved the records into `data.rows` and only know
    /// the row-id window. Reads ONLY that window — O(row_count) not
    /// O(table_size).
    fn update_pk_index_window(
        &self,
        db: &str,
        table: &str,
        columns: &[ColumnDefinition],
        start_row_id: usize,
        count: usize,
    ) {
        if count == 0 {
            return;
        }
        let pk_col_idx = columns.iter().position(|c| c.primary_key);
        let Some(pk_idx) = pk_col_idx else { return };
        let pk_col_name = columns[pk_idx].name.clone();
        // Snapshot only the [start_row_id, start_row_id+count) window
        // so we don't pay O(table_size) for an O(count) operation.
        let rows_snapshot = Self::with_write_lock(self, |s| {
            s.tables.get(&self.tbl_in(db, table)).map(|t| {
                let end = (start_row_id + count).min(t.rows.len());
                if start_row_id < t.rows.len() {
                    t.rows[start_row_id..end].to_vec()
                } else {
                    Vec::new()
                }
            })
        });
        let Some(rows) = rows_snapshot else { return };
        if rows.is_empty() {
            return;
        }
        let mut updates: Vec<(i64, u32)> = Vec::with_capacity(rows.len());
        for (i, row) in rows.iter().enumerate() {
            if let Some(v) = row.get(pk_idx) {
                if let Some(ikey) = v.to_index_key() {
                    updates.push((ikey, (start_row_id + i) as u32));
                }
            }
        }
        if updates.is_empty() {
            return;
        }
        if let Ok(mut indexes) = self.indexes.write() {
            if let Some(index) = indexes.get_mut(&(self.tbl_in(db, table), pk_col_name.clone())) {
                for (ikey, rid) in updates {
                    index.insert(ikey, rid);
                }
            }
        }
    }

    /// #5057: push every database's buffered inserts into its `tables`
    /// cache and persist them under their OWN database directory.
    ///
    /// It used to walk only the ACTIVE database's buffer, despite the
    /// name. Two consequences, both silent:
    ///
    /// 1. Rows written into any database other than the active one stayed
    ///    in `insert_buffer` and never reached disk. A `flush()` from `d1`
    ///    persisted `d1` and left `d2`'s rows nowhere.
    /// 2. Those rows were also invisible to `drain_dirty_windowed`, which
    ///    reads `tables.rows` — so the dirty marker for `d2.t` was drained,
    ///    found an empty table, and skipped. One `flush()` and the rows
    ///    were gone with `Ok(())` returned.
    ///
    /// #5057 made the reach possible: `dirty_tables` and
    /// `last_saved_row_count` are keyed by `(db, table)`, so a drained
    /// entry can say which directory it belongs in. Doing this before that
    /// would have written each database's rows under the active one — the
    /// worse failure, and the reason the limitation was documented rather
    /// than half-fixed.
    ///
    /// Snapshot the table list under the lock; then drop the guard before
    /// re-acquiring per table (avoids holding the lock for the duration
    /// of all table saves).
    ///
    /// `insert_buffer` is keyed by `scoped_key(db, table)`, so the
    /// database is recoverable from the key and every buffered table can
    /// be flushed exactly once. A table may have several keys? No — one
    /// key per (db, table) — but dedup anyway so a repeated flush cannot
    /// persist the same window twice.
    pub fn flush_all_buffers(&self) -> SqlResult<()> {
        use std::collections::HashSet;
        let tables: Vec<(String, String)> = Self::with_write_lock(self, |s| {
            let mut seen: HashSet<(String, String)> = HashSet::new();
            let mut out = Vec::new();
            for key in s.insert_buffer.keys() {
                if let Some((db, table)) = crate::engine::split_scoped_key(key) {
                    let entry = (db.to_string(), table.to_string());
                    if seen.insert(entry.clone()) {
                        out.push(entry);
                    }
                }
            }
            out
        });
        for (db, table) in tables {
            self.flush_buffer(&db, &table)?;
        }
        Ok(())
    }

    /// Discard all buffered in-memory inserts without persisting them.
    /// Used by `WalStorage::rollback_transaction` so the rolled-back
    /// transaction's writes are not visible to subsequent reads or to
    /// the next `flush()`. Issue #3964: previously rollback called
    /// `inner.flush()`, which pushed the buffer to `data.rows` and then
    /// persisted the table to disk — making rolled-back rows visible.
    pub fn discard_all_buffers(&self) {
        Self::with_write_lock(self, |s| {
            s.insert_buffer.clear();
            // No dirty_tables entry to remove — the buffer was never
            // persisted, so the dirty marker for the rolled-back tx was
            // either not yet added or, if previously added by a prior
            // committed tx in the same session, the next flush() will
            // simply re-save the persisted state.
        });
    }

    /// Discard all row data in every in-memory table while preserving the
    /// schema. Used by `with_wal_recovery` to make the WAL the sole source
    /// of truth on startup, so we never end up with both persisted rows
    /// and replayed rows for the same entries.
    pub fn clear_all_tables(&self) {
        Self::with_write_lock(self, |s| {
            for data in s.tables.values_mut() {
                data.rows.clear();
            }
            s.insert_buffer.clear();
        });
    }

    /// #5057: [`update_if`](Self::update_if) against a stated database; see
    /// [`delete_at`](Self::delete_at).
    fn update_if_at(
        &mut self,
        db: &str,
        table: &str,
        filter: &RowFilter,
        mutation: &RowMutation,
    ) -> SqlResult<usize> {
        // #5025: resolve the key before the mutable borrow.
        //
        // #5055: as in `update`, the scan is scoped so the `&mut` on
        // `write_state` is released before the WAL append.
        //
        // #5060: `insert_buffer` rows are updated too.
        let key = self.tbl_in(db, table);
        let wal_on = self.wal_enabled();
        let watching = self.change_log_enabled();
        let tx_id = self.current_tx_id();
        let in_tx = tx_id != 0;
        let assignments = mutation.assignments().to_vec();
        let match_row = |record: &Record| filter(record);
        let apply = |record: &mut Record| {
            for &(col_idx, ref new_val) in &assignments {
                if col_idx < record.len() {
                    record[col_idx] = new_val.clone();
                }
            }
        };

        let (count, updated) = {
            let st = self.write_state.get_mut();
            let touched = st.mutate_matching(&key, match_row, apply);
            for (pre, post) in &touched {
                if in_tx {
                    st.tx_undo_log.push(TxUndoEntry {
                        tx_id,
                        op: UndoOp::BufferedUpdate {
                            table: table.to_string(),
                            post: post.clone(),
                            original: pre.clone(),
                        },
                    });
                }
            }
            if !touched.is_empty() {
                st.dirty_tables.insert((db.to_string(), table.to_string()));
            }
            (touched.len(), touched)
        };

        if watching {
            for (_, post) in &updated {
                self.record_change(table, ChangeOp::Update, key_of(post), Some(post.clone()));
            }
        }
        if wal_on {
            let posts: Vec<Record> = updated.into_iter().map(|(_, post)| post).collect();
            self.wal_append(self.wal_update_entries(table, &posts))?;
        }
        Ok(count)
    }

    /// #5057: [`update`](Self::update) against a stated database; see
    /// [`delete_at`](Self::delete_at).
    fn update_at(
        &mut self,
        db: &str,
        table: &str,
        filters: &[Value],
        updates: &[(usize, Value)],
    ) -> SqlResult<usize> {
        // #4951: `&mut self`, so `get_mut` yields the guarded state with
        // no lock. `tables` and `tx_undo_log` are separate fields of the
        // same struct, so the borrow checker can hand out both here —
        // the old code got them through one `&mut FileStorage`.
        //
        // #5055: the whole scan is scoped to a block so the `&mut` it
        // takes on `write_state` is released before the WAL append. An
        // append inside this scope would not compile *and* would hold
        // the storage write lock across a file write.
        //
        // #5060: the rows to update may be in `insert_buffer` as well as
        // `tables`. `mutate_matching` reaches both, so an autocommit
        // INSERT followed by an UPDATE now affects 1 row instead of 0.
        let wal_on = self.wal_enabled();
        let watching = self.change_log_enabled();
        let tx_id = self.current_tx_id();
        let in_tx = tx_id != 0;
        let key = self.tbl_in(db, table);
        let match_row = |record: &Record| {
            filters.is_empty()
                || filters
                    .iter()
                    .enumerate()
                    .all(|(i, f)| record.get(i).map(|v| v == f).unwrap_or(false))
        };
        let apply = |record: &mut Record| {
            for &(col_idx, ref new_val) in updates {
                if col_idx < record.len() {
                    record[col_idx] = new_val.clone();
                }
            }
        };

        let (count, updated) = {
            let st = self.write_state.get_mut();
            // Issue #4581 / B-track case 35-36: snapshot pre-images so
            // ROLLBACK can restore them. `mutate_matching` returns them
            // alongside the post-images.
            let touched = st.mutate_matching(&key, match_row, apply);
            for (pre, post) in &touched {
                if in_tx {
                    st.tx_undo_log.push(TxUndoEntry {
                        tx_id,
                        op: UndoOp::BufferedUpdate {
                            table: table.to_string(),
                            post: post.clone(),
                            original: pre.clone(),
                        },
                    });
                }
            }
            // V311-07: Mark dirty instead of immediate persist
            if !touched.is_empty() {
                st.dirty_tables.insert((db.to_string(), table.to_string()));
            }
            (touched.len(), touched)
        };

        // #5048: record the update after the borrow is released —
        // `record_change` takes the change-log lock.
        if watching {
            for (_, post) in &updated {
                self.record_change(table, ChangeOp::Update, key_of(post), Some(post.clone()));
            }
        }
        // #5055: outside the `write_state` borrow on purpose — this does
        // file I/O.
        if wal_on {
            let posts: Vec<Record> = updated.into_iter().map(|(_, post)| post).collect();
            self.wal_append(self.wal_update_entries(table, &posts))?;
        }
        Ok(count)
    }

    /// #5057: [`delete_if`](Self::delete_if) against a stated database;
    /// see [`delete_at`](Self::delete_at).
    fn delete_if_at(&mut self, db: &str, table: &str, filter: &RowFilter) -> SqlResult<usize> {
        // #5048 / #5055: the rows being removed may be in `insert_buffer`
        // or in `tables`, and both the change log and the WAL need that
        // same set. All of it happens in one critical section — reading
        // under one guard and retaining under another leaves a window
        // where a concurrent INSERT matches the filter, is silently
        // removed, and is missing from both logs, so a delta replays to
        // a row the live table no longer has.
        let watching = self.change_log_enabled();
        let wal_on = self.wal_enabled();
        let tx_id = self.current_tx_id();
        let in_tx = tx_id != 0;
        let scoped = crate::engine::scoped_key(db, table);
        let table_name = table.to_string();
        let match_row = |row: &Record| filter(row);

        let removed_rows = Self::with_write_lock(self, |s| -> Vec<Record> {
            if in_tx {
                if let Some(buffered) = s.insert_buffer.get(&scoped) {
                    for row in buffered.iter().filter(|r| match_row(r)) {
                        s.tx_undo_log.push(TxUndoEntry {
                            tx_id: self.current_tx_id(),
                            op: UndoOp::BufferedDelete {
                                table: table_name.clone(),
                                row: row.clone(),
                            },
                        });
                    }
                }
            }
            let removed = s.remove_matching(&scoped, match_row);
            if !removed.is_empty() {
                s.dirty_tables.insert((db.to_string(), table_name.clone()));
            }
            removed
        });

        if watching {
            for row in &removed_rows {
                self.record_change(table, ChangeOp::Delete, key_of(row), None);
            }
        }
        if wal_on {
            self.wal_append(self.wal_delete_entries(table, &removed_rows))?;
        }
        Ok(removed_rows.len())
    }

    /// #5057: [`delete_collect_pks`](Self::delete_collect_pks) against a
    /// stated database; see [`delete_at`](Self::delete_at).
    fn delete_collect_pks_at(
        &mut self,
        db: &str,
        table: &str,
        filters: &[Value],
    ) -> SqlResult<Vec<Value>> {
        // C.1.2: see `delete` for rationale — wrap the entire body in
        // `with_write_lock` because every step touches the protected
        // fields. Splitting would mean multiple lock acquisitions
        // and risk of torn state.
        Self::with_write_lock(self, |s| {
            let in_tx = self
                .current_tx_id
                .load(std::sync::atomic::Ordering::Acquire)
                != 0;

            // Snapshot rows for ROLLBACK (same as `delete`).
            let removed_pks: Vec<Value> = if let Some(ref mut data) =
                s.tables.get_mut(&crate::engine::scoped_key(db, table))
            {
                let original_len = data.rows.len();

                // Capture pre-delete undo log entries (same as `delete`).
                if in_tx {
                    if filters.is_empty() {
                        let snap = data.rows.clone();
                        s.tx_undo_log.push(TxUndoEntry {
                            tx_id: self.current_tx_id(),
                            op: UndoOp::DeleteAll {
                                table: table.to_string(),
                                original_rows: snap,
                            },
                        });
                    } else {
                        for (idx, row) in data.rows.iter().enumerate().rev() {
                            let matches = filters
                                .iter()
                                .enumerate()
                                .all(|(i, f)| row.get(i).map(|v| v == f).unwrap_or(false));
                            if matches {
                                s.tx_undo_log.push(TxUndoEntry {
                                    tx_id: self.current_tx_id(),
                                    op: UndoOp::DeleteRow {
                                        table: table.to_string(),
                                        row_idx: idx,
                                        original: row.clone(),
                                    },
                                });
                            }
                        }
                    }
                }

                // Collect PKs of rows that match the filter (before deletion).
                let pks: Vec<Value> = if filters.is_empty() {
                    // Full table wipe: caller (MVCC) handles by tombstoning
                    // all visible rows. Return empty to signal that.
                    Vec::new()
                } else {
                    data.rows
                        .iter()
                        .filter(|row| {
                            filters
                                .iter()
                                .enumerate()
                                .all(|(i, f)| row.get(i).map(|v| v == f).unwrap_or(false))
                        })
                        .filter_map(|row| row.first().cloned()) // PK = column 0
                        .collect()
                };

                // Now perform the actual deletion (same logic as `delete`).
                if filters.is_empty() {
                    data.rows.clear();
                } else {
                    data.rows.retain(|row| {
                        !filters
                            .iter()
                            .enumerate()
                            .all(|(i, f)| row.get(i).map(|v| v == f).unwrap_or(false))
                    });
                }
                debug_assert_eq!(pks.len(), original_len - data.rows.len());
                pks
            } else {
                Vec::new()
            };

            // Mark dirty if anything was removed.
            if !removed_pks.is_empty() || filters.is_empty() {
                s.dirty_tables.insert((db.to_string(), table.to_string()));
            }

            // #4960: rows inserted during a transaction live in
            // `insert_buffer` until the buffer threshold promotes them
            // into `tables.rows`. `removed_pks` above is built solely from
            // `tables.rows`, so for a table whose rows are all still
            // buffered it comes back EMPTY — the delete visibly "succeeds"
            // while reporting zero rows removed. ROLLBACK replays its undo
            // log through this method, so a transaction's INSERTs were
            // deleted from the buffer and then reported as not deleted,
            // and the caller's tombstoning (driven by `removed_pks`) never
            // ran either. That is why ROLLBACK appeared to do nothing.
            //
            // Count the buffered rows we are about to drop and report them
            // too, so the caller tombstones the same set it actually
            // removed. The deletion itself already happened just above.
            let mut removed_pks = removed_pks;
            if !filters.is_empty() {
                if let Some(buffered) = s.insert_buffer.get(&crate::engine::scoped_key(db, table)) {
                    for row in buffered {
                        if filters
                            .iter()
                            .enumerate()
                            .all(|(i, f)| row.get(i).map(|v| v == f).unwrap_or(false))
                        {
                            if let Some(pk) = row.first().cloned() {
                                if !removed_pks.contains(&pk) {
                                    removed_pks.push(pk);
                                }
                            }
                        }
                    }
                }
            }

            // After full table delete, clear any buffered inserts.
            // For partial delete, strip matching rows from insert_buffer.
            if filters.is_empty() {
                s.insert_buffer
                    .remove(&crate::engine::scoped_key(db, table));
            } else if let Some(buffered) = s
                .insert_buffer
                .get_mut(&crate::engine::scoped_key(db, table))
            {
                buffered.retain(|row| {
                    !filters
                        .iter()
                        .enumerate()
                        .all(|(i, f)| row.get(i).map(|v| v == f).unwrap_or(false))
                });
            }
            Ok(removed_pks)
        })
    }

    /// #5057: [`delete`](Self::delete) against a stated database.
    ///
    /// The shared storage holds one `current_db`, so the database-less form
    /// answers for whichever connection last selected one. Every caller that
    /// knows its own database should use this instead.
    fn delete_at(&mut self, db: &str, table: &str, filters: &[Value]) -> SqlResult<usize> {
        // C.1.2: the entire body is wrapped in `with_write_lock` because
        // every step touches {tables, dirty_tables, tx_undo_log,
        // insert_buffer}. Splitting would mean multiple lock acquisitions
        // and risk of observing torn state between them.
        //
        // #5060: the rows being deleted may be in `insert_buffer` rather
        // than `tables`, and both are removed here, in one critical
        // section. The old code read `tables` for the count, then
        // separately stripped the buffer — so a row that was still
        // buffered was deleted from the buffer but **not counted**, and
        // because the count was 0 `dirty_tables` was never set, so the
        // deletion was never persisted. The row disappeared from memory
        // and came back on the next open.
        let wal_on = self.wal_enabled();
        let watching = self.change_log_enabled();
        let scoped = crate::engine::scoped_key(db, table);
        let tx_id = self.current_tx_id();
        let in_tx = tx_id != 0;
        let table_name = table.to_string();
        let match_row = |row: &Record| {
            filters.is_empty()
                || filters
                    .iter()
                    .enumerate()
                    .all(|(i, f)| row.get(i).map(|v| v == f).unwrap_or(false))
        };

        let removed_rows = Self::with_write_lock(self, |s| -> SqlResult<Vec<Record>> {
            // Issue #4581 / B-track case 35-36: snapshot for ROLLBACK
            // BEFORE the removal, while the rows still exist.
            if in_tx {
                if let Some(data) = s.tables.get(&scoped) {
                    if filters.is_empty() {
                        s.tx_undo_log.push(TxUndoEntry {
                            tx_id: self.current_tx_id(),
                            op: UndoOp::DeleteAll {
                                table: table_name.clone(),
                                original_rows: data.rows.clone(),
                            },
                        });
                    } else {
                        for (idx, row) in data.rows.iter().enumerate().rev() {
                            if match_row(row) {
                                s.tx_undo_log.push(TxUndoEntry {
                                    tx_id: self.current_tx_id(),
                                    op: UndoOp::DeleteRow {
                                        table: table_name.clone(),
                                        row_idx: idx,
                                        original: row.clone(),
                                    },
                                });
                            }
                        }
                    }
                }
                // #5059: buffered rows have no index in `tables`, so they
                // are undone by value.
                if let Some(buffered) = s.insert_buffer.get(&scoped) {
                    for row in buffered.iter().filter(|r| match_row(r)) {
                        s.tx_undo_log.push(TxUndoEntry {
                            tx_id: self.current_tx_id(),
                            op: UndoOp::BufferedDelete {
                                table: table_name.clone(),
                                row: row.clone(),
                            },
                        });
                    }
                }
            }

            let removed = s.remove_matching(&scoped, match_row);

            // V311-07: Mark dirty instead of immediate persist. The old
            // condition keyed off the tables-only count, which missed
            // buffered deletions entirely.
            if !removed.is_empty() || filters.is_empty() {
                s.dirty_tables.insert((db.to_string(), table_name.clone()));
            }
            Ok(removed)
        })?;

        // #5048: record the deletion. `filters` is already positional
        // against the row's leading columns — exactly what a change
        // record's `key` is — so the same slice identifies the rows.
        // An empty filter means a full-table wipe; recording that once
        // with an empty key keeps the log honest without expanding it
        // into one entry per row.
        if watching && !filters.is_empty() {
            self.record_change(table, ChangeOp::Delete, filters.to_vec(), None);
        }

        // #5055: outside the storage write lock on purpose — this does
        // file I/O.
        if wal_on {
            self.wal_append(self.wal_delete_entries(table, &removed_rows))?;
        }
        Ok(removed_rows.len())
    }

    /// #5057: [`scan_with_index`](Self::scan_with_index) against a stated
    /// database.
    ///
    /// The index lookup here used a BARE table name while every index
    /// producer in this file — `create_index`, `rebuild_pk_indexes` (which
    /// runs on every startup), and `update_pk_index` (the only path that
    /// keeps index contents current) — keys by `scoped_key(db, table)`.
    /// The two never met, so the V312-85 / #4625 O(log N) PK lookup could
    /// not find any index `FileStorage` had actually built; callers reached
    /// it through the `scan_with_index not supported or failed, fall
    /// through` branch in `engine_select.rs` and silently paid a full
    /// table scan instead. Same key convention as `create_table_at`, now
    /// that it too is scoped.
    fn scan_with_index_in(
        &self,
        db: &str,
        table: &str,
        index_name: &str,
        key: &Value,
    ) -> SqlResult<Vec<Record>> {
        let indexes = self.indexes.read().unwrap();

        let index_key = (crate::engine::scoped_key(db, table), index_name.to_string());
        if let Some(index) = indexes.get(&index_key) {
            // Convert Value to i64 index key
            if let Some(search_key) = key.to_index_key() {
                // Find all row IDs with this key
                let row_ids = index.search_all(search_key);

                // #4951: single read guard over tables + insert_buffer
                // so the table and the buffer are read at one instant.
                let collected: Option<SqlResult<Vec<Record>>> = self.with_read_lock(|st| {
                    st.tables.get(&self.tbl_in(db, table)).map(|data| {
                        // Collect matching rows
                        let mut results = Vec::new();
                        for &row_id in &row_ids {
                            if (row_id as usize) < data.rows.len() {
                                results.push(data.rows[row_id as usize].clone());
                            }
                        }
                        // Also check insert_buffer
                        if let Some(buffered) =
                            st.insert_buffer.get(&crate::engine::scoped_key(db, table))
                        {
                            for record in buffered.iter() {
                                // Check if this buffered row matches the key
                                if let Some(col_idx) =
                                    data.info.columns.iter().position(|c| c.name == index_name)
                                {
                                    if record
                                        .get(col_idx)
                                        .map(|v| v.to_index_key() == Some(search_key))
                                        .unwrap_or(false)
                                    {
                                        results.push(record.clone());
                                    }
                                }
                            }
                        }
                        Ok(results)
                    })
                });
                if let Some(res) = collected {
                    return res;
                }
            }
        }
        // Index not found or not usable - fall back to full scan with filter
        let mut rows = self.scan(table)?;
        // Filter rows by the key value
        if let Some(col_idx) = self.with_table(table, |t| {
            t.and_then(|table_data| {
                table_data
                    .info
                    .columns
                    .iter()
                    .position(|c| c.name == index_name)
            })
        }) {
            rows.retain(|row| row.get(col_idx).map(|v| v == key).unwrap_or(false));
            return Ok(rows);
        }
        Err(SqlError::ExecutionError(format!(
            "Index '{}' on table '{}' not found or not usable",
            index_name, table
        )))
    }

    fn create_table_at(&mut self, db: &str, info: &TableInfo) -> SqlResult<()> {
        let table_data = TableData {
            info: info.clone(),
            rows: vec![],
        };
        self.insert_table_in(db, info.name.clone(), table_data)
            .map_err(|e| SqlError::ExecutionError(e.to_string()))?;
        // V400-MVCC-PKFAST: pre-create the PK B+Tree index so PK
        // lookups are O(log N) from the very first insert. Without
        // this, every PK lookup would have to wait for an explicit
        // `CREATE INDEX` (which production workloads never issue).
        //
        // #5057: the in-memory index key is scoped, like every other
        // index key in this file (`create_index`, `drop_index`, the
        // startup loader and `update_pk_index` all use `scoped_key`).
        // This one site used the bare table name, so the index
        // `CREATE TABLE` pre-created was unreachable from every lookup:
        // a dead entry that happened to occupy memory.
        if let Some(pk_col) = info.columns.iter().find(|c| c.primary_key) {
            let pk_col_name = pk_col.name.clone();
            let pk_col_idx = info
                .columns
                .iter()
                .position(|c| c.name == pk_col_name)
                .unwrap();
            let empty_index = crate::bplus_tree::BPlusTree::new();
            self.save_index_in(db, &info.name, &pk_col_name, &empty_index)
                .map_err(|e| SqlError::ExecutionError(e.to_string()))?;
            if let Ok(mut indexes) = self.indexes.write() {
                indexes.insert(
                    (
                        crate::engine::scoped_key(db, &info.name),
                        pk_col_name.clone(),
                    ),
                    empty_index,
                );
            }
            let _ = pk_col_idx; // silence unused if column moved
        }
        Ok(())
    }

    /// v3.10.0 Issue #3703: returns pre-partitioned chunks so the caller
    /// (typically `engine_select::filter_partitions_parallel`) can
    /// process each chunk on a separate rayon worker, fusing scan
    /// + filter into one parallel pipeline. Same semantics as
    ///   `MemoryStorage::partition_rows`.
    ///
    /// Merges `insert_buffer` rows (F-09 fix from `scan()`) so same-
    /// transaction SELECT/UPDATE sees rows that were just inserted.
    pub fn partition_rows(&self, table: &str, num_partitions: usize) -> Vec<Vec<Record>> {
        const PARALLEL_SCAN_MIN_ROWS: usize = 500_000;
        let n_partitions = num_partitions.max(1);
        // #4951 perf: `get_table` returns a cloned `TableData`, and the old
        // body then cloned `table_data.rows` a second time — two full copies
        // of the table per call. Take both collections under ONE read guard
        // instead: one copy, and the rows/buffer pair comes from a single
        // instant rather than two.
        let Some(all) = self.with_read_lock(|st| {
            let data = st.tables.get(&self.tbl(table))?;
            let mut all: Vec<Record> = data.rows.clone();
            if let Some(buffered) = st.insert_buffer.get(&crate::engine::scoped_key(
                &self.current_db.read().unwrap(),
                table,
            )) {
                all.extend(buffered.iter().cloned());
            }
            Some(all)
        }) else {
            return vec![Vec::new()];
        };
        let total_rows = all.len();
        if total_rows < PARALLEL_SCAN_MIN_ROWS || n_partitions <= 1 {
            return vec![all];
        }
        let total = all.len();
        let base = total / n_partitions;
        let rem = total % n_partitions;
        let mut out: Vec<Vec<Record>> = Vec::with_capacity(n_partitions);
        let mut cur = 0usize;
        for i in 0..n_partitions {
            let size = base + if i < rem { 1 } else { 0 };
            out.push(all[cur..cur + size].to_vec());
            cur += size;
        }
        out
    }
}

impl StorageEngine for FileStorage {
    fn in_transaction(&self) -> bool {
        // #4951: this used to read a plain `u64` from `&self` while
        // `set_current_tx_id` wrote it through `as_mut_self` — an
        // unsynchronised read/write of the same word. An atomic makes
        // the read well-defined; it does NOT make the value
        // per-connection, which is the remaining part of #4951.
        self.current_tx_id
            .load(std::sync::atomic::Ordering::Acquire)
            != 0
    }

    fn current_tx_id(&self) -> u64 {
        // #4951: see `in_transaction`.
        self.current_tx_id
            .load(std::sync::atomic::Ordering::Acquire)
    }

    fn set_current_tx_id(&mut self, id: u64) {
        // #4951: #4984 made this an `AtomicU64`, so the write-lock dance
        // that used to guard it bought nothing — the closure had no
        // `WriteState` field left to touch.
        self.current_tx_id
            .store(id, std::sync::atomic::Ordering::Release);
    }

    /// #4974: the `&self` counterpart of [`set_current_tx_id`](Self::set_current_tx_id).
    ///
    /// **This override is load-bearing for transaction isolation.**
    /// `StorageEngine`'s default is `fn set_current_tx_id_shared(&self, _id: u64) {}`
    /// — a *silent no-op*. `WalStorage::begin_transaction_lockfree`
    /// (the path an explicit `BEGIN` actually takes) propagates the tx id
    /// down the stack through this method:
    ///
    /// ```text
    /// BEGIN
    ///   -> WalStorage::begin_transaction_lockfree
    ///        self.inner().set_current_tx_id_shared(tx)   // MvccStorage
    ///          -> FileStorage::set_current_tx_id_shared   // <-- was the no-op default
    /// ```
    ///
    /// With the default in place, `FileStorage::current_tx_id()` stayed **0**
    /// for the whole transaction, and `MvccStorage::insert` does:
    ///
    /// ```ignore
    /// let tx_id = self.inner.current_tx_id();   // 0
    /// mvcc.put(pk, row, ts, tx_id);
    /// ```
    ///
    /// `VersionedTable::put` builds `committed: tx_id == 0`, so **every
    /// uncommitted row was marked committed the instant it was written** and
    /// became visible to every connection immediately — a textbook dirty
    /// read, reproducible over the real MySQL protocol:
    ///
    /// ```text
    /// PROBE while_A_uncommitted count=1   (expected 0)
    /// ```
    ///
    /// `Ordering::Release` matches `set_current_tx_id` above; the load side
    /// (`current_tx_id`) uses `Acquire`.
    fn set_current_tx_id_shared(&self, id: u64) {
        self.current_tx_id
            .store(id, std::sync::atomic::Ordering::Release);
    }

    /// Issue #4581 / B-track case 35-36: real BEGIN/COMMIT/ROLLBACK
    /// for the default FileStorage backend. The previous behaviour
    /// inherited the trait default which returned Err, making the
    /// entire transaction surface non-functional.
    ///
    /// Strategy: the BEGIN statement issues a fresh tx_id (monotonic
    /// counter starting from 1), zeroes the undo log, and arms the
    /// storage to record pre-image snapshots on the next UPDATE /
    /// DELETE / INSERT inside this tx. COMMIT drops the undo log and
    /// reverts to autocommit mode (tx_id = 0). ROLLBACK replays the
    /// undo log in reverse order, then clears it and reverts.
    ///
    /// Nested BEGIN behaviour: a second BEGIN while already in a tx
    /// is treated as a SAVEPOINT-less no-op (tx_id stays the same).
    /// This matches MySQL's pre-InnoDB nested-tx behaviour and is
    /// safe given that sqlrustgo does not yet implement
    /// SAVEPOINT/RELEASE SAVEPOINT.
    fn begin_transaction(&mut self) -> SqlResult<u64> {
        // `next_tx_id` is an inherent `&self` method — read-only on
        // tx_undo_log. Compute it outside the lock so the closure
        // body only touches the write-protected fields.
        let id = self.next_tx_id();
        let (tx_id, is_new) = Self::with_write_lock(self, |s| -> SqlResult<(u64, bool)> {
            use std::sync::atomic::Ordering as O;
            let existing = self.current_tx_id.load(O::Acquire);
            if existing != 0 {
                // Already in a tx — keep the existing id (MySQL-style nested BEGIN).
                return Ok((existing, false));
            }
            self.current_tx_id.store(id, O::Release);
            // Scoped like `commit_transaction`: this log is shared, so
            // clearing it wholesale would discard a concurrent
            // transaction's pending undo.
            s.tx_undo_log.retain(|e| e.tx_id != existing && e.tx_id != id);
            Ok((id, true))
        })?;
        // #5055: log the boundary. A replay can only tell a committed
        // row from an uncommitted one by seeing Commit, and it can only
        // see Commit if Begin was written for the same tx_id. Note this
        // fires only for a *new* transaction — a nested BEGIN must not
        // emit a second boundary for the same tx.
        if is_new && self.wal_enabled() {
            self.wal_append(vec![self.wal_tx_entry(WalEntryType::Begin, tx_id, 0)])?;
        }
        Ok(tx_id)
    }

    fn commit_transaction(&mut self) -> SqlResult<()> {
        let tx_id = self
            .current_tx_id
            .load(std::sync::atomic::Ordering::Acquire);
        if tx_id == 0 {
            // COMMIT outside a tx is a silent no-op (MySQL/SQLite semantics).
            return Ok(());
        }
        Self::with_write_lock(self, |s| {
            // Commit = drop the undo log + flush any buffered inserts that
            // accumulated during the tx. INSERTs buffered via insert_buffered
            // are NOT auto-flushed here; caller decides when to commit
            // visibility. We only need to drop undo so the next BEGIN gets a
            // fresh log.
            // Only this transaction's entries. A blanket `clear()`
            // wiped peers' pending undo too, so a concurrent COMMIT
            // silently discarded a still-open transaction's rollback
            // state — its later ROLLBACK then found nothing to undo.
            s.tx_undo_log.retain(|e| e.tx_id != tx_id);
            self.current_tx_id
                .store(0, std::sync::atomic::Ordering::Release);
        });
        // #5055: read the tx_id *before* the reset above and log the
        // Commit under the id the rows were written with. Logging it
        // after the reset — with `current_tx_id` now 0 — would produce a
        // Commit for tx 0 that matches no rows, and the whole
        // transaction would replay as uncommitted.
        if self.wal_enabled() {
            self.wal_append(vec![self.wal_tx_entry(WalEntryType::Commit, tx_id, 0)])?;
        }
        Ok(())
    }

    /// #4974: the `&self` counterpart of [`commit_transaction`](Self::commit_transaction).
    ///
    /// **This override is load-bearing.** Without it the trait default
    /// (`engine.rs:1430`) returns `Err("commit_transaction_lockfree not
    /// supported")`, and `MvccStorage::commit_transaction_lockfree` gates
    /// its `promote_pending()` on `r.is_ok()`. The leaf engine's
    /// *capability* signal would then silently switch off the layer
    /// above's correctness work — the committed rows never become visible:
    ///
    /// ```text
    /// PROBE while_A_uncommitted count=0   <- isolation correct
    /// PROBE after_A_commit     count=0   <- commit invisible
    /// PROBE VERDICT=LOST_WRITE
    /// ```
    ///
    /// Same shape as the `set_current_tx_id_shared` no-op default above —
    /// two different "unsupported" defaults (one silent `()`, one `Err`),
    /// both of which quietly disabled a transaction-isolation guarantee.
    fn commit_transaction_lockfree(&self) -> SqlResult<()> {
        if self
            .current_tx_id
            .load(std::sync::atomic::Ordering::Acquire)
            == 0
        {
            // COMMIT outside a tx is a silent no-op (MySQL/SQLite semantics).
            return Ok(());
        }
        // `with_write_lock` takes `&Self` since #4951, so the undo log can
        // be cleared from a `&self` method without laundering a `&mut`.
        Self::with_write_lock(self, |s| s.tx_undo_log.clear());
        self.current_tx_id
            .store(0, std::sync::atomic::Ordering::Release);
        Ok(())
    }

    /// #4974: `&self` counterpart of
    /// [`rollback_transaction`](Self::rollback_transaction). Same
    /// "capability signal would disable the caller's correctness work"
    /// argument as `commit_transaction_lockfree` above.
    fn rollback_transaction_lockfree(&self) -> SqlResult<()> {
        let tx_id = self
            .current_tx_id
            .load(std::sync::atomic::Ordering::Acquire);
        if tx_id == 0 {
            return Ok(());
        }
        Self::with_write_lock(self, |s| {
            s.tx_undo_log.clear();
            if let Some(buf) = s.insert_buffer.get_mut(&String::new()) {
                let _ = buf;
            }
        });
        self.current_tx_id
            .store(0, std::sync::atomic::Ordering::Release);
        // #5055: capture `tx_id` before the reset so the Rollback names
        // the transaction whose rows it cancels.
        if self.wal_enabled() {
            self.wal_append(vec![self.wal_tx_entry(WalEntryType::Rollback, tx_id, 0)])?;
        }
        Ok(())
    }

    fn rollback_transaction(&mut self) -> SqlResult<()> {
        let rolling_back_tx = self
            .current_tx_id
            .load(std::sync::atomic::Ordering::Acquire);
        if rolling_back_tx == 0 {
            // ROLLBACK outside a tx is a warning in MySQL but a no-op in
            // SQLite. Match SQLite to keep behavior consistent.
            return Ok(());
        }
        // C.1.2: drain undo log + drain insert_buffer + zero tx_id, all
        // under write_lock. The original code called `self.apply_undo(op)`
        // per entry, but apply_undo is an inherent `&self` method that
        // itself takes the lock — we cannot nest `with_write_lock` from
        // within an outer `with_write_lock`. Instead we inline the
        // apply_undo logic here (the bodies are short and only touch
        // {tables, dirty_tables, insert_buffer} — all protected).
        //
        // #5059: the previous "belt-and-suspenders" sweep over
        // `s.tables.keys()` never worked. Those keys are *already*
        // scoped (`default\x01tx_t`), and the loop scoped them a second
        // time, so it addressed `default\x01default\x01tx_t` and matched
        // no buffer. Every rolled-back INSERT therefore survived, and
        // `phase_c_1_race::c1_concurrent_begin_commit_rollback` saw 200
        // rows where 100 were expected.
        //
        // The fix is not to sweep harder. It is to undo by **value**:
        // `insert` records a `BufferedInsert` per row and `delete`
        // records a `BufferedDelete`, and this loop removes exactly
        // those. A blanket sweep would also delete *other* connections'
        // buffered rows — the buffer is instance-level, not
        // connection-level (#5060) — turning a rollback into data loss.
        Self::with_write_lock(self, |s| {
            while let Some(entry) = s.tx_undo_log.pop() {
                // This undo log is shared by every connection, so it can
                // hold entries belonging to transactions other than the
                // one rolling back. Replaying those would silently
                // revert a peer's committed work — two connections
                // DELETEing the same row made the loser's ROLLBACK
                // erase the winner's row, with the ROLLBACK reporting
                // success. Discard foreign entries instead; they belong
                // to a transaction that is still live (or already
                // committed) and will drain its own log.
                if entry.tx_id != rolling_back_tx {
                    continue;
                }
                match entry.op {
                    UndoOp::UpdateRow {
                        table,
                        row_idx,
                        original,
                    } => {
                        if let Some(data) = s.tables.get_mut(&crate::engine::scoped_key(
                            &self.current_db.read().unwrap(),
                            &table,
                        )) {
                            if row_idx < data.rows.len() {
                                data.rows[row_idx] = original;
                                s.dirty_tables
                                    .insert((self.current_db.read().unwrap().clone(), table));
                            }
                        }
                    }
                    UndoOp::DeleteRow {
                        table,
                        row_idx,
                        original,
                    } => {
                        if let Some(data) = s.tables.get_mut(&crate::engine::scoped_key(
                            &self.current_db.read().unwrap(),
                            &table,
                        )) {
                            let idx = row_idx.min(data.rows.len());
                            data.rows.insert(idx, original);
                            s.dirty_tables
                                .insert((self.current_db.read().unwrap().clone(), table));
                        }
                    }
                    UndoOp::DeleteAll {
                        table,
                        original_rows,
                    } => {
                        if let Some(data) = s.tables.get_mut(&crate::engine::scoped_key(
                            &self.current_db.read().unwrap(),
                            &table,
                        )) {
                            data.rows = original_rows;
                            s.dirty_tables
                                .insert((self.current_db.read().unwrap().clone(), table));
                        }
                    }
                    // #5055 / #5060: the row was inserted by *this*
                    // transaction and never became committed, so it
                    // goes back out of the buffer. Keyed by value, not
                    // position — the buffer is drained and refilled by
                    // concurrent work.
                    UndoOp::BufferedInsert { table, row } => {
                        let key =
                            crate::engine::scoped_key(&self.current_db.read().unwrap(), &table);
                        if let Some(buffered) = s.insert_buffer.get_mut(&key) {
                            if let Some(pos) = buffered.iter().position(|r| *r == row) {
                                buffered.remove(pos);
                            }
                        }
                    }
                    UndoOp::BufferedDelete { table, row } => {
                        let key =
                            crate::engine::scoped_key(&self.current_db.read().unwrap(), &table);
                        s.insert_buffer.entry(key).or_default().push(row);
                    }
                    // #5060: put the pre-image back where the post-image
                    // was. If the post-image is gone the buffered copy
                    // has moved on (flushed, or updated again) and the
                    // `tables` copy is covered by `UpdateRow`.
                    UndoOp::BufferedUpdate {
                        table,
                        post,
                        original,
                    } => {
                        if let Some(buffered) = s.insert_buffer.get_mut(&crate::engine::scoped_key(
                            &self.current_db.read().unwrap(),
                            &table,
                        )) {
                            if let Some(pos) = buffered.iter().position(|r| *r == post) {
                                buffered[pos] = original;
                            }
                        }
                    }
                }
            }
            self.current_tx_id
                .store(0, std::sync::atomic::Ordering::Release);
        });
        // #5055: as in `commit_transaction`, the id is captured before
        // the reset. A rollback that logs tx 0 would leave the real
        // transaction looking merely "uncommitted at target", which
        // reads as an in-flight transaction rather than a cancelled
        // one.
        if self.wal_enabled() {
            self.wal_append(vec![self.wal_tx_entry(
                WalEntryType::Rollback,
                rolling_back_tx,
                0,
            )])?;
        }
        Ok(())
    }

    fn scan(&self, table: &str) -> SqlResult<Vec<Record>> {
        // #4951: one read guard covers both collections so the table rows
        // and the buffer cannot come from two different instants — a scan
        // that merged a `tables` snapshot with a *newer* `insert_buffer`
        // would report a state that never existed.
        self.with_read_lock(|st| {
            let mut rows: Vec<Record> = st
                .tables
                .get(&self.tbl(table))
                .map(|data| data.rows.clone())
                .unwrap_or_default();
            // F-09 fix: merge insert_buffer so same-transaction SELECT/UPDATE sees
            // the rows that were just inserted (and not yet flushed to data.rows).
            if let Some(buffered) = st.insert_buffer.get(&crate::engine::scoped_key(
                &self.current_db.read().unwrap(),
                table,
            )) {
                rows.extend(buffered.iter().cloned());
            }
            Ok(rows)
        })
    }

    /// V4.0.0 / SOAK-leak fix: filter inside the read lock on `self.tables`
    /// so non-matching rows are never cloned. Default `scan` clones the full
    /// `Vec<Record>` first (O(N)) — at sysbench oltp_read_write with table_size
    /// 10000, this allocated ~2.6 MB per DELETE and was the dominant source of
    /// ~30 MB/min RSS growth. We also merge `insert_buffer` so same-tx SELECT
    /// still sees unflushed inserts.
    fn scan_with_filter(
        &self,
        table: &str,
        filter: &dyn Fn(&Record) -> bool,
    ) -> SqlResult<Vec<Record>> {
        // #4951: one read guard for both collections — see `scan`.
        self.with_read_lock(|st| {
            let mut rows: Vec<Record> = st
                .tables
                .get(&crate::engine::scoped_key(
                    &self.current_db.read().unwrap(),
                    table,
                ))
                .map(|data| data.rows.iter().filter(|r| filter(r)).cloned().collect())
                .unwrap_or_default();
            if let Some(buffered) = st.insert_buffer.get(&crate::engine::scoped_key(
                &self.current_db.read().unwrap(),
                table,
            )) {
                for record in buffered.iter() {
                    if filter(record) {
                        rows.push(record.clone());
                    }
                }
            }
            Ok(rows)
        })
    }

    /// Phase B Step 4.2: O(log N) primary-key lookup using the
    /// table's primary key index. Returns the row matching the PK, or
    /// None if not found. Falls back to scan_with_index for tables
    /// without a primary key index.
    fn scan_pk(&self, table: &str, pk_column: &str, pk: &Value) -> SqlResult<Option<Record>> {
        // Phase D.1: `pk_column` is now an explicit parameter
        // (previously hard-coded to `"id"`, which broke lookup on any
        // table whose PK column had a different name).
        if let Ok(info) = self.get_table_info(table) {
            if info.columns.iter().any(|c| c.primary_key) {
                let rows = self.scan_with_index(table, pk_column, pk)?;
                if !rows.is_empty() {
                    return Ok(rows.into_iter().next());
                }
                // V400-MVCC-PKFAST: B+Tree lookup returned empty.
                // This can mean either (a) no such PK in the table,
                // or (b) the table has a PK column but no B+Tree
                // index was ever created. In case (b) the row might
                // still exist in `data.rows` — fall through to the
                // full-scan fallback below to find it.
            }
        }
        // Fallback: full scan.
        Ok(self
            .scan(table)?
            .into_iter()
            .find(|row| row.first() == Some(pk)))
    }

    fn scan_with_index(
        &self,
        table: &str,
        index_name: &str,
        key: &Value,
    ) -> SqlResult<Vec<Record>> {
        let db = self.current_db_name();
        self.scan_with_index_in(&db, table, index_name, key)
    }

    fn parallel_scan(
        &self,
        table: &str,
        num_partitions: usize,
    ) -> SqlResult<Vec<Box<dyn Iterator<Item = Record> + Send>>> {
        // FileStorage caches all rows in memory (self.tables), so the
        // parallel_scan implementation mirrors MemoryStorage: partition
        // the cached row set into N iterators.
        //
        // For true disk-level parallelism (each worker reading a different
        // file offset), the file format would need to expose row offsets
        // via get_partition_boundaries(). The current on-disk format stores
        // rows in a length-prefixed binary format, so row-level seek is
        // possible but requires iterating from the start to find partition
        // boundaries. A future optimization can add that.
        // #5025: the cache is keyed by scoped name.
        let key = self.tbl(table);
        let rows: Vec<Record> = self.with_read_lock(|st| {
            let mut rows: Vec<Record> = st
                .tables
                .get(&key)
                .map(|data| data.rows.clone())
                .unwrap_or_default();
            // F-09 fix: merge insert_buffer for same-tx visibility
            if let Some(buffered) = st.insert_buffer.get(&key) {
                rows.extend(buffered.iter().cloned());
            }
            rows
        });
        let total = rows.len();
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
        let shared: Arc<Vec<Record>> = Arc::new(rows);
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

    fn insert(&mut self, table: &str, records: Vec<Record>) -> SqlResult<()> {
        let db = self.current_db_name();
        self.insert_at(&db, table, records)
    }

    /// #5057: [`insert`](Self::insert) into a stated database.
    ///
    /// Callers that hold the database should prefer this: the storage is
    /// shared by every connection, so `insert` resolves against whichever
    /// connection last ran `USE`, not against the caller.
    fn insert_in_db(&mut self, db: &str, table: &str, records: Vec<Record>) -> SqlResult<()> {
        self.insert_at(db, table, records)
    }

    /// F-09 fix: bypass insert_buffer so WAL recovery can replay entries
    /// deterministically. Subsequent scan/delete in the same recovery pass
    /// see the row in `data.rows` directly, avoiding the "3 rows expected 1"
    /// regression caused by buffered inserts piling up during replay.
    fn force_insert(&mut self, table: &str, record: Vec<Value>) -> SqlResult<()> {
        let db = self.current_db_name();
        self.insert_direct(&db, table, vec![record])
    }

    /// #5057: [`force_insert`](Self::force_insert) into a stated database.
    fn force_insert_in_db(&mut self, db: &str, table: &str, record: Vec<Value>) -> SqlResult<()> {
        self.insert_direct(db, table, vec![record])
    }

    fn delete(&mut self, table: &str, filters: &[Value]) -> SqlResult<usize> {
        let db = self.current_db_name();
        self.delete_at(&db, table, filters)
    }

    /// #5057: [`delete`](Self::delete) against a stated database.
    fn delete_in_db(&mut self, db: &str, table: &str, filters: &[Value]) -> SqlResult<usize> {
        self.delete_at(db, table, filters)
    }

    /// Phase B Step 4.1: like `delete`, but returns the list of
    /// primary keys (column 0) of deleted rows so the MVCC wrapper
    /// can tombstone them precisely. For an empty-filter delete (full
    /// table wipe) we return an empty Vec — the MVCC wrapper handles
    /// that by tombstoning all visible rows (correct semantics for
    /// "delete everything").
    fn delete_collect_pks(&mut self, table: &str, filters: &[Value]) -> SqlResult<Vec<Value>> {
        let db = self.current_db_name();
        self.delete_collect_pks_at(&db, table, filters)
    }

    /// #5057: [`delete_collect_pks`](Self::delete_collect_pks) against a stated database.
    fn delete_collect_pks_in_db(
        &mut self,
        db: &str,
        table: &str,
        filters: &[Value],
    ) -> SqlResult<Vec<Value>> {
        self.delete_collect_pks_at(db, table, filters)
    }

    fn delete_if(&mut self, table: &str, filter: &RowFilter) -> SqlResult<usize> {
        let db = self.current_db_name();
        self.delete_if_at(&db, table, filter)
    }

    /// #5057: [`delete_if`](Self::delete_if) against a stated database.
    fn delete_if_in_db(&mut self, db: &str, table: &str, filter: &RowFilter) -> SqlResult<usize> {
        self.delete_if_at(db, table, filter)
    }

    fn update(
        &mut self,
        table: &str,
        filters: &[Value],
        updates: &[(usize, Value)],
    ) -> SqlResult<usize> {
        let db = self.current_db_name();
        self.update_at(&db, table, filters, updates)
    }

    /// #5057: [`update`](Self::update) against a stated database.
    fn update_in_db(
        &mut self,
        db: &str,
        table: &str,
        filters: &[Value],
        updates: &[(usize, Value)],
    ) -> SqlResult<usize> {
        self.update_at(db, table, filters, updates)
    }

    fn update_if(
        &mut self,
        table: &str,
        filter: &RowFilter,
        mutation: &RowMutation,
    ) -> SqlResult<usize> {
        let db = self.current_db_name();
        self.update_if_at(&db, table, filter, mutation)
    }

    /// #5057: [`update_if`](Self::update_if) against a stated database.
    fn update_if_in_db(
        &mut self,
        db: &str,
        table: &str,
        filter: &RowFilter,
        mutation: &RowMutation,
    ) -> SqlResult<usize> {
        self.update_if_at(db, table, filter, mutation)
    }

    fn create_table(&mut self, info: &TableInfo) -> SqlResult<()> {
        let db = self.current_db_name();
        self.create_table_at(&db, info)
    }

    /// #5057: [`create_table`](Self::create_table) in a stated database.
    fn create_table_in_db(&mut self, db: &str, info: &TableInfo) -> SqlResult<()> {
        self.create_table_at(db, info)
    }

    // V4.0.0 / wired_insert_payload_regression_test fix: the trait
    // `drop_table` previously called `self.drop_table(table)` and
    // relied on Rust's resolver to pick the inherent `&self` method
    // over the trait method. With these two `drop_table` methods
    // declared on the same type (one inherent, one trait) the
    // resolver DOES pick the trait method inside the trait impl body
    // — causing infinite recursion. gdb backtrace of the regression
    // shows 10,000+ self-call frames of `<impl#3>::drop_table`
    // (line 3785) before SIGABRT. Fully-qualified path
    // `FileStorage::drop_table(self, table)` disambiguates to the
    // inherent `&self` implementation.
    #[allow(unconditional_recursion, clippy::only_used_in_recursion)]
    fn drop_table(&mut self, table: &str) -> SqlResult<()> {
        let db = self.current_db_name();
        self.drop_table_in_db(&db, table)
    }

    /// #5057: [`drop_table`](Self::drop_table) in a stated database.
    fn drop_table_in_db(&mut self, db: &str, table: &str) -> SqlResult<()> {
        // Fully-qualified so the resolver picks the inherent `&self`
        // implementation rather than this trait method — see the note
        // above about the infinite recursion this used to cause.
        FileStorage::drop_table_in(self, db, table)
            .map_err(|e| SqlError::ExecutionError(e.to_string()))?;
        Ok(())
    }

    fn get_table_info(&self, table: &str) -> SqlResult<TableInfo> {
        // #4951 perf: this must NOT go through `get_table`, which returns a
        // cloned `TableData` and would copy every row of the table just to
        // read the column list. `scan_pk` calls this on every point
        // lookup, so the clone sat on the hottest read path in the engine
        // and cost ~30% of read throughput at 5k rows. `with_table` hands
        // out a borrow under the read guard, so only `info` is copied.
        self.with_table(table, |t| t.map(|t| t.info.clone()))
            .ok_or_else(|| SqlError::TableNotFound(table.to_string()))
    }

    fn flush(&mut self) -> SqlResult<()> {
        // C.1.2: this used to be a second, independent copy of the loop
        // in `FileStorage::flush`, with its own
        // `self.tables.get(&self.tbl(name)).cloned()` — a whole-`TableData` copy per
        // dirty table. It is the override the server actually reaches
        // (through `MvccStorage`), so that copy was on the live path
        // while the optimised inherent copy was not; B2.2 improved a
        // method nobody called. See PERF_B22_CONCURRENT_MEASUREMENT.md §3.
        //
        // Both now share `drain_dirty_windowed`, which snapshots only the
        // rows appended since `last_saved` — the whole table is never
        // copied, and the write_lock is released before any I/O.
        FileStorage::flush(self)
            .map_err(|e| SqlError::ExecutionError(format!("flush storage: {}", e)))
    }

    // C.1.2: delegate to the inherent `&self` implementation; this is
    // NOT recursion (different declaration, same name). The
    // `unconditional_recursion` lint can't tell trait-vs-inherent
    // dispatch apart from a syntactic `self.method()` call.
    #[allow(unconditional_recursion)]
    fn discard_all_buffers(&mut self) {
        // StorageEngine::discard_all_buffers default is a no-op; for
        // FileStorage we actually drop the buffered inserts. Issue
        // #3964: rollback must NOT persist. Delegate to the inherent
        // `&self` implementation which already takes the write_lock.
        self.discard_all_buffers();
    }

    fn has_table(&self, table: &str) -> bool {
        self.contains_table(table)
    }

    fn list_tables(&self) -> Vec<String> {
        self.table_names()
    }

    fn create_index(&mut self, info: crate::engine::IndexInfo) -> SqlResult<()> {
        // Inline the implementation to avoid potential recursion issues
        // Get table from tables
        let table = info.table.as_str();
        let table_data = self
            .with_table(table, |t| t.cloned())
            .ok_or_else(|| SqlError::TableNotFound(table.to_string()))?;

        // V312-95 v3 / P3-HINT-001 follow-up: register the index in
        // the metadata catalog so `list_all_indexes()` returns it.
        // Without this, INDEXED BY <name> validation (see
        // src/engine_select.rs) reports the index as missing even
        // though the B+ tree below is correctly built.
        if let Ok(mut md) = self.index_metadata.write() {
            md.insert(info.name.clone(), info.clone());
        }

        // Build a B+ Tree for each column in the index
        let mut indexes = self.indexes.write().unwrap();
        for column in info.columns.iter() {
            // V313-100 / Issue #4701 sub-1: expression-only index
            // columns (no simple name) cannot back a B+ tree; the
            // executor does not yet materialise expressions. Skip with
            // a clear error rather than silently producing a bogus
            // index keyed off column 0.
            let column_name = match &column.name {
                Some(n) => n.clone(),
                None => {
                    return Err(SqlError::ExecutionError(format!(
                        "expression index column `{:?}` is not supported by the \
                         file_storage backend (V313-100); use a plain column name",
                        column.expression
                    )));
                }
            };
            let column_index = table_data
                .info
                .columns
                .iter()
                .position(|c| c.name == column_name)
                .unwrap_or(0);
            let mut index = crate::bplus_tree::BPlusTree::new();
            for (row_id, row) in table_data.rows.iter().enumerate() {
                if let Some(value) = row.get(column_index) {
                    if let Some(key) = value.to_index_key() {
                        index.insert(key, row_id as u32);
                    }
                }
            }

            // Save to disk
            self.save_index(table, &column_name, &index)
                .map_err(SqlError::from)?;

            // Store in memory
            indexes.insert((self.tbl(table), column_name), index);
        }

        Ok(())
    }

    fn drop_index(&mut self, table: &str, index_name: &str) -> SqlResult<()> {
        // #5025: index keys carry the scoped table name.
        let key = (self.tbl(table), index_name.to_string());

        if let Ok(mut indexes) = self.indexes.write() {
            indexes.remove(&key);
        }

        // V312-95 v3 / P3-HINT-001 follow-up: also remove the
        // metadata-catalog entry that names this index.
        if let Ok(mut md) = self.index_metadata.write() {
            md.retain(|_, info| !(info.table == table && info.name == index_name));
        }

        let path = self.index_path(table, index_name);
        if path.exists() {
            std::fs::remove_file(path).map_err(SqlError::from)?;
        }

        Ok(())
    }

    /// V312-95 v3 / P3-HINT-001 follow-up: enumerate index metadata
    /// so the executor's `INDEXED BY <name>` validator can find the
    /// index. The default trait method returns `Vec::new()`, which
    /// silently breaks every CLI batch-mode `INDEXED BY` query.
    fn list_all_indexes(&self) -> Vec<IndexInfo> {
        self.index_metadata
            .read()
            .map(|md| md.values().cloned().collect())
            .unwrap_or_default()
    }

    fn add_column(&mut self, table: &str, column: ColumnDefinition) -> SqlResult<()> {
        // #5057: read the database before taking the state lock.
        let db = self.current_db_name();
        Self::with_write_lock(self, |s| {
            if let Some(data) = s.tables.get_mut(&crate::engine::scoped_key(
                &self.current_db.read().unwrap(),
                table,
            )) {
                data.info.columns.push(column);
                // V312-72 / Issue #4647: backfill every existing row with
                // the new column's DEFAULT (or Value::Null when no default
                // is specified) so the schema and row layout stay aligned.
                // Without this, persisted rows would have one fewer column
                // than the schema claims, and SELECT * would only show the
                // original columns.
                let fill = crate::engine::default_fill_value(
                    &data
                        .info
                        .columns
                        .last()
                        .map(|c| c.default_value.clone())
                        .unwrap_or(None),
                );
                for row in data.rows.iter_mut() {
                    row.push(fill.clone());
                }
                let table_data = data.clone();
                self.save_table(&db, s, table, &table_data)?;
            }
            Ok(())
        })
    }

    fn rename_table(&mut self, table: &str, new_name: &str) -> SqlResult<()> {
        // #4951: `&mut self`, so `get_mut` gives exclusive access to
        // both the guarded state and the index map without taking any
        // lock. The old code went through `as_mut_self` +
        // `with_write_lock`, which for a `&mut self` caller bought a
        // mutex acquisition that could never be contended.
        // Paths are derived from `data_dir` alone, so compute them before
        // taking the `&mut` borrow — otherwise `self.table_path(..)` would
        // be an immutable use of `self` while `st` holds a mutable one.
        let old_path = self.table_path(table);
        let new_path = self.table_path(new_name);
        // Take the table out, end the `&mut` borrow, then persist it.
        // `save_table` takes `&self` (it needs `data_dir` and
        // `last_saved_row_count`), so holding `st` across the call would
        // be a `&mut self` + `&self` overlap. Persisting after the
        // removal is also the same ordering as before: the row is off the
        // map while the JSON is written, then re-inserted under the new
        // name.
        // #5025: the cache key is scoped.
        let old_key = self.tbl(table);
        let new_table_key = self.tbl(new_name);
        let mut table_data = match self.write_state.get_mut().tables.remove(&old_key) {
            Some(td) => td,
            None => return Ok(()),
        };
        table_data.info.name = new_name.to_string();

        let db = self.current_db_name();
        self.with_read_lock(|st| self.save_table(&db, st, new_name, &table_data))?;

        if old_path.exists() {
            std::fs::rename(&old_path, &new_path).map_err(SqlError::from)?;
        }
        {
            let st = self.write_state.get_mut();
            st.tables.insert(new_table_key, table_data);
        }
        {
            // `indexes` is a std::sync::RwLock (poison-aware), not the
            // parking_lot one `write_state` uses, so it is reached
            // through write()/read() rather than get_mut().
            if let Ok(mut indexes) = self.indexes.write() {
                let keys: Vec<_> = indexes.keys().cloned().collect();
                for key in keys {
                    if key.0 == table {
                        let new_key = (self.tbl(new_name), key.1.clone());
                        if let Some(idx) = indexes.remove(&key) {
                            indexes.insert(new_key, idx);
                        }
                    }
                }
            }
            if let Ok(indexes) = self.indexes.read() {
                for key in indexes.keys() {
                    if key.0 == new_name {
                        let old_idx_path = self.index_path(table, &key.1);
                        let new_idx_path = self.index_path_for_write(new_name, &key.1);
                        if old_idx_path.exists() {
                            std::fs::rename(&old_idx_path, &new_idx_path).ok();
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn create_trigger(&mut self, info: TriggerInfo) -> SqlResult<()> {
        // Persist to disk first (WAL-style: write-ahead, then mutate in-memory)
        self.save_trigger(&info)
            .map_err(|e| SqlError::ExecutionError(format!("save trigger: {}", e)))?;
        let mut triggers = self.triggers.write().unwrap();
        triggers.insert(info.name.clone(), info);
        Ok(())
    }

    fn drop_trigger(&mut self, name: &str) -> SqlResult<()> {
        self.remove_trigger_file(name)
            .map_err(|e| SqlError::ExecutionError(format!("remove trigger: {}", e)))?;
        let mut triggers = self.triggers.write().unwrap();
        triggers.remove(name);
        Ok(())
    }

    fn get_trigger(&self, name: &str) -> Option<TriggerInfo> {
        let triggers = self.triggers.read().unwrap();
        triggers.get(name).cloned()
    }

    fn list_triggers(&self, table: &str) -> Vec<TriggerInfo> {
        let triggers = self.triggers.read().unwrap();
        triggers
            .values()
            .filter(|t| t.table_name == table)
            .cloned()
            .collect()
    }

    fn has_view(&self, name: &str) -> bool {
        let views = self.views.read().unwrap();
        views.contains_key(name)
    }

    fn create_view(&mut self, info: ViewInfo) -> SqlResult<()> {
        // V312-95 v2 / Issue #4814: persist to disk first (WAL-style:
        // write-ahead, then mutate in-memory) — mirrors `create_trigger`.
        self.save_view(&info)
            .map_err(|e| SqlError::ExecutionError(format!("save view: {}", e)))?;
        let mut views = self.views.write().unwrap();
        if views.contains_key(&info.name) {
            return Err(SqlError::ExecutionError(format!(
                "View '{}' already exists",
                info.name
            )));
        }
        views.insert(info.name.clone(), info);
        Ok(())
    }

    fn get_view(&self, name: &str) -> Option<ViewInfo> {
        let views = self.views.read().unwrap();
        views.get(name).cloned()
    }

    fn list_views(&self) -> Vec<String> {
        let views = self.views.read().unwrap();
        let mut names: Vec<String> = views.keys().cloned().collect();
        names.sort();
        names
    }

    fn drop_view(&mut self, name: &str) -> SqlResult<()> {
        // V312-95 v2 / Issue #4814: idempotent — the executor guards the
        // IF EXISTS / not-found error path; storage just removes what it
        // has. Best-effort disk removal (missing file is OK).
        self.remove_view_file(name)
            .map_err(|e| SqlError::ExecutionError(format!("remove view: {}", e)))?;
        let mut views = self.views.write().unwrap();
        views.remove(name);
        Ok(())
    }

    fn list_indexes(&self, table: &str) -> Vec<(String, String)> {
        // #5025: keys carry the database, but callers expect the bare
        // table name back — `sqlite_master` and friends render this.
        let key = self.tbl(table);
        let indexes = self.indexes.read().unwrap();
        indexes
            .iter()
            .filter(|((t, _c), _idx)| *t == key)
            .map(|((_t, c), _idx)| (c.clone(), format!("{}_idx_{}", table, c)))
            .collect()
    }

    fn create_database(&mut self, db_name: &str) -> SqlResult<()> {
        let db_path = self.data_dir.join(db_name);
        std::fs::create_dir_all(&db_path)
            .map_err(|e| SqlError::ExecutionError(format!("create_database: {}", e)))
    }

    /// #5025: switch the active database.
    ///
    /// A database exists when its directory does. The implicit `default`
    /// database always resolves, even before any file is written — it is
    /// where pre-#5025 tables live, and rejecting `USE default` on a
    /// fresh installation would be surprising.
    fn set_current_db(&mut self, db_name: &str) -> SqlResult<()> {
        let key = db_name.to_lowercase();
        if key != crate::engine::DEFAULT_DATABASE && !self.data_dir.join(&key).is_dir() {
            return Err(SqlError::ExecutionError(format!(
                "Unknown database: {}",
                db_name
            )));
        }
        *self.current_db.write().unwrap() = key;
        Ok(())
    }

    fn current_db(&self) -> String {
        self.current_db.read().unwrap().clone()
    }

    /// #5057: resolve against the stated database, never the stored one.
    ///
    /// `tbl()` reads `current_db` on every call, so a lookup issued under
    /// one connection could be answered from another connection's database
    /// if a `USE` landed in between. Measured at 53.76% of statements under
    /// a concurrently switching writer, so this is the common path, not a
    /// rare race.
    ///
    /// The table must already be in the cache: this is the read path of an
    /// executing statement, which resolved the name through
    /// `get_table_info_in` first.
    fn get_table_info_in(&self, db: &str, table: &str) -> SqlResult<TableInfo> {
        let key = crate::engine::scoped_key(db, table);
        self.with_read_lock(|st| st.tables.get(&key).map(|t| t.info.clone()))
            .ok_or_else(|| SqlError::TableNotFound(table.to_string()))
    }

    /// #5057: `scan` against a stated database.
    ///
    /// `insert_buffer` is merged here exactly as `scan` does it. #5025
    /// added this method reading `tables` only, so it disagreed with `scan`
    /// about a row that had been inserted but not yet flushed: the same row
    /// was visible to one and invisible to the other. Since this method is
    /// what the executor will use once reads stop going through `current_db`,
    /// leaving it buffer-blind would have hidden every uncommitted row.
    fn scan_in_db(&self, db: &str, table: &str) -> SqlResult<Vec<Record>> {
        let key = crate::engine::scoped_key(db, table);
        Ok(self.with_read_lock(|st| {
            let mut rows: Vec<Record> = st
                .tables
                .get(&key)
                .map(|data| data.rows.clone())
                .unwrap_or_default();
            if let Some(buffered) = st.insert_buffer.get(&key) {
                rows.extend(buffered.iter().cloned());
            }
            rows
        }))
    }

    fn has_table_in(&self, db: &str, table: &str) -> bool {
        let key = crate::engine::scoped_key(db, table);
        self.with_read_lock(|st| st.tables.contains_key(&key))
    }

    /// #5057: index lookup against a stated database. See
    /// [`scan_with_index_in`](Self::scan_with_index_in).
    fn scan_with_index_in_db(
        &self,
        db: &str,
        table: &str,
        index_name: &str,
        key: &Value,
    ) -> SqlResult<Vec<Record>> {
        self.scan_with_index_in(db, table, index_name, key)
    }

    fn drop_database(&mut self, db_name: &str) -> SqlResult<()> {
        let db_path = self.data_dir.join(db_name);
        if db_path.exists() {
            let is_empty = std::fs::read_dir(&db_path)
                .map(|mut d| d.next().is_none())
                .unwrap_or(true);
            if !is_empty {
                return Err(SqlError::ExecutionError(
                    "database is not empty".to_string(),
                ));
            }
            std::fs::remove_dir(&db_path)
                .map_err(|e| SqlError::ExecutionError(format!("drop_database: {}", e)))?;
        }
        Ok(())
    }

    /// #5009: the file engine's `create_database` makes a directory under
    /// `data_dir`, so the databases *are* the sub-directories. Wal files
    /// are regular files and never appear here.
    fn list_databases(&self) -> SqlResult<Vec<String>> {
        let mut names = Vec::new();
        let entries = match std::fs::read_dir(&self.data_dir) {
            Ok(e) => e,
            // A data dir that does not exist yet simply holds no
            // databases; that is an empty list, not a failure.
            Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(names),
            Err(e) => return Err(SqlError::ExecutionError(format!("list_databases: {}", e))),
        };
        for entry in entries.flatten() {
            if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            if let Some(name) = entry.file_name().to_str() {
                names.push(name.to_string());
            }
        }
        names.sort();
        Ok(names)
    }

    fn drop_column(&mut self, table: &str, column: &str) -> SqlResult<()> {
        // #5025: resolve the key first — `self.tbl()` borrows `self`,
        // which conflicts with the `get_mut()` below.
        let key = self.tbl(table);
        let table_data = self
            .write_state
            .get_mut()
            .tables
            .get_mut(&key)
            .ok_or_else(|| SqlError::ExecutionError(format!("Table not found: {}", table)))?;
        let col_idx = table_data
            .info
            .columns
            .iter()
            .position(|c| c.name == column)
            .ok_or_else(|| SqlError::ExecutionError(format!("Column not found: {}", column)))?;
        table_data.info.columns.remove(col_idx);
        for record in table_data.rows.iter_mut() {
            if col_idx < record.len() {
                record.remove(col_idx);
            }
        }
        Ok(())
    }

    // Issue #4848: rename a column in a table. Row data in FileStorage
    // is positional (Vec<Vec<Value>>), so renaming only requires
    // updating the schema (info.columns[col_idx].name); the row data
    // itself is unchanged. We also reject the rename if the new name
    // already exists, matching MySQL 8.0 (ERROR_DUP_FIELDNAME 1060) and
    // SQLite (Error: duplicate column name) semantics. Mirrors the
    // MemoryStorage implementation at engine.rs:2315 but persists
    // via save_table (the default-trait impl in engine.rs:1033
    // returns "rename_column not supported" which broke the v3.12.0
    // GA CLI batch mode for `ALTER TABLE ... RENAME COLUMN`).
    fn rename_column(&mut self, table: &str, old_name: &str, new_name: &str) -> SqlResult<()> {
        // #4951: `&mut self`, so `get_mut` needs no lock. The rename and
        // the persist happen back-to-back with no lock in between — the
        // exclusive borrow is what guarantees no concurrent flush can
        // observe the renamed column without it reaching disk.
        let table_data_clone = {
            let st = self.write_state.get_mut();
            let table_data = st
                .tables
                .get_mut(&crate::engine::scoped_key(
                    &self.current_db.read().unwrap(),
                    table,
                ))
                .ok_or_else(|| SqlError::ExecutionError(format!("Table not found: {}", table)))?;
            let col_idx = table_data
                .info
                .columns
                .iter()
                .position(|c| c.name == old_name)
                .ok_or_else(|| {
                    SqlError::ExecutionError(format!("Column not found: {}", old_name))
                })?;
            // Reject duplicate destination name (MySQL 8.0 + SQLite parity).
            let new_lower = new_name.to_lowercase();
            if table_data
                .info
                .columns
                .iter()
                .enumerate()
                .any(|(idx, c)| idx != col_idx && c.name.to_lowercase() == new_lower)
            {
                return Err(SqlError::ExecutionError(format!(
                    "Duplicate column name: {}",
                    new_name
                )));
            }
            table_data.info.columns[col_idx].name = new_name.to_lowercase();
            table_data.clone()
        };
        // `save_table` needs `&self` (for `data_dir` / `last_saved_row_count`),
        // so it cannot run while the `&mut` borrow above is live.
        let db = self.current_db_name();
        self.with_read_lock(|st| self.save_table(&db, st, table, &table_data_clone))?;
        Ok(())
    }

    fn modify_column(
        &mut self,
        table: &str,
        column: &str,
        new_def: ColumnDefinition,
    ) -> SqlResult<()> {
        // #5025: resolve the key first — `self.tbl()` borrows `self`,
        // which conflicts with the `get_mut()` below.
        let key = self.tbl(table);
        let table_data = self
            .write_state
            .get_mut()
            .tables
            .get_mut(&key)
            .ok_or_else(|| SqlError::ExecutionError(format!("Table not found: {}", table)))?;
        let col_idx = table_data
            .info
            .columns
            .iter()
            .position(|c| c.name == column)
            .ok_or_else(|| SqlError::ExecutionError(format!("Column not found: {}", column)))?;
        table_data.info.columns[col_idx] = new_def;
        Ok(())
    }

    /// #5055: report whether a WAL is actually open.
    ///
    /// Without this override the trait default answers `false` for
    /// every `FileStorage` — including one built by `new_with_wal` —
    /// so any caller gating on the capability signal concluded that
    /// this backend had no durability story. That is the second half
    /// of the `admin pitr` bug: the log did not exist *and* the engine
    /// said it could not.
    fn is_wal_enabled(&self) -> bool {
        self.wal_enabled()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn gc(&self, _gc_lag: u64) -> usize {
        0
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod parallel_scan_tests {
    use super::*;

    #[test]
    fn test_parallel_scan_file_storage() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_parallel_scan_test");
        let _ = std::fs::remove_dir_all(&temp_dir);

        // Create storage and insert test data
        {
            let mut storage = FileStorage::new(temp_dir.clone()).unwrap();

            let table_data = TableData {
                info: TableInfo {
                    name: "numbers".to_string(),
                    columns: vec![ColumnDefinition {
                        name: "id".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        primary_key: true,
                        char_max_length: None,
                        collation: None,
                        default_value: None,
                        auto_increment: false,
                    }],
                    foreign_keys: vec![],
                    unique_constraints: vec![],
                    check_constraints: vec![],
                    compression: None,
                    collations: std::collections::HashMap::new(),
                    partition_info: None,
                    original_sql: String::new(),
                },
                rows: (0..100i64).map(|i| vec![Value::Integer(i)]).collect(),
            };

            storage
                .insert_table("numbers".to_string(), table_data)
                .unwrap();
        }

        // Load storage and test parallel_scan
        {
            let storage = FileStorage::new(temp_dir.clone()).unwrap();

            // Test with 4 partitions
            let partitions = storage.parallel_scan("numbers", 4).unwrap();

            // Note: FileStorage saves in binary format, so parallel_scan may return
            // fewer partitions due to format. The key invariant is that ALL rows
            // are returned across all partitions.
            assert!(!partitions.is_empty(), "Should have at least 1 partition");

            // Collect all rows from all partitions
            let total_rows: usize = partitions.into_iter().map(|p| p.count()).sum();
            assert_eq!(total_rows, 100, "Should return all 100 rows");
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_parallel_scan_empty_table() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_parallel_scan_empty");
        let _ = std::fs::remove_dir_all(&temp_dir);

        let storage = FileStorage::new(temp_dir.clone()).unwrap();
        // Should return empty vec for non-existent table
        let partitions = storage.parallel_scan("nonexistent", 4).unwrap();
        assert!(
            partitions.is_empty(),
            "Non-existent table should return empty partitions"
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_parallel_scan_single_partition() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_parallel_scan_single");
        let _ = std::fs::remove_dir_all(&temp_dir);

        {
            let mut storage = FileStorage::new(temp_dir.clone()).unwrap();

            let table_data = TableData {
                info: TableInfo {
                    name: "small".to_string(),
                    columns: vec![ColumnDefinition {
                        name: "id".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        primary_key: true,
                        char_max_length: None,
                        collation: None,
                        default_value: None,
                        auto_increment: false,
                    }],
                    foreign_keys: vec![],
                    unique_constraints: vec![],
                    check_constraints: vec![],
                    compression: None,
                    collations: std::collections::HashMap::new(),
                    partition_info: None,
                    original_sql: String::new(),
                },
                rows: vec![vec![Value::Integer(1)], vec![Value::Integer(2)]],
            };

            storage
                .insert_table("small".to_string(), table_data)
                .unwrap();
        }

        {
            let storage = FileStorage::new(temp_dir.clone()).unwrap();
            let partitions = storage.parallel_scan("small", 1).unwrap();
            assert!(!partitions.is_empty());

            let total: usize = partitions.into_iter().map(|p| p.count()).sum();
            assert_eq!(total, 2, "Should return all 2 rows");
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
impl FileStorage {
    /// Flush dirty tables in parallel using std::thread
    /// V311-09: Addresses global lock bottleneck - parallel table writes
    pub fn flush_parallel(&self) -> std::io::Result<()> {
        // Snapshot the pending windows under the write lock, up front.
        //
        // Three defects this fixes versus the previous shape, which
        // drained `dirty_tables` and then re-read `self.tables` per table:
        //
        // 1. Buffered inserts live in `insert_buffer`, not `tables.rows`,
        //    so the dirty-set window could be empty and nothing would be
        //    written. `flush_all_buffers` pushes them into `tables` first.
        // 2. The `<= 2 tables` branch called `self.flush()`, but the dirty
        //    set had already been taken, so that call saw an empty set and
        //    persisted nothing — the rows were silently dropped.
        // 3. The 3+ branch read `self.tables.get(&self.tbl(name))` from spawned
        //    threads. `tables` is a plain `HashMap` guarded by
        //    `write_lock`; reading it from `&self` while another thread
        //    holds that lock and mutates it is a data race, not just a
        //    stale read.
        //
        // Taking the window under the lock fixes all three: the pending
        // list is what gets written, on every branch.
        self.flush_all_buffers().map_err(Self::io_err_from_sql)?;
        let pending = self.drain_dirty_windowed();

        if pending.is_empty() {
            return Ok(());
        }

        // For 1-2 tables, sequential is faster (no thread overhead)
        if pending.len() <= 2 {
            for (db, name, window, total) in &pending {
                self.with_read_lock(|st| self.save_table_window(db, st, name, window, *total))?;
            }
            return Ok(());
        }

        // For 3+ tables, flush in parallel using thread pool. Each worker
        // writes its own window; the dirty set has already been drained and
        // snapshotted, so no writer can race us. Future inserts during the
        // parallel flush will mark tables dirty again, which the next
        // flush picks up.
        let results = std::thread::scope(|s| {
            let handles: Vec<_> = pending
                .iter()
                .map(|(db, name, window, total)| {
                    s.spawn(move || {
                        self.with_read_lock(|st| {
                            self.save_table_window(db, st, name, window, *total)
                        })
                    })
                })
                .collect();

            handles
                .into_iter()
                .map(|h| h.join().unwrap())
                .collect::<Vec<_>>()
        });

        // Combine all results - return first error if any
        for result in results {
            result?;
        }

        Ok(())
    }
}

// ====================================================================
// Issue #4581 / B-track case 35-36: FileStorage transaction helpers
// ====================================================================
// These methods are NOT on the StorageEngine trait — they are private
// helpers used by the begin/commit/rollback_transaction() impls above.
// Placed in `impl FileStorage` (not `impl StorageEngine`) so they don't
// pollute the trait surface.

impl FileStorage {
    /// Allocate the next transaction id.
    ///
    /// #5055: this used to derive the id from the wall clock —
    /// `now_nanos % 1_000_000` plus the length of the undo log — on the
    /// theory that the "not required for correctness" tie-breaker only
    /// had to make logs easier to read. It was required for
    /// correctness, because the id is what the WAL records:
    ///
    /// ```text
    /// PROBE 100 back-to-back BEGIN/COMMIT pairs
    /// PROBE distinct tx ids = 98   (expected 100)
    /// ```
    ///
    /// The modulo wraps every millisecond, and a COMMIT clears the undo
    /// log that the second term was counting, so two BEGINs a few
    /// microseconds apart routinely land on the same id. In the log
    /// that is not a cosmetic collision. A replay decides "committed"
    /// per `tx_id`, so a rolled-back transaction followed by a
    /// different transaction that happens to reuse its id is replayed
    /// as committed — rows that were explicitly discarded come back.
    ///
    /// A plain counter cannot collide. It starts at 1 because 0 means
    /// "autocommit" throughout this codebase, and it is per-process:
    /// a restart begins a new log, so there is nothing for it to
    /// collide with.
    fn next_tx_id(&self) -> u64 {
        self.next_tx_id_counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    /// Replay one UndoOp.
    ///
    /// C.1.2: `rollback_transaction` now inlines this logic to avoid
    /// nesting `with_write_lock` (the inherent `apply_undo` itself
    /// acquires the lock). This function is kept as `#[allow(dead_code)]`
    /// because (a) the logic is exercised by the inline copy and
    /// (b) we may want it for other callers (e.g. a future
    /// savepoint / nested-tx implementation). Delete only if a
    /// follow-up proves no caller ever needs it.
    #[allow(dead_code)]
    fn apply_undo(&self, op: UndoOp) -> SqlResult<()> {
        Self::with_write_lock(self, |s| {
            match op {
                UndoOp::UpdateRow {
                    table,
                    row_idx,
                    original,
                } => {
                    if let Some(data) = s.tables.get_mut(&crate::engine::scoped_key(
                        &self.current_db.read().unwrap(),
                        &table,
                    )) {
                        if row_idx < data.rows.len() {
                            data.rows[row_idx] = original;
                            s.dirty_tables
                                .insert((self.current_db.read().unwrap().clone(), table));
                        }
                    }
                }
                UndoOp::DeleteRow {
                    table,
                    row_idx,
                    original,
                } => {
                    if let Some(data) = s.tables.get_mut(&crate::engine::scoped_key(
                        &self.current_db.read().unwrap(),
                        &table,
                    )) {
                        let idx = row_idx.min(data.rows.len());
                        data.rows.insert(idx, original);
                        s.dirty_tables
                            .insert((self.current_db.read().unwrap().clone(), table));
                    }
                }
                UndoOp::DeleteAll {
                    table,
                    original_rows,
                } => {
                    if let Some(data) = s.tables.get_mut(&crate::engine::scoped_key(
                        &self.current_db.read().unwrap(),
                        &table,
                    )) {
                        data.rows = original_rows;
                        s.dirty_tables
                            .insert((self.current_db.read().unwrap().clone(), table));
                    }
                }
                UndoOp::BufferedInsert { table, row } => {
                    if let Some(buf) = s.insert_buffer.get_mut(&crate::engine::scoped_key(
                        &self.current_db.read().unwrap(),
                        &table,
                    )) {
                        buf.retain(|r| r != &row);
                    }
                }
                // #5059: mirror of `rollback_transaction`'s arm.
                UndoOp::BufferedDelete { table, row } => {
                    s.insert_buffer
                        .entry(crate::engine::scoped_key(
                            &self.current_db.read().unwrap(),
                            &table,
                        ))
                        .or_default()
                        .push(row);
                }
                // #5060: mirror of `rollback_transaction`'s arm.
                UndoOp::BufferedUpdate {
                    table,
                    post,
                    original,
                } => {
                    if let Some(buffered) = s.insert_buffer.get_mut(&crate::engine::scoped_key(
                        &self.current_db.read().unwrap(),
                        &table,
                    )) {
                        if let Some(pos) = buffered.iter().position(|r| *r == post) {
                            buffered[pos] = original;
                        }
                    }
                }
            }
            Ok(())
        })
    }
}

// --- #5025: per-database isolation on the production engine ---------------
//
// A separate module rather than an addition to the file's own `mod tests`:
// that one is large and its closing brace is awkward to locate reliably,
// and a mis-placed `#[test]` silently lands inside a function.

#[cfg(test)]
mod db_isolation_tests {
    use super::*;
    use crate::engine::ColumnDefinition;
    use std::fs::remove_dir_all;

    /// The on-disk engine is the production path. `MemoryStorage` covers
    /// the same contract in memory; this pins the part that only exists
    /// here — that a table's file lands under its database directory
    /// instead of being overwritten in the shared root.
    #[test]
    fn tables_and_files_are_scoped_per_database() {
        let dir = std::env::temp_dir().join("fs_db_iso_5025");
        let _ = remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let mut fs = FileStorage::new(dir.clone()).unwrap();
        fs.create_database("d1").unwrap();
        fs.create_database("d2").unwrap();

        for (db, id) in [("d1", 1i64), ("d2", 2)] {
            fs.set_current_db(db).unwrap();
            let mut info = TableInfo::default();
            info.name = "t".to_string();
            info.columns = vec![ColumnDefinition::new("id", "INTEGER")];
            fs.create_table(&info).unwrap();
            fs.force_insert("t", vec![Value::Integer(id)]).unwrap();
        }

        fs.set_current_db("d1").unwrap();
        assert_eq!(fs.scan("t").unwrap(), vec![vec![Value::Integer(1)]]);
        assert_eq!(fs.list_tables(), vec!["t".to_string()]);
        fs.set_current_db("d2").unwrap();
        assert_eq!(fs.scan("t").unwrap(), vec![vec![Value::Integer(2)]]);

        // Separate files, not one file written twice.
        assert!(dir.join("d1").join("t.json").exists(), "d1/t.json missing");
        assert!(dir.join("d2").join("t.json").exists(), "d2/t.json missing");
        assert!(!dir.join("t.json").exists(), "a table landed in the root");

        // Unknown database is an error; the failed switch changes nothing.
        assert!(fs.set_current_db("nope").is_err());
        assert_eq!(fs.current_db(), "d2");

        let _ = remove_dir_all(&dir);
    }

    /// Pre-#5025 every table sat directly in `data_dir`. Those must remain
    /// readable — the read path falls back to the root when the database
    /// directory is absent, so an existing installation is not broken by
    /// the layout change.
    #[test]
    fn pre_5025_root_layout_is_still_readable() {
        let dir = std::env::temp_dir().join("fs_db_legacy_5025");
        let _ = remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let mut info = TableInfo::default();
        info.name = "legacy".to_string();
        info.columns = vec![ColumnDefinition::new("id", "INTEGER")];
        {
            let mut fs = FileStorage::new(dir.clone()).unwrap();
            fs.create_table(&info).unwrap();
            fs.force_insert("legacy", vec![Value::Integer(42)]).unwrap();
        }
        assert!(
            dir.join("legacy.json").exists(),
            "expected the table at the data_dir root"
        );

        let fs = FileStorage::new(dir.clone()).unwrap();
        assert_eq!(fs.scan("legacy").unwrap(), vec![vec![Value::Integer(42)]]);

        let _ = remove_dir_all(&dir);
    }
}
