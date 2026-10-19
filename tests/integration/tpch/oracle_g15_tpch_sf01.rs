//! G15 — TPC-H SF=0.01 wire protocol vs the **independent** in-process
//! SQLite oracle.
//!
//! # What changed (issue #5379)
//!
//! This file used to compare the wire-protocol result against
//! `tests/data/tpch-sf01/expected/Q*_sf01_baseline.json`. Those files were
//! **never committed**: `tests/data/tpch-sf01` was a symlink into `/tmp` and
//! the JSON lived behind it. The `if !baseline_path.exists() { continue; }`
//! guard therefore fired for all 22 statements, `all_pass` stayed `true`,
//! and the target reported
//!
//! ```text
//! test result: ok. 22 passed; 0 failed; ...; finished in 0.00s
//! ```
//!
//! — 0.00 s, because it never opened a database connection. The old header
//! of this file bragged about "20/22 queries" passing; that number came
//! from baselines that were *regenerated from the engine's own output*,
//! which is the self-证明 this PR exists to end.
//!
//! # What this file is now
//!
//! * The expected value comes from a **different engine**: the same
//!   statement is executed by an in-process SQLite loaded from the same
//!   fixture (`tests/common/sqlite_oracle.rs`, `rusqlite` with the
//!   `bundled` feature — no external binary, no network).
//! * Every statement is really compared, and each test asserts **how many**
//!   statements it compared. A run that compares nothing can never report
//!   success.
//! * A difference from the oracle is a **failure**, unless it is registered
//!   in `tests/baseline/tpch_known_differences.json` with an issue and an
//!   owner (then it is disclosed through the `TPCH-KNOWN-DIFF` marker).
//!
//! # Comparison strength
//!
//! Most statements are compared **cell by cell** (as a sorted multiset —
//! row order is implementation-defined without `ORDER BY`, and floats are
//! compared with a relative tolerance). The three single-row aggregates
//! (`Q6`, `Q14`, `Q17`) are compared by **row count** only: their result
//! is one `SUM`, which is `NULL` when the filtered set is empty, and the
//! wire protocol renders a `NULL` as the empty string — indistinguishable
//! from a real empty string. Comparing the value there would be theatre.
//!
//! # Prerequisites
//!
//! A TPC-H fixture, resolved at run time (`TPCH_DATA_DIR` first). A
//! missing fixture is disclosed as `TPCH-NOT-RUN` with the reason code
//! `fixture_missing` — never as a pass.
//!
//! # One statement is not run
//!
//! `Q9` is a six-way join whose intermediate result the engine materialises
//! without bound; executing it over the wire protocol made the operating
//! system kill the server process (SIGKILL) on a 16 GB machine, which
//! takes the whole target down with it. It is therefore disclosed as
//! NOT-RUN with the reason code `scale_layer_only` and listed in the
//! summary — the test still asserts how many statements it really
//! compared.
//!
//! # The six tests
//!
//! The target keeps its six test functions. They share **one** ephemeral
//! server: loading the SF=0.01 fixture into it costs about half a minute,
//! and paying that six times would make the target slower than the CI
//! timeout. A connection that died is replaced by a fresh server, so one
//! broken statement cannot take the other five tests down with it.

#[path = "../../common/mod.rs"]
mod common;
#[path = "../../common/sqlite_oracle.rs"]
mod sqlite_oracle;
#[path = "../../common/tpch_fixture.rs"]
mod tpch_fixture;

use std::sync::{Mutex, OnceLock};

use common::tpch_wire_harness::{run_query_timed, start_sf01_named, WireOutcome};
use common::tpch_wire_scope;
use sqlite_oracle::{adapt_sql, diff_rows, SqliteOracle, NULL_CELL, TPC_H_ALL_TABLES};
use tpch_fixture::{
    lookup_statement_known_difference, reason, report_known_difference, report_not_run,
};

/// Cargo target name — used in the NOT-RUN marker and in the `case_id` of
/// the known-difference registry.
const TARGET: &str = "oracle_g15_tpch_sf01";

