//! V312-58 Sprint 5 followup-2 — TPC-H Q4 residual filter re-evaluation test.
//!
//! Verifies that the canonical TPC-H Q4 shape — `EXISTS (SELECT * FROM lineitem
//! WHERE l_orderkey = o_orderkey AND l_commitdate < l_receiptdate)` — is now
//! served by the `HashSemiJoinIndex` call site, not the SubqueryIndex /
//! `pre_eval_exists_subquery_fast` fallback. The residual
//! `l_commitdate < l_receiptdate` is purely static (no outer-row references),
//! so the `try_build_hash_semi_join_index_for_subq` build-time filter path
//! applies it before the HSJ sees any inner row, and per-outer-row probes
//! stay O(1) (no SubqueryIndex bucket scan, no recursive `execute_select`).
//!
//! Acceptance criteria:
//!  1. **Functional** — the result row count and per-priority group counts
//!     match the truth captured from Q4-mini fixtures in the same file
//!     shape.
//!  2. **Behavioral** — `DIAG_HASH_SEMI_JOIN_BUILDS > 0` (shape gate
//!     accepted the residual-bearing WHERE) and `DIAG_HASH_SEMI_JOIN_PROBE_HITS > 0`
//!     (probe call site reached at least once). Without the residual-filter
//!     shape gate the index would not be built at all.
//!  3. **Regression guard** — `q4_hash_semi_join_test` (SubqueryIndex path,
//!     same Q4 SQL) continues to pass; the residual path is parallel and
//!     must not perturb the SubqueryIndex flow.

use parking_lot::RwLock;
use sqlrustgo::{
    dump_v312_58_sprint5_diag, reset_v312_58_sprint5_diag, snapshot_v312_58_sprint5_diag,
    ExecutionEngine, MemoryStorage, Value,
};
use std::sync::Arc;

fn fresh_engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

/// Canonical TPC-H Q4 SQL — the residual `l_commitdate < l_receiptdate`
/// is the case the `HashSemiJoinIndex` shape gate must now accept.
const Q4_REAL_SQL: &str = "SELECT o_orderpriority, COUNT(*) AS order_count \
    FROM orders \
    WHERE o_orderdate >= '1993-07-01' AND o_orderdate < '1993-10-01' \
      AND EXISTS (SELECT * FROM lineitem \
                  WHERE l_orderkey = o_orderkey \
                    AND l_commitdate < l_receiptdate) \
    GROUP BY o_orderpriority \
    ORDER BY o_orderpriority";

/// Helper: extract the integer value of `<key>=<value>` from the
/// multi-line `dump_v312_58_sprint5_diag()` snapshot string.
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

