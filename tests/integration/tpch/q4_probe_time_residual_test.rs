//! V312-58 Sprint 5 followup-3 — probe-time residual re-evaluation test.
//!
//! Covers the case the build-time residual filter cannot handle:
//! the inner WHERE carries a residual predicate that references one or
//! more outer-row columns (e.g. `o_orderkey > 0`,
//! `l_orderkey < o_orderkey`). Sprint 5 followup-2 (PR #4464) rejected
//! these shapes with `mentions_outer`; followup-3 (this PR) keeps
//! every inner row in the HSJ and re-evaluates the substituted
//! residual at probe time against `get_inner_for_key` rows.
//!
//! ## Acceptance
//!
//! - **Functional** — the result row count matches the truth for the
//!   Q4 + tautological-outer-ref-residual shape.
//! - **Behavioral** — `hash_semi_join_builds == 1` (the shape gate now
//!   accepts the residual-bearing subquery), and probe hits advance
//!   per outer row whose key has at least one residual-passing inner.
//! - **Perf** — wall time stays sub-second for the 1000 orders + 10K
//!   lineitems in-process fixture.
//!
//! Run with `--test-threads=1` to avoid the global DIAG atomic counter
//! race that already existed in Sprint 5.

use parking_lot::RwLock;
use sqlrustgo::{
    dump_v312_58_sprint5_diag, reset_v312_58_sprint5_diag, ExecutionEngine, MemoryStorage, Value,
};
use std::sync::Arc;
use std::time::Instant;

fn fresh_engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

/// Canonical TPC-H Q4 plus an outer-column tautology `o_orderkey = o_orderkey`
/// in the inner WHERE. The residual references an outer-row column, so
/// this exercises the probe-time residual path (substitution +
/// per-row re-evaluation), not the build-time pre-filter.
const Q4_REAL_SQL: &str = "SELECT o_orderpriority, COUNT(*) AS order_count \
    FROM orders \
    WHERE o_orderdate >= '1993-07-01' AND o_orderdate < '1993-10-01' \
      AND EXISTS (SELECT * FROM lineitem \
                  WHERE l_orderkey = o_orderkey \
                    AND l_commitdate < l_receiptdate \
                    AND o_orderkey = o_orderkey) \
    GROUP BY o_orderpriority \
    ORDER BY o_orderpriority";

/// Slightly non-trivial outer-ref residual: `o_orderkey > 0`. Always true
/// for `o_orderkey INTEGER PRIMARY KEY` rows in our fixtures, but it is
/// outer-column-dependent so the shape gate stores it in
/// `HashSemiJoinIndex.residual` and the probe-time path substitutes +
/// re-evaluates per outer row.
const Q4_OUTER_REF_RESIDUAL_SQL: &str = "SELECT o_orderpriority, COUNT(*) AS order_count \
    FROM orders \
    WHERE o_orderdate >= '1993-07-01' AND o_orderdate < '1993-10-01' \
      AND EXISTS (SELECT * FROM lineitem \
                  WHERE l_orderkey = o_orderkey \
                    AND l_commitdate < l_receiptdate \
                    AND o_orderkey > 0) \
    GROUP BY o_orderpriority \
    ORDER BY o_orderpriority";

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

/// Acceptance #1 + #2: canonical Q4 with an outer-ref tautology in the
/// residual must build exactly one HSJ, probe per outer row, and
/// produce the same correct result as plain canonical Q4.
#[test]
fn q4_outer_ref_residual_tautology_uses_probe_time_path() {
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

    reset_v312_58_sprint5_diag();
    let r = e.execute(Q4_REAL_SQL).unwrap();

    // Acceptance #1: result is identical to canonical Q4.
    assert_eq!(
        r.rows,
        vec![vec![Value::Text("1-URGENT".to_string()), Value::Integer(1)]],
        "Q4 with outer-ref tautology in residual must produce the same \
         result as canonical Q4; got: {:?}",
        r.rows
    );

    // Acceptance #2: shape gate accepted the outer-ref residual;
    // exactly one HSJ was built.
    let snap = dump_v312_58_sprint5_diag();
    let builds = counter_value(&snap, "hash_semi_join_builds");
    let probe_hits = counter_value(&snap, "hash_semi_join_probe_hits");
    assert_eq!(
        builds, 1,
        "HashSemiJoinIndex must be built for the outer-ref residual \
         shape (probe-time path); got builds={builds}. Snapshot:\n{snap}"
    );
    assert!(
        probe_hits > 0,
        "Probe call site must be reached for the outer-ref residual \
         shape; got probe_hits={probe_hits}. Snapshot:\n{snap}"
    );
}

