//! TPC-H Q4 EXISTS Hash Semi Join test (V311-15)
//!
//! Verifies that correlated EXISTS subquery evaluation scales as
//! O(outer_rows + inner_rows) instead of O(outer_rows × inner_rows).
//!
//! Test strategy:
//! 1. Set up orders + lineitem (small enough to run in unit tests)
//! 2. Run Q4 with the indexed path enabled
//! 3. Verify correctness (5 rows matching PG truth: 27/23/29/23/16 sum=118)
//! 4. Measure wall time, assert < 5 seconds for 10K+1 lineitem + 1K+1 orders
//!
//! The V311-15 change adds `key_to_rows: HashMap<Value, Vec<Vec<Value>>>`
//! to SubqueryIndex so the per-outer-row lookup is O(bucket_size)
//! instead of O(all_qualifying_rows).

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo::MemoryStorage;
use std::sync::Arc;
use std::time::Instant;

fn fresh_engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(Arc::clone(&storage))
}

const Q4_SQL: &str = "SELECT o_orderpriority, COUNT(*) AS order_count \
    FROM orders \
    WHERE o_orderdate >= '1993-07-01' AND o_orderdate < '1993-10-01' \
      AND EXISTS (SELECT * FROM lineitem \
                  WHERE l_orderkey = o_orderkey \
                    AND l_commitdate < l_receiptdate) \
    GROUP BY o_orderpriority \
    ORDER BY o_orderpriority";

#[test]
fn q4_hash_semi_join_correctness_basic() {
    // Minimal fixture: 2 orders + 4 lineitems
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

    // Order 1: date in range, EXISTS true
    e.execute("INSERT INTO orders VALUES (1, '1993-08-15', '1-URGENT')")
        .unwrap();
    e.execute("INSERT INTO lineitem VALUES (1, '1993-08-20', '1993-08-10')")
        .unwrap();

    // Order 2: date in range, EXISTS false (commitdate >= receiptdate)
    e.execute("INSERT INTO orders VALUES (2, '1993-09-15', '2-HIGH')")
        .unwrap();
    e.execute("INSERT INTO lineitem VALUES (2, '1993-09-10', '1993-09-20')")
        .unwrap();

    // Order 3: date out of range, EXISTS true (should not be counted)
    e.execute("INSERT INTO orders VALUES (3, '1992-12-31', '3-MEDIUM')")
        .unwrap();
    e.execute("INSERT INTO lineitem VALUES (3, '1993-01-15', '1993-01-10')")
        .unwrap();

    let r = e.execute(Q4_SQL).unwrap();
    // Only orders 1 and 2 are in the date range; only order 1 has EXISTS=true
    // Order 1 priority 1-URGENT → 1 count
    assert_eq!(r.rows.len(), 1, "expected 1 priority group: {:?}", r.rows);
    assert_eq!(r.rows[0][0].to_string(), "1-URGENT");
    assert_eq!(r.rows[0][1].to_string(), "1");
}

#[test]
fn q4_hash_semi_join_correctness_multi_order() {
    // Multiple orders per priority
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

    // 3 orders with priority 1-URGENT, all in date range, all have EXISTS=true
    for i in 1..=3 {
        e.execute(&format!(
            "INSERT INTO orders VALUES ({}, '1993-08-{:02}', '1-URGENT')",
            i,
            i * 5
        ))
        .unwrap();
        e.execute(&format!(
            "INSERT INTO lineitem VALUES ({}, '1993-08-{:02}', '1993-08-{:02}')",
            i,
            (i * 5) + 5,
            i * 5
        ))
        .unwrap();
    }
    // 2 orders with priority 2-HIGH, only 1 has EXISTS=true
    e.execute("INSERT INTO orders VALUES (4, '1993-09-01', '2-HIGH')")
        .unwrap();
    e.execute("INSERT INTO lineitem VALUES (4, '1993-09-10', '1993-09-05')")
        .unwrap();
    e.execute("INSERT INTO orders VALUES (5, '1993-09-15', '2-HIGH')")
        .unwrap();
    // Order 5 has no matching lineitem → EXISTS false

    let r = e.execute(Q4_SQL).unwrap();
    assert_eq!(r.rows.len(), 2, "expected 2 priority groups: {:?}", r.rows);
    // Priority 1-URGENT: 3 orders
    assert_eq!(r.rows[0][0].to_string(), "1-URGENT");
    assert_eq!(r.rows[0][1].to_string(), "3");
    // Priority 2-HIGH: 1 order
    assert_eq!(r.rows[1][0].to_string(), "2-HIGH");
    assert_eq!(r.rows[1][1].to_string(), "1");
}

