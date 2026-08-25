//! V312-58 / Issue #4444 (Phase 3 followup — Sprint 5): wire
//! `HashSemiJoin::from_select` as a call site in the engine's
//! `pre_evaluate_correlated_exists` path so that the
//! single-table / single-equality EXISTS subquery shape picks up the
//! hash probe fast path instead of re-executing the subquery per
//! outer row.
//!
//! The acceptance criteria from the Sprint 5 design:
//!
//!   1. **Functional** — a Q4-mini-shape correlated EXISTS still
//!      returns the correct rows (regression guard against accidentally
//!      breaking the WHERE evaluator when inserting the new arm).
//!   2. **Behavioral** — the diagnostic counter
//!      `DIAG_HASH_SEMI_JOIN_PROBE_HITS` advances, proving the new
//!      call site is reached (RED→GREEN signal).  The counter is part
//!      of the `v312_58_sprint5_diag` set wired into
//!      `dump_v312_58_sprint5_diag()`.
//!
//! Q4-mini shape (the simplest correlated EXISTS the constructor can
//! handle):
//!
//!   SELECT o_orderkey FROM orders
//!   WHERE EXISTS (
//!     SELECT 1 FROM lineitem
//!     WHERE l_orderkey = o_orderkey
//!   )
//!
//! Out of scope for this PR (deferred to follow-up Sprints):
//!   * Composite correlation keys (Q20 L0/L5 full SUM,
//!     `ps_partkey = partsupp.ps_partkey AND ps_suppkey = partsupp.ps_suppkey`)
//!   * NOT EXISTS / anti-semi-join
//!   * Additional residual filters on the inner WHERE
//!     (e.g. `l_commitdate < l_receiptdate`). The constructor in
//!     `crates/executor/src/join/hash_semi_join.rs:198` only models
//!     the single-equality shape; extending it to accept and apply
//!     residual predicates is a separate change.

use parking_lot::RwLock;
use sqlrustgo::{
    dump_v312_58_sprint5_diag, reset_v312_58_sprint5_diag, ExecutionEngine, MemoryStorage, Value,
};
use std::sync::Arc;

fn fresh_engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

/// Helper: extract the integer value of `<key>=<value>` from the
/// multi-line `dump_v312_58_sprint5_diag()` snapshot string.
/// Returns 0 when the key is not present. Stays test-local so it
/// doesn't leak into production.
fn counter_value(snap: &str, key: &str) -> u64 {
    let needle = format!("{key}=");
    for line in snap.lines() {
        if let Some(idx) = line.find(&needle) {
            let rest = &line[idx + needle.len()..];
            let end = rest
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(rest.len());
            return rest[..end].parse().unwrap_or(0);
        }
    }
    0
}

/// Acceptance #1 + #2 (V312-58 / Sprint 5): a Q4-mini-shape correlated
/// EXISTS with a single equality on a single inner table runs end-to-end
/// and the new HashSemiJoin call site is exercised (counter advances).
#[test]
fn q4_mini_single_equality_exists_uses_hash_semi_join_call_site() {
    let mut e = fresh_engine();
    e.execute("CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_orderdate TEXT)")
        .unwrap();
    e.execute("CREATE TABLE lineitem (l_orderkey INTEGER, l_shipdate TEXT)")
        .unwrap();

    // Three orders.  Orders 1 and 2 each have at least one matching
    // lineitem.  Order 3 has no matching lineitem.
    e.execute("INSERT INTO orders VALUES (1, '1994-01-01')")
        .unwrap();
    e.execute("INSERT INTO orders VALUES (2, '1994-01-01')")
        .unwrap();
    e.execute("INSERT INTO orders VALUES (3, '1994-01-01')")
        .unwrap();
    e.execute("INSERT INTO lineitem VALUES (1, '1994-02-01')")
        .unwrap();
    e.execute("INSERT INTO lineitem VALUES (1, '1994-03-01')")
        .unwrap();
    e.execute("INSERT INTO lineitem VALUES (2, '1994-02-01')")
        .unwrap();
    // Lineitem for an unrelated orderkey — must not affect result.
    e.execute("INSERT INTO lineitem VALUES (99, '1994-04-01')")
        .unwrap();

    // Reset diag counters so we only observe THIS query's effect.
    reset_v312_58_sprint5_diag();

    let r = e
        .execute(
            "SELECT o_orderkey FROM orders \
             WHERE EXISTS ( \
               SELECT 1 FROM lineitem \
               WHERE l_orderkey = o_orderkey \
             ) \
             ORDER BY o_orderkey",
        )
        .unwrap();

    // Functional regression: only orders 1 and 2 are returned.
    assert_eq!(
        r.rows,
        vec![vec![Value::Integer(1)], vec![Value::Integer(2)]],
        "Q4-mini EXISTS: only orders with at least one matching \
         lineitem should appear"
    );

    // Behavioral: HashSemiJoin call site was reached.
    let snap = dump_v312_58_sprint5_diag();
    let hits = counter_value(&snap, "hash_semi_join_probe_hits");
    assert!(
        hits > 0,
        "Expected HashSemiJoin call site to be reached at least once \
         for the Q4-mini-shape EXISTS; got 0. \
         Snapshot:\n{snap}"
    );
}

/// Regression guard: a Q4-mini-shape with NO matching lineitem at
/// all must produce zero rows AND still go through the call site
/// (the probe path should be probed once per outer row, finding no
/// matches each time).
#[test]
fn q4_mini_no_matches_still_returns_zero_rows() {
    let mut e = fresh_engine();
    e.execute("CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_orderdate TEXT)")
        .unwrap();
    e.execute("CREATE TABLE lineitem (l_orderkey INTEGER, l_shipdate TEXT)")
        .unwrap();
    e.execute("INSERT INTO orders VALUES (1, '1994-01-01')")
        .unwrap();
    e.execute("INSERT INTO orders VALUES (2, '1994-01-01')")
        .unwrap();
    // No lineitem at all.
    reset_v312_58_sprint5_diag();
    let r = e
        .execute(
            "SELECT o_orderkey FROM orders \
             WHERE EXISTS ( \
               SELECT 1 FROM lineitem \
               WHERE l_orderkey = o_orderkey \
             ) \
             ORDER BY o_orderkey",
        )
        .unwrap();
    assert_eq!(
        r.rows,
        Vec::<Vec<Value>>::new(),
        "Q4-mini EXISTS with empty inner: no outer rows should pass"
    );
    // The call site should still be reached (probe executed per outer
    // row, even if every probe returns NotMatched).
    let snap = dump_v312_58_sprint5_diag();
    let hits = counter_value(&snap, "hash_semi_join_probe_hits");
    assert!(
        hits > 0,
        "Expected HashSemiJoin call site to be reached even when no \
         inner rows match; got 0. Snapshot:\n{snap}"
    );
}
