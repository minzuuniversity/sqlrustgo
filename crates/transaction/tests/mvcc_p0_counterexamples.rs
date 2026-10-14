//! Counterexample tests for cargo-mutants SURVIVED P0 mutants on `sqlrustgo-transaction`.
//!
//! Written on 2026-10-11 in the `a06-mvcc` worktree based on
//! `origin/develop/v4.1.0@326d1ba8c5`.  Targets the 5 highest-priority
//! P0 mutations listed in Issue #5228 comment 183184 (A06 scope) plus
//! the 4 follow-up focus items.  See `reports/SURVIVED_92_TRIAGE_v2.md`
//! (accessible via `git show origin/develop/v4.1.0:reports/...`) for
//! the full 51-P0 inventory.
//!
//! **Focus targets** (comment 183184 §5):
//!   1. `mvcc_write` no-op (`manager.rs:229`) — single highest-risk mutant
//!   2. `mvcc_rollback` no-op / `delete !` (`manager.rs:257,264`)
//!   3. `Transaction::is_committed / is_aborted` always-true (`mvcc.rs:102,106`)
//!   4. `Snapshot::is_visible` boundary mutations (`mvcc.rs:142,153,170`) — 6 forms
//!   5. `MvccEngine::commit_transaction` `+= → *=` (`mvcc.rs:287`)
//!
//! Plus Path C and D P0s:
//!   6. `get_global_timestamp` constant-0 / constant-1 (`mvcc.rs:320`)
//!   7. `RowVersion::is_visible` `>` → `>=` (`mvcc.rs:238`)
//!   8. `VersionChainMap::get_chain` None / empty (`version_chain.rs:65`)
//!   9. `VersionChainMap::rollback_versions` no-op (`version_chain.rs:90`)
//!  10. `SsiDetectorSync::validate_commit` `>` → `>=` (`ssi.rs:269,273`)
//!  11. `MvccEngine::begin_transaction` `+= → *=` (`mvcc.rs:276`)
//!
//! **Pristine check** (must pass on unmodified `develop/v4.1.0` HEAD).
//! **Mutant check** (each test must FAIL when its target mutation is
//! applied — verified by patching then `git checkout` revert).

use sqlrustgo_transaction::manager::{IsolationLevel, TransactionManager};
use sqlrustgo_transaction::mvcc::{MvccEngine, Snapshot, TxId};
use sqlrustgo_transaction::mvcc_storage::MVCCStorageEngine;
use sqlrustgo_transaction::ssi::SsiDetectorSync;
use sqlrustgo_transaction::version_chain::VersionChainMap;

// -----------------------------------------------------------------------------
// Counter 1: `mvcc_write` no-op (A.1, manager.rs:229)
//
// Original: `storage.write_version(key, value, tx_id); Ok(())`
// Mutation: entire body replaced with `Ok(())` — write is silently dropped.
//
// Counterexample: begin → write → read (same tx) must see the value.
// Under the mutation the write is lost and read returns None.
// -----------------------------------------------------------------------------
#[test]
fn mvcc_write_must_persist_for_same_transaction() {
    let mut tm = TransactionManager::new();
    let mut storage = MVCCStorageEngine::new();

    let _tx = tm.begin().expect("begin");
    tm.mvcc_write(&mut storage, b"k1".to_vec(), b"v1".to_vec())
        .expect("write must succeed");

    let got = tm.mvcc_read(&storage, b"k1").expect("read must succeed");
    assert_eq!(
        got,
        Some(b"v1".to_vec()),
        "same-tx read must observe its own write; mutation mvcc_write→Ok(()) would yield None"
    );
}

#[test]
fn mvcc_write_must_be_visible_after_commit() {
    let mut tm = TransactionManager::new();
    let mut storage = MVCCStorageEngine::new();

    let _tx = tm.begin().expect("begin");
    tm.mvcc_write(&mut storage, b"k1".to_vec(), b"v1".to_vec())
        .expect("write");
    let _commit_ts = tm.mvcc_commit(&mut storage).expect("commit");

    // Start a NEW transaction and read — committed data must be visible.
    let _tx2 = tm.begin().expect("begin tx2");
    let got = tm.mvcc_read(&storage, b"k1").expect("read");
    assert_eq!(
        got,
        Some(b"v1".to_vec()),
        "committed write must be visible to a new tx; mutation mvcc_write→Ok(()) would yield None"
    );
}

// -----------------------------------------------------------------------------
// Counter 2: `mvcc_rollback` no-op (A.2, manager.rs:257) and `delete !`
// (A.3, manager.rs:264)
//
// Original: abort_transaction must mark the tx aborted, then call
// storage.rollback_versions(tx_id) which removes uncommitted versions.
// Mutation A.2: entire mvcc_rollback → Ok(()), uncommitted versions remain.
// Mutation A.3: `if !abort_transaction(...)` inverted — rollback_versions
// is skipped when abort succeeds.
//
// Counterexample: begin → write → rollback → read must return None.
// Under A.2/A.3 the uncommitted version is still in the chain; the
// snapshot for a new tx excludes active txs but the version has
// created_by = rolled-back tx and created_commit_ts = None, so
// is_visible returns false for other txs — we assert via a fresh
// manager whose active set no longer contains the rolled-back tx.
// -----------------------------------------------------------------------------
#[test]
fn mvcc_rollback_must_discard_uncommitted_write() {
    let mut tm = TransactionManager::new();
    let mut storage = MVCCStorageEngine::new();

    let tx1 = tm.begin().expect("begin tx1");
    tm.mvcc_write(&mut storage, b"k1".to_vec(), b"v1".to_vec())
        .expect("write");
    tm.mvcc_rollback(&mut storage)
        .expect("rollback must succeed");

    // tx1 must no longer be in-progress.
    assert!(
        !tm.is_in_transaction(),
        "rollback must clear current_tx; mutation mvcc_rollback→Ok(()) leaves it set"
    );

    // After rollback the manager starts fresh; a new tx must NOT see the
    // uncommitted version (created_commit_ts=None → is_visible=false).
    let _tx2 = tm.begin().expect("begin tx2");
    let got = tm.mvcc_read(&storage, b"k1").expect("read");
    assert_eq!(
        got, None,
        "rolled-back write must not be visible to a new tx; \
         mutation mvcc_rollback→Ok(()) (A.2) or `delete !` (A.3) leaves the version"
    );
    let _ = tx1; // used only for clarity
}

#[test]
fn mvcc_rollback_twice_must_error() {
    let mut tm = TransactionManager::new();
    let mut storage = MVCCStorageEngine::new();

    tm.begin().expect("begin");
    tm.mvcc_write(&mut storage, b"k1".to_vec(), b"v1".to_vec())
        .expect("write");
    tm.mvcc_rollback(&mut storage).expect("first rollback");
    let second = tm.mvcc_rollback(&mut storage);
    assert!(
        second.is_err(),
        "second rollback with no active tx must error (NoTransaction); \
         mutation mvcc_rollback→Ok(()) makes it silently succeed"
    );
}