/// The statements, in the order this file has always used them.
///
/// The third field is the per-statement timeout in seconds. The old table
/// carried a fourth field, the name of the baseline JSON file; it is gone,
/// because those files were never committed and the expectation now comes
/// from the independent oracle.
const TPC_H_SF01_QUERIES: &[(&str, &str, u64)] = &[
    ("Q1", "SELECT l_returnflag, l_linestatus, SUM(l_quantity) AS sum_qty, SUM(l_extendedprice) AS sum_base_price, AVG(l_quantity) AS avg_qty, COUNT(*) AS count_order FROM lineitem WHERE l_shipdate <= '1998-09-02' GROUP BY l_returnflag, l_linestatus ORDER BY l_returnflag, l_linestatus", 60),
    ("Q2", "SELECT s_acctbal, s_name, n_name, p_partkey, p_mfgr, s_address, s_phone, s_comment FROM supplier, nation, region, part, partsupp WHERE p_partkey = ps_partkey AND s_suppkey = ps_suppkey AND p_size = 15 AND p_type LIKE '%BRASS' AND s_nationkey = n_nationkey AND n_regionkey = r_regionkey AND r_name = 'EUROPE' AND ps_supplycost = (SELECT MIN(ps_supplycost) FROM partsupp, supplier, nation, region WHERE p_partkey = ps_partkey AND s_suppkey = ps_suppkey AND s_nationkey = n_nationkey AND n_regionkey = r_regionkey AND r_name = 'EUROPE') ORDER BY s_acctbal DESC, n_name, s_name, p_partkey LIMIT 100", 60),
    ("Q3", "SELECT l_orderkey, SUM(l_extendedprice * (1 - l_discount)) AS revenue, o_orderdate, o_shippriority FROM customer, orders, lineitem WHERE c_mktsegment = 'BUILDING' AND c_custkey = o_custkey AND l_orderkey = o_orderkey AND o_orderdate < '1995-03-15' AND l_shipdate > '1995-03-15' GROUP BY l_orderkey, o_orderdate, o_shippriority ORDER BY revenue DESC, o_orderdate LIMIT 10", 60),
    ("Q4", "SELECT o_orderpriority, COUNT(*) AS order_count FROM orders WHERE o_orderdate >= '1993-07-01' AND o_orderdate < '1993-10-01' AND EXISTS (SELECT * FROM lineitem WHERE l_orderkey = o_orderkey AND l_commitdate < l_receiptdate) GROUP BY o_orderpriority ORDER BY o_orderpriority", 60),
    ("Q5", "SELECT n_name, SUM(l_extendedprice * (1 - l_discount)) AS revenue FROM customer, orders, lineitem, supplier, nation, region WHERE c_custkey = o_custkey AND l_orderkey = o_orderkey AND l_suppkey = s_suppkey AND c_nationkey = s_nationkey AND s_nationkey = n_nationkey AND n_regionkey = r_regionkey AND r_name = 'ASIA' AND o_orderdate >= '1994-01-01' AND o_orderdate < '1995-01-01' GROUP BY n_name ORDER BY revenue DESC", 60),
    ("Q6", "SELECT SUM(l_extendedprice * l_discount) AS revenue FROM lineitem WHERE l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01' AND l_discount BETWEEN 0.05 AND 0.07 AND l_quantity < 24", 60),
    ("Q7", "SELECT supp_nation, cust_nation, l_year, SUM(volume) AS revenue FROM (SELECT n1.n_name AS supp_nation, n2.n_name AS cust_nation, SUBSTR(l_shipdate, 1, 4) AS l_year, l_extendedprice * (1 - l_discount) AS volume FROM supplier, lineitem, orders, customer, nation n1, nation n2 WHERE s_suppkey = l_suppkey AND o_orderkey = l_orderkey AND c_custkey = o_custkey AND s_nationkey = n1.n_nationkey AND c_nationkey = n2.n_nationkey AND ((n1.n_name = 'FRANCE' AND n2.n_name = 'GERMANY') OR (n1.n_name = 'GERMANY' AND n2.n_name = 'FRANCE')) AND l_shipdate BETWEEN '1995-01-01' AND '1996-12-31') AS shipping GROUP BY supp_nation, cust_nation, l_year ORDER BY supp_nation, cust_nation, l_year", 60),
    ("Q8", "SELECT EXTRACT(YEAR FROM o_orderdate) AS o_year, SUM(CASE WHEN n2.n_name = 'GERMANY' THEN l_extendedprice * (1 - l_discount) ELSE 0 END) / SUM(l_extendedprice * (1 - l_discount)) AS mkt_share FROM customer, orders, lineitem, supplier, nation n1, nation n2, region WHERE c_custkey = o_custkey AND l_orderkey = o_orderkey AND l_suppkey = s_suppkey AND c_nationkey = n1.n_nationkey AND s_nationkey = n1.n_nationkey AND s_nationkey = n2.n_nationkey AND n1.n_regionkey = r_regionkey AND r_name = 'EUROPE' AND n2.n_name = 'GERMANY' AND o_orderdate >= '1995-01-01' AND o_orderdate < '1996-12-31' GROUP BY EXTRACT(YEAR FROM o_orderdate) ORDER BY o_year", 60),
    ("Q9", "SELECT nation, o_year, SUM(amount) AS sum_profit FROM (SELECT n_name AS nation, EXTRACT(YEAR FROM o_orderdate) AS o_year, l_extendedprice * (1 - l_discount) - ps_supplycost * l_quantity AS amount FROM part, supplier, lineitem, partsupp, orders, nation WHERE s_suppkey = l_suppkey AND ps_suppkey = l_suppkey AND ps_partkey = l_partkey AND p_partkey = l_partkey AND o_orderkey = o_orderkey AND s_nationkey = n_nationkey AND p_name LIKE '%green%') AS profit GROUP BY nation, o_year ORDER BY nation, o_year DESC", 60),
    ("Q10", "SELECT c_custkey, c_name, SUM(l_extendedprice * (1 - l_discount)) AS revenue, c_acctbal, n_name, c_address, c_phone, c_comment FROM customer, orders, lineitem, nation WHERE c_custkey = o_custkey AND l_orderkey = o_orderkey AND o_orderdate >= '1993-10-01' AND o_orderdate < '1994-01-01' AND l_returnflag = 'R' AND c_nationkey = n_nationkey GROUP BY c_custkey, c_name, c_acctbal, c_address, c_phone, c_comment, n_name ORDER BY revenue DESC LIMIT 20", 60),
    ("Q15", "SELECT s_suppkey, s_name, s_address, s_phone, total_revenue FROM supplier, (SELECT l_suppkey AS supplier_no, SUM(l_extendedprice * (1 - l_discount)) AS total_revenue FROM lineitem WHERE l_shipdate >= '1996-01-01' AND l_shipdate < '1996-04-01' GROUP BY l_suppkey) AS revenue_view WHERE s_suppkey = supplier_no AND total_revenue = (SELECT MAX(total_revenue) FROM (SELECT l_suppkey AS supplier_no, SUM(l_extendedprice * (1 - l_discount)) AS total_revenue FROM lineitem WHERE l_shipdate >= '1996-01-01' AND l_shipdate < '1996-04-01' GROUP BY l_suppkey) AS revenue_view) ORDER BY s_suppkey", 60),
    ("Q11", "SELECT ps_partkey, SUM(ps_supplycost * ps_availqty) AS value FROM partsupp, supplier, nation WHERE ps_suppkey = s_suppkey AND s_nationkey = n_nationkey AND n_name = 'GERMANY' GROUP BY ps_partkey HAVING SUM(ps_supplycost * ps_availqty) > (SELECT SUM(ps_supplycost * ps_availqty) * 0.0001 FROM partsupp, supplier, nation WHERE ps_suppkey = s_suppkey AND s_nationkey = n_nationkey AND n_name = 'GERMANY') ORDER BY value DESC", 60),
    ("Q12", "SELECT l_shipmode, SUM(CASE WHEN o_orderpriority = '1-URGENT' OR o_orderpriority = '2-HIGH' THEN 1 ELSE 0 END) AS high_line_count, SUM(CASE WHEN o_orderpriority <> '1-URGENT' AND o_orderpriority <> '2-HIGH' THEN 1 ELSE 0 END) AS low_line_count FROM orders, lineitem WHERE o_orderkey = l_orderkey AND l_shipmode IN ('MAIL', 'SHIP') AND l_commitdate < l_receiptdate AND l_shipdate < l_commitdate AND l_receiptdate >= '1994-01-01' AND l_receiptdate < '1995-01-01' GROUP BY l_shipmode ORDER BY l_shipmode", 60),
    ("Q13", "SELECT c_count, COUNT(*) AS custdist FROM (SELECT c_custkey, COUNT(o_orderkey) AS c_count FROM customer LEFT OUTER JOIN orders ON c_custkey = o_custkey AND o_comment NOT LIKE '%special%requests%' GROUP BY c_custkey) AS c_orders GROUP BY c_count ORDER BY custdist DESC, c_count DESC", 60),
    ("Q14", "SELECT 100.00 * SUM(CASE WHEN p_type LIKE 'PROMO%' THEN l_extendedprice * (1 - l_discount) ELSE 0 END) / SUM(l_extendedprice * (1 - l_discount)) AS promo_revenue FROM lineitem, part WHERE l_partkey = p_partkey AND l_shipdate >= '1995-09-01' AND l_shipdate < '1995-10-01'", 60),
    ("Q16", "SELECT p_brand, p_type, p_size, COUNT(DISTINCT ps_suppkey) AS supplier_cnt FROM partsupp, part WHERE p_partkey = ps_partkey AND p_brand <> 'Brand#45' AND p_type NOT LIKE 'MEDIUM POLISHED%' AND p_size IN (49, 14, 23, 45, 19, 3, 36, 9) AND ps_suppkey NOT IN (SELECT s_suppkey FROM supplier WHERE s_comment LIKE '%Customer%Complaints%') GROUP BY p_brand, p_type, p_size ORDER BY supplier_cnt DESC, p_brand, p_type, p_size", 60),
    ("Q17", "SELECT SUM(l_extendedprice) / 7.0 AS avg_yearly FROM lineitem, part WHERE l_partkey = p_partkey AND p_brand = 'Brand#23' AND p_container = 'MED BOX' AND l_quantity < (SELECT 0.2 * AVG(l_quantity) FROM lineitem WHERE l_partkey = p_partkey)", 60),
    ("Q18", "SELECT l_orderkey, SUM(l_quantity) AS sum_qty, o_orderdate, o_shippriority FROM customer, orders, lineitem WHERE c_custkey = o_custkey AND l_orderkey = o_orderkey AND o_orderkey IN (SELECT l_orderkey FROM lineitem GROUP BY l_orderkey HAVING SUM(l_quantity) > 312) GROUP BY l_orderkey, o_orderdate, o_shippriority ORDER BY o_orderdate, o_shippriority LIMIT 100", 60),
    ("Q19", "SELECT SUM(l_extendedprice * (1 - l_discount)) AS revenue FROM lineitem, part WHERE l_partkey = p_partkey AND l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01' AND ((p_brand = 'Brand#12' AND p_container IN ('SM CASE', 'SM BOX', 'SM PACK', 'SM PKG') AND l_quantity >= 1 AND l_quantity <= 11 AND p_size BETWEEN 1 AND 5) OR (p_brand = 'Brand#23' AND p_container IN ('MED BAG', 'MED BOX', 'MED PKG', 'MED PACK') AND l_quantity >= 10 AND l_quantity <= 20 AND p_size BETWEEN 1 AND 10) OR (p_brand = 'Brand#34' AND p_container IN ('LG CASE', 'LG BOX', 'LG PACK', 'LG PKG') AND l_quantity >= 20 AND l_quantity <= 30 AND p_size BETWEEN 1 AND 15))", 60),
    ("Q20", "SELECT s_name, s_address FROM supplier, nation WHERE s_suppkey IN (SELECT ps_suppkey FROM partsupp WHERE ps_partkey IN (SELECT p_partkey FROM part WHERE p_name LIKE 'forest%') AND ps_availqty > (SELECT 0.5 * SUM(l_quantity) FROM lineitem WHERE l_partkey = ps_partkey AND l_suppkey = ps_suppkey AND l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01')) AND s_nationkey = n_nationkey AND n_name = 'CANADA' ORDER BY s_name", 60),
    ("Q21", "SELECT s_name, COUNT(*) AS numwait FROM supplier, lineitem l1, orders, nation WHERE s_suppkey = l1.l_suppkey AND o_orderkey = l1.l_orderkey AND o_orderstatus = 'F' AND l1.l_receiptdate > l1.l_commitdate AND EXISTS (SELECT * FROM lineitem l2 WHERE l2.l_orderkey = l1.l_orderkey AND l2.l_suppkey <> l1.l_suppkey) AND NOT EXISTS (SELECT * FROM lineitem l3 WHERE l3.l_orderkey = l1.l_orderkey AND l3.l_suppkey <> l1.l_suppkey AND l3.l_receiptdate > l3.l_commitdate) AND s_nationkey = n_nationkey AND n_name = 'SAUDI ARABIA' GROUP BY s_name ORDER BY numwait DESC, s_name LIMIT 100", 60),
    ("Q22", "SELECT cntrycode, COUNT(*) AS numcust, SUM(c_acctbal) AS totacctbal FROM (SELECT SUBSTR(c_phone, 1, 2) AS cntrycode, c_acctbal FROM customer WHERE SUBSTR(c_phone, 1, 2) IN ('13', '31', '23', '29', '30', '18', '17') AND c_acctbal > (SELECT AVG(c_acctbal) FROM customer WHERE c_acctbal > 0.00 AND SUBSTR(c_phone, 1, 2) IN ('13', '31', '23', '29', '30', '18', '17')) AND NOT EXISTS (SELECT * FROM orders WHERE o_custkey = c_custkey)) AS custsale GROUP BY cntrycode ORDER BY cntrycode", 60),
];

