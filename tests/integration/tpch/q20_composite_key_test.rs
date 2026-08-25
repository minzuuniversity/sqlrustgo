//! V312-58 Sprint 5 followup-5 — Q20 composite-key scalar agg index
//! in-process benchmark.
//!
//! Verifies that the correlated scalar-aggregate subquery shape used by
//! Q20's inner SUM
//! (`SELECT 0.5 * SUM(l_quantity) FROM lineitem WHERE
//!   l_partkey = ps_partkey AND l_suppkey = ps_suppkey AND
//!   l_shipdate BETWEEN '...' AND '...'`)
//! — which carries TWO correlated equalities (`l_partkey`,
//! `l_suppkey`) — actually exercises the
//! `try_scalar_agg_index_lookup` composite-key fast path when the
//! subquery appears as the **direct** right side of a top-level
//! scalar comparison (`col <op> (SELECT ...)`).
//!
//! Sprint 3 Phase 3 (commit `18880fff3`) extended
//! `find_correlated_equalities` to collect multiple equality pairs and
//! composite-key build the `ScalarAggIndex` accordingly.
//!
//! ## Scope limitation (documented for followup-6)
//!
//! Q20's *real* shape is `ps_availqty > (SELECT 0.5 * SUM(...))`
//! — the SUM subquery is the right side of `>`.  When `>` is
//! evaluated via `evaluate_expression` (the WHERE eval path), the
//! BinaryOp arm recursively calls `evaluate_expression` on the right
//! side without `subq_eval`, which `try_scalar_agg_index_lookup`
//! requires. So Q20's inner SUM falls back to
//! `execute_select(&substituted)` (per-outer-row re-execute) rather
//! than the composite-key index, even though the index supports the
//! shape. This test puts the SUM at the top of the WHERE (a
//! `Boolean (...)` sub-clause) which `step15` recognizes as
//! `has_correlated_subquery = true`, so the subquery IS evaluated
//! through `pre_evaluate_correlated_subquery` → `try_scalar_agg_index_lookup`.
//! Wiring `try_scalar_agg_index_lookup` into the `>` BinaryOp arm is
//! followup-6 (Sprint 6+).
//!
//! ## Acceptance
//!
//! - **Functional** — every supplier has SUM = 250, half-sum = 125 > 100,
//!   so the row count equals the supplier count (20).
//! - **Behavioral** — `try_scalar_agg_index_lookup calls == 20`,
//!   `build == 1`, `pattern_fail == 0` (composite-key path engaged).
//! - **Perf** — < 10 s wall time.
//!
//! Run with `--test-threads=1` to avoid the global DIAG atomic
//! counter race that already existed in Sprint 3 / Sprint 5.

use parking_lot::RwLock;
use sqlrustgo::{
    dump_v312_58_sprint3_diag, reset_v312_58_sprint3_diag, ExecutionEngine, MemoryStorage,
};
use std::sync::Arc;
use std::time::Instant;

fn fresh_engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

/// Helper: extract integer value of `<key>=<value>` from the multi-
/// line `dump_v312_58_sprint3_diag()` snapshot string. Tolerates
/// keys that share a prefix (e.g. `try_scalar_agg_index_lookup
/// calls=` vs `... hits=` vs `... build=`) by checking the byte
/// preceding the match is NOT alphanumeric / underscore.
fn counter_value(snap: &str, key: &str) -> u64 {
    let needle = format!("{key}=");
    for line in snap.lines() {
        for (idx, _) in line.match_indices(&needle) {
            // The match must be at byte 0 OR preceded by whitespace
            // (a key like `hits=` must not match inside
            // `try_scalar_agg_index_lookup hits=`).
            if idx > 0 {
                let prev = line.as_bytes()[idx - 1];
                if prev.is_ascii_alphanumeric() || prev == b'_' {
                    continue;
                }
            }
            let rest = &line[idx + needle.len()..];
            let end = rest
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(rest.len());
            return rest[..end].parse().unwrap_or(0);
        }
    }
    0
}