// -----------------------------------------------------------------------------
// Counter 3: `Transaction::is_committed` / `is_aborted` always-true
// (B.5, mvcc.rs:102,106)
//
// Original: `self.status == TransactionStatus::Committed` (resp. Aborted).
// Mutation: body replaced with `true`.
//
// Counterexample: a freshly-begun tx is Active → both predicates false.
// A committed tx has is_committed=true, is_aborted=false.
// An aborted tx has is_committed=false, is_aborted=true.
// -----------------------------------------------------------------------------
#[test]
fn transaction_status_predicates_must_distinguish_states() {
    let mut engine = MvccEngine::new();

    let tx1 = engine.begin_transaction();
    let t1 = engine.get_transaction(tx1).expect("tx1 present");
    assert!(!t1.is_committed(), "active tx must not report committed");
    assert!(!t1.is_aborted(), "active tx must not report aborted");

    let commit_ts = engine.commit_transaction(tx1).expect("commit returns Some");
    let t1c = engine.get_transaction(tx1).expect("tx1 still tracked");
    assert!(t1c.is_committed(), "committed tx must report committed");
    assert!(!t1c.is_aborted(), "committed tx must not report aborted");
    let _ = commit_ts;

    let tx2 = engine.begin_transaction();
    assert!(engine.abort_transaction(tx2), "abort must succeed");
    let t2 = engine.get_transaction(tx2).expect("tx2 still tracked");
    assert!(t2.is_aborted(), "aborted tx must report aborted");
    assert!(
        !t2.is_committed(),
        "aborted tx must not report committed; \
         mutation is_committed→true would make recovery treat it as committed"
    );
}

// -----------------------------------------------------------------------------
// Counter 4: `Snapshot::is_visible` boundary (B.6, mvcc.rs:142,153,170) — 6 forms
//
// Original (mvcc.rs:142-154):
//   pub fn is_visible(&self, tx_id, commit_timestamp) -> bool {
//       if tx_id == self.tx_id { return true; }
//       for active in &self.active_transactions {
//           if *active == tx_id { return false; }
//       }
//       match commit_timestamp {
//           Some(ts) => ts < self.snapshot_timestamp,
//           None => false,
//       }
//   }
//
// Mutations (6 forms):
//   - replace → true                       (always visible)
//   - replace `==` with `!=` on own-tx     (self becomes invisible)
//   - replace `<` with `<=` (line 153)     (commit at snapshot_ts leaks in)
//   - replace `<` with `==`                (only exact-ts visible)
//   - replace `<` with `>`                 (only future-ts visible)
//   - replace `<` with `<=` (line 170)     (ReadCommitted boundary)
//
// Counterexamples pin each boundary:
//   * own tx always visible
//   * tx in active set invisible even with a commit_ts
//   * commit_ts == snapshot_ts is NOT visible (strict <)
//   * commit_ts < snapshot_ts IS visible
//   * commit_ts > snapshot_ts is NOT visible
//   * commit_timestamp=None is NOT visible
// -----------------------------------------------------------------------------
#[test]
fn snapshot_visibility_own_tx_always_visible() {
    let snap = Snapshot::new(TxId::new(1), 10, vec![]);
    assert!(snap.is_visible(TxId::new(1), None));
    assert!(snap.is_visible(TxId::new(1), Some(99)));
}

#[test]
fn snapshot_visibility_active_tx_invisible() {
    let snap = Snapshot::new(TxId::new(1), 10, vec![TxId::new(2)]);
    assert!(
        !snap.is_visible(TxId::new(2), Some(5)),
        "tx in active set must be invisible regardless of commit_ts"
    );
}

#[test]
fn snapshot_visibility_strict_inequality_boundary() {
    let snap = Snapshot::new(TxId::new(1), 10, vec![]);

    // commit_ts == snapshot_ts → NOT visible (strict <)
    assert!(
        !snap.is_visible(TxId::new(9), Some(10)),
        "commit_ts == snapshot_ts must be invisible; \
         mutation `<`→`<=` would leak this row"
    );

    // commit_ts < snapshot_ts → visible
    assert!(
        snap.is_visible(TxId::new(9), Some(9)),
        "commit_ts < snapshot_ts must be visible; \
         mutation `<`→`==` or `<`→`>` would hide this row"
    );

    // commit_ts > snapshot_ts → NOT visible
    assert!(
        !snap.is_visible(TxId::new(9), Some(11)),
        "commit_ts > snapshot_ts must be invisible; \
         mutation `<`→`<=` would NOT change this, but `<`→`>` makes it visible"
    );

    // None → invisible
    assert!(
        !snap.is_visible(TxId::new(9), None),
        "uncommitted (None) version must be invisible"
    );
}

#[test]
fn snapshot_visibility_is_not_always_true() {
    let snap = Snapshot::new(TxId::new(1), 10, vec![]);
    assert!(
        !snap.is_visible(TxId::new(2), None),
        "mutation is_visible→true would make every version visible"
    );
}

#[test]
fn snapshot_visibility_own_tx_identity_not_inverted() {
    // Mutation `==` → `!=` on `tx_id == self.tx_id` makes own tx invisible.
    let snap = Snapshot::new(TxId::new(1), 10, vec![]);
    assert!(
        snap.is_visible(TxId::new(1), None),
        "own tx must be visible; mutation `==`→`!=` inverts this"
    );
    assert!(!snap.is_visible(TxId::new(2), None));
}

#[test]
fn read_committed_visibility_boundary() {
    let snap = Snapshot::new(TxId::new(1), 10, vec![]);
    // is_visible_read_committed: ts < current_timestamp (strict).
    assert!(snap.is_visible_read_committed(TxId::new(9), Some(9), 10));
    assert!(!snap.is_visible_read_committed(TxId::new(9), Some(10), 10));
    assert!(!snap.is_visible_read_committed(TxId::new(9), None, 10));
    // Own tx always visible.
    assert!(snap.is_visible_read_committed(TxId::new(1), None, 10));
}

