//! V312-58 / Issue #4444 followup (Sprint 6) — wire
//! `sqlrustgo_optimizer::decorrelate::try_decorrelate` into
//! `execute_select` so patterns 1-3 (ExistsSemi, NotExistsAnti,
//! InToInnerJoin) pick up a one-shot materialization instead of
//! per-outer-row subquery re-execution. Pattern 4 (ScalarAggInWhere /
//! Q17) is already wired via `prewarm_scalar_agg_index_for_select`;
//! this file's first test pins that existing path so the refactor in
//! commits 3-5 cannot regress it.
//!
//! The acceptance criteria per pattern:
//!
//!   1. **Functional** — a TPC-H-mini-shape correlated subquery
//!      returns the correct rows for the pattern under test.
//!   2. **Behavioral** — when the wiring for a given pattern lands,
//!      the Sprint 6 counter advances (RED→GREEN signal). When the
//!      wiring has not landed yet (Pattern 4 = Pattern 1 only on
//!      Commit 2), the counter is permitted to stay at 0; the
//!      pre-existing `sprint3_diag` counter for that pattern is the
//!      evidence.
//!
//! Out of scope for this PR (deferred to follow-up Sprints):
//!   * Decorrelate WHERE patterns with multi-level correlation
//!     (Q20 supplier-tree).
//!   * Convert `try_decorrelate`'s output into a real `LogicalPlan`
//!     (currently `execute_select` operates on `SelectStatement` AST).
//!   * Promote `try_decorrelate`'s `LeftOuterGroupBy` join kind
//!     (currently unused — only 4 of 5 patterns in openspec are
//!     wired even after this PR).

use parking_lot::{Mutex, RwLock};
use sqlrustgo::{
    dump_v312_58_sprint3_diag, dump_v312_58_sprint6_diag, reset_v312_58_sprint3_diag,
    reset_v312_58_sprint6_diag, ExecutionEngine, MemoryStorage, Value,
};
use std::sync::{Arc, OnceLock};

fn fresh_engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

/// Process-wide mutex that serializes tests in this binary.
///
/// Why this exists: the Sprint 3 / Sprint 6 DIAG counters are
/// `static AtomicU64` in `engine_select.rs`. Each test calls
/// `reset_v312_58_sprintN_diag()` immediately before its own
/// `execute()` and then `dump_v312_58_sprintN_diag()` immediately
/// after. Under `cargo test`'s default parallel-execution policy, a
/// second test's `reset_*_diag()` can land between this test's
/// `execute()` and `dump_*_diag()` and zero the counter the assertion
/// is reading — producing ~50% flakiness (verified empirically: see
/// task #9). Holding the guard across `reset`/`execute`/`dump`
/// eliminates the race window entirely without depending on the
/// `serial_test` crate (which is not in this repo's dependency
/// tree) and without giving up the RED→GREEN behavioral signal.
fn diag_serial_guard() -> parking_lot::MutexGuard<'static, ()> {
    static SERIAL: OnceLock<Mutex<()>> = OnceLock::new();
    SERIAL.get_or_init(|| Mutex::new(())).lock()
}

/// Helper: extract the integer value of `<key>=<value>` from the
/// multi-line `dump_v312_58_sprintN_diag()` snapshot string.
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

// -------------------------------------------------------------------------
// Pattern 4 (ScalarAggInWhere) — regression guard
// -------------------------------------------------------------------------
//
// This pattern is already wired via
// `prewarm_scalar_agg_index_for_select` and the
// `try_scalar_agg_index_lookup` fast path. The test pins behavior so
// the commits 3-5 refactor (which extends `DerivedResult` and adds
// new `__decorrelated_N` materialization) cannot silently regress the
// Q17 path.