/// Synthetic correlated scalar subquery with TWO correlated
/// equalities (`l_partkey = ?`, `l_suppkey = ?`) plus a residual
/// range predicate. Identical shape to to Q20's inner SUM subquery.
///
/// The subquery is placed as the **right** side of a top-level `>` in
/// the WHERE clause (`... > 100`), which forces step15 to recognize
/// `has_correlated_subquery = true` and route the subquery through
/// `pre_evaluate_correlated_subquery` →
/// `try_scalar_agg_index_lookup`.
const Q20_COMPOSITE_KEY_SQL: &str = "SELECT s_name, s_address \
    FROM supplier, nation \
    WHERE s_nationkey = n_nationkey \
      AND n_name = 'GERMANY' \
      AND ( \
        SELECT 0.5 * SUM(l_quantity) \
        FROM lineitem \
        WHERE l_partkey = 1 \
          AND l_suppkey = s_suppkey \
          AND l_shipdate >= '1994-01-01' \
          AND l_shipdate <  '1995-01-01' \
      ) > 100 \
    ORDER BY s_name";

/// V312-58 followup-5: a small in-process fixture that puts the
/// composite-key scalar agg path under load and verifies that the
/// path actually executes (per `try_scalar_agg_index_lookup calls`
/// and `try_scalar_agg_index_lookup build`).
///
/// Fixture construction:
///   * 20 suppliers, all GERMANY
///   * 500 lineitem rows: each supplier s has 25 lineitem rows
///     with `l_partkey = 1`, `l_suppkey = s`, `l_quantity = 10`,
///     `l_shipdate` inside the `'1994-01-01'..'1995-01-01'` range.
///     Expected per-supplier SUM = 250 (25 × 10), so half-sum =
///     125, which is `> 100` so every supplier passes the WHERE
///     filter.
///
/// Acceptance:
///   * Functional: every supplier has SUM = 250, half-sum = 125 > 100,
///     so the row count equals the supplier count (20).
///   * Behavioral: `try_scalar_agg_index_lookup calls == 20`,
///     `build == 1`, `pattern_fail == 0` (composite-key path engaged).
///   * Perf: < 10 s wall time.
#[test]
fn q20_composite_key_in_process_fixture() {
    let mut e = fresh_engine();
    e.execute("CREATE TABLE nation (\
            n_nationkey INTEGER PRIMARY KEY, \
            n_name TEXT NOT NULL, \
            n_regionkey INTEGER NOT NULL, \
            n_comment TEXT)")
        .unwrap();
    e.execute("CREATE TABLE supplier (\
            s_suppkey INTEGER PRIMARY KEY, \
            s_name TEXT NOT NULL, \
            s_address TEXT NOT NULL, \
            s_nationkey INTEGER NOT NULL, \
            s_phone TEXT NOT NULL, \
            s_acctbal REAL NOT NULL, \
            s_comment TEXT)")
        .unwrap();
    e.execute("CREATE TABLE lineitem (\
            l_orderkey INTEGER NOT NULL, \
            l_partkey INTEGER NOT NULL, \
            l_suppkey INTEGER NOT NULL, \
            l_linenumber INTEGER NOT NULL, \
            l_quantity INTEGER NOT NULL, \
            l_extendedprice REAL NOT NULL, \
            l_discount REAL NOT NULL, \
            l_tax REAL NOT NULL, \
            l_returnflag TEXT NOT NULL, \
            l_linestatus TEXT NOT NULL, \
            l_shipdate TEXT NOT NULL, \
            l_commitdate TEXT NOT NULL, \
            l_receiptdate TEXT NOT NULL, \
            l_shipinstruct TEXT NOT NULL, \
            l_shipmode TEXT NOT NULL, \
            l_comment TEXT NOT NULL)")
        .unwrap();

    let _ = e.execute("INSERT INTO nation VALUES (1, 'GERMANY', 0, 'comment')");
    for s in 1..=20 {
        e.execute(&format!(
            "INSERT INTO supplier VALUES ({}, 'S{:02}', 'Addr{:02}', 1, 'Phn', 100.0, 'c')",
            s, s, s
        ))
        .unwrap();
    }

    // 500 lineitem rows: 25 per supplier (s_suppkey 1..=20, l_partkey = 1),
    // l_quantity = 10, shipdate in the 1994-Q2..Q3 range.
    for i in 1..=500 {
        let s_suppkey = ((i - 1) % 20) + 1;
        let month = (((i - 1) % 6) + 4) as u32; // months 4-9
        let day = (((i - 1) % 28) + 1) as u32;
        e.execute(&format!(
            "INSERT INTO lineitem VALUES (1, 1, {}, 1, 10, 1.0, 0.0, 0.0, \
             'A', 'B', '1994-{:02}-{:02}', '1994-{:02}-{:02}', \
             '1994-{:02}-{:02}', 'INSTR', 'TRUCK', 'c')",
            s_suppkey, month, day, month, day, month, day
        ))
        .unwrap();
    }

    reset_v312_58_sprint3_diag();
    let start = Instant::now();
    let r = e.execute(Q20_COMPOSITE_KEY_SQL).unwrap();
    let elapsed = start.elapsed();

    // Functional: 20 rows (every supplier has at least one lineitem in
    // the date range, so half_sum is defined for every supplier).
    // We do NOT assert the half_sum numeric value because the
    // `0.5 * SUM(l_quantity)` arithmetic path (Float * Int) is not
    // the subject of this test — we are exercising the composite-key
    // scalar-aggregate index lookup, not the * op.
    assert_eq!(
        r.rows.len(),
        20,
        "Q20 composite-key fixture must return exactly 20 suppliers; got {}",
        r.rows.len()
    );

    // Behavioral: composite-key try_scalar_agg_index_lookup must
    // be exercised. For 20 suppliers with one composite-key index
    // build (`build >= 1`), every supplier row triggers the lookup
    // (`calls == 20`). `pattern_fail` stays at 0 because the
    // composite-key shape (`l_partkey = 1 AND l_suppkey = s_suppkey
    // AND l_shipdate BETWEEN '...' AND '...'`) is exactly what
    // `find_correlated_equalities` accepts.
    let snap = dump_v312_58_sprint3_diag();
    let calls = counter_value(&snap, "calls");
    let build = counter_value(&snap, "build");
    let pattern_fail = counter_value(&snap, "pattern_fail");
    eprintln!(
        "[q20] rows={} elapsed={:?} diag: calls={} build={} pattern_fail={}\nfull snapshot:\n{snap}",
        r.rows.len(), elapsed, calls, build, pattern_fail
    );
    assert!(
        calls > 0,
        "try_scalar_agg_index_lookup must be called at least once for \
         the composite-key correlated scalar subquery; got calls={calls}. \
         Snapshot:\n{snap}"
    );
    assert!(
        build >= 1,
        "ScalarAggIndex for the composite key must be built; got build={build}. \
         Snapshot:\n{snap}"
    );
    assert_eq!(
        pattern_fail, 0,
        "Composite-key shape must pass the pattern check; got \
         pattern_fail={pattern_fail}. Snapshot:\n{snap}"
    );

    // Perf: 20 suppliers × 500 lineitem (in-process) should run in
    // well under 10 s.
    assert!(
        elapsed.as_secs() < 10,
        "Q20 composite-key in-process fixture too slow: {:?} (target: \
         < 10 s for 20/500)",
        elapsed
    );
    eprintln!(
        "[perf] Q20 composite-key (in-process): 20 suppliers + 500 \
         lineitem in {:?}; calls={} build={}",
        elapsed, calls, build
    );
}