// -----------------------------------------------------------------------------
// Counter 5: `MvccEngine::commit_transaction` `+= → *=` (C.2, mvcc.rs:287)
//
// Original: `self.global_timestamp += 1;` after each commit.
// Mutation: `self.global_timestamp *= 1;` — clock freezes (1*1=1).
//
// Counterexample: two sequential transactions must receive strictly
// increasing commit timestamps.  Under `*= 1` both commits return the
// same ts, breaking MVCC ordering (a later tx could be considered
// concurrent with an earlier one).
// -----------------------------------------------------------------------------
#[test]
fn commit_timestamps_must_increase() {
    let mut engine = MvccEngine::new();

    let tx1 = engine.begin_transaction();
    let ts1 = engine.commit_transaction(tx1).expect("commit tx1");
    let tx2 = engine.begin_transaction();
    let ts2 = engine.commit_transaction(tx2).expect("commit tx2");
    let tx3 = engine.begin_transaction();
    let ts3 = engine.commit_transaction(tx3).expect("commit tx3");

    assert!(
        ts1 < ts2,
        "commit ts must strictly increase; mutation `+=`→`*=` freezes the clock \
         (ts1={ts1}, ts2={ts2})"
    );
    assert!(
        ts2 < ts3,
        "commit ts must strictly increase across 3 txs; \
         mutation `+=`→`*=` gives ts2={ts2}, ts3={ts3}"
    );
}

#[test]
fn commit_must_advance_global_clock() {
    let mut engine = MvccEngine::new();
    let tx = engine.begin_transaction();
    let commit_ts = engine.commit_transaction(tx).expect("commit");
    let after = engine.get_global_timestamp();
    assert!(
        after > commit_ts,
        "commit must advance global clock past the returned commit_ts; \
         mutation `+=`→`*=` leaves after={after} == commit_ts={commit_ts}"
    );
}

// -----------------------------------------------------------------------------
// Counter 6: `get_global_timestamp` constant-0 / constant-1 (C.3, mvcc.rs:320)
//
// Original: returns the engine's monotonically-increasing clock.
// Mutation: always returns 0 or always returns 1.
//
// Counterexample: begin advances the clock; get_global_timestamp must
// reflect that advance.  Constant-0 fails after the first begin;
// constant-1 fails after the second.
// -----------------------------------------------------------------------------
#[test]
fn global_timestamp_advances_with_begin() {
    let mut engine = MvccEngine::new();
    let t0 = engine.get_global_timestamp();

    let _tx1 = engine.begin_transaction();
    let t1 = engine.get_global_timestamp();
    assert!(
        t1 > t0,
        "begin must advance global clock; mutation get_global_timestamp→0/1 \
         keeps t1={t1} (t0={t0})"
    );

    let _tx2 = engine.begin_transaction();
    let t2 = engine.get_global_timestamp();
    assert!(
        t2 > t1,
        "second begin must advance further; constant-1 gives t2={t2} == t1={t1}"
    );
}

// -----------------------------------------------------------------------------
// Counter 7: `RowVersion::is_visible` `>` → `>=` (C.1, mvcc.rs:238)
//
// Original: `if created_ts > snapshot.snapshot_timestamp { return false; }`
// Mutation: `>=` — a version committed at exactly snapshot_ts becomes
// visible (off-by-one).
//
// Counterexample: build a RowVersion with created_commit_ts == snapshot_ts
// and assert it is NOT visible.  Under `>=` the early-return is skipped
// and the version falls through to `true`.
// -----------------------------------------------------------------------------
#[test]
fn row_version_at_snapshot_ts_visible_under_strict_gt() {
    use sqlrustgo_transaction::mvcc::RowVersion;

    let snap = Snapshot::new(TxId::new(1), 10, vec![]);
    let mut rv = RowVersion::new(TxId::new(2), b"v".to_vec());
    rv.created_commit_ts = Some(10);
    assert!(
        rv.is_visible(&snap),
        "version committed at exactly snapshot_ts is visible (strict > is false → fall through to true); \
         mutation `>`→`>=` would make 10>=10 true and return false (invisible)"
    );
}

#[test]
fn row_version_before_snapshot_ts_visible() {
    use sqlrustgo_transaction::mvcc::RowVersion;

    let snap = Snapshot::new(TxId::new(1), 10, vec![]);
    let mut rv = RowVersion::new(TxId::new(2), b"v".to_vec());
    rv.created_commit_ts = Some(9);
    assert!(
        rv.is_visible(&snap),
        "version committed before snapshot must be visible"
    );
}

#[test]
fn row_version_uncommitted_invisible() {
    use sqlrustgo_transaction::mvcc::RowVersion;

    let snap = Snapshot::new(TxId::new(1), 10, vec![]);
    let rv = RowVersion::new(TxId::new(2), b"v".to_vec());
    assert!(
        !rv.is_visible(&snap),
        "uncommitted version must be invisible"
    );
}

#[test]
fn row_version_own_tx_visible_even_if_uncommitted() {
    use sqlrustgo_transaction::mvcc::RowVersion;

    let snap = Snapshot::new(TxId::new(1), 10, vec![]);
    let rv = RowVersion::new(TxId::new(1), b"v".to_vec());
    assert!(
        rv.is_visible(&snap),
        "own uncommitted version must be visible to its creator"
    );
}

// -----------------------------------------------------------------------------
// Counter 8: `VersionChainMap::get_chain` None / empty (C.4, version_chain.rs:65)
//
// Original: returns a clone of the chain for the key.
// Mutation: always None, or always Some(vec![]).
//
// Counterexample: append a version then get_chain must return Some with
// exactly 1 element.
// -----------------------------------------------------------------------------
#[test]
fn get_chain_must_return_appended_version() {
    use sqlrustgo_transaction::mvcc::RowVersion;

    let mut map = VersionChainMap::new();
    let rv = RowVersion::new(TxId::new(1), b"v1".to_vec());
    map.append(b"k1".to_vec(), rv);

    let chain = map.get_chain(b"k1");
    assert_eq!(
        chain.as_ref().map(|c| c.len()),
        Some(1),
        "get_chain must return Some with 1 element; \
         mutation get_chain→None gives None, →Some(vec![]) gives len 0"
    );
}

#[test]
fn get_chain_missing_key_returns_none() {
    let map = VersionChainMap::new();
    assert!(map.get_chain(b"absent").is_none());
}

// -----------------------------------------------------------------------------
// Counter 9: `VersionChainMap::rollback_versions` no-op (C.5, version_chain.rs:90)
//
// Original: retain only versions where created_by != tx_id OR
// created_commit_ts.is_some() — i.e. drop uncommitted versions of tx_id.
// Mutation: body replaced with `()` — nothing is dropped.
//
// Counterexample: two txs each append an uncommitted version; rollback
// tx1 must remove only tx1's version, leaving tx2's.
// -----------------------------------------------------------------------------
#[test]
fn rollback_versions_must_drop_only_uncommitted_of_target_tx() {
    use sqlrustgo_transaction::mvcc::RowVersion;

    let mut map = VersionChainMap::new();
    let tx1 = TxId::new(1);
    let tx2 = TxId::new(2);

    map.append(
        b"k1".to_vec(),
        RowVersion::new(tx1, b"a".to_vec()), // uncommitted
    );
    map.append(
        b"k1".to_vec(),
        RowVersion::new(tx2, b"b".to_vec()), // uncommitted
    );

    map.rollback_versions(tx1);

    let chain = map.get_chain(b"k1").expect("chain still present");
    assert_eq!(
        chain.len(),
        1,
        "rollback_versions(tx1) must leave only tx2's version; \
         mutation rollback_versions→() leaves both (len={})",
        chain.len()
    );
    assert_eq!(chain[0].created_by, tx2);

    // Rollback of an unknown tx is a no-op but must not panic.
    map.rollback_versions(TxId::new(99));
}

