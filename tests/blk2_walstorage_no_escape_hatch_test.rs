// BLK-2 regression guard — the `*_transaction_lockfree(&self)` paths must
// not derive a `&mut S` from a shared reference, and the wrapper chain
// must actually forward the `&self` setters they rely on.
//
// ## Why this test exists (and why an earlier one was not enough)
//
// The first BLK-2 fix (PR #4932) added `set_current_tx_id_shared` /
// `discard_all_buffers_shared` to the `StorageEngine` trait, implemented
// them on the concrete backends, and wired the `BoxStorageEngine` type
// erasure wrapper to forward them. All of that was verified by a
// mutation test on the wrapper.
//
// It did NOT switch `WalStorage`'s own call sites over. The comments
// there were updated to describe the safe path, but the code below them
// still called `self.as_inner_mut()`. So the shipped `develop/v4.1.0`
// at `905549d9b5` carried a comment claiming "no `&mut` is derived"
// immediately above a line that derives one — the safety argument was
// false, and `as_inner_mut` (an `unsafe { &mut *self.inner.get() }`
// behind `#[allow(clippy::mut_from_ref)]`) was still live.
//
// `blk2_shared_tx_path_test.rs` did not catch this because it only
// asserted the wrapper forwards. Forwarding correctly into a call site
// that is never taken proves nothing.
//
// This file therefore asserts the property directly: the unsafety escape
// hatch must not exist, and the shared path must be what actually runs.

use sqlrustgo_storage::wal_storage::WalStorage;
use sqlrustgo_storage::{BoxStorageEngine, FileBackedWalManager, FileStorage, StorageEngine};

// ---------------------------------------------------------------------------
// Part 1 — the escape hatch must not exist
// ---------------------------------------------------------------------------

/// A source-level guard, because the property being protected is
/// "this function does not exist", which no runtime assertion can
/// observe. `as_inner_mut` hands out `&mut S` from `&self` by
/// transmuting through an `UnsafeCell`; on this type it aliased
/// whichever other connection held the storage write lock, because
/// every connection has its own `ExecutionEngine` over one shared
/// `Arc<RwLock<Storage>>`.
///
/// It is also reachable with zero call sites, which is exactly the
/// state PR #4932 left behind: dead, unsafe, and one careless call
/// away from a server-wide deadlock.
#[test]
fn wal_storage_has_no_mut_from_ref_escape_hatch() {
    let src = include_str!("../crates/storage/src/wal_storage.rs");

    assert!(
        !src.contains("fn as_inner_mut"),
        "wal_storage.rs reintroduced `as_inner_mut` — a `&mut S` derived from \
         `&self` via UnsafeCell. Every connection has its own ExecutionEngine over \
         one shared Arc<RwLock<Storage>>, so that `&mut` aliases whichever other \
         connection holds the write lock. This is the 8-thread server-wide \
         deadlock. Use the `*_shared` trait methods instead."
    );

    // The clippy suppression that kept the unsafety alive must go too.
    assert!(
        !src.contains("mut_from_ref"),
        "wal_storage.rs still carries #[allow(clippy::mut_from_ref)] — the \
         suppression that hid this unsafety from the linter."
    );
}

/// The lockfree paths must actually call the `&self` setters, not just
/// mention them in comments. Counting non-comment lines avoids the
/// false pass that "the word appears somewhere" would give.
#[test]
fn wal_storage_lockfree_paths_use_the_shared_setters() {
    let src = include_str!("../crates/storage/src/wal_storage.rs");

    let code_lines: Vec<&str> = src
        .lines()
        .filter(|l| {
            let t = l.trim_start();
            !t.starts_with("//") && !t.starts_with("///") && !t.starts_with('*')
        })
        .collect();

    let shared_tx_calls = code_lines
        .iter()
        .filter(|l| l.contains("set_current_tx_id_shared("))
        .count();
    let shared_discard_calls = code_lines
        .iter()
        .filter(|l| l.contains("discard_all_buffers_shared("))
        .count();

    assert!(
        shared_tx_calls >= 3,
        "expected all three {{begin,commit,rollback}}_transaction_lockfree to call \
         set_current_tx_id_shared, found {shared_tx_calls}"
    );
    assert!(
        shared_discard_calls >= 1,
        "expected at least one discard_all_buffers_shared call, found {shared_discard_calls}"
    );
}

// ---------------------------------------------------------------------------
// Part 2 — the wrapper must forward (regression guard for PR #4932)
// ---------------------------------------------------------------------------

/// `BoxStorageEngine` is a type-erasure wrapper, so a missing override
/// silently falls back to the trait's no-op default instead of failing
/// to compile. Every server-created storage is wrapped, so a missing
/// override makes the whole `*_shared` mechanism invisible in
/// production while all concrete-backend unit tests keep passing.
#[test]
fn box_storage_engine_forwards_shared_tx_id() {
    let boxed = BoxStorageEngine::new(sqlrustgo_storage::MemoryStorage::new());
    boxed.set_current_tx_id_shared(4242);
    assert_eq!(
        boxed.current_tx_id(),
        4242,
        "BoxStorageEngine did not forward set_current_tx_id_shared — the `*_shared` \
         mechanism is dead in production because every server storage is wrapped"
    );
}

// ---------------------------------------------------------------------------
// Part 3 — the shared path is actually exercised end to end
// ---------------------------------------------------------------------------

/// The defect's real signature is a hang, not an assertion: the wedged
/// server put every sampled thread in
/// `WalStorage::rollback_transaction_lockfree` and
/// `RawRwLock::lock_{shared,exclusive}_slow`. This drives concurrent
/// transactions through a real WAL-backed storage and asserts they all
/// finish. Run it under a harness timeout.
#[test]
fn concurrent_transactions_over_shared_wal_storage_complete() {
    use parking_lot::RwLock;
    use std::sync::Arc;
    use std::time::Duration;

    let dir = std::env::temp_dir().join("blk2_actual_fix_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");

    let file = FileStorage::new(dir.clone()).expect("FileStorage");
    let wal = FileBackedWalManager::new(dir.join("t.wal")).expect("wal");
    let storage = Arc::new(RwLock::new(WalStorage::new(file, wal).expect("WalStorage")));

    let mut handles = Vec::with_capacity(8);
    for t in 0..8u64 {
        let s = storage.clone();
        handles.push(std::thread::spawn(move || {
            for r in 0..40u64 {
                let tx_id = t * 100 + r + 1;
                {
                    let g = s.read();
                    g.begin_transaction_lockfree(tx_id).expect("begin");
                }
                {
                    let g = s.read();
                    g.commit_transaction_lockfree().expect("commit");
                }
                {
                    let g = s.read();
                    g.rollback_transaction_lockfree().expect("rollback");
                }
            }
        }));
    }

    for h in handles {
        h.join()
            .expect("worker must not panic or deadlock (checked under a harness timeout)");
    }

    let _ = Duration::from_secs(0);
    let _ = std::fs::remove_dir_all(&dir);
}
