// BLK-2 — the lock-free transaction path must not launder a `&mut` out of
// a read guard, and the type-erasure wrapper must actually forward it.
//
// ## The bug
//
// `WalStorage::{begin,commit,rollback}_transaction_lockfree` are declared
// `&self` so the engine can skip the global `Arc<RwLock<Storage>>` write
// lock on the tx-control path. The original implementations reached the
// backend through `as_inner_mut()`:
//
//     fn begin_transaction_lockfree(&self, tx_id: u64) -> SqlResult<()> {
//         self.as_inner_mut().set_current_tx_id(tx_id);  // &mut from &self
//         Ok(())
//     }
//
// Every connection owns its own `ExecutionEngine` over one shared
// `Arc<RwLock<Storage>>`. Handing a `&mut` out of a read guard therefore
// aliases whatever other connection currently holds the write lock. Under
// 8 concurrent read/write threads that race deadlocked the whole server.
//
// The fix routes the write through new `&self` trait methods
// (`set_current_tx_id_shared` / `discard_all_buffers_shared`) which mutate
// through an `AtomicU64` (MemoryStorage) or the backend's own internal
// write lock (FileStorage).
//
// ## Why this test exists
//
// Forwarding is easy to drop again: `BoxStorageEngine` is a type-erasure
// wrapper, so a missing override silently falls back to the trait's
// no-op default instead of failing to compile. Every server-created
// storage is wrapped in `BoxStorageEngine`, so a missing override makes
// the whole BLK-2 fix invisible in production while all concrete-backend
// unit tests keep passing. These tests assert the forwarding itself, so
// re-removing it fails here rather than in an 8-thread soak.

use sqlrustgo_storage::{BoxStorageEngine, MemoryStorage, StorageEngine};

// ---------------------------------------------------------------------
// Part 1 — forwarding through the type-erasure wrapper
// ---------------------------------------------------------------------

/// The regression this test locks down: dropping the two
/// `BoxStorageEngine` overrides makes the shared setters no-ops, so a
/// wrapped backend never learns the new transaction id.
#[test]
fn box_storage_engine_forwards_shared_tx_id() {
    let boxed = BoxStorageEngine::new(MemoryStorage::new());

    boxed.set_current_tx_id_shared(4242);

    assert_eq!(
        boxed.current_tx_id(),
        4242,
        "BoxStorageEngine did not forward set_current_tx_id_shared to the boxed backend; \
         the BLK-2 fix is dead in production because every server storage is wrapped"
    );
}

/// The `&mut` setter and the `&self` setter must agree, otherwise a
/// caller that mixes them silently loses a transaction id.
#[test]
fn box_storage_engine_shared_and_mut_setters_agree() {
    let boxed = BoxStorageEngine::new(MemoryStorage::new());

    boxed.set_current_tx_id_shared(7);
    assert_eq!(boxed.current_tx_id(), 7, "shared setter must be visible");

    let mut boxed = boxed;
    boxed.set_current_tx_id(9);
    assert_eq!(boxed.current_tx_id(), 9, "mut setter must overwrite the shared one");
}

/// `discard_all_buffers_shared` must be callable through the wrapper
/// without touching the global write lock. It has no observable state on
/// `MemoryStorage`, so this asserts safety and non-deadlock only — the
/// regression it guards is the missing override making production fall
/// back to the write-lock path.
#[test]
fn box_storage_engine_shared_discard_is_callable() {
    let boxed = BoxStorageEngine::new(MemoryStorage::new());
    boxed.discard_all_buffers_shared();
}

/// The concrete backend must store the tx id in an atomic, because the
/// shared path is called concurrently from several connections.
#[test]
fn memory_storage_shared_tx_id_survives_concurrent_writers() {
    use std::sync::Arc;

    let storage = Arc::new(MemoryStorage::new());

    let handles: Vec<_> = (1..=8u64)
        .map(|id| {
            let s = storage.clone();
            std::thread::spawn(move || s.set_current_tx_id_shared(id))
        })
        .collect();

    for h in handles {
        h.join().expect("worker must not panic");
    }

    // Whichever worker landed last, the id must be one of the writers'
    // and never 0 (the sentinel meaning "not in a transaction").
    let observed = storage.current_tx_id();
    assert!(
        (1..=8).contains(&observed),
        "current_tx_id should be one of the 8 writers, got {observed}"
    );

    // And a later write must still land, proving the atomic is live
    // rather than a per-thread copy that got discarded.
    storage.set_current_tx_id_shared(99);
    assert_eq!(storage.current_tx_id(), 99);
}

// ---------------------------------------------------------------------
// Part 2 — the aliasing bug the fix removes
// ---------------------------------------------------------------------

/// Reproduces the original defect shape: several connections, each with
/// its own engine, driving concurrent transactions over one shared
/// storage. Under the old `as_inner_mut()` implementation this
/// deadlocked (the `&mut` handed out by a read guard aliased the other
/// connection's write-lock holder). With the atomic-backed shared setter
/// it must complete.
///
/// The test asserts only that all workers finish. A deadlock shows up as
/// this test hanging rather than as an assertion failure, so it is
/// written to be run under a harness timeout in CI.
#[test]
fn concurrent_transactions_over_shared_storage_complete() {
    use sqlrustgo::ExecutionEngine;
    use std::sync::Arc;

    let storage = Arc::new(parking_lot::RwLock::new(MemoryStorage::new()));
    {
        let mut engine = ExecutionEngine::new(storage.clone());
        engine
            .execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER)")
            .expect("CREATE TABLE must succeed");
    }

    let mut handles = Vec::with_capacity(8);
    for t in 0..8u64 {
        let storage = storage.clone();
        handles.push(std::thread::spawn(move || {
            let mut engine = ExecutionEngine::new(storage);
            for i in 0..50u64 {
                engine
                    .execute(&format!(
                        "INSERT INTO t (id, v) VALUES ({}, {})",
                        t * 1000 + i,
                        t * 50 + i
                    ))
                    .expect("INSERT must succeed");
            }
            engine.execute("BEGIN").expect("BEGIN must succeed");
            engine.execute("COMMIT").expect("COMMIT must succeed");
        }));
    }

    for h in handles {
        h.join().expect("worker must not panic or deadlock");
    }

    let mut engine = ExecutionEngine::new(storage);
    let result = engine.execute("SELECT COUNT(*) FROM t").expect("SELECT must succeed");
    assert_eq!(
        result.rows.len(),
        1,
        "COUNT must return exactly one row; got {result:?}"
    );
}
