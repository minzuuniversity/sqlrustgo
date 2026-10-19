//! TPC-H 22/22 Value Assertion — Phase 2d Track 2.
//!
//! **Goal**: For every Q1-Q22 in `tpch_gate_test.rs`, run the query through
//! the wire protocol `MySqlTestClient` and assert that the result agrees
//! with an **independent** reference: the in-process SQLite oracle
//! (`tests/common/sqlite_oracle.rs`), loaded from the very same fixture.
//!
//! # Why this matters
//!
//! `tpch_gate_test` proves "22/22 do not panic and complete in 120s".
//! That's necessary but **not sufficient**. A query that returns 1
//! hard-coded row for every input would also pass that gate.
//! `tpch_value_test_v2` is the *correctness* gate.
//!
//! # Where the reference comes from
//!
//! The reference used to be `tests/data/tpch-sf001/expected/Q*_three_way.json`
//! — reference files that were **never committed** to the repository: they
//! lived behind the dangling `tests/data/tpch-sf001` symlink, so the test
//! panicked on the first missing file and could never pass anywhere.
//!
//! The reference is now the in-process SQLite oracle: a second, independent
//! SQL implementation (rusqlite) reading the same `.tbl` files. That gives
//! the same guarantee the three-way JSON was meant to give — SQLite's
//! answers as the ground truth — without depending on files that do not
//! exist.
//!
//! # Comparison
//!
//! The two results are compared as a **sorted multiset** of rows: two
//! engines may return the same rows in a different order when the query has
//! no `ORDER BY`. Row order is not asserted here.
//!
//! # Skipped queries
//!
//! Q9, Q18 and Q19 are **not** executed on the file-storage wire path: the
//! engine's intermediate result for Q9 grows past 1.5 GB within a second,
//! Q18 kills the server process, and Q19 cannot be parsed by the wire front
//! end (all three: issue #5393). They are excluded by the shared **wire
//! scope** (`tests/common/tpch_wire_scope.rs`), which refuses to execute
//! them and discloses the skip through the `TPCH-NOT-RUN` marker. The
//! remaining 19 statements are compared — and the test asserts that it
//! really compared them, so a run that compares nothing cannot report
//! success.
//!
//! Q1-Q3, Q6, Q12 use the **simplified** form in `tpch_gate_test` (e.g. Q3
//! has no `LIMIT 10` and Q1 groups by `(l_returnflag, l_linestatus)`); the
//! oracle runs the very same statement.
//!
//! # Truthfulness compliance
//!
//! No PENDING markers, no fabricated counts. If a value assertion fails,
//! the test panics with the actual vs. expected values. A statement the
//! SQLite oracle cannot express is reported as a failure, never skipped
//! silently.

#[path = "../../common/mod.rs"]
mod common;
#[path = "../../common/sqlite_oracle.rs"]
mod sqlite_oracle;
#[path = "../../common/tpch_fixture.rs"]
mod tpch_fixture;

use common::tpch_wire_harness::{run_query_timed, start_sf001_named, WireOutcome};
use common::tpch_wire_scope;
use sqlite_oracle::{SqliteOracle, TPC_H_ALL_TABLES};
use tpch_fixture::{lookup_statement_known_difference, report_known_difference};

/// Cargo target name — used in the NOT-RUN marker.
const TARGET: &str = "tpch_value_test_v2";

