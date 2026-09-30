//! #4912 / v4.1.0-perf — lock-free transaction path forwarding regression.
//!
//! `WalStorage` implements `begin_transaction_lockfree` /
//! `commit_transaction_lockfree` / `rollback_transaction_lockfree`
//! (`crates/storage/src/wal_storage.rs`). The `StorageEngine` trait default
//! impls return `Err`, and `ExecutionEngine` probes them with
//! `is_ok()` before falling back to the global `Arc<RwLock<storage>>`
//! write lock (`src/execution_engine_methods.rs`).
//!
//! Every server-created storage is wrapped in `BoxStorageEngine`
//! (`crates/mysql-server/src/lib.rs`), so if the wrapper does not forward
//! these methods the type erasure silently resolves to the `Err` default
//! and the whole lock-free path becomes dead code — serialising all
//! concurrent readers. These tests pin that forwarding in place.

use sqlrustgo_storage::{
    BoxStorageEngine, MemoryStorage, MemoryWalManager, StorageEngine, WalStorage,
};

fn boxed_wal_storage() -> BoxStorageEngine {
    let wal_storage = WalStorage::new(MemoryStorage::new(), MemoryWalManager::new())
        .expect("WalStorage::new should succeed");
    BoxStorageEngine::new(wal_storage)
}

/// The wrapped engine itself must support the lock-free path (this is the
/// precondition that makes the wrapper test meaningful).
#[test]
fn wal_storage_begin_transaction_lockfree_is_supported() {
    let storage = WalStorage::new(MemoryStorage::new(), MemoryWalManager::new())
        .expect("WalStorage::new should succeed");

    assert!(
        storage.begin_transaction_lockfree(7).is_ok(),
        "WalStorage must implement begin_transaction_lockfree; without it the \
         engine falls back to the global storage write lock"
    );
}

/// Regression for #4912: `BoxStorageEngine` must forward the lock-free
/// BEGIN to the wrapped engine instead of inheriting the `Err` default.
#[test]
fn box_storage_engine_forwards_begin_transaction_lockfree() {
    let storage = boxed_wal_storage();

    assert!(
        storage.begin_transaction_lockfree(7).is_ok(),
        "BoxStorageEngine dropped begin_transaction_lockfree: the type erasure \
         resolves to the trait default (Err), forcing every BEGIN onto the \
         global storage write lock (#4912)"
    );
}

/// Regression for #4912: same for COMMIT.
#[test]
fn box_storage_engine_forwards_commit_transaction_lockfree() {
    let storage = boxed_wal_storage();

    // Begin first so the commit has a live transaction to close.
    assert!(storage.begin_transaction_lockfree(9).is_ok());

    assert!(
        storage.commit_transaction_lockfree().is_ok(),
        "BoxStorageEngine dropped commit_transaction_lockfree: every COMMIT \
         falls back to the global storage write lock (#4912)"
    );
}

/// Regression for #4912: same for ROLLBACK.
#[test]
fn box_storage_engine_forwards_rollback_transaction_lockfree() {
    let storage = boxed_wal_storage();

    assert!(storage.begin_transaction_lockfree(11).is_ok());

    assert!(
        storage.rollback_transaction_lockfree().is_ok(),
        "BoxStorageEngine dropped rollback_transaction_lockfree: every \
         ROLLBACK falls back to the global storage write lock (#4912)"
    );
}

/// A `BoxStorageEngine` wrapping an engine without an atomic implementation
/// must still report `Err` (so callers keep their write-lock fallback)
/// rather than reporting success it cannot honour.
#[test]
fn box_storage_engine_does_not_fake_support_for_plain_engines() {
    let storage = BoxStorageEngine::new(MemoryStorage::new());

    assert!(
        storage.begin_transaction_lockfree(13).is_err(),
        "MemoryStorage has no lock-free begin; the wrapper must surface that \
         as Err so the engine takes the documented fallback"
    );
}
