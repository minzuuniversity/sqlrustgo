//! TPC-H SF=0.1 Sanity Test — wire protocol (LOAD DATA LOCAL INFILE + MySqlTestClient)
//!
//! Migrated from in-process `ExecutionEngine` + `bulk_insert_records` to wire
//! protocol per docs/plans/2026-06-13-tpch-e2e-migration-design.md §3.2.
//!
//! SF=0.1 uses 60K lineitem rows. 8 tables (region/nation/supplier/customer/
//! part/partsupp/orders/lineitem) load via `start_sf01()` helper which sets
//! 60s read/write timeouts to accommodate larger data.
//!
//! Run: cargo test --test tpch_sf01_inprocess_test -- --nocapture
//!
//! # The wire scope
//!
//! Three of the 22 standard statements — **Q9**, **Q18** and **Q19** — cannot be
//! executed on the file-storage wire path: Q9's intermediate result grows past
//! 1.5 GB within a second, Q18 kills the server process, and Q19 cannot be
//! parsed by the wire front end (all three: issue #5393). A killed process
//! takes the **whole test binary** with it, so the CI job would die with
//! `signal: 9` instead of reporting a test failure.
//!
//! The shared **wire scope** (`tests/common/tpch_wire_scope.rs`) refuses to
//! execute those statements: `run_query_timed` takes the query id and returns
//! `WireOutcome::Skipped` for them. This file therefore executes the 19
//! in-scope statements, discloses the 3 skips through the `TPCH-NOT-RUN`
//! marker, and asserts that exactly those 3 were skipped — neither more (a
//! silently dropped statement) nor fewer (a statement that could have run).

#[path = "../../common/mod.rs"]
mod common;
#[path = "../../common/tpch_fixture.rs"]
mod tpch_fixture;
use common::tpch_wire_harness::*;
use common::tpch_wire_scope;

/// Cargo target name — used in the NOT-RUN marker.
const TARGET: &str = "tpch_sf01_inprocess_test";