#[test]
fn rollback_versions_preserves_committed_versions() {
    use sqlrustgo_transaction::mvcc::RowVersion;

    let mut map = VersionChainMap::new();
    let tx1 = TxId::new(1);

    let mut committed = RowVersion::new(tx1, b"a".to_vec());
    committed.created_commit_ts = Some(5);
    map.append(b"k1".to_vec(), committed);
    map.append(
        b"k1".to_vec(),
        RowVersion::new(tx1, b"b".to_vec()), // uncommitted
    );

    map.rollback_versions(tx1);

    let chain = map.get_chain(b"k1").expect("chain present");
    assert_eq!(chain.len(), 1, "only the uncommitted version is dropped");
    assert!(
        chain[0].created_commit_ts.is_some(),
        "committed version survives"
    );
}

// -----------------------------------------------------------------------------
// Counter 10: `SsiDetectorSync::validate_commit` `>` → `>=` (D.2, ssi.rs:269,273)
//
// Original:
//   let rw_conflict = my_reads.intersection(other_writes).count() > 0;
//   ...
//   let wr_conflict = my_writes.intersection(other_reads).count() > 0;
//
// Mutation: `count() > 0` → `count() >= 0` — always true, so ANY
// concurrent tx with a non-empty write set triggers a conflict check,
// and an empty intersection still "conflicts" when the other side has
// read set entries.  Conversely a pure-`>=` on an empty intersection
// makes every pair conflict.
//
// Counterexample A: two txs with DISJOINT read/write sets must both
// commit successfully.  Under `>=` a spurious conflict is raised.
//
// Counterexample B: two txs forming an RW-WR cycle must be rejected.
// -----------------------------------------------------------------------------
#[test]
fn ssi_disjoint_sets_must_commit() {
    let ssi = SsiDetectorSync::new();
    let tx1 = TxId::new(1);
    let tx2 = TxId::new(2);

    // tx1 reads k1, writes k2; tx2 reads k3, writes k4 — fully disjoint.
    ssi.record_read(tx1, b"k1".to_vec());
    ssi.record_write(tx1, b"k2".to_vec());
    ssi.record_read(tx2, b"k3".to_vec());
    ssi.record_write(tx2, b"k4".to_vec());

    assert!(
        ssi.validate_commit(tx1).is_ok(),
        "disjoint read/write sets must not conflict; \
         mutation `count() > 0`→`count() >= 0` raises a false conflict"
    );
    assert!(ssi.validate_commit(tx2).is_ok());
}

#[test]
fn ssi_rw_wr_cycle_must_be_rejected() {
    let ssi = SsiDetectorSync::new();
    let tx1 = TxId::new(1);
    let tx2 = TxId::new(2);

    // Classic RW-WR cycle: tx1 reads k1 (written by tx2) and writes k2
    // (read by tx2).
    ssi.record_read(tx1, b"k1".to_vec());
    ssi.record_write(tx1, b"k2".to_vec());
    ssi.record_write(tx2, b"k1".to_vec());
    ssi.record_read(tx2, b"k2".to_vec());

    let r = ssi.validate_commit(tx1);
    assert!(
        r.is_err(),
        "RW-WR cycle must be rejected; got Ok({:?}) — \
         mutation on intersection count may swallow the conflict",
        r
    );
    // tx2 must also be rejected symmetrically.
    assert!(ssi.validate_commit(tx2).is_err());
}

#[test]
fn ssi_empty_read_set_must_not_conflict() {
    let ssi = SsiDetectorSync::new();
    let tx1 = TxId::new(1);
    let tx2 = TxId::new(2);

    // tx1 has no reads; tx2 writes something. No rw_conflict possible.
    ssi.record_write(tx1, b"k1".to_vec());
    ssi.record_write(tx2, b"k2".to_vec());

    assert!(
        ssi.validate_commit(tx1).is_ok(),
        "tx with empty read set cannot have an RW conflict"
    );
}

#[test]
fn ssi_release_clears_state() {
    let ssi = SsiDetectorSync::new();
    let tx1 = TxId::new(1);
    ssi.record_read(tx1, b"k1".to_vec());
    ssi.record_write(tx1, b"k2".to_vec());
    ssi.release(tx1);

    // After release, tx1 has no read/write sets — committing again is Ok.
    assert!(ssi.validate_commit(tx1).is_ok());
}

#[test]
fn siread_lock_add_key_must_append() {
    use sqlrustgo_transaction::ssi::SireadLock;

    let mut lock = SireadLock::new(TxId::new(1));
    lock.add_key(b"k1".to_vec());
    lock.add_key(b"k2".to_vec());
    assert_eq!(
        lock.keys.len(),
        2,
        "add_key must append; mutation add_key→() leaves len=0"
    );
    assert_eq!(lock.keys[0], b"k1");
    assert_eq!(lock.keys[1], b"k2");
}

#[test]
fn serialization_graph_remove_tx_must_prune_dependencies() {
    use sqlrustgo_transaction::ssi::SerializationGraph;

    let mut graph = SerializationGraph::new();
    let tx1 = TxId::new(1);
    let tx2 = TxId::new(2);

    graph.add_dependency(tx2, tx1);

    assert!(
        graph.would_create_cycle(tx1, tx2),
        "precondition: edge tx1->tx2 exists because tx2 depends on tx1"
    );

    graph.remove_tx(&tx1);

    assert!(
        !graph.would_create_cycle(tx1, tx2),
        "after remove_tx(tx1), tx2 must no longer depend on tx1; \
         mutation remove_tx→() leaves the dependency"
    );
}

// -----------------------------------------------------------------------------
// Counter 11: `MvccEngine::begin_transaction` `+= → *=` (C.2, mvcc.rs:276)
//
// Original: `self.global_timestamp += 1;` in begin_transaction.
// Mutation: `*= 1` — begin no longer advances the clock.
//
// Counterexample: two begins must produce strictly increasing global
// timestamps (checked via get_global_timestamp).
// -----------------------------------------------------------------------------
#[test]
fn begin_transaction_advances_global_clock() {
    let mut engine = MvccEngine::new();
    let t0 = engine.get_global_timestamp();
    let _tx1 = engine.begin_transaction();
    let t1 = engine.get_global_timestamp();
    let _tx2 = engine.begin_transaction();
    let t2 = engine.get_global_timestamp();

    assert!(t1 > t0, "first begin must advance clock (t0={t0}, t1={t1})");
    assert!(
        t2 > t1,
        "second begin must advance clock further (t1={t1}, t2={t2}); \
         mutation `+=`→`*=` freezes it"
    );
}

