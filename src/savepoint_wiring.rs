//! #4519 (清华 MySQL 课程第 9 章核心): SAVEPOINT / ROLLBACK TO SAVEPOINT
//! physical-undo helpers.
//!
//! Prior to this commit, `TransactionManager::rollback_to_savepoint`
//! accepted a no-op closure (`|_| Ok(())`) so the per-tx
//! `SavepointManager` would only pop entries from its undo log without
//! actually restoring storage state. As a result, after
//! `ROLLBACK TO SAVEPOINT sp1` the rows were still mutated — students
//! running the Tsinghua MySQL §9 examples would observe `val=200`
//! instead of the expected `val=100`.
//!
//! The functions in this module are the bridge between the DML
//! executors (`src/engine_dml.rs`) and the typed
//! `sqlrustgo_transaction::savepoint::UndoRecord` enum. They capture
//! enough information — table name, primary-key column values, and
//! the full pre-image row — to let `execute_savepoint` drive
//! `storage.delete` / `storage.insert` in reverse order.
//!
//! ## Scope discipline (v3.12 RC scope)
//! The wiring is **only** active when the active transaction has at
//! least one savepoint (`has_active_savepoint(tx_id) == true`). The
//! guard short-circuits before any heap allocation so savepoint-less
//! transactions pay zero cost.

use sqlrustgo_storage::TableInfo;
use sqlrustgo_transaction::savepoint::UndoRecord;
use sqlrustgo_transaction::TransactionManager;
use sqlrustgo_types::Value;

/// Compute the primary-key column values for a row in table order.
///
/// Returns an empty `Vec` when the table has no primary key (the
/// `storage.delete(table, &[Value])` API then degrades to "delete by
/// full-row match" which is the legacy fallback behaviour).
pub fn primary_key_values(table_info: &TableInfo, row: &[Value]) -> Vec<Value> {
    table_info
        .columns
        .iter()
        .enumerate()
        .filter_map(|(idx, col)| {
            if col.primary_key {
                row.get(idx).cloned()
            } else {
                None
            }
        })
        .collect()
}

/// Append a typed `UndoRecord::Insert` to the transaction's undo log.
///
/// Callers MUST invoke this AFTER `storage.insert` succeeds; the
/// record exists so a `ROLLBACK TO` can delete the new row by primary
/// key. The no-op short-circuit (`has_active_savepoint(tx_id)`) keeps
/// savepoint-less transactions off the hot path.
pub fn record_insert_undo(
    tx_manager: &mut TransactionManager,
    tx_id: sqlrustgo_transaction::TxId,
    table: &str,
    table_info: &TableInfo,
    row: &[Value],
) {
    if !tx_manager.has_active_savepoint(tx_id) {
        return;
    }
    let key = primary_key_values(table_info, row);
    let _ = tx_manager.add_undo_record(
        tx_id,
        UndoRecord::Insert {
            table: table.to_string(),
            key,
        },
    );
}

/// Append a typed `UndoRecord::Delete` (so a `ROLLBACK TO` re-inserts
/// the deleted row verbatim). Call AFTER `storage.delete` succeeds.
pub fn record_delete_undo(
    tx_manager: &mut TransactionManager,
    tx_id: sqlrustgo_transaction::TxId,
    table: &str,
    table_info: &TableInfo,
    row: &[Value],
) {
    if !tx_manager.has_active_savepoint(tx_id) {
        return;
    }
    let key = primary_key_values(table_info, row);
    let _ = tx_manager.add_undo_record(
        tx_id,
        UndoRecord::Delete {
            table: table.to_string(),
            key,
            old_value: row.to_vec(),
        },
    );
}

/// Append a typed `UndoRecord::Update` so a `ROLLBACK TO` can restore
/// the pre-image row. `prior_row` MUST be the snapshot taken BEFORE
/// the SET clauses were applied.
pub fn record_update_undo(
    tx_manager: &mut TransactionManager,
    tx_id: sqlrustgo_transaction::TxId,
    table: &str,
    table_info: &TableInfo,
    prior_row: &[Value],
) {
    if !tx_manager.has_active_savepoint(tx_id) {
        return;
    }
    let key = primary_key_values(table_info, prior_row);
    let _ = tx_manager.add_undo_record(
        tx_id,
        UndoRecord::Update {
            table: table.to_string(),
            key,
            old_value: prior_row.to_vec(),
        },
    );
}
