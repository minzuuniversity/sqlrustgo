//! Integration smoke test for the public `GroupCommitCoordinator` API.
//!
//! The actual coalescing math is covered by unit tests inside the
//! `wal::group_commit` module (see `crates/storage/src/wal/group_commit.rs`).
//! This file is a regression guard against accidental API breakage.

use sqlrustgo_storage::wal::group_commit::GroupCommitCoordinator;
use sqlrustgo_storage::wal::memory_wal_manager::MemoryWalManager;

#[test]
fn coordinator_public_api_is_reachable() {
    let coord = GroupCommitCoordinator::new(MemoryWalManager::new());
    coord.commit_lsn(0).unwrap();
    coord.commit_lsn(1).unwrap();
    coord.sync_now().unwrap();
}

#[test]
fn coordinator_with_limits_works() {
    let coord = GroupCommitCoordinator::with_limits(MemoryWalManager::new(), 4, 500);
    for i in 0..3 {
        coord.commit_lsn(i).unwrap();
    }
    coord.sync_now().unwrap();
}

#[test]
#[should_panic(expected = "max_batch must be > 0")]
fn rejects_zero_max_batch() {
    let _ = GroupCommitCoordinator::with_limits(MemoryWalManager::new(), 0, 1_000);
}

#[test]
#[should_panic(expected = "max_wait_us must be > 0")]
fn rejects_zero_max_wait() {
    let _ = GroupCommitCoordinator::with_limits(MemoryWalManager::new(), 4, 0);
}