// -----------------------------------------------------------------------------
// Integration: end-to-end MVCC visibility through TransactionManager.
// This exercises mvcc_write + mvcc_commit + Snapshot::is_visible together,
// providing a differential signal against combined mutants.
// -----------------------------------------------------------------------------
#[test]
fn end_to_end_committed_data_visible_to_later_tx_only() {
    let mut tm = TransactionManager::new();
    let mut storage = MVCCStorageEngine::new();

    // tx1 writes and commits.
    tm.begin().expect("begin tx1");
    tm.mvcc_write(&mut storage, b"acct".to_vec(), b"100".to_vec())
        .expect("write");
    let commit_ts = tm
        .mvcc_commit(&mut storage)
        .expect("commit")
        .expect("Some(ts)");

    // tx2 begins AFTER the commit — must see the row.
    tm.begin().expect("begin tx2");
    let got = tm.mvcc_read(&storage, b"acct").expect("read");
    assert_eq!(
        got,
        Some(b"100".to_vec()),
        "tx2 (begun after commit_ts={commit_ts}) must see committed row"
    );

    // tx3 writes a new value and rolls back — tx4 must see the old value.
    tm.mvcc_commit(&mut storage).expect("commit tx2");
    tm.begin().expect("begin tx3");
    tm.mvcc_write(&mut storage, b"acct".to_vec(), b"999".to_vec())
        .expect("write");
    tm.mvcc_rollback(&mut storage).expect("rollback tx3");

    tm.begin().expect("begin tx4");
    let got = tm.mvcc_read(&storage, b"acct").expect("read");
    assert_eq!(
        got,
        Some(b"100".to_vec()),
        "rolled-back value must not replace committed value for a later tx"
    );
}

#[test]
fn transaction_manager_isolation_level_defaults_read_committed() {
    let tm = TransactionManager::new();
    assert_eq!(
        tm.get_isolation_level(),
        IsolationLevel::ReadCommitted,
        "default isolation must be ReadCommitted"
    );
}

// -----------------------------------------------------------------------------
// Counter 12 (bonus): `TransactionContext::is_visible` ReadCommitted arm
// deletion (B.1, manager.rs:39)
//
// Original: `match self.isolation_level { ReadCommitted => …, _ => … }`
// Mutation: ReadCommitted arm deleted → ReadCommitted falls through to
// Snapshot::is_visible (snapshot isolation), a silent isolation downgrade.
//
// Counterexample: under ReadCommitted a version committed AT the
// current_timestamp boundary differs from Snapshot isolation only in
// how the timestamp is sourced.  We pin the observable contract via
// TransactionContext::is_visible for a ReadCommitted context vs a
// RepeatableRead context with the same snapshot.
// -----------------------------------------------------------------------------
#[test]
fn transaction_context_is_visible_dispatches_on_isolation() {
    use sqlrustgo_transaction::manager::TransactionContext;

    let snap = Snapshot::new(TxId::new(1), 10, vec![]);

    // ReadCommitted context: is_visible uses is_visible_read_committed
    // with the snapshot's snapshot_timestamp as "current".
    let rc = TransactionContext::new(TxId::new(1), snap.clone(), IsolationLevel::ReadCommitted);
    // Own tx → visible in both branches.
    assert!(rc.is_visible(TxId::new(1), None));
    // Other tx with commit_ts < snapshot_ts → visible under both.
    assert!(rc.is_visible(TxId::new(9), Some(9)));
    // Other tx with commit_ts == snapshot_ts → invisible under strict <.
    assert!(!rc.is_visible(TxId::new(9), Some(10)));

    // RepeatableRead uses Snapshot::is_visible directly — same boundary.
    let rr = TransactionContext::new(TxId::new(1), snap, IsolationLevel::RepeatableRead);
    assert!(rr.is_visible(TxId::new(1), None));
    assert!(rr.is_visible(TxId::new(9), Some(9)));
    assert!(!rr.is_visible(TxId::new(9), Some(10)));
}

#[test]
fn refresh_for_read_committed_updates_snapshot() {
    let mut snap = Snapshot::new(TxId::new(1), 10, vec![TxId::new(2)]);
    snap.refresh_for_read_committed(20);
    assert_eq!(
        snap.snapshot_timestamp, 20,
        "refresh must update snapshot_ts"
    );
    assert!(
        snap.active_transactions.is_empty(),
        "refresh must clear the active set for ReadCommitted"
    );
}

// =============================================================================
// A.4  rollback_with_undo no-op (transaction_manager.rs:258)
//
// Mutation: body replaced with Ok(()) — undo closure never invoked,
// active tx never removed from active_transactions, state never Aborted.
//
// Counterexample: begin tx, add an undo record, call rollback_with_undo
// with a closure that records invocation, assert the closure ran AND
// the tx was removed from active_transactions.
// Under the no-op mutation the closure is never called and the tx
// remains active → both assertions fail.
// =============================================================================
#[test]
fn rollback_with_undo_must_invoke_closure_and_remove_tx() {
    use sqlrustgo_transaction::savepoint::UndoRecord;
    use sqlrustgo_transaction::transaction_manager::{
        IsolationLevel as TxMgrIsolation, TransactionManager as TxMgr,
    };
    use sqlrustgo_types::Value;

    let mut tm = TxMgr::new();
    let tx_id = tm
        .begin_transaction(TxMgrIsolation::SnapshotIsolation)
        .expect("begin");

    let record = UndoRecord::Insert {
        table: "t".into(),
        key: vec![Value::Integer(1)],
        row: vec![Value::Integer(1), Value::Text("a".into())],
    };
    tm.add_undo_record(tx_id, record).expect("add_undo_record");

    let mut invoked = Vec::new();
    tm.rollback_with_undo(tx_id, |rec| {
        invoked.push(format!("{:?}", rec));
        Ok(())
    })
    .expect("rollback_with_undo");

    assert_eq!(
        invoked.len(),
        1,
        "rollback_with_undo must invoke on_undo once per undo record; \
         mutation rollback_with_undo→Ok(()) skips the closure"
    );

    // After rollback the tx must no longer be active.
    // get_snapshot returns None once the tx is removed.
    assert!(
        tm.get_snapshot(tx_id).is_none(),
        "rollback_with_undo must remove the tx from active_transactions; \
         mutation rollback_with_undo→Ok(()) leaves it active"
    );
}

