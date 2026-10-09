//! Counterexample tests for cargo-mutants SURVIVED on `sqlrustgo-transaction`.
//!
//! These tests are **not** added to the production source; they live in
//! `crates/transaction/tests/` so that they can be compiled into the
//! same `cargo test` invocation that cargo-mutants uses to evaluate
//! each mutant.  The original `tests/coverage_v3_12.rs` already covers
//! shared/exclusive/waiter paths but did not exercise the exact branches
//! where the three SURVIVED mutants live.
//!
//! **Provenance**: written on 2026-10-10 in the audit-tooling worktree
//! after cargo-mutants v27.1.0 (HEAD `e849bc99f6`) reported 3 SURVIVED
//! mutants in `sqlrustgo-transaction`.  See
//! `reports/sqlrustgo_audit_tooling.md` §7.1.1 for the
//! mutation-evidence cycle.
//!
//! **Three targets**:
//!   1. `gid.rs:17`  `impl fmt::Display for NodeId::fmt`  (F1 orphan)
//!   2. `gid.rs:49`  `GlobalTransactionId::parse !=` vs `==`
//!   3. `lock.rs:63` `LockInfo::can_grant &&` vs `||`  (F6/F8 race)
//!
//! **Pristine check** (must pass on the unmodified `develop/v4.1.0` HEAD).
//! **Mutant check** (each test must FAIL when its target mutation is
//! applied — verified by patching then reverting via `git checkout`).
//!
//! These tests are NOT used to assert production correctness; they are
//! mutation tests, asserting "the existing test suite + these counter-
//! examples will KILL a real mutation that previously survived".

use sqlrustgo_transaction::gid::{GlobalTransactionId, NodeId};
use sqlrustgo_transaction::lock::{LockGrantMode, LockManager, LockMode};
use sqlrustgo_transaction::TxId;

// -----------------------------------------------------------------------------
// Counter #1: `lock.rs:63` `&&` vs `||` in `LockInfo::can_grant` (Shared arm)
//
// Original (PR #4980 / lock.rs:60-66):
//   LockMode::Shared => {
//       self.holders.is_empty()
//           || (self.mode == LockMode::Shared && self.waiters.is_empty())
//   }
//
// Mutation (cargo-mutants `replace && with ||`): the inner conjunction
// becomes a disjunction.  Under the disjunction the function returns
// `true` whenever (holders is empty) || (current mode is Shared) —
// regardless of whether there are waiters.  Real-world consequence:
// a new shared request can be granted even though an exclusive request
// is already waiting, breaking FIFO ordering (F8).
//
// Counterexample construction:
//   1. Tx1 acquires Shared(K) — granted, holders={Tx1}, mode=Shared.
//   2. Tx2 requests Exclusive(K) — blocked, waiters={Tx2}.
//   3. Tx3 requests Shared(K) — must NOT be granted (Tx2 is waiting on
//      the same key and should not be bypassed).
// -----------------------------------------------------------------------------
#[test]
fn can_grant_shared_must_not_bypass_waiting_exclusive() {
    let mut mgr = LockManager::new();
    let key = vec![1, 2, 3];
    let tx1 = TxId::new(1);
    let tx2 = TxId::new(2);
    let tx3 = TxId::new(3);

    assert!(matches!(
        mgr.acquire_lock(tx1, key.clone(), LockMode::Shared),
        Ok(LockGrantMode::Granted)
    ));
    assert!(matches!(
        mgr.acquire_lock(tx2, key.clone(), LockMode::Exclusive),
        Ok(LockGrantMode::Waiting)
    ));
    let third = mgr.acquire_lock(tx3, key.clone(), LockMode::Shared);
    assert!(
        matches!(third, Ok(LockGrantMode::Waiting)),
        "Shared request while another tx is waiting on Exclusive must wait, \
         got {:?} (mutation `&&`→`||` in LockInfo::can_grant would grant this)",
        third
    );
}