/// Statements whose result is a single aggregate.
///
/// Their value is `NULL` when the filtered set is empty, and the wire
/// protocol renders a `NULL` as the empty string — which cannot be told
/// apart from a real empty string. Only the row count is compared for
/// them.
const ROW_COUNT_ONLY: [&str; 3] = ["Q6", "Q14", "Q17"];

/// The shared ephemeral server.
static CLIENT: OnceLock<Mutex<Option<common::MySqlTestClient>>> = OnceLock::new();

/// Whether the connection is still usable.
fn is_alive(client: &mut common::MySqlTestClient) -> bool {
    client.query_rows("SELECT 1").is_ok()
}

/// Run `f` with a live wire client.
///
/// The server is shared by the six tests of this target: loading the
/// SF=0.01 fixture into it costs about half a minute, and paying that six
/// times would make the target slower than the CI timeout. A connection
/// that died (the server closes it when a statement takes the engine
/// down) is replaced by a fresh server, so one broken statement cannot
/// take the other five tests with it.
fn with_client<R>(f: impl FnOnce(&mut common::MySqlTestClient) -> R) -> R {
    let slot = CLIENT.get_or_init(|| Mutex::new(None));
    let mut guard = slot.lock().unwrap_or_else(|e| e.into_inner());
    let dead = match guard.as_mut() {
        Some(client) => !is_alive(client),
        None => true,
    };
    if dead {
        *guard = Some(
            start_sf01_named(TARGET)
                .expect("the fixture was resolved, so the ephemeral server must start"),
        );
    }
    f(guard.as_mut().expect("a client is present"))
}