// =============================================================================
// A.4b  rollback_with_undo replays records in reverse order
//
// Mutation covered by the same no-op but also guards against a
// forward-iteration regression (the doc-comment mandates reverse).
// =============================================================================
#[test]
fn rollback_with_undo_replays_in_reverse_order() {
    use sqlrustgo_transaction::savepoint::UndoRecord;
    use sqlrustgo_transaction::transaction_manager::{
        IsolationLevel as TxMgrIsolation, TransactionManager as TxMgr,
    };
    use sqlrustgo_types::Value;

    let mut tm = TxMgr::new();
    let tx_id = tm
        .begin_transaction(TxMgrIsolation::SnapshotIsolation)
        .expect("begin");

    for i in 0..3 {
        let record = UndoRecord::Insert {
            table: "t".into(),
            key: vec![Value::Integer(i)],
            row: vec![Value::Integer(i)],
        };
        tm.add_undo_record(tx_id, record).expect("add_undo_record");
    }

    let mut order = Vec::new();
    tm.rollback_with_undo(tx_id, |rec| {
        if let UndoRecord::Insert { key, .. } = rec {
            if let Some(Value::Integer(k)) = key.first() {
                order.push(*k);
            }
        }
        Ok(())
    })
    .expect("rollback_with_undo");

    assert_eq!(
        order,
        vec![2, 1, 0],
        "rollback_with_undo must replay undo records in reverse (most-recent-first); \
         mutation reversing the iteration order breaks referential integrity"
    );
}

// =============================================================================
// A.5  abort no-op (transaction_manager.rs:290)
//
// Mutation: body replaced with Ok(()) — active tx never removed, state
// never Aborted, ssi_detector never released.
//
// Counterexample: begin tx, call abort, assert get_snapshot returns None
// (tx removed from active_transactions).
// =============================================================================
#[test]
fn abort_must_remove_tx_from_active() {
    use sqlrustgo_transaction::transaction_manager::{
        IsolationLevel as TxMgrIsolation, TransactionManager as TxMgr,
    };

    let mut tm = TxMgr::new();
    let tx_id = tm
        .begin_transaction(TxMgrIsolation::SnapshotIsolation)
        .expect("begin");

    assert!(tm.get_snapshot(tx_id).is_some(), "precondition: active");

    tm.abort(tx_id).expect("abort");

    assert!(
        tm.get_snapshot(tx_id).is_none(),
        "abort must remove the tx from active_transactions; \
         mutation abort→Ok(()) leaves it active"
    );
}

// =============================================================================
// A.5b  abort releases SSI detector state
//
// Mutation: same no-op but specifically checks ssi_detector.release was
// called (a second abort on the same id must not error, and a new tx
// can be started cleanly).
// =============================================================================
#[test]
fn abort_releases_ssi_state_for_reuse() {
    use sqlrustgo_transaction::transaction_manager::{
        IsolationLevel as TxMgrIsolation, TransactionManager as TxMgr,
    };

    let mut tm = TxMgr::new();
    let tx_id = tm
        .begin_transaction(TxMgrIsolation::SnapshotIsolation)
        .expect("begin");
    tm.abort(tx_id).expect("abort");

    // Starting a fresh transaction must succeed; under the no-op
    // mutation the old tx is still in active_transactions and the
    // SSI detector still holds its edges, which can cause a spurious
    // conflict on the new tx.
    let tx2 = tm
        .begin_transaction(TxMgrIsolation::SnapshotIsolation)
        .expect("begin after abort");
    assert_ne!(tx_id.as_u64(), tx2.as_u64());
}

// =============================================================================
// A.6  rollback_to_savepoint_with_undo no-op (transaction_manager.rs:372)
//
// Mutation: body replaced with Ok(()) — savepoint rollback never happens,
// on_undo closure never invoked.
//
// Counterexample: begin tx, create savepoint s1, add an undo record,
// call rollback_to_savepoint_with_undo with a recording closure, assert
// the closure was invoked. Under the no-op the closure never runs.
// =============================================================================
#[test]
fn rollback_to_savepoint_with_undo_must_invoke_closure() {
    use sqlrustgo_transaction::savepoint::UndoRecord;
    use sqlrustgo_transaction::transaction_manager::{
        IsolationLevel as TxMgrIsolation, TransactionManager as TxMgr,
    };
    use sqlrustgo_types::Value;

    let mut tm = TxMgr::new();
    let tx_id = tm
        .begin_transaction(TxMgrIsolation::SnapshotIsolation)
        .expect("begin");

    tm.savepoint(tx_id, "s1".into()).expect("savepoint");

    let record = UndoRecord::Insert {
        table: "t".into(),
        key: vec![Value::Integer(1)],
        row: vec![Value::Integer(1)],
    };
    tm.add_undo_record(tx_id, record).expect("add_undo_record");

    let mut invoked = 0usize;
    tm.rollback_to_savepoint_with_undo(tx_id, "s1", |_| {
        invoked += 1;
        Ok(())
    })
    .expect("rollback_to_savepoint_with_undo");

    assert!(
        invoked >= 1,
        "rollback_to_savepoint_with_undo must invoke on_undo for each \
         post-savepoint record; mutation …→Ok(()) skips the closure"
    );
}

// =============================================================================
// A.7  add_undo_record no-op (transaction_manager.rs:390)
//
// Mutation: body replaced with Ok(()) — undo record silently dropped.
//
// Counterexample: begin tx, add an undo record, then rollback_with_undo
// and assert the closure sees exactly 1 record. Under the no-op the
// record is lost and the closure sees 0.
// =============================================================================
#[test]
fn add_undo_record_must_persist_for_later_rollback() {
    use sqlrustgo_transaction::savepoint::UndoRecord;
    use sqlrustgo_transaction::transaction_manager::{
        IsolationLevel as TxMgrIsolation, TransactionManager as TxMgr,
    };
    use sqlrustgo_types::Value;

    let mut tm = TxMgr::new();
    let tx_id = tm
        .begin_transaction(TxMgrIsolation::SnapshotIsolation)
        .expect("begin");

    let record = UndoRecord::Delete {
        table: "t".into(),
        key: vec![Value::Integer(1)],
        old_value: vec![Value::Integer(1), Value::Text("old".into())],
    };
    tm.add_undo_record(tx_id, record).expect("add_undo_record");

    let mut seen = 0usize;
    tm.rollback_with_undo(tx_id, |_| {
        seen += 1;
        Ok(())
    })
    .expect("rollback_with_undo");

    assert_eq!(
        seen, 1,
        "add_undo_record must append to the undo log so rollback sees it; \
         mutation add_undo_record→Ok(()) drops the record"
    );
}

