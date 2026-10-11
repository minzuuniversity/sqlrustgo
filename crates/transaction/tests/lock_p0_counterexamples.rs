//! Counterexample tests for cargo-mutants SURVIVED P0 lock-protocol mutants
//! on `sqlrustgo-transaction` (Path G, IAIA-410 §8 #8).
//!
//! Written on 2026-10-11 in the `a06-mvcc` worktree based on
//! `origin/develop/v4.1.0@326d1ba8c5`.  Targets the remaining Path G P0
//! mutants not already covered by `audit_mutation_counters.rs`
//! (which pins `can_grant` Shared `||`→`&&` and Exclusive `&&`→`||`
//! case A).
//!
//! **Focus targets** (SURVIVED_92_TRIAGE_v2.md §7):
//!   1. `detect_deadlock` → `None` / `Some(vec![])` / `Some(vec![Default::default()])`
//!   2. `add_waiter` `==` → `!=` (lock.rs:78)
//!   3. `remove_waiter` → `()` (lock.rs:84)
//!   4. `remove_waiter` `!=` → `==` (lock.rs:84)
//!
//! **Important structural finding**: `LockManager::detect_deadlock`
//! (lock.rs:105-107) is a thin wrapper around `DeadlockDetector::detect_cycle`
//! with **no production caller** — `acquire_lock` invokes
//! `self.deadlock_detector.detect_cycle(tx_id)` directly at lock.rs:140,
//! bypassing the wrapper.  Therefore the three `detect_deadlock` mutants
//! can ONLY be killed by a test that calls the wrapper method explicitly.
//! Tests G.1/G.2 below do exactly that; an earlier draft that asserted on
//! `acquire_lock`'s return value was verified NON-KILLING (mutation applied,
//! test still passed) because the mutated code was never reached.
//!
//! **Pristine check** (must pass on unmodified `develop/v4.1.0` HEAD).
//! **Mutant check** (each test must FAIL when its target mutation is
//! applied — verified by patching then `git checkout` revert).

use sqlrustgo_transaction::lock::{LockGrantMode, LockManager, LockMode};
use sqlrustgo_transaction::TxId;

// -----------------------------------------------------------------------------
// Counter G.1: `detect_deadlock` → `None` (false negative)
//
// Original: wrapper returns `detect_cycle(tx_id)`'s result.
// Mutation: body replaced with `None` — deadlock never reported.
//
// Counterexample: construct a real 2-tx wait-for cycle via acquire_lock
// (tx1 holds k1, tx2 waits on k1; tx2 holds k2, tx1's Exclusive on k2
// adds the closing edge tx1→tx2 and returns Err(Deadlock) — but the
// edges remain registered in the detector).  Then call the WRAPPER
// `mgr.detect_deadlock(tx1)` directly and assert the cycle is reported
// and contains both participants.
// -----------------------------------------------------------------------------
#[test]
fn detect_deadlock_must_report_cycle_participants() {
    let mut mgr = LockManager::new();
    let k1 = vec![1u8];
    let k2 = vec![2u8];
    let tx1 = TxId::new(1);
    let tx2 = TxId::new(2);

    mgr.acquire_lock(tx1, k1.clone(), LockMode::Shared)
        .expect("tx1 holds k1");
    assert!(matches!(
        mgr.acquire_lock(tx2, k1.clone(), LockMode::Exclusive),
        Ok(LockGrantMode::Waiting)
    ));
    mgr.acquire_lock(tx2, k2.clone(), LockMode::Shared)
        .expect("tx2 holds k2");
    // Closes the cycle; returns Err but registers edge tx1→tx2.
    let closing = mgr.acquire_lock(tx1, k2.clone(), LockMode::Exclusive);
    assert!(
        matches!(
            closing,
            Err(sqlrustgo_transaction::lock::LockError::Deadlock)
        ),
        "cycle-closing acquire must be detected internally, got {:?}",
        closing
    );

    let cycle = mgr.detect_deadlock(tx1);
    assert!(
        cycle.is_some(),
        "detect_deadlock must report a cycle when one exists, got None \
         (mutation detect_deadlock→None would return None)"
    );
    let cycle = cycle.expect("checked above");
    assert!(
        cycle.contains(&tx1) && cycle.contains(&tx2),
        "cycle must contain both participants tx1 and tx2, got {:?}",
        cycle
    );
}