/// Acceptance #1 (Pattern 4 — Q17-mini): correlated scalar aggregate
/// in WHERE still returns correct rows after the Sprint 6 DIAG +
/// Pattern 4-3 wiring has landed. The query shape is identical to
/// `issue_4374_regression::q17_correlated_avg_filter_is_applied_before_aggregation`
/// to keep the cell-diff oracle stable.
#[test]
fn pattern4_scalar_agg_in_where_still_correct_after_sprint6() {
    let _diag_guard = diag_serial_guard();
    let mut e = fresh_engine();
    e.execute("CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_brand TEXT, p_container TEXT)")
        .unwrap();
    e.execute(
        "CREATE TABLE lineitem (l_partkey INTEGER, l_quantity INTEGER, \
         l_extendedprice REAL, l_shipdate TEXT)",
    )
    .unwrap();
    e.execute("INSERT INTO part VALUES (1, 'Brand#23', 'LG CASE')")
        .unwrap();
    e.execute("INSERT INTO lineitem VALUES (1, 1, 10.0, '1994-01-01')")
        .unwrap();
    e.execute("INSERT INTO lineitem VALUES (1, 2, 20.0, '1994-01-01')")
        .unwrap();
    e.execute("INSERT INTO lineitem VALUES (1, 100, 100.0, '1994-01-01')")
        .unwrap();
    e.execute("INSERT INTO lineitem VALUES (1, 200, 200.0, '1994-01-01')")
        .unwrap();

    // Reset BOTH Sprint 3 (Pattern 4 path) and Sprint 6 (Patterns 1-3)
    // counters so we only observe THIS query's effect.
    reset_v312_58_sprint3_diag();
    reset_v312_58_sprint6_diag();

    let r = e
        .execute(
            "SELECT SUM(l_extendedprice) / 7.0 AS avg_yearly FROM lineitem, part \
             WHERE p_partkey = l_partkey AND p_brand = 'Brand#23' \
             AND p_container = 'LG CASE' AND l_quantity < \
               (SELECT 0.2 * AVG(l_quantity) FROM lineitem WHERE l_partkey = p_partkey)",
        )
        .unwrap();

    // Functional: only l_quantity=1 and l_quantity=2 pass the < (0.2 * 75.75)=15.15
    // predicate; their extendedprice sum is 10 + 20 = 30; 30 / 7 = ~4.2857.
    assert_eq!(r.rows.len(), 1);
    let got = match r.rows[0][0] {
        Value::Float(v) => v,
        ref other => panic!("expected Float, got {:?}", other),
    };
    let expected = 30.0 / 7.0;
    assert!(
        (got - expected).abs() < 1e-9,
        "Q17-mini got {}, expected {}",
        got,
        expected
    );

    // Behavioral: the existing Pattern 4 path was reached.
    // `try_scalar_agg_index_lookup` is the canonical Q17 fast path; it
    // must have been entered at least once.
    let s3 = dump_v312_58_sprint3_diag();
    let calls = counter_value(&s3, "try_scalar_agg_index_lookup calls");
    assert!(
        calls > 0,
        "Expected Q17 fast path to be entered for ScalarAggInWhere; \
         got 0. Sprint 3 snapshot:\n{s3}"
    );

    // Sprint 6 wiring: `try_decorrelate` is now called for every
    // `execute_select` with a WHERE clause. Pattern 4 (ScalarAggInWhere)
    // produces exactly one inner_select with kind=ScalarAggInWhere
    // (already supported by the rewrite in
    // `crates/optimizer/src/decorrelate.rs:480-507`). The counters
    // therefore advance even for the existing Pattern 4 path, which
    // is the witness that the unified entry point is wired.
    let s6 = dump_v312_58_sprint6_diag();
    let td_calls = counter_value(&s6, "try_decorrelate_calls");
    let td_hits = counter_value(&s6, "try_decorrelate_hits");
    let td_mat = counter_value(&s6, "try_decorrelate_materialize");
    assert!(
        td_calls >= 1,
        "Expected try_decorrelate to be entered at least once for \
         Pattern 4 (ScalarAggInWhere); got {td_calls}. Sprint 6 snapshot:\n{s6}"
    );
    assert!(
        td_hits >= 1,
        "Expected try_decorrelate to find >=1 inner_select (the \
         ScalarAggInWhere subquery) for Pattern 4; got {td_hits}. \
         Sprint 6 snapshot:\n{s6}"
    );
    assert!(
        td_mat >= 1,
        "Expected DIAG_DECORRELATED_MATERIALIZE to advance for the \
         ScalarAggInWhere inner_select; got {td_mat}. Sprint 6 snapshot:\n{s6}"
    );
}

// -------------------------------------------------------------------------
// Pattern 1 (ExistsSemi) — Q4-mini regression test
// -------------------------------------------------------------------------
//
// The Q4-mini shape is the simplest correlated EXISTS the constructor
// can handle. Mirrors the fixture in
// `issue_4444_hash_semi_join_call_site::q4_mini_single_equality_exists_uses_hash_semi_join_call_site`
// so the cell-diff oracle stays stable across both files.
//
//   SELECT o_orderkey FROM orders
//   WHERE EXISTS (
//     SELECT 1 FROM lineitem WHERE l_orderkey = o_orderkey
//   )
//
// Acceptance:
//   1. **Functional** — only orders with at least one matching lineitem
//      are returned.
//   2. **Behavioral** — `try_decorrelate` is entered (>=1 call) AND
//      it produces at least one inner_select (>=1 hit) AND the
//      materialize counter advances for the Semi inner_select.