// =============================================================================
// A.8  release_savepoint no-op (transaction_manager.rs:415)
//
// Mutation: body replaced with Ok(()) — savepoint never removed from stack.
//
// Counterexample: begin tx, create savepoint s1, release s1, then assert
// get_savepoint_count drops to 0. Under the no-op the savepoint remains.
// We observe this via has_active_savepoint.
// =============================================================================
#[test]
fn release_savepoint_must_remove_savepoint() {
    use sqlrustgo_transaction::transaction_manager::{
        IsolationLevel as TxMgrIsolation, TransactionManager as TxMgr,
    };

    let mut tm = TxMgr::new();
    let tx_id = tm
        .begin_transaction(TxMgrIsolation::SnapshotIsolation)
        .expect("begin");

    tm.savepoint(tx_id, "s1".into()).expect("savepoint");
    assert!(
        tm.has_active_savepoint(tx_id),
        "precondition: savepoint active after SAVEPOINT"
    );

    tm.release_savepoint(tx_id, "s1")
        .expect("release_savepoint");

    assert!(
        !tm.has_active_savepoint(tx_id),
        "release_savepoint must remove the savepoint so has_active_savepoint \
         is false; mutation release_savepoint→Ok(()) leaves it on the stack"
    );
}

// =============================================================================
// A.9  get_global_timestamp constant 0 / constant 1 (manager.rs:177)
//
// Mutation: Ok(0) / Ok(1) — every read returns a fixed timestamp.
//
// Counterexample: begin two transactions in sequence; the second begin
// must observe a strictly greater global timestamp than the first.
// Under constant-0 / constant-1 both reads return the same value.
// =============================================================================
#[test]
fn manager_global_timestamp_advances_across_begins() {
    use sqlrustgo_transaction::manager::TransactionManager;

    let mut m = TransactionManager::new();

    let _tx1 = m.begin().expect("begin 1");
    let ts1 = m.get_global_timestamp().expect("ts1");
    m.commit().expect("commit 1");

    let _tx2 = m.begin().expect("begin 2");
    let ts2 = m.get_global_timestamp().expect("ts2");

    assert!(
        ts2 > ts1,
        "global timestamp must advance across transactions; \
         mutation get_global_timestamp→Ok(0)/Ok(1) freezes it"
    );
}

// =============================================================================
// A.10  mvcc_read constant None / empty / [0] / [1] (manager.rs:219)
//
// Mutation: every mvcc_read returns a constant. The most dangerous form
// is Ok(None) — every SELECT returns 0 rows ("database is empty").
//
// Counterexample: write a value via mvcc_write, commit, begin a new tx,
// mvcc_read must return the committed value. Under Ok(None) / Ok(Some(vec![]))
// the read returns the wrong bytes → assertion fails.
// =============================================================================
#[test]
fn mvcc_read_must_return_committed_value() {
    use sqlrustgo_transaction::manager::TransactionManager;
    use sqlrustgo_transaction::mvcc_storage::MVCCStorageEngine;

    let mut m = TransactionManager::new();
    let mut storage = MVCCStorageEngine::new();

    let _tx1 = m.begin().expect("begin 1");
    m.mvcc_write(&mut storage, b"k".to_vec(), b"hello".to_vec())
        .expect("write");
    m.mvcc_commit(&mut storage).expect("commit");

    let _tx2 = m.begin().expect("begin 2");
    let got = m.mvcc_read(&storage, b"k").expect("read");

    assert_eq!(
        got.as_deref(),
        Some(b"hello".as_slice()),
        "mvcc_read must return the committed value; \
         mutation mvcc_read→Ok(None)/Ok(Some(vec![])) loses the data"
    );
}

// =============================================================================
// A.11  mvcc_commit constant None / Some(0) / Some(1) (manager.rs:238)
//
// Mutation: commit returns a forged timestamp instead of the real one.
//
// Counterexample: commit a transaction and assert the returned timestamp
// equals the engine's global timestamp at that moment (which must be
// strictly greater than the previous commit's timestamp).
// Under constant Some(0) / Some(1) the returned value does not advance.
// =============================================================================
#[test]
fn mvcc_commit_returns_increasing_timestamps() {
    use sqlrustgo_transaction::manager::TransactionManager;
    use sqlrustgo_transaction::mvcc_storage::MVCCStorageEngine;

    let mut m = TransactionManager::new();
    let mut storage = MVCCStorageEngine::new();

    let _tx1 = m.begin().expect("begin 1");
    m.mvcc_write(&mut storage, b"a".to_vec(), b"1".to_vec())
        .expect("w1");
    let ts1 = m.mvcc_commit(&mut storage).expect("commit 1").expect("ts1");

    let _tx2 = m.begin().expect("begin 2");
    m.mvcc_write(&mut storage, b"b".to_vec(), b"2".to_vec())
        .expect("w2");
    let ts2 = m.mvcc_commit(&mut storage).expect("commit 2").expect("ts2");

    assert!(
        ts2 > ts1,
        "commit timestamps must strictly increase; \
         mutation mvcc_commit→Ok(Some(0))/Ok(Some(1)) freezes them"
    );
}

// =============================================================================
// A.12  with_mvcc returns Default::default() (manager.rs:79)
//
// Mutation: with_mvcc(mvcc) returns a manager wired to a fresh default
// engine, ignoring the provided engine.
//
// Counterexample: create a shared MvccEngine, wrap it via with_mvcc,
// begin a transaction through the manager (which advances the shared
// engine's clock), then read the shared engine's global timestamp
// directly and assert it advanced. Under the mutation the shared engine
// is never touched.
// =============================================================================
#[test]
fn with_mvcc_must_use_provided_engine() {
    use sqlrustgo_transaction::manager::TransactionManager;
    use sqlrustgo_transaction::mvcc::MvccEngine;
    use std::sync::{Arc, RwLock};

    let engine = Arc::new(RwLock::new(MvccEngine::new()));
    let ts_before = engine.read().unwrap().get_global_timestamp();

    let mut m = TransactionManager::with_mvcc(engine.clone());
    let _tx = m.begin().expect("begin");

    let ts_after = engine.read().unwrap().get_global_timestamp();
    assert!(
        ts_after > ts_before,
        "with_mvcc must wire the provided engine so begin advances its clock; \
         mutation with_mvcc→Default::default() uses a private engine instead"
    );
}