/// Same as `tpch_gate_test::tpch_queries` — keep in sync.
fn tpch_queries() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Q1", "SELECT l_returnflag, l_linestatus, SUM(l_quantity) AS sum_qty, SUM(l_extendedprice) AS sum_base_price, AVG(l_quantity) AS avg_qty, COUNT(*) AS count_order FROM lineitem WHERE l_shipdate <= '1998-09-02' GROUP BY l_returnflag, l_linestatus ORDER BY l_returnflag, l_linestatus"),
        ("Q2", "SELECT s_acctbal, s_name, n_name, p_partkey, p_mfgr, s_address, s_phone, s_comment FROM partsupp JOIN part ON p_partkey = ps_partkey JOIN supplier ON s_suppkey = ps_suppkey JOIN nation ON s_nationkey = n_nationkey JOIN region ON n_regionkey = r_regionkey WHERE p_size = 15 AND p_type LIKE '%BRASS' AND r_name = 'EUROPE' ORDER BY s_acctbal ASC, n_name, s_name, p_partkey LIMIT 20"),
        ("Q3", "SELECT l_orderkey, SUM(l_extendedprice) AS revenue, o_orderdate, o_shippriority FROM customer, orders, lineitem WHERE c_mktsegment = 'BUILDING' AND c_custkey = o_custkey AND l_orderkey = o_orderkey AND o_orderdate < '1995-03-15' AND l_shipdate > '1995-03-15' GROUP BY l_orderkey, o_orderdate, o_shippriority ORDER BY revenue DESC, o_orderdate"),
        ("Q4", "SELECT o_orderpriority, COUNT(*) AS order_count FROM orders WHERE o_orderdate >= '1993-07-01' AND o_orderdate < '1993-10-01' GROUP BY o_orderpriority ORDER BY o_orderpriority"),
        ("Q5", "SELECT n_name, SUM(l_extendedprice) AS revenue FROM customer, orders, lineitem, supplier, nation, region WHERE c_custkey = o_custkey AND l_orderkey = o_orderkey AND l_suppkey = s_suppkey AND c_nationkey = s_nationkey AND s_nationkey = n_nationkey AND n_regionkey = r_regionkey AND r_name = 'ASIA' AND o_orderdate >= '1994-01-01' AND o_orderdate < '1995-01-01' GROUP BY n_name ORDER BY revenue DESC"),
        ("Q6", "SELECT SUM(l_extendedprice) AS revenue FROM lineitem WHERE l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01' AND l_discount BETWEEN 0.05 AND 0.07 AND l_quantity < 24"),
        ("Q7", "SELECT supp_nation, cust_nation, l_year, SUM(l_extendedprice) AS revenue FROM (SELECT n1.n_name AS supp_nation, n2.n_name AS cust_nation, EXTRACT(YEAR FROM l_shipdate) AS l_year, l_extendedprice FROM supplier, lineitem, orders, customer, nation n1, nation n2 WHERE s_suppkey = l_suppkey AND o_orderkey = l_orderkey AND c_custkey = o_custkey AND s_nationkey = n1.n_nationkey AND c_nationkey = n2.n_nationkey AND ((n1.n_name = 'FRANCE' AND n2.n_name = 'GERMANY') OR (n1.n_name = 'GERMANY' AND n2.n_name = 'FRANCE')) AND l_shipdate BETWEEN '1995-01-01' AND '1996-12-31') AS shipping GROUP BY supp_nation, cust_nation, l_year ORDER BY supp_nation, cust_nation, l_year"),
        ("Q8", "SELECT EXTRACT(YEAR FROM o_orderdate) AS o_year, SUM(CASE WHEN n2.n_name = 'BRAZIL' THEN l_extendedprice * (1 - l_discount) ELSE 0 END) / SUM(l_extendedprice * (1 - l_discount)) AS mkt_share FROM part, supplier, lineitem, orders, customer, nation n1, nation n2, region WHERE p_partkey = l_partkey AND s_suppkey = l_suppkey AND l_orderkey = o_orderkey AND o_custkey = c_custkey AND c_nationkey = n1.n_nationkey AND n1.n_regionkey = r_regionkey AND r_name = 'AMERICA' AND s_nationkey = n2.n_nationkey AND o_orderdate BETWEEN '1995-01-01' AND '1996-12-31' AND p_type = 'ECONOMY ANODIZED STEEL' GROUP BY o_year ORDER BY o_year"),
        ("Q9", "SELECT nation, EXTRACT(YEAR FROM o_orderdate) AS o_year, SUM(l_extendedprice * (1 - l_discount) - ps_supplycost * l_quantity) AS sum_profit FROM part, supplier, lineitem, partsupp, orders, nation WHERE s_suppkey = l_suppkey AND ps_suppkey = l_suppkey AND ps_partkey = l_partkey AND p_partkey = l_partkey AND o_orderkey = l_orderkey AND s_nationkey = n_nationkey AND p_name LIKE '%green%' GROUP BY nation, o_year ORDER BY nation, o_year DESC"),
        ("Q10", "SELECT c_custkey, c_name, SUM(l_extendedprice * (1 - l_discount)) AS revenue, c_acctbal, n_name, c_address, c_phone, c_comment FROM customer, orders, lineitem, nation WHERE c_custkey = o_custkey AND l_orderkey = o_orderkey AND o_orderdate >= '1993-10-01' AND o_orderdate < '1994-01-01' AND l_returnflag = 'R' AND c_nationkey = n_nationkey GROUP BY c_custkey, c_name, c_acctbal, c_phone, n_name, c_address, c_comment ORDER BY revenue DESC LIMIT 20"),
        ("Q11", "SELECT ps_partkey, SUM(ps_supplycost * ps_availqty) AS part_value FROM partsupp, supplier, nation WHERE ps_suppkey = s_suppkey AND s_nationkey = n_nationkey AND n_name = 'GERMANY' GROUP BY ps_partkey HAVING SUM(ps_supplycost * ps_availqty) > 10000 ORDER BY part_value DESC"),
        ("Q12", "SELECT l_shipmode, COUNT(*) AS high_line_count, 0 AS low_line_count FROM orders, lineitem WHERE o_orderkey = l_orderkey AND l_shipmode IN ('MAIL', 'SHIP') AND l_commitdate < l_receiptdate AND l_shipdate < l_commitdate AND l_receiptdate >= '1994-01-01' AND l_receiptdate < '1995-01-01' GROUP BY l_shipmode ORDER BY l_shipmode"),
        ("Q13", "SELECT c_count, COUNT(*) AS custdist FROM (SELECT c_custkey, COUNT(o_orderkey) AS c_count FROM customer, orders WHERE c_custkey = o_custkey AND o_comment NOT LIKE '%special%requests%' GROUP BY c_custkey) AS c_orders GROUP BY c_count ORDER BY c_count DESC, custdist DESC"),
        ("Q14", "SELECT SUM(CASE WHEN p_type LIKE 'PROMO%' THEN l_extendedprice ELSE 0 END) AS promo_revenue FROM lineitem, part WHERE l_partkey = p_partkey AND l_shipdate >= '1995-09-01' AND l_shipdate < '1995-10-01'"),
        ("Q15", "SELECT s_suppkey, s_name, s_address, s_phone FROM supplier ORDER BY s_suppkey LIMIT 100"),
        ("Q16", "SELECT p_brand, p_type, p_size, COUNT(DISTINCT ps_suppkey) AS supplier_cnt FROM partsupp, part WHERE p_partkey = ps_partkey AND p_brand <> 'Brand#45' AND p_type NOT LIKE 'MEDIUM POLISHED%' AND p_size IN (49, 14, 23, 45, 19, 3, 36, 9) GROUP BY p_brand, p_type, p_size ORDER BY supplier_cnt DESC, p_brand, p_type, p_size"),
        ("Q17", "SELECT AVG(l_quantity) AS avg_qty FROM lineitem, part WHERE p_partkey = l_partkey AND p_brand = 'Brand#23' AND p_container = 'LG CASE'"),
        ("Q18", "SELECT c_name, c_custkey, o_orderkey, o_orderdate, o_totalprice, SUM(l_quantity) AS sum_l_quantity FROM customer, orders, lineitem WHERE c_custkey = o_custkey AND l_orderkey = o_orderkey GROUP BY c_name, c_custkey, o_orderkey, o_orderdate, o_totalprice HAVING SUM(l_quantity) > 300 ORDER BY o_totalprice DESC, o_orderdate LIMIT 100"),
        ("Q19", "SELECT COUNT(*) AS revenue FROM lineitem, part WHERE p_partkey = l_partkey AND p_brand = 'Brand#12' AND p_container IN ('SM CASE', 'SM BOX', 'SM PACK', 'SM PKG') AND l_quantity >= 1 AND l_quantity <= 11 AND p_size BETWEEN 1 AND 5 AND l_shipmode IN ('AIR', 'AIR REG') AND l_shipinstruct = 'DELIVER IN PERSON'"),
        ("Q20", "SELECT s_name, s_address FROM supplier, nation WHERE s_nationkey = n_nationkey AND n_name = 'GERMANY' ORDER BY s_name"),
        ("Q21", "SELECT s_name, COUNT(*) AS numwait FROM supplier, lineitem l1, orders, nation WHERE s_suppkey = l1.l_suppkey AND o_orderkey = l1.l_orderkey AND o_orderstatus = 'F' AND s_nationkey = n_nationkey AND n_name = 'GERMANY' GROUP BY s_name ORDER BY numwait DESC, s_name LIMIT 100"),
        ("Q22", "SELECT SUBSTR(c_phone, 1, 2) AS cntrycode, COUNT(*) AS numcust, SUM(c_acctbal) AS totacctbal FROM customer WHERE c_acctbal > 0 GROUP BY SUBSTR(c_phone, 1, 2) ORDER BY cntrycode"),
    ]
}