#[test]
fn pattern1_exists_semi_wires_through_try_decorrelate() {
    let _diag_guard = diag_serial_guard();
    let mut e = fresh_engine();
    e.execute("CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_orderdate TEXT)")
        .unwrap();
    e.execute("CREATE TABLE lineitem (l_orderkey INTEGER, l_shipdate TEXT)")
        .unwrap();

    // Three orders; orders 1 and 2 each have at least one matching
    // lineitem; order 3 has none. One lineitem (orderkey=99) is
    // unrelated and must not affect the result.
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
    e.execute("INSERT INTO lineitem VALUES (99, '1994-04-01')")
        .unwrap();

    // Reset Sprint 6 counters so we only observe THIS query's effect.
    reset_v312_58_sprint6_diag();

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

    // Functional: only orders 1 and 2 pass the EXISTS filter.
    assert_eq!(
        r.rows,
        vec![vec![Value::Integer(1)], vec![Value::Integer(2)]],
        "Q4-mini EXISTS: only orders with at least one matching lineitem"
    );

    // Behavioral: try_decorrelate was entered and produced at least
    // one Semi inner_select for this query.
    let s6 = dump_v312_58_sprint6_diag();
    let td_calls = counter_value(&s6, "try_decorrelate_calls");
    let td_hits = counter_value(&s6, "try_decorrelate_hits");
    let td_mat = counter_value(&s6, "try_decorrelate_materialize");
    assert!(
        td_calls >= 1,
        "Expected try_decorrelate to be entered at least once for \
         Pattern 1 (ExistsSemi); got {td_calls}. Sprint 6 snapshot:\n{s6}"
    );
    assert!(
        td_hits >= 1,
        "Expected try_decorrelate to find >=1 inner_select (the \
         Semi subquery) for Pattern 1; got {td_hits}. Sprint 6 snapshot:\n{s6}"
    );
    assert!(
        td_mat >= 1,
        "Expected DIAG_DECORRELATED_MATERIALIZE to advance for the \
         Semi inner_select; got {td_mat}. Sprint 6 snapshot:\n{s6}"
    );
}

// -------------------------------------------------------------------------
// Pattern 2 (NotExistsAnti) — Q20-mini regression test
// -------------------------------------------------------------------------
//
// The Q20-mini NOT EXISTS shape: orders WITH a matching lineitem are
// filtered OUT; only orders WITHOUT a matching lineitem survive. Uses
// the same fixture as Pattern 1 so the cell-diff oracle stays stable.
//
//   SELECT o_orderkey FROM orders
//   WHERE NOT EXISTS (
//     SELECT 1 FROM lineitem WHERE l_orderkey = o_orderkey
//   )
//
// Acceptance:
//   1. **Functional** — only order 3 (no matching lineitem) survives.
//   2. **Behavioral** — `try_decorrelate` is entered (>=1 call) AND
//      it produces at least one inner_select (>=1 hit) AND the
//      materialize counter advances for the Anti inner_select.

#[test]
fn pattern2_not_exists_anti_wires_through_try_decorrelate() {
    let _diag_guard = diag_serial_guard();
    let mut e = fresh_engine();
    e.execute("CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_orderdate TEXT)")
        .unwrap();
    e.execute("CREATE TABLE lineitem (l_orderkey INTEGER, l_shipdate TEXT)")
        .unwrap();

    // Same fixture as Pattern 1: orders 1 and 2 each have a matching
    // lineitem; order 3 has no matching lineitem and must therefore
    // survive the NOT EXISTS filter.
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
    e.execute("INSERT INTO lineitem VALUES (99, '1994-04-01')")
        .unwrap();

    // Reset Sprint 6 counters so we only observe THIS query's effect.
    reset_v312_58_sprint6_diag();

    let r = e
        .execute(
            "SELECT o_orderkey FROM orders \
             WHERE NOT EXISTS ( \
               SELECT 1 FROM lineitem \
               WHERE l_orderkey = o_orderkey \
             ) \
             ORDER BY o_orderkey",
        )
        .unwrap();

    // Functional: only order 3 (no matching lineitem) survives.
    assert_eq!(
        r.rows,
        vec![vec![Value::Integer(3)]],
        "Q20-mini NOT EXISTS: only orders WITHOUT a matching lineitem \
         should appear (order 3)"
    );

    // Behavioral: try_decorrelate was entered and produced at least
    // one Anti inner_select for this query.
    let s6 = dump_v312_58_sprint6_diag();
    let td_calls = counter_value(&s6, "try_decorrelate_calls");
    let td_hits = counter_value(&s6, "try_decorrelate_hits");
    let td_mat = counter_value(&s6, "try_decorrelate_materialize");
    assert!(
        td_calls >= 1,
        "Expected try_decorrelate to be entered at least once for \
         Pattern 2 (NotExistsAnti); got {td_calls}. Sprint 6 snapshot:\n{s6}"
    );
    assert!(
        td_hits >= 1,
        "Expected try_decorrelate to find >=1 inner_select (the \
         Anti subquery) for Pattern 2; got {td_hits}. Sprint 6 snapshot:\n{s6}"
    );
    assert!(
        td_mat >= 1,
        "Expected DIAG_DECORRELATED_MATERIALIZE to advance for the \
         Anti inner_select; got {td_mat}. Sprint 6 snapshot:\n{s6}"
    );
}

