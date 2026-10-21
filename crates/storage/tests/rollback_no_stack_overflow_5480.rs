//! #5480 — a ROLLBACK against a file-backed store must not kill the process.
//!
//! The defect this pins is **not** in the WAL recovery path (which is where
//! #5480 originally pointed). It is in the **ROLLBACK** path:
//!
//! `crates/storage/src/file_storage.rs` had a `StorageEngine` trait
//! implementation that called `self.discard_all_buffers()` — a syntactic
//! self-call. The method resolver matched the trait method itself (the
//! receiver candidate `&mut FileStorage` matches the trait method's `self`
//! type by value, *before* the inherent `&self` implementation is ever
//! considered), so the call recursed forever:
//!
//! ```text
//! thread 'main' has overflowed its stack
//! fatal runtime error: stack overflow, aborting
//! ```
//!
//! SIGABRT takes the **whole process** down, so this is a server-crash
//! defect, not a connection-scoped one.
//!
//! This file is deliberately a **separate test target** from the isolation
//! contract test (`rollback_isolation_5480.rs`): a stack overflow aborts
//! the entire test binary, and the abort must not take the other
//! assertions down with it.
//!
//! Both the direct shape (`WalStorage<FileStorage, _>`) and the production
//! shape (`WalStorage<MvccStorage<FileStorage>, _>`,
//! `src/engine_builder.rs:294` / `:353` / `:411`) are covered — the
//! production shape reaches the same recursion through the generic
//! `MvccStorage<S: StorageEngine>` forwarding, where `S::discard_all_buffers`
//! can only resolve to the trait method.

use sqlrustgo_storage::engine::{ColumnDefinition, StorageEngine, TableInfo, Value};
use sqlrustgo_storage::file_storage::FileStorage;
use sqlrustgo_storage::mvcc_storage::MvccStorage;
use sqlrustgo_storage::wal::FileBackedWalManager;
use sqlrustgo_storage::WalStorage;

fn table_info(name: &str) -> TableInfo {
    let mut id = ColumnDefinition::new("id", "INTEGER");
    id.primary_key = true;
    TableInfo {
        name: name.to_string(),
        columns: vec![id, ColumnDefinition::new("v", "INTEGER")],
        ..Default::default()
    }
}

fn fresh_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "rollback_no_stack_overflow_5480_{tag}_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

/// The shape used by `tests/integration/stress/process_kill_crash_test.rs`.
#[test]
fn rollback_over_a_directly_wrapped_file_storage_does_not_overflow_the_stack() {
    let dir = fresh_dir("direct");
    let inner = FileStorage::new(dir.clone()).expect("FileStorage::new");
    let wal = FileBackedWalManager::new(dir.join("sqlrustgo.wal")).expect("wal manager");
    let mut storage = WalStorage::new(inner, wal).expect("WalStorage::new");

    storage
        .create_table(&table_info("t"))
        .expect("create table");
    let tx_id = 42u64;
    storage.set_current_tx_id(tx_id);
    storage
        .insert("t", vec![vec![Value::Integer(1), Value::Integer(1)]])
        .expect("insert");

    // Before the fix this call recursed until the stack overflowed and the
    // process was killed with SIGABRT.
    storage
        .rollback_transaction()
        .expect("rollback must not abort");
}

/// The **production** shape: `WalStorage<MvccStorage<FileStorage>, _>`.
///
/// Before the fix this shape aborted the process too: `MvccStorage` is
/// generic over `S: StorageEngine`, so `self.inner.discard_all_buffers()`
/// can only resolve to the trait method, which monomorphises to
/// `FileStorage`'s self-recursive implementation.
#[test]
fn rollback_over_the_production_storage_stack_does_not_overflow_the_stack() {
    let dir = fresh_dir("production");
    let file = FileStorage::new(dir.clone()).expect("FileStorage::new");
    let inner = MvccStorage::new(file);
    let wal = FileBackedWalManager::new(dir.join("sqlrustgo.wal")).expect("wal manager");
    let mut storage = WalStorage::new(inner, wal).expect("WalStorage::new");

    storage
        .create_table(&table_info("t"))
        .expect("create table");
    let tx_id = 42u64;
    storage.set_current_tx_id(tx_id);
    storage
        .insert("t", vec![vec![Value::Integer(1), Value::Integer(1)]])
        .expect("insert");

    storage
        .rollback_transaction()
        .expect("rollback must not abort");
}

/// The rollback must also be **semantically** correct, not merely
/// non-fatal: the aborted transaction's row must be gone afterwards.
///
/// This is the second half of #5480's root cause. `WalStorage` had two
/// `rollback_transaction` methods — an inherent one and the trait
/// implementation — and the inherent one **shadowed** the trait one at the
/// method-resolution step. The inherent copy never called
/// `inner.rollback_transaction()`, so `MvccStorage`'s pending versions were
/// never discarded and the aborted row stayed visible.
#[test]
fn rollback_removes_the_aborted_transaction_rows_from_the_production_stack() {
    let dir = fresh_dir("semantics");
    let file = FileStorage::new(dir.clone()).expect("FileStorage::new");
    let inner = MvccStorage::new(file);
    let wal = FileBackedWalManager::new(dir.join("sqlrustgo.wal")).expect("wal manager");
    let mut storage = WalStorage::new(inner, wal).expect("WalStorage::new");

    storage
        .create_table(&table_info("t"))
        .expect("create table");
    let tx_id = 42u64;
    storage.set_current_tx_id(tx_id);
    storage
        .insert("t", vec![vec![Value::Integer(7), Value::Integer(7)]])
        .expect("insert");
    assert_eq!(
        storage.scan("t").expect("scan").len(),
        1,
        "the row must be visible before the rollback"
    );

    storage.rollback_transaction().expect("rollback");

    assert_eq!(
        storage.scan("t").expect("scan").len(),
        0,
        "an aborted transaction's row must not survive its ROLLBACK"
    );
}