/// TPC-H 22 queries (simplified SQLRustGo-compatible versions)
/// Kept in sync with `tpch_gate_test::tpch_queries` — same SQL strings.
fn tpch_queries() -> Vec<(&'static str, &'static str)> {
    // v3.8.0 parser doesn't support arithmetic expressions inside aggregate functions.
    // Simplified versions use pre-computed columns or simpler aggregates.
    // For full TPC-H correctness verification, use bench-cli (check_tpch.sh passes 22/22).
    vec![
        // Q1 - Pricing Summary Report (simplified: COUNT and GROUP BY only)
        ("Q1", "SELECT l_returnflag, l_linestatus, SUM(l_quantity) AS sum_qty, SUM(l_extendedprice) AS sum_base_price, AVG(l_quantity) AS avg_qty, COUNT(*) AS count_order FROM lineitem WHERE l_shipdate <= '1998-09-02' GROUP BY l_returnflag, l_linestatus ORDER BY l_returnflag, l_linestatus"),
        // Q3 - Shipping Priority (simplified: SUM without expression)
        ("Q3", "SELECT l_orderkey, SUM(l_extendedprice) AS revenue, o_orderdate, o_shippriority FROM customer, orders, lineitem WHERE c_mktsegment = 'BUILDING' AND c_custkey = o_custkey AND l_orderkey = o_orderkey AND o_orderdate < '1995-03-15' AND l_shipdate > '1995-03-15' GROUP BY l_orderkey, o_orderdate, o_shippriority ORDER BY revenue DESC, o_orderdate"),
        // Q5 - Local Supplier (simplified: no expression in SUM)
        ("Q5", "SELECT n_name, SUM(l_extendedprice) AS revenue FROM customer, orders, lineitem, supplier, nation, region WHERE c_custkey = o_custkey AND l_orderkey = o_orderkey AND l_suppkey = s_suppkey AND c_nationkey = s_nationkey AND s_nationkey = n_nationkey AND n_regionkey = r_regionkey AND r_name = 'ASIA' AND o_orderdate >= '1994-01-01' AND o_orderdate < '1995-01-01' GROUP BY n_name ORDER BY revenue DESC"),
        // Q6 - Forecasting Revenue (simplified)
        ("Q6", "SELECT COUNT(*) AS revenue FROM lineitem WHERE l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01' AND l_quantity < 25"),
        // Q7 - Volume Shipping (simplified: subquery without expression)
        ("Q7", "SELECT supp_nation, cust_nation, l_year, COUNT(*) AS revenue FROM (SELECT n1.n_name AS supp_nation, n2.n_name AS cust_nation, CAST(SUBSTR(l_shipdate, 1, 4) AS INTEGER) AS l_year FROM supplier, lineitem, orders, customer, nation n1, nation n2 WHERE s_suppkey = l_suppkey AND o_orderkey = l_orderkey AND c_custkey = o_custkey AND s_nationkey = n1.n_nationkey AND c_nationkey = n2.n_nationkey AND ((n1.n_name = 'FRANCE' AND n2.n_name = 'GERMANY') OR (n1.n_name = 'GERMANY' AND n2.n_name = 'FRANCE')) AND l_shipdate BETWEEN '1995-01-01' AND '1996-12-31') AS shipping GROUP BY supp_nation, cust_nation, l_year ORDER BY supp_nation, cust_nation, l_year"),
        // Q8 - National Market (simplified)
        ("Q8", "SELECT o_year, COUNT(*) AS mkt_share FROM (SELECT CAST(SUBSTR(o_orderdate, 1, 4) AS INTEGER) AS o_year, n2.n_name AS nation FROM part, supplier, lineitem, orders, customer, nation n1, nation n2, region WHERE p_partkey = l_partkey AND s_suppkey = l_suppkey AND l_orderkey = o_orderkey AND o_custkey = c_custkey AND c_nationkey = n1.n_nationkey AND n1.n_regionkey = r_regionkey AND r_name = 'AMERICA' AND s_nationkey = n2.n_nationkey AND o_orderdate BETWEEN '1995-01-01' AND '1996-12-31' AND p_type = 'ECONOMY ANODIZED STEEL') AS all_nations GROUP BY o_year ORDER BY o_year"),
        // Q9 - Product Profit (simplified)
        ("Q9", "SELECT nation, o_year, COUNT(*) AS sum_profit FROM (SELECT n_name AS nation, CAST(SUBSTR(o_orderdate, 1, 4) AS INTEGER) AS o_year FROM part, supplier, lineitem, partsupp, orders, nation WHERE s_suppkey = l_suppkey AND ps_suppkey = l_suppkey AND ps_partkey = l_partkey AND p_partkey = l_partkey AND o_orderkey = l_orderkey AND s_nationkey = n_nationkey AND p_name LIKE '%green%') AS profit GROUP BY nation, o_year ORDER BY nation, o_year DESC"),
        // Q10 - Returned Item (simplified: no expression in SUM)
        ("Q10", "SELECT c_custkey, c_name, SUM(l_extendedprice) AS revenue, c_acctbal, n_name, c_address, c_phone, c_comment FROM customer, orders, lineitem, nation WHERE c_custkey = o_custkey AND l_orderkey = o_orderkey AND o_orderdate >= '1993-10-01' AND o_orderdate < '1994-01-01' AND l_returnflag = 'R' AND c_nationkey = n_nationkey GROUP BY c_custkey, c_name, c_acctbal, n_name, c_address, c_phone, c_comment ORDER BY revenue DESC"),
        // Q12 - Shipping Modes (simplified: no CASE in SUM)
        ("Q12", "SELECT l_shipmode, COUNT(*) AS high_line_count, 0 AS low_line_count FROM orders, lineitem WHERE o_orderkey = l_orderkey AND l_shipmode IN ('MAIL', 'SHIP') AND l_commitdate < l_receiptdate AND l_shipdate < l_commitdate AND l_receiptdate >= '1994-01-01' AND l_receiptdate < '1995-01-01' GROUP BY l_shipmode ORDER BY l_shipmode"),
        // Q19 - Discount Revenue (simplified)
        ("Q19", "SELECT COUNT(*) AS revenue FROM lineitem, part WHERE p_partkey = l_partkey AND p_brand = 'Brand#12' AND p_container IN ('SM CASE', 'SM BOX', 'SM PACK', 'SM PKG') AND l_quantity >= 1 AND l_quantity <= 11 AND p_size BETWEEN 1 AND 5 AND l_shipmode IN ('AIR', 'AIR REG') AND l_shipinstruct = 'DELIVER IN PERSON'"),
        // Q18 - Large Volume Customer Query (basic SQL, no subquery)
        ("Q18", "SELECT c_name, c_custkey, o_orderkey, o_orderdate, o_totalprice, SUM(l_quantity) AS sum_l_quantity FROM customer, orders, lineitem WHERE c_custkey = o_custkey AND l_orderkey = o_orderkey GROUP BY c_name, c_custkey, o_orderkey, o_orderdate, o_totalprice HAVING SUM(l_quantity) > 300 ORDER BY o_totalprice DESC, o_orderdate LIMIT 100"),
        // Q2 - Minimum Cost Supplier Query (rc2 Week 1 Day 4 fix):
        // Original comma-list form: `FROM part, supplier, partsupp, ...`
        // Fails because `find_join_predicate` extracts
        // `s_suppkey = ps_suppkey` for the supplier JOIN — but
        // `ps_suppkey` is in partsupp (the 3rd table, not yet joined),
        // so `find_join_key_index` can't resolve it. The `part` →
        // `supplier` join is therefore created with an ON that
        // references a column that doesn't yet exist on either side.
        //
        // Fix: rewrite as explicit `JOIN ... ON` with proper join
        // order. We start from `partsupp` (the most-connected table —
        // it joins to `part`, `supplier`, and via the subquery to
        // `nation` + `region`) and walk outward. This sidesteps the
        // auto-rewriter's "first valid equality predicate" heuristic
        // that mis-orders the 5-table chain.
        ("Q2", "SELECT s_acctbal, s_name, n_name, p_partkey, p_mfgr, s_address, s_phone, s_comment FROM partsupp JOIN part ON p_partkey = ps_partkey JOIN supplier ON s_suppkey = ps_suppkey JOIN nation ON s_nationkey = n_nationkey JOIN region ON n_regionkey = r_regionkey WHERE p_size = 15 AND p_type LIKE '%BRASS' AND r_name = 'EUROPE' ORDER BY s_acctbal ASC, n_name, s_name, p_partkey LIMIT 20"),
        // Q11 - Important Stock Identification Query (arithmetic in agg + arithmetic in HAVING)
        ("Q11", "SELECT ps_partkey, SUM(ps_supplycost * ps_availqty) AS part_value FROM partsupp, supplier, nation WHERE ps_suppkey = s_suppkey AND s_nationkey = n_nationkey AND n_name = 'GERMANY' GROUP BY ps_partkey HAVING SUM(ps_supplycost * ps_availqty) > 10000 ORDER BY part_value DESC"),

        // ---- v3.8.0-rc2 Week 1: 9 missing queries added ----
        // Q4 - Order Priority Checking Query
        ("Q4", "SELECT o_orderpriority, COUNT(*) AS order_count FROM orders WHERE o_orderdate >= '1993-07-01' AND o_orderdate < '1993-10-01' GROUP BY o_orderpriority ORDER BY o_orderpriority"),

        // Q13 - Customer Distribution Query (without NOT IN subquery for rc2 starter)
        ("Q13", "SELECT c_count, COUNT(*) AS custdist FROM (SELECT c_custkey, COUNT(o_orderkey) AS c_count FROM customer, orders WHERE c_custkey = o_custkey AND o_comment NOT LIKE '%special%requests%' GROUP BY c_custkey) AS c_orders GROUP BY c_count ORDER BY c_count DESC, custdist DESC"),

        // Q14 - Promotion Effect Query (simplified: returns sum of promo revenue, not divided)
        ("Q14", "SELECT SUM(CASE WHEN p_type LIKE 'PROMO%' THEN l_extendedprice ELSE 0 END) AS promo_revenue FROM lineitem, part WHERE l_partkey = p_partkey AND l_shipdate >= '1995-09-01' AND l_shipdate < '1995-10-01'"),

        // Q16 - Parts/Supplier Relationship (without NOT IN subquery for rc2 starter)
        ("Q16", "SELECT p_brand, p_type, p_size, COUNT(DISTINCT ps_suppkey) AS supplier_cnt FROM partsupp, part WHERE p_partkey = ps_partkey AND p_brand <> 'Brand#45' AND p_type NOT LIKE 'MEDIUM POLISHED%' AND p_size IN (49, 14, 23, 45, 19, 3, 36, 9) GROUP BY p_brand, p_type, p_size ORDER BY supplier_cnt DESC, p_brand, p_type, p_size"),

        // Q17 - Small-Quantity-Order Revenue (without subquery for rc2 starter)
        ("Q17", "SELECT AVG(l_quantity) AS avg_qty FROM lineitem, part WHERE p_partkey = l_partkey AND p_brand = 'Brand#23' AND p_container = 'LG CASE'"),

        // Q20 - Potential Part Promotion (without subqueries for rc2 starter)
        ("Q20", "SELECT s_name, s_address FROM supplier, nation WHERE s_nationkey = n_nationkey AND n_name = 'GERMANY' ORDER BY s_name"),

        // Q21 - Suppliers Who Kept Orders Waiting (without EXISTS/NOT EXISTS for rc2 starter)
        ("Q21", "SELECT s_name, COUNT(*) AS numwait FROM supplier, lineitem l1, orders, nation WHERE s_suppkey = l1.l_suppkey AND o_orderkey = l1.l_orderkey AND o_orderstatus = 'F' AND s_nationkey = n_nationkey AND n_name = 'GERMANY' GROUP BY s_name ORDER BY numwait DESC, s_name LIMIT 100"),

        // Q22 - Global Sales Opportunity (without NOT EXISTS / NOT IN for rc2 starter)
        ("Q22", "SELECT SUBSTR(c_phone, 1, 2) AS cntrycode, COUNT(*) AS numcust, SUM(c_acctbal) AS totacctbal FROM customer WHERE c_acctbal > 0 GROUP BY SUBSTR(c_phone, 1, 2) ORDER BY cntrycode"),

        // Q15 - Top Supplier Query (without subquery in FROM for rc2 starter)
        ("Q15", "SELECT s_suppkey, s_name, s_address, s_phone FROM supplier ORDER BY s_suppkey LIMIT 100"),
    ]
}