// -----------------------------------------------------------------------------
// Counter G.2: `detect_deadlock` → `Some(vec![])` and
//              `detect_deadlock` → `Some(vec![Default::default()])`
//
// Mutations: always return Some(...) regardless of cycle existence.
//   - `Some(vec![])`     : caller sees "deadlock" with no victim.
//   - `Some(vec![TxId::default()])` : caller would abort TxId(0).
//
// Counterexample: tx1 holds k1; tx2 waits on k1 — edge tx2→tx1 exists
// but there is NO cycle (tx1 does not wait on anyone).  Calling the
// wrapper must return None.  Under either always-Some mutation the
// assertion fails, killing both forms with one test.
// -----------------------------------------------------------------------------
#[test]
fn detect_deadlock_must_return_none_without_cycle() {
    let mut mgr = LockManager::new();
    let k1 = vec![1u8];
    let tx1 = TxId::new(1);
    let tx2 = TxId::new(2);

    mgr.acquire_lock(tx1, k1.clone(), LockMode::Shared)
        .expect("tx1 holds k1");
    assert!(matches!(
        mgr.acquire_lock(tx2, k1.clone(), LockMode::Exclusive),
        Ok(LockGrantMode::Waiting)
    ));

    assert!(
        mgr.detect_deadlock(tx2).is_none(),
        "detect_deadlock(tx2) with no cycle must be None \
         (mutations →Some(vec![]) / →Some(vec![Default]) would return Some)"
    );
    assert!(
        mgr.detect_deadlock(tx1).is_none(),
        "detect_deadlock(tx1) (the holder, not waiting) must also be None"
    );
}

// -----------------------------------------------------------------------------
// Counter G.4: `add_waiter` `==` → `!=` (lock.rs:78)
//
// Original: `if !self.waiters.iter().any(|(t, _)| *t == tx_id)` — skip
// if the tx is already waiting (no duplicates).
// Mutation: `!=` — logic inverted; a second request from the SAME tx
// is appended, producing a duplicate waiter entry.
//
// Counterexample: tx1 holds Shared(K).  tx2 requests Exclusive(K) twice
// (simulating a retry).  Under the mutation the waiters list contains
// [(tx2,Excl),(tx2,Excl)].  We then release tx1 (promotes tx2, whose
// remove_waiter clears ALL tx2 entries) and release tx2.  Observable
// invariant: after both releases the key must be fully unlocked.
//
// Kill matrix (verified 2026-10-11 by applying each mutation and
// observing the suite result):
//   - M4 `add_waiter` `==`→`!=`: this test PASSES (non-killing) — the
//     duplicate entry for tx2 is purged wholesale by remove_waiter's
//     `retain(*t != tx_id)` when tx2 is promoted, so no invariant is
//     observable here. M4 is instead KILLED by G.6 (the mutation makes
//     add_waiter reject a waiter whose tx differs from existing waiters,
//     so tx3 never enters the queue and is never promoted).
//   - M5 `remove_waiter`→`()`: this test FAILS (killing) — the stale
//     waiter causes tx2 to be re-promoted after its own release.
//   - M6 `remove_waiter` `!=`→`==`: this test FAILS (killing) — the
//     inverted retain keeps only the target's entry and drops tx3.
// -----------------------------------------------------------------------------
#[test]
fn add_waiter_second_request_from_same_tx_preserves_invariants() {
    let mut mgr = LockManager::new();
    let key = vec![1u8];
    let tx1 = TxId::new(1);
    let tx2 = TxId::new(2);

    mgr.acquire_lock(tx1, key.clone(), LockMode::Shared)
        .expect("tx1 holds shared");
    assert!(matches!(
        mgr.acquire_lock(tx2, key.clone(), LockMode::Exclusive),
        Ok(LockGrantMode::Waiting)
    ));
    assert!(matches!(
        mgr.acquire_lock(tx2, key.clone(), LockMode::Exclusive),
        Ok(LockGrantMode::Waiting)
    ));

    mgr.release_lock(tx1, &key).expect("release tx1");
    assert!(
        mgr.is_locked_by_tx(&key, tx2),
        "tx2 must be promoted to holder after tx1 releases"
    );

    mgr.release_lock(tx2, &key).expect("release tx2");
    assert!(
        !mgr.is_locked(&key),
        "after both txs release the key must be fully unlocked"
    );
}

