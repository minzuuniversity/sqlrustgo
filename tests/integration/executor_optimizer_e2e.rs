//! Executor ↔ Optimizer end-to-end integration tests
//!
//! V312-22 / Issue #4182 — Production integration boundary gap:
//! Hash Semi Join + CBO + Histogram end-to-end pipeline.
//!
//! This file exercises the *production query path* — that is, engineering
//! constructs a representative multi-query workload, runs it through
//! `ExecutionEngine::execute(sql)`, and verifies that:
//!
//! 1. **Hash Semi Join path** — `EXISTS` / `IN (subquery)` queries produce
//!    correct results (the executor's subquery path uses HashSemiJoin under
//!    the hood; we verify behaviorally).
//! 2. **CBO + Histogram consumption** — after `ANALYZE`, the unified cost
//!    model returns selectivity estimates driven by the per-column histogram
//!    (not the per-operator heuristic fallback).
//! 3. **5-table schema workload** — a TPC-H-style 5-table schema, with
//!    ANALYZE → CBO-optimized query, executes end-to-end through the
//!    executor (no panic, no NOT IMPLEMENTED, no manual rewriting).
//!
//! These tests close the V312-22 production-integration boundary gap that
//! codex identified in #3887 Round-17 audit. The operator units exist
//! (HashSemiJoin in `crates/executor/src/join/hash_semi_join.rs`;
//! Histogram in `crates/optimizer/src/stats.rs`); what was missing is
//! evidence that the production query path exercises them together.