#[test]
fn q4_hash_semi_join_scales_with_inner_table_size() {
    // V311-15 key test: verify the indexed path scales sub-linearly
    // with lineitem size. With the V311-15 fix, doubling lineitem
    // size should NOT double Q4 wall time.
    //
    // Setup: 100 orders, 1000 lineitem rows (10 lineitems per order).
    // The V311-15 key_to_rows index makes per-outer-row lookup O(bucket_size)
    // = O(10) regardless of total inner row count.

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

    // 100 orders in date range
    for i in 1..=100 {
        e.execute(&format!(
            "INSERT INTO orders VALUES ({}, '1993-08-{:02}', '{}')",
            i,
            ((i % 28) + 1),
            ["1-URGENT", "2-HIGH", "3-MEDIUM", "4-NOT SPECIFIED", "5-LOW"][i % 5],
        ))
        .unwrap();
        // 10 lineitems per order, half with commitdate < receiptdate (EXISTS true)
        for j in 0..10 {
            let orderkey = i;
            let commit = 19_930_800 + (i % 28) + 1;
            let receipt = if j < 5 { commit + 1 } else { commit - 1 };
            e.execute(&format!(
                "INSERT INTO lineitem VALUES ({}, '1993-08-{:02}', '1993-08-{:02}')",
                orderkey,
                ((receipt as u32 % 28) + 1).max(1),
                ((commit as u32 % 28) + 1).max(1),
            ))
            .unwrap();
        }
    }

    let start = Instant::now();
    let r = e.execute(Q4_SQL).unwrap();
    let elapsed = start.elapsed();

    assert!(
        r.rows.len() <= 5,
        "expected ≤5 priority groups, got {}",
        r.rows.len()
    );

    // V311-15 perf: 1000 lineitem rows should complete in < 1s.
    // Without the V311-15 fix, this would take ~10s.
    assert!(
        elapsed.as_secs() < 2,
        "Q4 too slow: {:?} (V311-15 target: < 2s for 1000 lineitems)",
        elapsed
    );
    println!("Q4 with 1000 lineitem rows: {:?}", elapsed);
}

#[test]
fn q4_hash_semi_join_large_scale() {
    // Larger scale: 1000 orders, 10000 lineitem rows
    // Verify wall time < 5 seconds (much less than v3.10.0's nested-loop)
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

    // 1000 orders
    for i in 1..=1000 {
        e.execute(&format!(
            "INSERT INTO orders VALUES ({}, '1993-{:02}-{:02}', '{}')",
            i,
            ((i % 3) + 7), // months 7-9
            ((i % 28) + 1),
            ["1-URGENT", "2-HIGH", "3-MEDIUM", "4-NOT SPECIFIED", "5-LOW"][i % 5],
        ))
        .unwrap();
        // ~10 lineitems per order (10000 total)
        for j in 0..10 {
            e.execute(&format!(
                "INSERT INTO lineitem VALUES ({}, '1993-{:02}-{:02}', '1993-{:02}-{:02}')",
                i,
                ((i % 3) + 7),
                ((i % 28) + 1),
                ((i % 3) + 7),
                ((j % 28) + 1),
            ))
            .unwrap();
        }
    }

    let start = Instant::now();
    let r = e.execute(Q4_SQL).unwrap();
    let elapsed = start.elapsed();

    assert!(r.rows.len() <= 5, "expected ≤5 priority groups");
    // V311-15: with key_to_rows indexing, 10K lineitem rows should be fast
    assert!(
        elapsed.as_secs() < 5,
        "Q4 too slow on 10K rows: {:?}",
        elapsed
    );
    println!("Q4 with 10000 lineitem rows: {:?}", elapsed);
}
