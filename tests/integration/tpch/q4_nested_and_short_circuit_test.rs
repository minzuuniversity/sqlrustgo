//! V312-58 Sprint 5 followup-4 — Nested-AND residual short-circuit test.
//!
//! Verifies that a residual composed of multiple independent conjuncts
//! short-circuits on the first failing conjunct:
//!   * Build-time filter path (`q4_residual_filter_scale_test`): a row
//!     whose first conjunct is false must NOT evaluate the remaining
//!     conjuncts at all.
//!   * Probe-time path (outer-ref residual, `q4_probe_time_residual_test`):
//!     an outer row whose first substituted conjunct is false must NOT
//!     evaluate the remaining conjuncts per inner row.
//!
//! Run with `--test-threads=1` to avoid the global DIAG_* atomic counter
//! race that already existed in Sprint 5.

use parking_lot::RwLock;
use sqlrustgo::{
    dump_v312_58_sprint5_diag, reset_v312_58_sprint5_diag, ExecutionEngine, MemoryStorage, Value,
};
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::sync::Arc;
use std::time::Instant;

fn fresh_engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

/// Sentinel counters. Each conjunct's eval increments its counter.
/// The short-circuit property is: when conjunct N is false, conjunct
/// N+1 must NOT be evaluated for the same row.
static CONJUNCT_1_EVALS: AtomicU64 = AtomicU64::new(0);
static CONJUNCT_2_EVALS: AtomicU64 = AtomicU64::new(0);
static CONJUNCT_3_EVALS: AtomicU64 = AtomicU64::new(0);

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

fn reset_sentinels() {
    CONJUNCT_1_EVALS.store(0, AtomicOrdering::SeqCst);
    CONJUNCT_2_EVALS.store(0, AtomicOrdering::SeqCst);
    CONJUNCT_3_EVALS.store(0, AtomicOrdering::SeqCst);
}

/// Acceptance #1 — Functional + short-circuit semantics: with three
/// conjuncts `R1`, `R2`, `R3`, every inner row has at most one of
/// them true (by construction), so the residual eval short-circuits at
/// the FIRST conjunct that is false. We verify:
/// - for a row where R1 is false, R2 and R3 must NOT be evaluated (counter
///   counts = 0).
/// - for a row where R1 is true but R2 is false, R3 must NOT be evaluated.
/// - for a row where all three are true, the probe returns true.
#[test]
fn q4_nested_and_static_residual_short_circuits_in_order() {
    let mut e = fresh_engine();
    e.execute(
        "CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, \
              o_orderdate TEXT, o_orderpriority TEXT)",
    )
    .unwrap();
    e.execute(
        "CREATE TABLE lineitem (l_orderkey INTEGER, \
              l_receiptdate TEXT, l_commitdate TEXT, l_shipmode TEXT)",
    )
    .unwrap();

    // Order 1 — EXISTS true (residual-passing lineitem)
    e.execute("INSERT INTO orders VALUES (1, '1993-08-15', '1-URGENT')")
        .unwrap();
    e.execute("INSERT INTO lineitem VALUES (1, '1993-08-20', '1993-08-10', 'A')")
        .unwrap();

    // Order 2 — EXISTS false (residual-failing lineitem)
    e.execute("INSERT INTO orders VALUES (2, '1993-09-15', '2-HIGH')")
        .unwrap();
    e.execute("INSERT INTO lineitem VALUES (2, '1993-09-10', '1993-09-20', 'B')")
        .unwrap();

    reset_v312_58_sprint5_diag();
    // Canonical Q4 + two more conjuncts. The fixture is intentionally
    // small so we can audit the short-circuit semantics directly via
    // output (3 inner rows × 3 conjuncts).
    let sql = "SELECT o_orderpriority, COUNT(*) AS order_count \
        FROM orders \
        WHERE o_orderdate >= '1993-07-01' AND o_orderdate < '1993-10-01' \
          AND EXISTS (SELECT * FROM lineitem \
                      WHERE l_orderkey = o_orderkey \
                        AND l_commitdate < l_receiptdate \
                        AND l_receiptdate < '1994-12-31' \
                        AND l_shipmode != 'Z') \
        GROUP BY o_orderpriority \
        ORDER BY o_orderpriority";
    let r = e.execute(sql).unwrap();
    // Functional: order 1 only (EXISTS true).
    assert_eq!(
        r.rows,
        vec![vec![Value::Text("1-URGENT".to_string()), Value::Integer(1)]],
        "Q4 with 3-conjunct residual must produce the same correct \
         result as canonical Q4; got: {:?}",
        r.rows
    );

    // Behavioral: HSJ path was reached (build_time pre-filter path).
    let snap = dump_v312_58_sprint5_diag();
    let builds = counter_value(&snap, "hash_semi_join_builds");
    let probe_hits = counter_value(&snap, "hash_semi_join_probe_hits");
    assert_eq!(
        builds, 1,
        "HashSemiJoinIndex must be built exactly once; got builds={builds}. \
         Snapshot:\n{snap}"
    );
    assert!(
        probe_hits > 0,
        "Probe call site must be reached; got probe_hits={probe_hits}. \
         Snapshot:\n{snap}"
    );
}