use parking_lot::RwLock;
use sqlrustgo::{MemoryExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

// ===========================================================================
// Helpers
// ===========================================================================

/// Build a 5-table TPC-H-style schema (region, nation, supplier, customer,
/// orders) and populate it with a small fixture suitable for subquery
/// semi-join tests.
fn build_5_table_engine() -> MemoryExecutionEngine {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = MemoryExecutionEngine::new(storage);

    // Schema: 5 tables, foreign-key relationships, integers + text.
    engine.execute("CREATE TABLE region (r_regionkey INTEGER, r_name TEXT)").unwrap();
    engine.execute("CREATE TABLE nation (n_nationkey INTEGER, n_regionkey INTEGER, n_name TEXT)").unwrap();
    engine.execute("CREATE TABLE supplier (s_suppkey INTEGER, s_nationkey INTEGER, s_name TEXT)").unwrap();
    engine.execute("CREATE TABLE customer (c_custkey INTEGER, c_nationkey INTEGER, c_name TEXT)").unwrap();
    engine.execute("CREATE TABLE orders (o_orderkey INTEGER, o_custkey INTEGER, o_total INTEGER)").unwrap();

    // Two regions. China has 2 nations; USA has 1.
    engine.execute("INSERT INTO region VALUES (1, 'CHINA')").unwrap();
    engine.execute("INSERT INTO region VALUES (2, 'USA')").unwrap();

    engine.execute("INSERT INTO nation VALUES (10, 1, 'CN-A')").unwrap();
    engine.execute("INSERT INTO nation VALUES (11, 1, 'CN-B')").unwrap();
    engine.execute("INSERT INTO nation VALUES (20, 2, 'US-A')").unwrap();

    // 6 suppliers across the 3 nations.
    for sk in 100..106 {
        let nk = match sk % 3 {
            0 => 10,
            1 => 11,
            _ => 20,
        };
        engine
            .execute(&format!(
                "INSERT INTO supplier VALUES ({}, {}, 'Sup{}')",
                sk, nk, sk
            ))
            .unwrap();
    }

    // 50 customers across 3 nations.
    for ck in 1..=50 {
        let nk = match ck % 3 {
            0 => 10,
            1 => 11,
            _ => 20,
        };
        engine
            .execute(&format!(
                "INSERT INTO customer VALUES ({}, {}, 'Cust{}')",
                ck, nk, ck
            ))
            .unwrap();
    }

    // 200 orders, each pointing at a customer.
    for ok in 1..=200 {
        let ck = (ok % 50) + 1;
        engine
            .execute(&format!(
                "INSERT INTO orders VALUES ({}, {}, {})",
                ok, ck, ok * 10
            ))
            .unwrap();
    }

    engine
}

// ===========================================================================
// Test 1: Hash Semi Join path — IN-subquery
// ===========================================================================

#[test]
fn test_in_subquery_returns_correct_rows() {
    // Q: SELECT c_custkey FROM customer
    //    WHERE c_nationkey IN (SELECT n_nationkey FROM nation WHERE n_regionkey = 1)
    //
    // This is a Semi Join scenario: outer= customer, inner=nation filtered
    // by region=1. Expected: customers whose nation is in {10, 11} — i.e.
    // ck % 3 == 0 or ck % 3 == 1. With ck in 1..=50, that's
    // ck ∈ {1, 3, 4, 6, 7, 9, 10, 12, 13, ...} — count = 50 * 2/3 = 34 (with
    // rounding edge cases).
    let mut engine = build_5_table_engine();
    let result = engine
        .execute(
            "SELECT c_custkey FROM customer \
             WHERE c_nationkey IN (SELECT n_nationkey FROM nation WHERE n_regionkey = 1)",
        )
        .unwrap();

    // Behavioral correctness: production query path produces a row count
    // matching the expected number of customers in CHINA region (nations 10, 11).
    // Each ck in 1..=50 with ck % 3 != 2 falls in nations 10 or 11.
    let expected = (1..=50).filter(|ck| ck % 3 != 2).count();
    assert_eq!(
        result.rows.len(),
        expected,
        "IN-subquery (semi join) row count should match expected"
    );
}

#[test]
fn test_exists_subquery_returns_correct_rows() {
    // Q: SELECT c_custkey FROM customer c
    //    WHERE EXISTS (SELECT 1 FROM orders o WHERE o.o_custkey = c.c_custkey)
    //
    // All 50 customers have at least one order (since ok in 1..=200
    // covers all ck in 1..=50 via (ok % 50) + 1). So expected = 50.
    let mut engine = build_5_table_engine();
    let result = engine
        .execute(
            "SELECT c_custkey FROM customer c \
             WHERE EXISTS (SELECT 1 FROM orders o WHERE o.o_custkey = c.c_custkey)",
        )
        .unwrap();

    assert_eq!(
        result.rows.len(),
        50,
        "EXISTS subquery (semi join) should match all 50 customers"
    );
}

#[test]
fn test_not_exists_subquery_returns_correct_rows() {
    // Q: SELECT c_custkey FROM customer c
    //    WHERE NOT EXISTS (SELECT 1 FROM orders o WHERE o.o_custkey = c.c_custkey)
    //
    // All customers have orders, so result should be 0 rows.
    let mut engine = build_5_table_engine();
    let result = engine
        .execute(
            "SELECT c_custkey FROM customer c \
             WHERE NOT EXISTS (SELECT 1 FROM orders o WHERE o.o_custkey = c.c_custkey)",
        )
        .unwrap();

    assert_eq!(
        result.rows.len(),
        0,
        "NOT EXISTS anti-join should return 0 rows when all customers have orders"
    );
}

// ===========================================================================
// Test 2: CBO + Histogram consumption after ANALYZE
// ===========================================================================

#[test]
fn test_analyze_populates_histogram_in_cost_model() {
    let mut engine = build_5_table_engine();

    // BEFORE ANALYZE: cost_model uses heuristic fallback (selectivity = 0.1 default).
    let before = engine.estimate_selectivity("customer", "c_custkey");
    assert_eq!(before, 0.1, "before ANALYZE, heuristic fallback = 0.1");

    // ANALYZE → collect stats → populate histogram.
    engine.execute("ANALYZE customer").unwrap();

    // After ANALYZE: histogram drives selectivity (1/NDV ≈ 1/50 = 0.02).
    let after = engine.estimate_selectivity("customer", "c_custkey");
    assert!(
        after < 0.1,
        "after ANALYZE, histogram-based selectivity should be < heuristic 0.1, got {}",
        after
    );
    assert!(
        after > 0.0,
        "after ANALYZE, selectivity should be > 0, got {}",
        after
    );
}

#[test]
fn test_cbo_selectivity_differs_uniform_vs_skewed() {
    // Two-engine comparison: uniform vs skewed selectors should produce
    // different CBO selectivity estimates after ANALYZE.
    let engine_uniform = {
        let storage = Arc::new(RwLock::new(MemoryStorage::new()));
        let mut e = MemoryExecutionEngine::new(storage);
        e.execute("CREATE TABLE t (id INTEGER)").unwrap();
        for i in 0..100 {
            e.execute(&format!("INSERT INTO t VALUES ({})", i)).unwrap();
        }
        e.execute("ANALYZE t").unwrap();
        e
    };

    let engine_skewed = {
        let storage = Arc::new(RwLock::new(MemoryStorage::new()));
        let mut e = MemoryExecutionEngine::new(storage);
        e.execute("CREATE TABLE t (id INTEGER)").unwrap();
        // 100 rows, but value 1 appears 99 times, value 2 appears once.
        for _ in 0..99 {
            e.execute("INSERT INTO t VALUES (1)").unwrap();
        }
        e.execute("INSERT INTO t VALUES (2)").unwrap();
        e.execute("ANALYZE t").unwrap();
        e
    };

    let sel_uniform = engine_uniform.estimate_selectivity("t", "id");
    let sel_skewed = engine_skewed.estimate_selectivity("t", "id");

    // Both should be < heuristic 0.1, but the skewed column should have
    // HIGHER selectivity for value 1 (since 99/100 rows match) than the
    // uniform column (1/100 rows match).
    assert!(sel_uniform < 0.1, "uniform 1/100 < 0.1");
    assert!(sel_skewed > sel_uniform, "skewed (99 hot) > uniform");
}

// ===========================================================================
// Test 3: 5-table CBO-optimized query
// ===========================================================================

#[test]
fn test_5_table_join_with_analyze_succeeds() {
    // TPC-H Q5-like: 5-table join (region, nation, supplier, customer, orders).
    // After ANALYZE, the CBO has accurate row counts for join-order planning.
    let mut engine = build_5_table_engine();

    for table in &["region", "nation", "supplier", "customer", "orders"] {
        engine.execute(&format!("ANALYZE {}", table)).unwrap();
    }

    // Run a 5-table join. This exercises the planner path with full stats.
    let result = engine
        .execute(
            "SELECT r.r_name, SUM(o.o_total) \
             FROM region r \
             JOIN nation n ON n.n_regionkey = r.r_regionkey \
             JOIN supplier s ON s.s_nationkey = n.n_nationkey \
             JOIN customer c ON c.c_nationkey = n.n_nationkey \
             JOIN orders o ON o.o_custkey = c.c_custkey \
             GROUP BY r.r_name \
             ORDER BY r.r_name",
        )
        .unwrap();

    // Two regions → two groups in the result.
    assert_eq!(result.rows.len(), 2, "5-table join should yield 2 region groups");
    assert_eq!(result.rows.len(), 2);
    // Verify that aggregate columns are populated (non-null sums).
    for row in &result.rows {
        match row.get(1) {
            Some(Value::Integer(s)) => assert!(*s > 0, "sum should be > 0"),
            Some(Value::Float(s)) => assert!(*s > 0.0, "sum should be > 0"),
            other => panic!("expected sum, got {:?}", other),
        }
    }
}

#[test]
fn test_5_table_analyze_then_subquery_semi_join() {
    // Composite: 5-table schema, ANALYZE all, then run a subquery semi-join
    // that filters across the joined schema. This composes the two primary
    // dimensions of the production integration boundary:
    //  - CBO + Histogram path (driven by ANALYZE)
    //  - Hash Semi Join / subquery path (used by IN/EXISTS)
    let mut engine = build_5_table_engine();

    for table in &["region", "nation", "supplier", "customer", "orders"] {
        engine.execute(&format!("ANALYZE {}", table)).unwrap();
    }

    // Q: customers whose nation is in a region with > 10 suppliers.
    // (Simplified: regions with 3 nations total, so all regions have 6 suppliers,
    // but we only check the join works.)
    let result = engine
        .execute(
            "SELECT c_custkey FROM customer c \
             WHERE c.c_nationkey IN (\
                 SELECT s.s_nationkey FROM supplier s \
                 WHERE s.s_nationkey = c.c_nationkey\
             )",
        )
        .unwrap();

    // All customers have a matching supplier (nation in nation table).
    assert_eq!(result.rows.len(), 50);
}

// ===========================================================================
// Test 4: CBO cost estimation uses histogram after ANALYZE
// ===========================================================================

#[test]
fn test_join_cost_uses_analyzed_stats() {
    let mut engine = build_5_table_engine();

    // Estimate join cost BEFORE ANALYZE (heuristic row count = 1000).
    let cost_before = engine.estimate_join_cost("orders", "customer", "hash");

    // ANALYZE → real row counts (200 orders, 50 customers).
    engine.execute("ANALYZE orders").unwrap();
    engine.execute("ANALYZE customer").unwrap();

    let cost_after = engine.estimate_join_cost("orders", "customer", "hash");

    // AFTER ANALYZE, the cost should be DIFFERENT — proving the cost model
    // consumed the new histogram-backed stats.
    assert!(
        (cost_before - cost_after).abs() > 0.0,
        "join cost should change after ANALYZE: before={}, after={}",
        cost_before,
        cost_after
    );
    assert!(cost_after > 0.0);
    assert!(cost_before > 0.0);
}

#[test]
fn test_join_order_uses_histogram_after_analyze() {
    // Verify that ANALYZE-driven stats change the optimizer's join order.
    let mut engine = build_5_table_engine();

    // Before ANALYZE, all tables have default 1000 row count.
    let before = engine.optimize_join_order(&["orders", "customer", "supplier"]);

    // ANALYZE with very different sizes.
    for table in &["orders", "customer", "supplier"] {
        engine.execute(&format!("ANALYZE {}", table)).unwrap();
    }

    let after = engine.optimize_join_order(&["orders", "customer", "supplier"]);

    // After ANALYZE, the order should reflect real row counts
    // (orders=200, customer=50, supplier=6). Different from heuristic.
    assert_eq!(after.len(), 3);
    assert_eq!(before.len(), 3);
    // Order should ideally start with smallest (supplier, 6 rows).
    assert_eq!(
        after[0], "supplier",
        "after ANALYZE, smallest table (supplier=6) should be first"
    );
}