// -------------------------------------------------------------------------
// Pattern 3 (InToInnerJoin) — Q20-mini regression test
// -------------------------------------------------------------------------
//
// The Q20-mini IN shape: `WHERE l_partkey IN (SELECT p_partkey FROM part
// WHERE p_name LIKE 'forest%')` is the canonical correlated subquery
// where the inner projection column drives an O(1) membership probe.
// This test uses a slightly simplified shape so it doesn't depend on
// the deeper supplier-tree from the real Q20:
//
//   SELECT o_orderkey FROM orders
//   WHERE o_orderkey IN (
//     SELECT l_orderkey FROM lineitem WHERE l_shipdate = '1994-02-01'
//   )
//
// Acceptance:
//   1. **Functional** — only orders whose orderkey appears as an
//      l_orderkey in lineitem with the matching shipdate are returned.
//   2. **Behavioral** — `try_decorrelate` is entered (>=1 call) AND
//      it produces at least one inner_select (>=1 hit) AND the
//      materialize counter advances for the Inner inner_select.

#[test]
fn pattern3_in_subquery_wires_through_try_decorrelate() {
    let _diag_guard = diag_serial_guard();
    let mut e = fresh_engine();
    e.execute("CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_orderdate TEXT)")
        .unwrap();
    e.execute("CREATE TABLE lineitem (l_orderkey INTEGER, l_shipdate TEXT)")
        .unwrap();

    // Three orders. lineitem has rows for orderkeys 1, 2, and 99.
    // Only orderkey 1 has a matching shipdate; orderkey 2 has a
    // different shipdate and therefore does not match.
    e.execute("INSERT INTO orders VALUES (1, '1994-01-01')")
        .unwrap();
    e.execute("INSERT INTO orders VALUES (2, '1994-01-01')")
        .unwrap();
    e.execute("INSERT INTO orders VALUES (3, '1994-01-01')")
        .unwrap();
    e.execute("INSERT INTO lineitem VALUES (1, '1994-02-01')")
        .unwrap();
    e.execute("INSERT INTO lineitem VALUES (2, '1994-03-01')")
        .unwrap();
    e.execute("INSERT INTO lineitem VALUES (99, '1994-02-01')")
        .unwrap();

    // Reset Sprint 6 counters so we only observe THIS query's effect.
    reset_v312_58_sprint6_diag();

    let r = e
        .execute(
            "SELECT o_orderkey FROM orders \
             WHERE o_orderkey IN ( \
               SELECT l_orderkey FROM lineitem \
               WHERE l_shipdate = '1994-02-01' \
             ) \
             ORDER BY o_orderkey",
        )
        .unwrap();

    // Functional: only order 1 matches (l_orderkey=1 with shipdate
    // '1994-02-01'). Order 2 has a different shipdate; order 3 has no
    // matching lineitem at all.
    assert_eq!(
        r.rows,
        vec![vec![Value::Integer(1)]],
        "Q20-mini IN: only orders whose orderkey appears as an \
         l_orderkey with the matching shipdate (order 1)"
    );

    // Behavioral: try_decorrelate was entered and produced at least
    // one Inner inner_select for this query.
    let s6 = dump_v312_58_sprint6_diag();
    let td_calls = counter_value(&s6, "try_decorrelate_calls");
    let td_hits = counter_value(&s6, "try_decorrelate_hits");
    let td_mat = counter_value(&s6, "try_decorrelate_materialize");
    assert!(
        td_calls >= 1,
        "Expected try_decorrelate to be entered at least once for \
         Pattern 3 (InToInnerJoin); got {td_calls}. Sprint 6 snapshot:\n{s6}"
    );
    assert!(
        td_hits >= 1,
        "Expected try_decorrelate to find >=1 inner_select (the \
         Inner subquery) for Pattern 3; got {td_hits}. Sprint 6 snapshot:\n{s6}"
    );
    assert!(
        td_mat >= 1,
        "Expected DIAG_DECORRELATED_MATERIALIZE to advance for the \
         Inner inner_select; got {td_mat}. Sprint 6 snapshot:\n{s6}"
    );
}