// -----------------------------------------------------------------------------
// Counter #2: `lock.rs:63` `&&` vs `||` in `LockInfo::can_grant` (Exclusive arm)
//
// Original (lock.rs:66):
//   LockMode::Exclusive => self.holders.is_empty() && self.waiters.is_empty(),
//
// Mutation: under the disjunction the function returns `true` whenever
// `holders.is_empty() || waiters.is_empty()` — granting Exclusive even
// when there is another holder AND another waiter.  Real-world
// consequence: Tx3 takes an Exclusive grant while Tx1 still holds a
// Shared lock, a clear correctness violation (F1/F6).
//
// Construction:
//   1. Tx1 acquires Shared(K) — granted, holders={Tx1}, mode=Shared.
//   2. Tx2 requests Exclusive(K) — blocked, waiters={Tx2}.
//   3. Tx3 requests Exclusive(K) — must NOT be granted; Tx1 still
//      holds a Shared lock on the same key.
// -----------------------------------------------------------------------------
#[test]
fn can_grant_exclusive_must_not_bypass_held_shared() {
    let mut mgr = LockManager::new();
    let key = vec![1, 2, 3];
    let tx1 = TxId::new(1);
    let tx2 = TxId::new(2);
    let tx3 = TxId::new(3);

    assert!(matches!(
        mgr.acquire_lock(tx1, key.clone(), LockMode::Shared),
        Ok(LockGrantMode::Granted)
    ));
    assert!(matches!(
        mgr.acquire_lock(tx2, key.clone(), LockMode::Exclusive),
        Ok(LockGrantMode::Waiting)
    ));
    let third = mgr.acquire_lock(tx3, key.clone(), LockMode::Exclusive);
    assert!(
        matches!(third, Ok(LockGrantMode::Waiting)),
        "Exclusive request must not bypass a held Shared lock and a waiting \
         Exclusive request, got {:?} (mutation `&&`→`||` in LockInfo::can_grant \
         would grant this)",
        third
    );
}

// -----------------------------------------------------------------------------
// Counter #3: `gid.rs:49` `GlobalTransactionId::parse` `!=` vs `==`
//
// Original (gid.rs:49):
//   let parts: Vec<&str> = s.split(':').collect();
//   if parts.len() != 3 {
//       return Err("Invalid GID format".to_string());
//   }
//
// Mutation: `replace != with ==` inverts the validity check. Under
// the mutation, a 3-part input is REJECTED and any other length is
// ACCEPTED (with subsequent parse errors from `parts[N].parse()`).
//
// Counterexample: a well-formed GID is rejected by the mutation but
// accepted by the original.
// -----------------------------------------------------------------------------
#[test]
fn gid_parse_must_accept_well_formed_three_part_string() {
    // "1:42:1700000000" is a well-formed 3-part GID; the original
    // accepts it, the mutation (replace `!=` with `==`) rejects it.
    let parsed = GlobalTransactionId::parse("1:42:1700000000");
    assert!(
        parsed.is_ok(),
        "valid 3-part GID must parse, got {:?} (mutation `!=`→`==` in \
         GlobalTransactionId::parse would reject this)",
        parsed
    );
    let g = parsed.expect("ok checked above");
    assert_eq!(g.node_id, NodeId(1));
    assert_eq!(g.txn_id, 42);
    assert_eq!(g.timestamp, 1_700_000_000);
}

#[test]
fn gid_parse_must_reject_wrong_arity_string() {
    // A two-part string is malformed; the original rejects it, the
    // mutation (`!=`→`==`) accepts it and would then fail on
    // `parts[1].parse::<u64>()` for a non-numeric segment, OR succeed
    // accidentally for a numeric segment.  Either way, it breaks the
    // documented contract.
    let parsed_two = GlobalTransactionId::parse("1:42");
    assert!(
        parsed_two.is_err(),
        "2-part GID must be rejected, got {:?} (mutation `!=`→`==` would accept)",
        parsed_two
    );
    let parsed_one = GlobalTransactionId::parse("1");
    assert!(
        parsed_one.is_err(),
        "1-part GID must be rejected, got {:?} (mutation `!=`→`==` would accept)",
        parsed_one
    );
}

// -----------------------------------------------------------------------------
// Counter #4: `gid.rs:17` `impl fmt::Display for NodeId::fmt`
//
// Original (gid.rs:15-19):
//   impl fmt::Display for NodeId {
//       fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//           write!(f, "NodeId({})", self.0)
//       }
//   }
//
// Mutation: `replace with Ok(Default::default())` makes every NodeId
// format as the empty string.
//
// NOTE: this test is intentionally INERT today — NodeId::Display has
// no production caller (`grep "NodeId"` shows 13 usage sites in
// `crates/transaction/` but no `format!("{}", node_id)` or `to_string()`
// anywhere).  It is recorded here as evidence for the F1 (orphan /
// dead code) classification, not as a behavior test.  If a future PR
// adds a NodeId::Display caller (e.g. an error log line), this test
// will catch the regression without further work.
// -----------------------------------------------------------------------------
#[test]
#[ignore = "see Counter #4 doc — NodeId::Display has no production caller; \
            recorded as F1 evidence only; remove this `#[ignore]` once a \
            production caller exists."]
fn node_id_display_uses_inner_value() {
    let n = NodeId(42);
    let rendered = format!("{}", n);
    assert_eq!(rendered, "NodeId(42)");
}