/// Acceptance #1 + #2: canonical TPC-H Q4 (residual-bearing EXISTS)
/// produces the correct priority-group counts AND exercises the
/// HashSemiJoinIndex call site.
///
/// Fixture: 3 orders (1, 2, 3) + 4 lineitems.
///   - order 1, date 1993-08-15 (in range), lineitem (1, receipt > commit) → EXISTS true
///   - order 2, date 1993-09-15 (in range), lineitem (2, commit >= receipt) → EXISTS false
///   - order 3, date 1992-12-31 (out of range)                              → not counted (date filter)
///
/// Expected: one priority group `1-URGENT` with count 1.
#[test]
fn q4_real_residual_filter_uses_hash_semi_join_call_site() {
    let mut e = fresh_engine();
    e.execute(
        "CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, \
              o_orderdate TEXT, o_orderpriority TEXT)",
    )
    .unwrap();
    e.execute(
        "CREATE TABLE lineitem (l_orderkey INTEGER, \
              l_receiptdate TEXT, l_commitdate TEXT)",
    )
    .unwrap();

    // Order 1: in date range, EXISTS true.
    e.execute("INSERT INTO orders VALUES (1, '1993-08-15', '1-URGENT')")
        .unwrap();
    e.execute("INSERT INTO lineitem VALUES (1, '1993-08-20', '1993-08-10')")
        .unwrap();

    // Order 2: in date range, EXISTS false (commitdate >= receiptdate).
    e.execute("INSERT INTO orders VALUES (2, '1993-09-15', '2-HIGH')")
        .unwrap();
    e.execute("INSERT INTO lineitem VALUES (2, '1993-09-10', '1993-09-20')")
        .unwrap();

    // Order 3: out of date range, EXISTS true — must be filtered by date.
    e.execute("INSERT INTO orders VALUES (3, '1992-12-31', '3-MEDIUM')")
        .unwrap();
    e.execute("INSERT INTO lineitem VALUES (3, '1993-01-15', '1993-01-10')")
        .unwrap();

    reset_v312_58_sprint5_diag();

    let r = e.execute(Q4_REAL_SQL).unwrap();

    // Acceptance #1: functional correctness — exactly one priority group,
    // order 1 only, count 1.
    assert_eq!(
        r.rows,
        vec![vec![Value::Text("1-URGENT".to_string()), Value::Integer(1)]],
        "Q4 residual EXISTS: only the in-range order with a residual-passing \
         lineitem should appear, got: {:?}",
        r.rows
    );

    // Acceptance #2: behavioral — the residual-bearing Q4 shape must build
    // at least one HashSemiJoinIndex and reach the probe call site at least
    // once (the order-1 EXISTS evaluation probes `l_orderkey = 1`, which
    // finds a residual-passing lineitem, so the index is consulted).
    let snap = dump_v312_58_sprint5_diag();
    let builds = counter_value(&snap, "hash_semi_join_builds");
    let probe_hits = counter_value(&snap, "hash_semi_join_probe_hits");
    assert!(
        builds > 0,
        "Expected HashSemiJoinIndex to be built for Q4 residual shape; \
         got builds={builds}. Snapshot:\n{snap}"
    );
    assert!(
        probe_hits > 0,
        "Expected HashSemiJoinIndex probe call site to be reached for \
         Q4 residual shape; got probe_hits={probe_hits}. Snapshot:\n{snap}"
    );
}

/// Regression guard #3: the same Q4 SQL but with **no inner lineitem at
/// all** must still go through the residual-bearing HSJ call site
/// (every outer row's probe returns NotMatched because the key_index
/// is empty for the residual-passing rows). Ensures the build-time
/// filter does not falsely exclude the entire inner table on empty
/// residual — `try_build_hash_semi_join_index_for_subq` only runs
/// the filter when the inner table has rows; an empty inner table
/// leaves the index empty, which is the correct empty-set behavior.
#[test]
fn q4_real_residual_filter_empty_inner_table() {
    let mut e = fresh_engine();
    e.execute(
        "CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, \
              o_orderdate TEXT, o_orderpriority TEXT)",
    )
    .unwrap();
    // Empty lineitem table.
    e.execute(
        "CREATE TABLE lineitem (l_orderkey INTEGER, \
              l_receiptdate TEXT, l_commitdate TEXT)",
    )
    .unwrap();
    e.execute("INSERT INTO orders VALUES (1, '1993-08-15', '1-URGENT')")
        .unwrap();
    e.execute("INSERT INTO orders VALUES (2, '1993-09-15', '2-HIGH')")
        .unwrap();

    reset_v312_58_sprint5_diag();
    let r = e.execute(Q4_REAL_SQL).unwrap();
    assert_eq!(
        r.rows,
        Vec::<Vec<Value>>::new(),
        "Q4 with empty inner: zero rows should pass EXISTS"
    );
    // With an empty lineitem, the shape gate still builds the index (builds=1)
    // but the probe finds no key_index hits. The probe call site is reached
    // once per outer row whose probe key is absent.
    let snap = dump_v312_58_sprint5_diag();
    let builds = counter_value(&snap, "hash_semi_join_builds");
    let probe_hits = counter_value(&snap, "hash_semi_join_probe_hits");
    assert!(
        builds > 0,
        "HashSemiJoinIndex must still be built even when inner is empty; \
         got builds={builds}. Snapshot:\n{snap}"
    );
    assert!(
        probe_hits > 0,
        "Probe call site must still be reached on empty inner; \
         got probe_hits={probe_hits}. Snapshot:\n{snap}"
    );
}