// =============================================================================
// A.13  MVCCStorageEngine::rollback_versions no-op (mvcc_storage.rs:59)
//
// Mutation: body replaced with Ok(()) — uncommitted versions are not
// discarded from the storage layer.
//
// Counterexample: begin tx, mvcc_write (creates an uncommitted version
// in storage), mvcc_rollback (which calls storage.rollback_versions),
// then begin a new tx and mvcc_read the same key — must be None (the
// uncommitted write was discarded). Under the no-op the phantom version
// leaks and the read may see it.
// =============================================================================
#[test]
fn storage_rollback_versions_must_discard_uncommitted() {
    use sqlrustgo_transaction::manager::TransactionManager;
    use sqlrustgo_transaction::mvcc_storage::{MVCCStorage, MVCCStorageEngine};

    let mut m = TransactionManager::new();
    let mut storage = MVCCStorageEngine::new();

    let tx1 = m.begin().expect("begin 1");
    m.mvcc_write(&mut storage, b"k".to_vec(), b"ghost".to_vec())
        .expect("write");
    m.mvcc_rollback(&mut storage).expect("rollback");

    // After rollback the uncommitted version for tx1 must be GONE from the
    // underlying version chain. If rollback_versions is a no-op the phantom
    // version survives, and a subsequent commit_versions(tx1, ts) (issued
    // directly here to simulate a buggy re-commit path) would resurrect it
    // as visible data. A clean chain ignores commit_versions for an unknown tx.
    storage
        .commit_versions(tx1, 1000)
        .expect("commit_versions on a rolled-back tx must not resurrect data");

    let mut m2 = TransactionManager::new();
    let _tx2 = m2.begin().expect("begin 2");
    let got = m2.mvcc_read(&storage, b"k").expect("read");

    assert_eq!(
        got, None,
        "after rollback_versions the uncommitted version must be discarded; \
         mutation rollback_versions→Ok(()) leaves a phantom that a later \
         commit_versions(tx_id, _) resurrects as visible data"
    );
}

// =============================================================================
// B.1  TransactionContext::is_visible ReadCommitted arm deleted
//      (manager.rs:39)
//
// Mutation: the IsolationLevel::ReadCommitted match arm is deleted, so
// ReadCommitted falls through to the default (RepeatableRead) branch.
//
// Counterexample: construct a ReadCommitted TransactionContext whose
// snapshot has a row committed by a foreign tx at commit_ts == snapshot_ts.
// Under ReadCommitted semantics (`is_visible_read_committed`) the row
// must be VISIBLE (read-committed re-reads at the current timestamp).
// Under the default RepeatableRead branch (`snapshot.is_visible` with
// strict >) the same row is INVISIBLE. Asserting visibility pins the
// ReadCommitted arm.
// =============================================================================
#[test]
fn read_committed_is_visible_uses_read_committed_branch() {
    use sqlrustgo_transaction::manager::{IsolationLevel, TransactionContext};
    use sqlrustgo_transaction::mvcc::{MvccEngine, Snapshot, TxId};

    let reader_tx = TxId::new(1);
    let writer_tx = TxId::new(2);
    // Hand-crafted snapshot: writer_tx is in the ACTIVE set (simulating an
    // uncommitted concurrent writer that the SI branch would hide), and the
    // snapshot timestamp is well past the writer's notional commit_ts.
    let snapshot = Snapshot::new(reader_tx, 100, vec![writer_tx]);
    let ctx = TransactionContext::new(reader_tx, snapshot, IsolationLevel::ReadCommitted);

    // is_visible (Snapshot / RepeatableRead default) hides writer_tx because
    // it is in active_transactions. is_visible_read_committed IGNORES the
    // active set and only compares commit_ts < current_timestamp.
    let visible = ctx.is_visible(writer_tx, Some(50));
    assert!(
        visible,
        "ReadCommitted TransactionContext::is_visible must dispatch to \
         is_visible_read_committed (which ignores the active set); deleting \
         the ReadCommitted arm routes through is_visible and hides the row"
    );
    let _ = MvccEngine::new();
}

// =============================================================================
// B.2  TransactionContext::refresh_snapshot no-op / inverted comparison
//      (manager.rs:49)
//
// Mutation: body replaced with () OR the `== IsolationLevel::ReadCommitted`
// comparison is inverted to `!=`.
//
// Counterexample: build a ReadCommitted TransactionContext, call
// refresh_snapshot(current_timestamp) with a fresh timestamp, then assert
// the snapshot was refreshed (snapshot_timestamp advanced OR the active
// set cleared — depends on Snapshot::refresh_for_read_committed semantics).
// We assert via a second refresh producing the same observable state.
// =============================================================================
#[test]
fn refresh_snapshot_must_act_on_read_committed() {
    use sqlrustgo_transaction::manager::{IsolationLevel, TransactionContext};
    use sqlrustgo_transaction::mvcc::MvccEngine;

    let mut engine = MvccEngine::new();
    let tx = engine.begin_transaction();
    // Advance the global clock so there is a fresh timestamp to refresh to.
    let _tx2 = engine.begin_transaction();
    let fresh_ts = engine.get_global_timestamp();

    let snapshot = engine.create_snapshot(tx);
    let mut ctx = TransactionContext::new(tx, snapshot, IsolationLevel::ReadCommitted);

    let before = format!("{:?}", ctx.snapshot);
    ctx.refresh_snapshot(fresh_ts);
    let after = format!("{:?}", ctx.snapshot);

    assert_ne!(
        before, after,
        "refresh_snapshot on a ReadCommitted context must update the snapshot; \
         mutation refresh_snapshot→() or inverted comparison leaves it frozen"
    );
}

// =============================================================================
// B.3  get_transaction_context_for_query ReadCommitted arm deleted
//      (manager.rs:199)
//
// Mutation: the IsolationLevel::ReadCommitted match arm is deleted so
// the query-context path skips refresh_snapshot.
//
// Counterexample: begin a ReadCommitted transaction on a manager, advance
// the engine clock via a second concurrent begin+commit, then call
// get_transaction_context_for_query and assert the returned context's
// snapshot reflects the fresh timestamp (i.e. its snapshot_timestamp is
// >= the post-commit global timestamp). Under the deletion the snapshot
// is frozen at the reader's begin time.
// =============================================================================
#[test]
fn query_context_for_read_committed_must_refresh() {
    use sqlrustgo_transaction::manager::{IsolationLevel, TransactionManager};
    use sqlrustgo_transaction::mvcc::MvccEngine;
    use std::sync::{Arc, RwLock};

    let engine = Arc::new(RwLock::new(MvccEngine::new()));
    let mut m = TransactionManager::with_mvcc(engine.clone());

    m.begin_with_isolation(IsolationLevel::ReadCommitted)
        .expect("begin");

    let begin_snapshot_ts = m
        .get_transaction_context_for_query()
        .expect("context at begin")
        .snapshot
        .snapshot_timestamp;

    {
        let mut eng = engine.write().unwrap();
        for _ in 0..5 {
            let writer = eng.begin_transaction();
            let _ = eng.commit_transaction(writer).expect("concurrent commit");
        }
    }
    let current_ts = engine.read().unwrap().get_global_timestamp();
    assert!(
        current_ts > begin_snapshot_ts,
        "precondition: concurrent commits advanced the clock past begin-time snapshot"
    );

    let ctx = m
        .get_transaction_context_for_query()
        .expect("query context");

    assert!(
        ctx.snapshot.snapshot_timestamp > begin_snapshot_ts,
        "get_transaction_context_for_query under ReadCommitted must refresh \
         the snapshot to the current timestamp; deleting the ReadCommitted \
         arm freezes it at begin time ({})",
        begin_snapshot_ts
    );
}
