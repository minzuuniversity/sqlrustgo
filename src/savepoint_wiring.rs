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
//! enough information — database name, table name, primary-key column
//! values, and the full pre-image row — to let `execute_savepoint`
//! drive `storage.delete_in_db` / `storage.insert_in_db` in reverse
//! order.
//!
//! #5896: the database is part of what has to be captured. The undo
//! log used to hold the bare table name only, so a replay could not
//! tell `default.t` from `other_db.t` and had to assume the default
//! database — which meant a ROLLBACK in a named database undid
//! nothing, while a ROLLBACK in the default database could reach into
//! a named database's same-named table. Every wrapper therefore takes
//! the database the write happened in and stores it in the record.
//!
//! ## Scope discipline (v3.12 RC scope)
//! Issue #4581 fix: the wiring is now active for ANY active
//! transaction (not just savepoint-bearing ones). This enables
//! top-level `BEGIN; UPDATE; ROLLBACK;` to actually restore data,
//! since `TransactionManager::rollback_with_undo` re-plays the undo
//! log via the closure passed by the engine.
//!
//! Cost: one `Vec::push` per DML when in a tx; zero when not in a
//! tx (the engine does not call these from autocommit path).

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
/// record exists so `ROLLBACK [TO SAVEPOINT]` (and top-level
/// `ROLLBACK` per Issue #4581) can delete the new row by primary key.
///
/// v312-60: also captures the full inserted row so replayers can fall
/// back to full-row matching (`storage.delete(table, &row)`) when the
/// table has no primary key — see the doc on `UndoRecord::Insert`.
///
/// #5896: `db` is the database the INSERT actually wrote into — the
/// same value the DML executor passed to `storage.insert_in_db`. The
/// undo log used to hold the bare table name only, so the replay could
/// not tell `default.t` from `other_db.t` and had to assume the default
/// database. The database therefore travels with the record; it is an
/// explicit field on `UndoRecord` rather than a prefix glued onto the
/// table name (folding it into `table` would silently change the shape
/// of the key every consumer of `table` relies on).
pub fn record_insert_undo(
    tx_manager: &mut TransactionManager,
    tx_id: sqlrustgo_transaction::TxId,
    db: &str,
    table: &str,
    table_info: &TableInfo,
    row: &[Value],
) {
    let key = primary_key_values(table_info, row);
    let _ = tx_manager.add_undo_record(
        tx_id,
        UndoRecord::Insert {
            db: db.to_string(),
            table: table.to_string(),
            key,
            row: row.to_vec(),
        },
    );
}

/// Append a typed `UndoRecord::Delete` (so `ROLLBACK` can re-insert
/// the deleted row verbatim). Call AFTER `storage.delete` succeeds.
///
/// #5896: `db` is the database the DELETE happened in — see
/// [`record_insert_undo`] for why the database travels with the record.
pub fn record_delete_undo(
    tx_manager: &mut TransactionManager,
    tx_id: sqlrustgo_transaction::TxId,
    db: &str,
    table: &str,
    table_info: &TableInfo,
    row: &[Value],
) {
    let key = primary_key_values(table_info, row);
    let _ = tx_manager.add_undo_record(
        tx_id,
        UndoRecord::Delete {
            db: db.to_string(),
            table: table.to_string(),
            key,
            old_value: row.to_vec(),
        },
    );
}

/// Append a typed `UndoRecord::Update` so `ROLLBACK` can restore
/// the pre-image row. `prior_row` MUST be the snapshot taken BEFORE
/// the SET clauses were applied; `new_row` MUST be the snapshot taken
/// AFTER (the post-update tuple the storage now holds).
///
/// v312-60: also captures `new_row` so replayers can fall back to
/// full-row matching (`storage.delete(table, &new_row)`) when the table
/// has no primary key — see the doc on `UndoRecord::Update`.
///
/// #5896: `db` is the database the UPDATE happened in — see
/// [`record_insert_undo`] for why the database travels with the record.
pub fn record_update_undo(
    tx_manager: &mut TransactionManager,
    tx_id: sqlrustgo_transaction::TxId,
    db: &str,
    table: &str,
    table_info: &TableInfo,
    prior_row: &[Value],
    new_row: &[Value],
) {
    let key = primary_key_values(table_info, prior_row);
    let _ = tx_manager.add_undo_record(
        tx_id,
        UndoRecord::Update {
            db: db.to_string(),
            table: table.to_string(),
            key,
            old_value: prior_row.to_vec(),
            new_value: new_row.to_vec(),
        },
    );
}