// -----------------------------------------------------------------------------
// Counter G.5: `remove_waiter` → `()` (lock.rs:84)
//
// Original: `self.waiters.retain(|(t, _)| *t != tx_id);`
// Mutation: body replaced with `()` — waiters never removed.
//
// Counterexample: tx1 holds Shared(K).  tx2 waits Exclusive(K).
// tx1 releases → promotes tx2 to holder and calls remove_waiter(tx2).
// Under the mutation the stale waiter (tx2,Excl) REMAINS.  When tx2
// then releases: remove_holder(tx2) empties holders, but waiters is
// still non-empty → the promotion block re-adds tx2 as holder.  The
// key is never freed.  Observable: after tx2's release, is_locked(key)
// must be false; under the mutation it stays true (lock leaked).
// -----------------------------------------------------------------------------
#[test]
fn remove_waiter_must_clear_promoted_tx() {
    let mut mgr = LockManager::new();
    let key = vec![1u8];
    let tx1 = TxId::new(1);
    let tx2 = TxId::new(2);

    mgr.acquire_lock(tx1, key.clone(), LockMode::Shared)
        .expect("tx1 holds shared");
    assert!(matches!(
        mgr.acquire_lock(tx2, key.clone(), LockMode::Exclusive),
        Ok(LockGrantMode::Waiting)
    ));

    mgr.release_lock(tx1, &key).expect("release tx1");
    assert!(
        mgr.is_locked_by_tx(&key, tx2),
        "tx2 must be promoted to holder after tx1 releases"
    );

    mgr.release_lock(tx2, &key).expect("release tx2");
    assert!(
        !mgr.is_locked(&key),
        "after tx2 releases the key must be fully unlocked; \
         mutation remove_waiter→() leaves a stale waiter and the second \
         promotion re-acquires the lock for tx2"
    );
}

// -----------------------------------------------------------------------------
// Counter G.6: `remove_waiter` `!=` → `==` (lock.rs:84)
//
// Original: `retain(|(t, _)| *t != tx_id)` — KEEP waiters whose tx_id
// is NOT the target (i.e. remove the target).
// Mutation: `retain(|(t, _)| *t == tx_id)` — KEEP only the target
// (remove everyone else).
//
// Counterexample: tx1 holds Shared(K).  tx2 waits Exclusive(K), tx3
// waits Shared(K).  waiters=[(tx2,Excl),(tx3,Shared)].  tx1 releases →
// promotes tx2, remove_waiter(tx2).  Under the mutation this KEEPS
// only tx2's entry and DROPS tx3.  When tx2 releases, tx3 is never
// promoted (it vanished from the queue).  Observable: after tx2's
// release, is_locked_by_tx(key, tx3) must be true (tx3 promoted);
// under the mutation tx3 is never a holder.
// -----------------------------------------------------------------------------
#[test]
fn remove_waiter_must_remove_only_target_tx() {
    let mut mgr = LockManager::new();
    let key = vec![1u8];
    let tx1 = TxId::new(1);
    let tx2 = TxId::new(2);
    let tx3 = TxId::new(3);

    mgr.acquire_lock(tx1, key.clone(), LockMode::Shared)
        .expect("tx1 holds shared");
    assert!(matches!(
        mgr.acquire_lock(tx2, key.clone(), LockMode::Exclusive),
        Ok(LockGrantMode::Waiting)
    ));
    assert!(matches!(
        mgr.acquire_lock(tx3, key.clone(), LockMode::Shared),
        Ok(LockGrantMode::Waiting)
    ));

    mgr.release_lock(tx1, &key).expect("release tx1");
    assert!(mgr.is_locked_by_tx(&key, tx2), "tx2 promoted");

    mgr.release_lock(tx2, &key).expect("release tx2");
    assert!(
        mgr.is_locked_by_tx(&key, tx3),
        "tx3 must still be waiting after remove_waiter(tx2) and get \
         promoted when tx2 releases; mutation remove_waiter `!=`→`==` \
         drops tx3 from the queue"
    );
}