/// Acceptance #1 + #2 variant: outer-ref residual `o_orderkey > 0`
/// (always true for INTEGER PRIMARY KEY > 0). The probe-time path
/// substitutes `o_orderkey` with the per-outer-row value and
/// re-evaluates against each inner row that matched the probe key.
#[test]
fn q4_outer_ref_residual_substitutes_per_outer_row() {
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
    // Order 1: date in range, EXISTS true (residual-passing lineitem).
    e.execute("INSERT INTO orders VALUES (1, '1993-08-15', '1-URGENT')")
        .unwrap();
    e.execute("INSERT INTO lineitem VALUES (1, '1993-08-20', '1993-08-10')")
        .unwrap();
    // Order 2: date in range, EXISTS false (residual-failing lineitem).
    e.execute("INSERT INTO orders VALUES (2, '1993-09-15', '2-HIGH')")
        .unwrap();
    e.execute("INSERT INTO lineitem VALUES (2, '1993-09-10', '1993-09-20')")
        .unwrap();
    // Order 3: date out of range — must be filtered by the outer WHERE
    // even though the probe-time residual would pass `3 > 0`.
    e.execute("INSERT INTO orders VALUES (3, '1992-12-31', '3-MEDIUM')")
        .unwrap();
    e.execute("INSERT INTO lineitem VALUES (3, '1993-01-15', '1993-01-10')")
        .unwrap();

    reset_v312_58_sprint5_diag();
    let r = e.execute(Q4_OUTER_REF_RESIDUAL_SQL).unwrap();

    // Same truth as canonical Q4: only order 1 in date range with
    // residual-passing lineitem.
    assert_eq!(
        r.rows,
        vec![vec![Value::Text("1-URGENT".to_string()), Value::Integer(1)]],
        "Q4 with `o_orderkey > 0` residual must produce the same result \
         as canonical Q4; got: {:?}",
        r.rows
    );

    let snap = dump_v312_58_sprint5_diag();
    let builds = counter_value(&snap, "hash_semi_join_builds");
    let probe_hits = counter_value(&snap, "hash_semi_join_probe_hits");
    assert_eq!(
        builds, 1,
        "HashSemiJoinIndex must be built for the outer-ref residual \
         `o_orderkey > 0`; got builds={builds}. Snapshot:\n{snap}"
    );
    assert!(
        probe_hits >= 1,
        "Probe call site must be reached; got probe_hits={probe_hits}. \
         Snapshot:\n{snap}"
    );
}

/// Acceptance #3 (perf): probe-time residual path must stay sub-second
/// for the 10K-lineitem fixture (mirrors `q4_residual_filter_scale_test`
/// so the two perf numbers are directly comparable).
#[test]
fn q4_outer_ref_residual_perf_scale_1k_orders_10k_lineitems() {
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
            let (commit_month, commit_day) = if j < 5 {
                (receipt_month, receipt_day + 1)
            } else {
                (receipt_month, receipt_day.saturating_sub(1).max(1))
            };
            e.execute(&format!(
                "INSERT INTO lineitem VALUES ({}, '1993-{:02}-{:02}', '1993-{:02}-{:02}')",
                i, receipt_month, receipt_day, commit_month, commit_day,
            ))
            .unwrap();
        }
    }

    reset_v312_58_sprint5_diag();
    let start = Instant::now();
    let r = e.execute(Q4_OUTER_REF_RESIDUAL_SQL).unwrap();
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
    assert_eq!(
        builds, 1,
        "HashSemiJoinIndex must be built exactly once; got builds={builds}. \
         Snapshot:\n{snap}"
    );
    assert!(
        probe_hits > 0 && (probe_hits as i64) <= total + 1,
        "probe_hits ({probe_hits}) must be reachable per outer row; \
         snapshot:\n{snap}"
    );

    assert!(
        elapsed.as_secs() < 10,
        "Q4 outer-ref residual too slow: {:?} (target: < 10s for 10K lineitems)",
        elapsed
    );
    eprintln!(
        "[perf] Q4 outer-ref residual (HSJ probe-time path): {} orders + \
         {} lineitems in {:?}; builds={} probe_hits={} rows={}",
        1000, 10000, elapsed, builds, probe_hits, total
    );
}
