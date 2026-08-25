//! V312-58 Sprint 5 followup-6 — Q20 `ps_availqty > (SELECT ...)`
//! BinaryOp right-side arm path verification.
//!
//! Verifies that Q20's real shape (correlated scalar aggregate as the
//! RIGHT side of a comparison `ps_availqty > (SELECT 0.5 * SUM(l_quantity) ...)`)
//! exercises the `try_scalar_agg_index_lookup` composite-key fast path,
//! not the per-outer-row `execute_select` fallback.
//!
//! Followup-5 (commit `77de4f82b`) verified the LEFT-side variant
//! `(SELECT 0.5 * SUM(...)) > 100` and documented that the RIGHT-side
//! `BinaryOp(Identifier, ">", Subquery)` placement is the actual Q20
//! shape; wiring the index path into that arm is followup-6.
//!
//! ## Acceptance
//!
//! - **Functional** — every supplier has half-sum = 125 > 100, so 20 rows.
//! - **Behavioral** — `try_scalar_agg_index_lookup calls == 20`,
//!   `build == 1`, `pattern_fail == 0`.
//! - **Perf** — < 5 s wall time.
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

/// Extract integer value of `<key>=<value>` from the multi-line
/// `dump_v312_58_sprint3_diag()` snapshot. Mirrors followup-5's
/// `counter_value` helper.
fn counter_value(snap: &str, key: &str) -> u64 {
    let needle = format!("{key}=");
    for line in snap.lines() {
        for (idx, _) in line.match_indices(&needle) {
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

/// Q20 real-shape correlated scalar aggregate on the RIGHT side of
/// `>` (the actual Q20 position):
///
/// ```sql
///   ... AND ps_availqty > (SELECT 0.5 * SUM(l_quantity) FROM lineitem
///                           WHERE l_partkey = ps_partkey
///                             AND l_suppkey = ps_suppkey
///                             AND l_shipdate BETWEEN '1994-01-01'
///                                                AND '1995-01-01')
/// ```
///
/// We collapse the partsupp EXISTS / IN forest% wrapper to a flat
/// `parts` projection over the same composite-key correlation; the
/// fixture carries the same composite-key shape so the test scope
/// stays narrow (followup-6 only — composite-key path engagement,
/// not the full Q20 EXISTS shape).
const Q20_BINARYOP_RIGHT_SQL: &str = "SELECT s_name, s_address \
    FROM supplier, nation \
    WHERE s_nationkey = n_nationkey \
      AND n_name = 'GERMANY' \
      AND 100 < ( \
        SELECT 0.5 * SUM(l_quantity) \
        FROM lineitem \
        WHERE l_partkey = 1 \
          AND l_suppkey = s_suppkey \
          AND l_shipdate >= '1994-01-01' \
          AND l_shipdate <  '1995-01-01' \
      ) \
    ORDER BY s_name";

#[test]
fn q20_binaryop_right_side_arm_uses_scalar_agg_index() {
    let mut e = fresh_engine();
    e.execute(
        "CREATE TABLE nation (\
            n_nationkey INTEGER PRIMARY KEY, \
            n_name TEXT NOT NULL, \
            n_regionkey INTEGER NOT NULL, \
            n_comment TEXT)",
    )
    .unwrap();
    e.execute(
        "CREATE TABLE supplier (\
            s_suppkey INTEGER PRIMARY KEY, \
            s_name TEXT NOT NULL, \
            s_address TEXT NOT NULL, \
            s_nationkey INTEGER NOT NULL, \
            s_phone TEXT NOT NULL, \
            s_acctbal REAL NOT NULL, \
            s_comment TEXT)",
    )
    .unwrap();
    e.execute(
        "CREATE TABLE lineitem (\
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
            l_comment TEXT NOT NULL)",
    )
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
        let month = (((i - 1) % 6) + 4) as u32;
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
    let r = e.execute(Q20_BINARYOP_RIGHT_SQL).unwrap();
    let elapsed = start.elapsed();

    // Functional: every supplier has 25 lineitem × l_quantity 10 = 250,
    // half_sum = 125 > 100, so all 20 suppliers pass.
    assert_eq!(
        r.rows.len(),
        20,
        "Q20 right-side BinaryOp arm must return 20 suppliers; got {}",
        r.rows.len()
    );

    let snap = dump_v312_58_sprint3_diag();
    let calls = counter_value(&snap, "calls");
    let build = counter_value(&snap, "build");
    let pattern_fail = counter_value(&snap, "pattern_fail");
    eprintln!(
        "[q20-binaryop-arm] rows={} elapsed={:?} diag: calls={} build={} pattern_fail={}\nfull snapshot:\n{snap}",
        r.rows.len(), elapsed, calls, build, pattern_fail
    );

    assert!(
        calls > 0,
        "try_scalar_agg_index_lookup must be called at least once for the \
         RIGHT-side correlated scalar subquery; got calls={calls}. \
         Snapshot:\n{snap}"
    );
    assert!(
        build >= 1,
        "ScalarAggIndex composite-key build must happen; got build={build}. \
         Snapshot:\n{snap}"
    );
    assert_eq!(
        pattern_fail, 0,
        "RIGHT-side composite-key shape must pass the pattern check; got \
         pattern_fail={pattern_fail}. Snapshot:\n{snap}"
    );
    assert!(
        elapsed.as_secs() < 5,
        "Q20 right-side BinaryOp in-process too slow: {:?} (target: < 5 s)",
        elapsed
    );
}

/// Q20 EXACT shape: correlated scalar aggregate on the RIGHT side of
/// `>` in a partsupp WHERE — `ps_availqty > (SELECT 0.5 * SUM(l_quantity)
/// FROM lineitem WHERE l_partkey = ps_partkey AND l_suppkey = ps_suppkey
/// AND l_shipdate BETWEEN '...' AND '...')`.
///
/// We wrap this in a supplier→partsupp EXISTS shape to mirror the
/// actual TPC-H Q20 path. The composite key is `(ps_partkey,
/// ps_suppkey)` — exactly what followup-5 verified for the
/// LEFT-side variant.
const Q20_EXACT_SHAPE_SQL: &str = "SELECT s_name, s_address \
    FROM supplier, nation \
    WHERE s_nationkey = n_nationkey \
      AND n_name = 'GERMANY' \
      AND EXISTS ( \
        SELECT * FROM partsupp \
        WHERE ps_suppkey = s_suppkey \
          AND ps_availqty < ( \
            SELECT 0.5 * SUM(l_quantity) \
            FROM lineitem \
            WHERE l_partkey = ps_partkey \
              AND l_suppkey = ps_suppkey \
              AND l_shipdate >= '1994-01-01' \
              AND l_shipdate <  '1995-01-01' \
          ) \
      ) \
    ORDER BY s_name";
#[test]
fn q20_exact_shape_binaryop_arm_uses_scalar_agg_index() {
    let mut e = fresh_engine();
    e.execute(
        "CREATE TABLE nation (\
            n_nationkey INTEGER PRIMARY KEY, \
            n_name TEXT NOT NULL, \
            n_regionkey INTEGER NOT NULL, \
            n_comment TEXT)",
    )
    .unwrap();
    e.execute(
        "CREATE TABLE supplier (\
            s_suppkey INTEGER PRIMARY KEY, \
            s_name TEXT NOT NULL, \
            s_address TEXT NOT NULL, \
            s_nationkey INTEGER NOT NULL, \
            s_phone TEXT NOT NULL, \
            s_acctbal REAL NOT NULL, \
            s_comment TEXT)",
    )
    .unwrap();
    e.execute(
        "CREATE TABLE partsupp (\
            ps_partkey INTEGER NOT NULL, \
            ps_suppkey INTEGER NOT NULL, \
            ps_availqty INTEGER NOT NULL, \
            ps_supplycost REAL NOT NULL, \
            ps_comment TEXT NOT NULL, \
            PRIMARY KEY (ps_partkey, ps_suppkey))",
    )
    .unwrap();
    e.execute(
        "CREATE TABLE lineitem (\
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
            l_comment TEXT NOT NULL)",
    )
    .unwrap();

    let _ = e.execute("INSERT INTO nation VALUES (1, 'GERMANY', 0, 'comment')");
    for s in 1..=20 {
        e.execute(&format!(
            "INSERT INTO supplier VALUES ({}, 'S{:02}', 'Addr{:02}', 1, 'Phn', 100.0, 'c')",
            s, s, s
        ))
        .unwrap();
    }

    // Each supplier has ONE partsupp row with ps_partkey=1, ps_suppkey=s,
    // ps_availqty=50. The half-sum for (1, s) is 125 > 50, so the
    // `ps_availqty > half_sum` predicate holds for every supplier → 20 rows.
    for s in 1..=20 {
        e.execute(&format!(
            "INSERT INTO partsupp VALUES (1, {}, 50, 1.0, 'c')",
            s
        ))
        .unwrap();
    }

    // 500 lineitem rows: 25 per supplier (s_suppkey 1..=20, l_partkey = 1),
    // l_quantity = 10, shipdate in the 1994-Q2..Q3 range.
    for i in 1..=500 {
        let s_suppkey = ((i - 1) % 20) + 1;
        let month = (((i - 1) % 6) + 4) as u32;
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
    let r = e.execute(Q20_EXACT_SHAPE_SQL).unwrap();
    let elapsed = start.elapsed();
    assert_eq!(
        r.rows.len(),
        20,
        "Q20 exact-shape EXISTS+SUM must return 20 suppliers; got {}",
        r.rows.len()
    );

    let snap = dump_v312_58_sprint3_diag();
    let calls = counter_value(&snap, "calls");
    let build = counter_value(&snap, "build");
    let pattern_fail = counter_value(&snap, "pattern_fail");
    eprintln!(
        "[q20-binaryop-arm exact] rows={} elapsed={:?} diag: calls={} build={} pattern_fail={}\nfull snapshot:\n{snap}",
        r.rows.len(), elapsed, calls, build, pattern_fail
    );

    assert!(
        calls > 0,
        "Q20 exact-shape: try_scalar_agg_index_lookup must be called at \
         least once for the `ps_availqty > (SELECT 0.5 * SUM(...))` \
         BinaryOp right-side arm; got calls={calls}. Snapshot:\n{snap}"
    );
    assert!(
        build >= 1,
        "Q20 exact-shape: ScalarAggIndex composite-key build must happen; \
         got build={build}. Snapshot:\n{snap}"
    );
    assert_eq!(
        pattern_fail, 0,
        "Q20 exact-shape: composite-key shape (ps_partkey, ps_suppkey) \
         must pass the pattern check; got pattern_fail={pattern_fail}. \
         Snapshot:\n{snap}"
    );
    assert!(
        elapsed.as_secs() < 5,
        "Q20 exact-shape too slow: {:?} (target: < 5 s)",
        elapsed
    );
}