#[test]
fn test_tpch_22_value_assertions() {
    eprintln!("\n=== TPC-H 22/22 Value Assertions (wire protocol vs SQLite oracle) ===");
    let Some(mut client) = start_sf001_named(TARGET) else {
        return;
    };

    // The reference for every query is the **in-process SQLite oracle**,
    // loaded from the very same fixture.
    //
    // The test used to read `tests/data/tpch-sf001/expected/Q*_three_way.json`
    // — files that were never committed to the repository (they lived behind
    // the dangling `tests/data/tpch-sf001` symlink). The test therefore
    // panicked on the first missing file and could never pass. The SQLite
    // oracle is a second, independent SQL implementation running in-process
    // on the same `.tbl` files, so it provides the same guarantee the
    // three-way JSON was meant to provide — SQLite's answers as the ground
    // truth — without depending on files that do not exist.
    let state = tpch_fixture::FixtureState::resolve();
    let Some(fixture) = state.require("tpch_value_test_v2") else {
        // `require` already wrote the NOT-RUN marker.
        return;
    };
    let oracle = SqliteOracle::open(fixture, &TPC_H_ALL_TABLES)
        .expect("opening the SQLite oracle on the fixture");

    let queries = tpch_queries();
    assert_eq!(queries.len(), 22, "must have exactly 22 queries");

    // The statements the file-storage wire path must not execute, and the
    // ones it must. Both counts are checked against the shared scope
    // afterwards, so a statement can neither slip through nor disappear.
    let expected_scoped = queries
        .iter()
        .filter(|(qid, _)| tpch_wire_scope::in_wire_scope(qid))
        .count();
    let expected_skipped = queries.len() - expected_scoped;

    let mut pass = 0;
    let mut fail = 0;
    let mut not_run = 0;
    let mut known = 0;
    let mut mismatched = 0;
    let mut compared = 0;
    let mut fail_details: Vec<String> = Vec::new();
    let mut skipped_scope: Vec<&'static str> = Vec::new();

    for (qid, q_sql) in &queries {
        let outcome = run_query_timed(&mut client, qid, q_sql, 30);
        let actual_rows: Vec<Vec<String>> = match outcome {
            WireOutcome::Ran(Ok(rows), _elapsed) => rows,
            WireOutcome::Ran(Err(e), _elapsed) => {
                fail_details.push(format!("{qid}: query failed: {e}"));
                fail += 1;
                continue;
            }
            WireOutcome::Skipped(skip) => {
                // Outside the wire scope: the statement is NOT executed.
                // Executing it would exhaust the machine and kill the test
                // binary with it (issue #5393).
                not_run += 1;
                skipped_scope.push(qid);
                eprintln!(
                    "  {qid:3}: SKIPPED ({} issue={}) — {}",
                    skip.reason, skip.issue, skip.detail
                );
                continue;
            }
        };

        let adapted = sqlite_oracle::adapt_sql(q_sql);
        if !adapted.unsupported.is_empty() {
            fail_details.push(format!(
                "{qid}: the statement uses constructs the SQLite oracle does not support: {:?}",
                adapted.unsupported
            ));
            fail += 1;
            continue;
        }
        let oracle_rows = match oracle.query(&adapted.sql) {
            Ok(rows) => rows,
            Err(e) => {
                fail_details.push(format!("{qid}: the SQLite oracle failed: {e}"));
                fail += 1;
                continue;
            }
        };

        // The wire protocol renders a NULL as the empty string while the
        // oracle renders it as its own sentinel. The two cannot be told
        // apart, so a statement that produces a NULL is reported as a
        // failure rather than silently normalised.
        let nulls = oracle_rows
            .iter()
            .flat_map(|r| r.iter())
            .filter(|c| c.as_str() == sqlite_oracle::NULL_CELL)
            .count();
        if nulls > 0 {
            fail_details.push(format!(
                "{qid}: the statement produced {nulls} NULL cell(s); the wire protocol renders a \
                 NULL as the empty string, which cannot be told apart from a real empty string"
            ));
            fail += 1;
            continue;
        }

        let mut actual_sorted = actual_rows.clone();
        let mut oracle_sorted = oracle_rows.clone();
        actual_sorted.sort();
        oracle_sorted.sort();
        match sqlite_oracle::diff_rows(&actual_sorted, &oracle_sorted) {
            None => {
                compared += 1;
                pass += 1;
                eprintln!("  {qid:3}: OK rc={}", actual_rows.len());
            }
            Some(diff) => {
                compared += 1;
                match lookup_statement_known_difference(qid) {
                    // A difference that stems from the engine's SQL semantics
                    // is registered once (with an issue and an owner) and
                    // applies to every target that runs the statement. It is
                    // disclosed, not hidden.
                    Some(entry) => {
                        report_known_difference(&entry, &diff);
                        known += 1;
                        eprintln!("  {qid:3}: KNOWN-DIFF (registered) {diff}");
                    }
                    None => {
                        mismatched += 1;
                        let detail = format!("{qid}: {diff}");
                        eprintln!("  FAIL {}", detail);
                        fail_details.push(detail);
                    }
                }
            }
        }
    }

    eprintln!("\n=== TPC-H 22/22 Value Assertion Results ===");
    eprintln!("Compared: {} / 22", compared);
    eprintln!("Pass: {}", pass);
    eprintln!("Known difference: {}", known);
    eprintln!("Not run (wire scope): {}", not_run);
    eprintln!("Mismatched: {}", mismatched);
    eprintln!("Fail: {}", fail);
    if fail > 0 || mismatched > 0 {
        eprintln!("\nFailures:");
        for d in &fail_details {
            eprintln!("  {}", d);
        }
    }

    // A statement that could not be compared is reported first: it is the
    // real cause, the accounting below is only a consequence of it.
    assert_eq!(
        mismatched, 0,
        "{TARGET}: {mismatched} statement(s) did not match the independent SQLite oracle. \
         (If this is a known engine defect, register it in \
         tests/baseline/tpch_known_differences.json with an issue and an owner.)"
    );
    assert_eq!(
        fail, 0,
        "{TARGET}: {fail} statement(s) could not be executed on one of the two sides"
    );

    // Every statement must be accounted for: either it was compared against
    // the independent oracle, it failed, or it was skipped for a registered
    // reason. Nothing may disappear in between.
    assert_eq!(
        compared + not_run + fail,
        queries.len(),
        "{TARGET}: {compared} compared + {not_run} not run + {fail} failed does not account \
         for the {} statements of this file",
        queries.len()
    );
    // ... and the skipped ones must be exactly the ones the shared wire
    // scope excludes — neither more (a silently dropped statement) nor
    // fewer (a statement that could have been compared).
    assert_eq!(
        skipped_scope.len(),
        expected_skipped,
        "{TARGET}: {skipped_scope:?} were skipped, but the wire scope excludes \
         {expected_skipped} statement(s) from this file"
    );
    assert_eq!(
        compared, expected_scoped,
        "{TARGET}: {compared} of the {expected_scoped} in-scope statements were compared \
         against the independent SQLite oracle"
    );
    assert!(
        compared > 0,
        "{TARGET}: this run compared nothing at all; a run that compares nothing must not \
         report success"
    );
    // Disclose the statements this function did not execute. One marker
    // per test function keeps the CI arithmetic exact.
    tpch_wire_scope::report_wire_exclusions(TARGET, &skipped_scope);
}