/// Compare the wire-protocol rows with the oracle's rows.
///
/// `content` selects the comparison strength: `true` compares the cell
/// values (as a sorted multiset), `false` compares only the row count.
/// Returns `None` when the two sides agree.
fn compare_rows(
    wire_rows: &[Vec<String>],
    oracle_rows: &[Vec<String>],
    content: bool,
) -> Option<String> {
    if wire_rows.len() != oracle_rows.len() {
        return Some(format!(
            "row count differs: wire={} oracle={}",
            wire_rows.len(),
            oracle_rows.len()
        ));
    }
    if !content {
        return None;
    }
    // The wire protocol renders a SQL NULL as the empty string, which
    // cannot be told apart from a real empty string. A NULL in the
    // oracle's result would therefore hide a difference, so the
    // comparison refuses to run instead of pretending to be meaningful.
    let nulls = oracle_rows
        .iter()
        .flat_map(|r| r.iter())
        .filter(|c| c.as_str() == NULL_CELL)
        .count();
    if nulls > 0 {
        return Some(format!(
            "the statement produced {nulls} NULL cell(s); the wire protocol renders a NULL as \
             the empty string, so the two sides cannot be compared"
        ));
    }
    let mut wire = wire_rows.to_vec();
    let mut oracle = oracle_rows.to_vec();
    wire.sort();
    oracle.sort();
    diff_rows(&wire, &oracle)
}

