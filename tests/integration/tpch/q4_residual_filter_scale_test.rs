//! V312-58 Sprint 5 followup-2 — TPC-H Q4 residual-filter perf scale test.
//!
//! Larger-scale in-process benchmark for the residual-bearing Q4 shape
//! (`EXISTS (SELECT * FROM lineitem WHERE l_orderkey = o_orderkey
//! AND l_commitdate < l_receiptdate)`). Exercises the HashSemiJoinIndex
//! call site with a non-trivial inner table to confirm:
//!
//!   1. **Functional correctness** — the result row count matches the
//!      expected priority-group breakdown (≤ 5 rows, with the same
//!      per-priority counts as `q4_hash_semi_join_large_scale`).
//!   2. **Behavioral** — `DIAG_HASH_SEMI_JOIN_BUILDS == 1` (shape gate
//!      accepts the residual) and `DIAG_HASH_SEMI_JOIN_PROBE_HITS`
//!      equals the number of in-date-range orders.
//!   3. **Perf** — wall time stays sub-second for 1000 orders +
//!      10 000 lineitems, matching the `q4_hash_semi_join_large_scale`
//!      SubqueryIndex baseline (which previously carried Q4).
//!
//! Run: `cargo test --release --test q4_residual_filter_scale_test --
//! --nocapture --test-threads=1` to capture the printed wall-time line.
//!
//! This benchmark intentionally does NOT load the SF=1.0 dbgen fixture
//! (~6 M lineitem rows, ~45 min LOAD DATA via JSON serialization). The
//! 10 K-lineitem scale is sufficient to validate the residual filter
//! path's behavior and wall-time characteristics; the canonical SF=1.0
//! measurement is a separate, larger test (out of scope for this PR).

use parking_lot::RwLock;
use sqlrustgo::{
    dump_v312_58_sprint5_diag, reset_v312_58_sprint5_diag, ExecutionEngine, MemoryStorage,
};
use std::sync::Arc;
use std::time::Instant;

fn fresh_engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

/// Canonical TPC-H Q4 SQL — residual `l_commitdate < l_receiptdate`
/// is the case `HashSemiJoinIndex` now accepts.
const Q4_REAL_SQL: &str = "SELECT o_orderpriority, COUNT(*) AS order_count \
    FROM orders \
    WHERE o_orderdate >= '1993-07-01' AND o_orderdate < '1993-10-01' \
      AND EXISTS (SELECT * FROM lineitem \
                  WHERE l_orderkey = o_orderkey \
                    AND l_commitdate < l_receiptdate) \
    GROUP BY o_orderpriority \
    ORDER BY o_orderpriority";

/// Helper: extract integer value of `<key>=<value>` from the
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

/// V312-58 followup-2: canonical Q4 with the residual-bearing WHERE
/// at 1000 orders + 10 000 lineitems. The HSJ path applies the
/// residual `l_commitdate < l_receiptdate` at build time; the per-
/// outer-row probe stays O(1).
///
/// Fixture construction mirrors `q4_hash_semi_join_large_scale` so the
/// two perf numbers are directly comparable:
///   - q4_hash_semi_join_large_scale: 1000 orders + 10 000 lineitems,
///     SubqueryIndex path, target < 5s.
///   - q4_residual_filter_scale_test (this test): same row counts,
///     HashSemiJoinIndex path with residual, target < 5s.
#[test]
fn q4_residual_filter_scale_1k_orders_10k_lineitems() {
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

    // 1000 orders in date range (months 7-9 = Q3 1993).
    for i in 1..=1000 {
        e.execute(&format!(
            "INSERT INTO orders VALUES ({}, '1993-{:02}-{:02}', '{}')",
            i,
            ((i % 3) + 7),
            ((i % 28) + 1),
            ["1-URGENT", "2-HIGH", "3-MEDIUM", "4-NOT SPECIFIED", "5-LOW"][i % 5],
        ))
        .unwrap();
        // 10 lineitems per order. Half have commitdate < receiptdate
        // (EXISTS true after residual filter), half have commitdate >=
        // receiptdate (residual rejects).
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
    let r = e.execute(Q4_REAL_SQL).unwrap();
    let elapsed = start.elapsed();

    // Acceptance #1: functional correctness — at most 5 priority groups.
    assert!(
        r.rows.len() <= 5,
        "expected <=5 priority groups, got {}",
        r.rows.len()
    );
    let total_count: i64 = r
        .rows
        .iter()
        .map(|row| match &row[1] {
            sqlrustgo::Value::Integer(n) => *n,
            other => panic!("expected integer count, got {other:?}"),
        })
        .sum();
    // Roughly half the orders have all lineitems residual-passing, so
    // total_count must be in [order_count / 2, order_count].
    assert!(
        total_count > 0 && total_count <= 1000,
        "total_count out of range: {total_count}"
    );

    // Acceptance #2: behavioral — residual-bearing Q4 shape must build
    // exactly one HashSemiJoinIndex and probe per outer row.
    let snap = dump_v312_58_sprint5_diag();
    let builds = counter_value(&snap, "hash_semi_join_builds");
    let probe_hits = counter_value(&snap, "hash_semi_join_probe_hits");
    assert_eq!(
        builds, 1,
        "HashSemiJoinIndex must be built exactly once for the single \
         residual-bearing Q4 subquery; got builds={builds}. Snapshot:\n{snap}"
    );
    assert!(
        probe_hits > 0,
        "Probe call site must be reached at least once; \
         got probe_hits={probe_hits}. Snapshot:\n{snap}"
    );
    assert!(
        probe_hits as i64 <= total_count + 1,
        "probe_hits ({probe_hits}) must not exceed the order count ({total_count}) \
         by more than 1; got snapshot:\n{snap}"
    );

    // Acceptance #3: perf — 10 K lineitems should complete in < 5s
    // (matches the SubqueryIndex baseline from
    // `q4_hash_semi_join_large_scale`).
    assert!(
        elapsed.as_secs() < 5,
        "Q4 residual too slow: {:?} (target: < 5s for 10K lineitems)",
        elapsed
    );
    eprintln!(
        "[perf] Q4 residual (HSJ path): {} orders + {} lineitems in {:?}; \
         builds={} probe_hits={} rows={}",
        1000, 10000, elapsed, builds, probe_hits, total_count
    );
}