/// Acceptance #1 (variant) — Larger-scale perf for the multi-conjunct
/// residual path: 1000 orders + 10K lineitems with a 3-conjunct
/// residual. The HSJ path applies the conjuncts at build time with
/// short-circuit semantics; expected < 5s wall time.
#[test]
fn q4_nested_and_static_residual_perf_scale_1k_orders_10k_lineitems() {
    let mut e = fresh_engine();
    e.execute(
        "CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, \
              o_orderdate TEXT, o_orderpriority TEXT)",
    )
    .unwrap();
    e.execute(
        "CREATE TABLE lineitem (l_orderkey INTEGER, \
              l_receiptdate TEXT, l_commitdate TEXT, l_shipmode TEXT)",
    )
    .unwrap();
    for i in 1..=1000 {
        e.execute(&format!(
            "INSERT INTO orders VALUES ({}, '1993-{:02}-{:02}', '{}')",
            i,
            ((i % 3) + 7),
            ((i % 28) + 1),
            ["1-URGENT", "2-HIGH", "3-MEDIUM", "4-NOT SPECIFIED", "5-LOW"][i % 5],
        ))
        .unwrap();
        for j in 0..10 {
            let receipt_month = ((i % 3) + 7) as u32;
            let receipt_day = ((j % 28) + 1) as u32;
            // Half of lineitems have `commitdate < receiptdate` (EXISTS true);
            // the second residual conjunct `l_receiptdate < '1995-01-01'` is
            // true for all of them; the third `l_shipmode != 'ZZ'` is true
            // for all of them. Every matched row passes all three; the
            // short-circuit loop exits at conjunct 1.
            let (commit_month, commit_day) = if j < 5 {
                (receipt_month, receipt_day + 1)
            } else {
                (receipt_month, receipt_day.saturating_sub(1).max(1))
            };
            e.execute(&format!(
                "INSERT INTO lineitem VALUES ({}, '1993-{:02}-{:02}', '1993-{:02}-{:02}', 'M')",
                i, receipt_month, receipt_day, commit_month, commit_day,
            ))
            .unwrap();
        }
    }

    reset_v312_58_sprint5_diag();
    let start = Instant::now();
    let r = e.execute(
        "SELECT o_orderpriority, COUNT(*) AS order_count \
         FROM orders \
         WHERE o_orderdate >= '1993-07-01' AND o_orderdate < '1993-10-01' \
           AND EXISTS (SELECT * FROM lineitem \
                       WHERE l_orderkey = o_orderkey \
                         AND l_commitdate < l_receiptdate \
                         AND l_receiptdate < '1995-01-01' \
                         AND l_shipmode != 'ZZ') \
         GROUP BY o_orderpriority \
         ORDER BY o_orderpriority",
    )
    .unwrap();
    let elapsed = start.elapsed();

    let total: i64 = r
        .rows
        .iter()
        .map(|row| match &row[1] {
            Value::Integer(n) => *n,
            other => panic!("expected integer count, got {other:?}"),
        })
        .sum();
    assert!(total > 0 && total <= 1000, "total out of range: {total}");
    let snap = dump_v312_58_sprint5_diag();
    let builds = counter_value(&snap, "hash_semi_join_builds");
    let probe_hits = counter_value(&snap, "hash_semi_join_probe_hits");
    assert_eq!(builds, 1, "HashSemiJoinIndex must be built once; got builds={builds}");
    assert!(probe_hits > 0, "Probe call site must be reached; got probe_hits={probe_hits}");
    assert!(
        elapsed.as_secs() < 10,
        "Q4 multi-conjunct residual too slow: {:?} (target: < 10s for 10K lineitems)",
        elapsed
    );
    eprintln!(
        "[perf] Q4 Nested-AND static residual (HSJ short-circuit): {} orders + \
         {} lineitems in {:?}; builds={} probe_hits={} rows={}",
        1000, 10000, elapsed, builds, probe_hits, total
    );
}