#[test]
fn tpch_sf01_sanity() {
    eprintln!("=== TPC-H SF=0.1 Sanity (wire protocol) ===");
    let Some(mut client) = start_sf01_named(TARGET) else {
        return;
    };

    eprintln!("[1/2] Verifying table row counts...");
    // The expected row counts come from the **fixture manifest**, not from
    // numbers hard-coded in this test.
    //
    // The test used to assert the row counts of a *dbgen* SF=0.001 dataset
    // (501 lineitem rows and friends). The fixture is now generated at run
    // time, so a hard-coded count is either wrong (the fixture has 60,000
    // lineitem rows, not 501) or right only by accident. The manifest
    // records the row count of every table as it was actually written, so
    // asserting against it checks the thing that matters: **the engine sees
    // every row the fixture file holds**. A storage backend that drops rows
    // — see issue #5392 — fails this assertion instead of silently
    // asserting against a fraction of the data.
    let state = tpch_fixture::FixtureState::resolve();
    let Some(fixture) = state.require("tpch_sf01_sanity") else {
        // `require` already wrote the NOT-RUN marker.
        return;
    };
    for tbl in TABLES {
        let expected_count = fixture
            .row_count(tbl)
            .unwrap_or_else(|| panic!("the fixture manifest has no row count for `{tbl}`"));
        let count = client
            .query_one_i64(&format!("SELECT COUNT(*) FROM {tbl}"))
            .expect("count");
        assert_eq!(
            count as u64, expected_count,
            "{tbl} count mismatch: the engine sees {count} rows but the fixture file holds \
             {expected_count}"
        );
        eprintln!("  ✅ {tbl}: {count} rows");
    }

    eprintln!("[2/2] Running TPC-H 22 queries on SF=0.1 (wire)...");
    let queries = tpch_queries();
    let mut passed = 0;
    let mut not_run = 0;
    let mut failed: Vec<(&str, String)> = Vec::new();
    // The statements the file-storage wire path must not execute, and the
    // ones it must. Both counts are checked against the shared scope
    // afterwards, so a statement can neither slip through nor disappear.
    let mut skipped_scope: Vec<&'static str> = Vec::new();
    let expected_scoped = queries
        .iter()
        .filter(|(qid, _)| tpch_wire_scope::in_wire_scope(qid))
        .count();
    let expected_skipped = queries.len() - expected_scoped;

    for (q_name, q_sql) in &queries {
        let qid: &'static str = q_name;
        let outcome = run_query_timed(&mut client, qid, q_sql, 60);
        let (result, elapsed) = match outcome {
            WireOutcome::Ran(result, elapsed) => (result, elapsed),
            WireOutcome::Skipped(skip) => {
                // Outside the wire scope: the statement is NOT executed.
                // Executing it would exhaust the machine and kill the test
                // binary with it (issue #5393).
                not_run += 1;
                skipped_scope.push(qid);
                eprintln!(
                    "  [SKIPPED] {qid}: ({} issue={}) — {}",
                    skip.reason, skip.issue, skip.detail
                );
                continue;
            }
        };
        match result {
            Ok(rows) => {
                eprintln!("  ✅ {qid}: {} rows in {:.2?}", rows.len(), elapsed);
                passed += 1;
            }
            Err(e) => {
                eprintln!("  [KNOWN-ISSUE] {qid}: {e} ({}ms)", elapsed.as_millis());
                failed.push((qid, e));
            }
        }
    }
    eprintln!(
        "\n=== TPC-H SF=0.1 Sanity: {passed}/{} passed, {not_run} not run (with known-issues) ===",
        queries.len()
    );

    // Every statement must be accounted for: either it ran, it failed, or
    // it was skipped for a registered reason. Nothing may disappear.
    assert_eq!(
        passed + not_run + failed.len(),
        queries.len(),
        "{TARGET}: {passed} passed + {not_run} not run + {} failed does not account for the {} \
         statements of this file",
        failed.len(),
        queries.len()
    );
    // ... and the skipped ones must be exactly the ones the shared wire
    // scope excludes — neither more (a silently dropped statement) nor
    // fewer (a statement that could have been run).
    assert_eq!(
        skipped_scope.len(),
        expected_skipped,
        "{TARGET}: {skipped_scope:?} were skipped, but the wire scope excludes \
         {expected_skipped} statement(s) from this file"
    );
    assert_eq!(
        passed, expected_scoped,
        "{TARGET}: {passed} of the {expected_scoped} in-scope statements ran; the statements \
         that failed are {failed:?}"
    );
    assert!(
        passed > 0,
        "{TARGET}: this run executed nothing at all; a run that executes nothing must not \
         report success"
    );

    // 至少 19/22 跑通 (留余量给 known-issue queries)
    assert!(
        passed >= 19,
        "expected ≥19/22 not panic, got {}/{} passed, {} failed, {not_run} not run",
        passed,
        queries.len(),
        failed.len()
    );

    // Disclose the statements this function did not execute. One marker
    // per test function keeps the CI arithmetic exact.
    tpch_wire_scope::report_wire_exclusions(TARGET, &skipped_scope);
}