/// Acceptance #3 (UPDATED for followup-3): residual that references an
/// outer-row column is **now accepted** by the shape gate (the build-time
/// filter is skipped, the residual is re-evaluated at probe time after
/// substituting outer refs). The HSJ IS built for this subquery; the
/// count of `hash_semi_join_builds` must be 1 (not 0 as in followup-2).
///
/// We pick a residual that is tautological (`o_orderkey = o_orderkey`)
/// so the result set is identical to canonical Q4 even though the
/// residual is outer-column-dependent. The probe-time path substitutes
/// outer refs and re-evaluates per outer row.
#[test]
fn q4_residual_with_outer_ref_uses_probe_time_path() {
    let mut e = fresh_engine();
    e.execute(
        "CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, \
              o_orderdate TEXT, o_orderpriority TEXT)",
    )
    .unwrap();
    e.execute(
        "CREATE TABLE lineitem (l_orderkey INTEGER, \
              l_receiptdate TEXT, l_commitdate TEXT)",
    )
    .unwrap();
    e.execute("INSERT INTO orders VALUES (1, '1993-08-15', '1-URGENT')")
        .unwrap();
    e.execute("INSERT INTO lineitem VALUES (1, '1993-08-20', '1993-08-10')")
        .unwrap();
    e.execute("INSERT INTO orders VALUES (2, '1993-09-15', '2-HIGH')")
        .unwrap();
    e.execute("INSERT INTO lineitem VALUES (2, '1993-09-10', '1993-09-20')")
        .unwrap();

    let (builds_before, probe_hits_before) = snapshot_v312_58_sprint5_diag();
    let sql = "SELECT o_orderpriority, COUNT(*) AS order_count \
        FROM orders \
        WHERE o_orderdate >= '1993-07-01' AND o_orderdate < '1993-10-01' \
          AND EXISTS (SELECT * FROM lineitem \
                      WHERE l_orderkey = o_orderkey \
                        AND l_commitdate < l_receiptdate \
                        AND o_orderkey = o_orderkey) \
        GROUP BY o_orderpriority \
        ORDER BY o_orderpriority";
    let r = e.execute(sql).unwrap();
    // Functional: same result as canonical Q4 (1-URGENT priority, count 1).
    assert_eq!(
        r.rows,
        vec![vec![Value::Text("1-URGENT".to_string()), Value::Integer(1)]],
        "Q4 with outer-ref tautology residual must produce the same result \
         as canonical Q4; got: {:?}",
        r.rows
    );
    // Behavioral: HSJ IS built (probe-time path takes over). The
    // counters are process-global `AtomicU64`; when cargo's test
    // runner executes the 3 tests in this binary in parallel,
    // parallel tests in the same binary ALSO bump the counter, so
    // we cannot use `assert_eq!(delta, 1)` for an exact count. We
    // require `delta >= 1` to verify the path was taken at least
    // once; functional correctness is independently verified above
    // via `r.rows`.
    let (builds_after, probe_hits_after) = snapshot_v312_58_sprint5_diag();
    let builds_delta = builds_after.saturating_sub(builds_before);
    let probe_hits_delta = probe_hits_after.saturating_sub(probe_hits_before);
    let snap = dump_v312_58_sprint5_diag();
    assert!(
        builds_delta >= 1,
        "HashSemiJoinIndex must be built at least once for the outer-ref \
         residual shape (probe-time path); got builds_delta={builds_delta} \
         (builds_before={builds_before} builds_after={builds_after}). \
         Snapshot:\n{snap}"
    );
    assert!(
        probe_hits_delta > 0,
        "Probe call site must be reached; got probe_hits_delta={probe_hits_delta} \
         (probe_hits_before={probe_hits_before} probe_hits_after={probe_hits_after}). \
         Snapshot:\n{snap}"
    );
}