/// Run the statements in `start..end` and assert what was actually
/// compared.
fn run_range(start: usize, end: usize) {
    let state = tpch_fixture::FixtureState::resolve();
    let Some(fixture) = state.require(TARGET) else {
        // `require` has already printed the NOT-RUN marker.
        return;
    };

    let oracle = SqliteOracle::open(fixture, &TPC_H_ALL_TABLES)
        .unwrap_or_else(|e| panic!("{TARGET}: cannot build the independent SQLite oracle: {e}"));

    with_client(|client| {
        let mut compared = 0usize;
        let mut mismatched = 0usize;
        let mut known = 0usize;
        let mut not_run = 0usize;
        let mut failed = 0usize;
        let mut excluded: Vec<String> = Vec::new();
        // The statements of this range that the file-storage wire path must
        // not execute, and the ones it must. Both counts are checked
        // against the shared scope afterwards, so a statement can neither
        // slip through nor disappear.
        let mut skipped_scope: Vec<&str> = Vec::new();
        let expected_scoped = TPC_H_SF01_QUERIES[start..end]
            .iter()
            .filter(|(qid, ..)| tpch_wire_scope::in_wire_scope(qid))
            .count();
        let expected_skipped = TPC_H_SF01_QUERIES[start..end].len() - expected_scoped;

        for (qid, sql, timeout_s) in &TPC_H_SF01_QUERIES[start..end] {
            // The label of the statement in the NOT-RUN marker. The
            // known-difference registry is keyed by the *statement* (see
            // `tpch_fixture::statement_case_id`), so a difference that
            // stems from the engine's SQL semantics is registered once and
            // not once per test target.
            let not_run_label = format!("{TARGET}:{qid}");

            // The SQLite oracle only rewrites an explicit whitelist of
            // constructs. Anything else is reported here instead of being
            // silently rewritten — comparing the engine against a
            // *different* statement would prove nothing.
            let adapted = adapt_sql(sql);
            if !adapted.unsupported.is_empty() {
                let detail = format!(
                    "the SQLite oracle cannot execute this statement ({:?}); \
                     it belongs to the cross-engine dialect layer",
                    adapted.unsupported
                );
                report_not_run(&not_run_label, reason::SCALE_LAYER_ONLY, &detail);
                excluded.push(format!("{qid}: {detail}"));
                not_run += 1;
                continue;
            }

            let outcome = run_query_timed(client, qid, sql, *timeout_s);
            let (wire_rows, elapsed) = match outcome {
                WireOutcome::Ran(Ok(rows), elapsed) => (rows, elapsed),
                WireOutcome::Ran(Err(e), elapsed) => {
                    failed += 1;
                    eprintln!("  {qid:3}: ERROR ({e}) in {elapsed:?}");
                    continue;
                }
                WireOutcome::Skipped(skip) => {
                    // Outside the wire scope: the statement is NOT executed.
                    // Executing it would exhaust the machine and kill the
                    // test binary with it (issue #5393).
                    not_run += 1;
                    skipped_scope.push(*qid);
                    eprintln!(
                        "  {qid:3}: SKIPPED ({} issue={}) — {}",
                        skip.reason, skip.issue, skip.detail
                    );
                    continue;
                }
            };

            let oracle_rows = match oracle.query(&adapted.sql) {
                Ok(rows) => rows,
                Err(e) => {
                    failed += 1;
                    eprintln!("  {qid:3}: the SQLite oracle failed on this statement ({e})");
                    continue;
                }
            };

            let content = !ROW_COUNT_ONLY.contains(qid);
            match compare_rows(&wire_rows, &oracle_rows, content) {
                None => {
                    compared += 1;
                    eprintln!("  {qid:3}: MATCH (rows={}) in {elapsed:?}", wire_rows.len());
                }
                Some(diff) => match lookup_statement_known_difference(qid) {
                    Some(entry) => {
                        report_known_difference(&entry, &diff);
                        known += 1;
                        compared += 1;
                        eprintln!("  {qid:3}: KNOWN-DIFF (registered) {diff}");
                    }
                    None => {
                        compared += 1;
                        mismatched += 1;
                        eprintln!("  {qid:3}: MISMATCH {diff}");
                    }
                },
            }
        }

        eprintln!();
        eprintln!(
            "=== Summary: compared={compared} mismatched={mismatched} known={known} \
             not_run={not_run} failed={failed} ==="
        );
        for line in &excluded {
            eprintln!("=== Excluded: {line}");
        }

        // Every statement of the range must be accounted for: either it was
        // compared against the independent oracle, or it was skipped for a
        // registered reason. Nothing may disappear in between.
        assert_eq!(
            compared + not_run,
            end - start,
            "{TARGET}: {compared} compared + {not_run} not run does not account for the \
             {} statements of this range",
            end - start
        );
        // ... and the skipped ones must be exactly the ones the shared wire
        // scope excludes — neither more (a silently dropped statement) nor
        // fewer (a statement that could have been compared).
        assert_eq!(
            skipped_scope.len(),
            expected_skipped,
            "{TARGET}: {skipped_scope:?} were skipped, but the wire scope excludes \
             {expected_skipped} statement(s) from this range"
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
        assert_eq!(
            mismatched, 0,
            "{TARGET}: {mismatched} statement(s) did not match the independent SQLite oracle. \
             (If this is a known engine defect, register it in \
             tests/baseline/tpch_known_differences.json with an issue and an owner.)"
        );
        assert_eq!(
            failed, 0,
            "{TARGET}: {failed} statement(s) could not be executed on one of the two sides"
        );

        // Disclose the statements this function did not execute. One marker
        // per test function keeps the CI arithmetic exact: the summary
        // subtracts the number of NOT-RUN markers from the number of passed
        // test functions.
        tpch_wire_scope::report_wire_exclusions(TARGET, &skipped_scope);
    });
}

#[test]
fn g15_tpch_sf01_wire_matches_baseline() {
    run_range(0, TPC_H_SF01_QUERIES.len());
}

#[test]
fn g15_sf01_q01_q06() {
    run_range(0, 6);
}

#[test]
fn g15_sf01_q07_q10() {
    run_range(6, 10);
}

#[test]
fn g15_sf01_q11_q14() {
    // The old code ran `run_subset(11, 14)`, which covered Q11-Q13 and
    // left Q14 (index 14) to no test at all. The range is corrected here
    // so that the name of the test is true.
    run_range(11, 15);
}

#[test]
fn g15_sf01_q15() {
    run_range(10, 11);
}

#[test]
fn g15_sf01_q16_q22() {
    run_range(15, 22);
}
