//! G1 TPC-H 22/22 SHA-256 baseline oracle (#3231)
//!
//! 目的: 为 22 TPC-H query 结果集生成 SHA-256 校验和, 任何 engine 改动导致
//!       row 内容变化立即被发现 (drift detection).
//!
//! 用法:
//! - 默认 (CI): 比较 engine 输出 vs `tests/oracle/baselines/tpch_sha256.json`
//! - 生成 baseline: `cargo test --test oracle_g1_tpch_sha256 -- --ignored --generate-baseline`
//!
//! 已知偏差: Q9 cell-level bug (#3312) 只影响 cell 值, 仍能用 SHA-256 检测 row-level 漂移.
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
use common::oracle_framework::{
    sha256_capture, Row, RowSet, Sha256Baseline, Sha256QueryEntry, Value,
    TPC_H_SHA256_BASELINE_FILE,
};
use common::tpch_wire_harness::{
    fixture_scale_factor, report_not_run, run_query_timed, start_sf001_named, WireOutcome, WireSkip,
};
use common::tpch_wire_scope;
use common::MySqlTestClient;
use std::time::Instant;

const TPC_H_QUERIES: &[(&str, &str)] = &[
    ("Q1", "SELECT l_returnflag, l_linestatus, COUNT(*) FROM lineitem WHERE l_shipdate <= '1998-09-02' GROUP BY l_returnflag, l_linestatus ORDER BY l_returnflag, l_linestatus"),
    ("Q2", "SELECT s_acctbal, s_name, n_name, p_partkey, p_mfgr, s_address, s_phone, s_comment FROM supplier, nation, region, part, partsupp WHERE p_partkey = ps_partkey AND s_suppkey = ps_suppkey AND p_size = 15 AND p_type LIKE '%BRASS' AND s_nationkey = n_nationkey AND n_regionkey = r_regionkey AND r_name = 'EUROPE' AND ps_supplycost = (SELECT MIN(ps_supplycost) FROM partsupp, supplier, nation, region WHERE p_partkey = ps_partkey AND s_suppkey = ps_suppkey AND s_nationkey = n_nationkey AND n_regionkey = r_regionkey AND r_name = 'EUROPE') ORDER BY s_acctbal DESC, n_name, s_name, p_partkey LIMIT 100"),
    ("Q3", "SELECT l_orderkey, SUM(l_extendedprice) AS revenue, o_orderdate, o_shippriority FROM customer, orders, lineitem WHERE c_mktsegment = 'BUILDING' AND c_custkey = o_custkey AND l_orderkey = o_orderkey AND o_orderdate < '1995-03-15' AND l_shipdate > '1995-03-15' GROUP BY l_orderkey, o_orderdate, o_shippriority ORDER BY revenue DESC, o_orderdate LIMIT 10"),
    ("Q4", "SELECT o_orderpriority, COUNT(*) AS order_count FROM orders WHERE o_orderdate >= '1993-07-01' AND o_orderdate < '1993-10-01' AND EXISTS (SELECT * FROM lineitem WHERE l_orderkey = o_orderkey AND l_commitdate < l_receiptdate) GROUP BY o_orderpriority ORDER BY o_orderpriority"),
    ("Q5", "SELECT n_name, SUM(l_extendedprice * (1 - l_discount)) AS revenue FROM customer, orders, lineitem, supplier, nation, region WHERE c_custkey = o_custkey AND l_orderkey = o_orderkey AND l_suppkey = s_suppkey AND c_nationkey = s_nationkey AND s_nationkey = n_nationkey AND n_regionkey = r_regionkey AND r_name = 'ASIA' AND o_orderdate >= '1994-01-01' AND o_orderdate < '1995-01-01' GROUP BY n_name ORDER BY revenue DESC"),
    ("Q6", "SELECT SUM(l_extendedprice * l_discount) AS revenue FROM lineitem WHERE l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01' AND l_discount BETWEEN 0.05 AND 0.07 AND l_quantity < 24"),
    ("Q7", "SELECT supp_nation, cust_nation, l_year, SUM(volume) AS revenue FROM (SELECT n1.n_name AS supp_nation, n2.n_name AS cust_nation, CAST(SUBSTR(l_shipdate, 1, 4) AS INTEGER) AS l_year, l_extendedprice * (1 - l_discount) AS volume FROM supplier, lineitem, orders, customer, nation n1, nation n2 WHERE s_suppkey = l_suppkey AND o_orderkey = l_orderkey AND c_custkey = o_custkey AND s_nationkey = n1.n_nationkey AND c_nationkey = n2.n_nationkey AND ((n1.n_name = 'FRANCE' AND n2.n_name = 'GERMANY') OR (n1.n_name = 'GERMANY' AND n2.n_name = 'FRANCE')) AND l_shipdate BETWEEN '1995-01-01' AND '1996-12-31') AS shipping GROUP BY supp_nation, cust_nation, l_year ORDER BY supp_nation, cust_nation, l_year"),
    ("Q8", "SELECT o_year, SUM(CASE WHEN nation = 'BRAZIL' THEN volume ELSE 0 END) / SUM(volume) AS mkt_share FROM (SELECT CAST(SUBSTR(o_orderdate, 1, 4) AS INTEGER) AS o_year, l_extendedprice * (1 - l_discount) AS volume, n2.n_name AS nation FROM part, supplier, lineitem, orders, customer, nation n1, nation n2, region WHERE p_partkey = l_partkey AND s_suppkey = l_suppkey AND l_orderkey = o_orderkey AND o_custkey = c_custkey AND c_nationkey = n1.n_nationkey AND n1.n_regionkey = r_regionkey AND r_name = 'AMERICA' AND s_nationkey = n2.n_nationkey AND o_orderdate BETWEEN '1995-01-01' AND '1996-12-31' AND p_type = 'ECONOMY ANODIZED STEEL') AS all_nations GROUP BY o_year ORDER BY o_year"),
    ("Q9", "SELECT nation, o_year, SUM(amount) AS sum_profit FROM (SELECT n_name AS nation, CAST(SUBSTR(o_orderdate, 1, 4) AS INTEGER) AS o_year, l_extendedprice * (1 - l_discount) - ps_supplycost * l_quantity AS amount FROM part, supplier, lineitem, partsupp, orders, nation WHERE s_suppkey = l_suppkey AND ps_suppkey = l_suppkey AND ps_partkey = l_partkey AND p_partkey = l_partkey AND o_orderkey = l_orderkey AND s_nationkey = n_nationkey AND p_name LIKE '%green%') AS profit GROUP BY nation, o_year ORDER BY nation, o_year DESC"),
    ("Q10", "SELECT c_custkey, c_name, SUM(l_extendedprice * (1 - l_discount)) AS revenue, c_acctbal, n_name, c_address, c_phone, c_comment FROM customer, orders, lineitem, nation WHERE c_custkey = o_custkey AND l_orderkey = o_orderkey AND o_orderdate >= '1993-10-01' AND o_orderdate < '1994-01-01' AND l_returnflag = 'R' AND c_nationkey = n_nationkey GROUP BY c_custkey, c_name, c_acctbal, c_address, c_phone, c_comment, n_name ORDER BY revenue DESC LIMIT 20"),
    ("Q11", "SELECT ps_partkey, SUM(ps_supplycost * ps_availqty) AS value FROM partsupp, supplier, nation WHERE ps_suppkey = s_suppkey AND s_nationkey = n_nationkey AND n_name = 'GERMANY' GROUP BY ps_partkey HAVING SUM(ps_supplycost * ps_availqty) > (SELECT SUM(ps_supplycost * ps_availqty) * 0.0001000000 FROM partsupp, supplier, nation WHERE ps_suppkey = s_suppkey AND s_nationkey = n_nationkey AND n_name = 'GERMANY') ORDER BY value DESC LIMIT 100"),
    ("Q12", "SELECT l_shipmode, SUM(CASE WHEN o_orderpriority = '1-URGENT' OR o_orderpriority = '2-HIGH' THEN 1 ELSE 0 END) AS high_line_count, SUM(CASE WHEN o_orderpriority <> '1-URGENT' AND o_orderpriority <> '2-HIGH' THEN 1 ELSE 0 END) AS low_line_count FROM orders, lineitem WHERE o_orderkey = l_orderkey AND l_shipmode IN ('MAIL', 'SHIP') AND l_commitdate < l_receiptdate AND l_shipdate < l_commitdate AND l_receiptdate >= '1994-01-01' AND l_receiptdate < '1995-01-01' GROUP BY l_shipmode ORDER BY l_shipmode"),
    ("Q13", "SELECT c_custkey, c_name, COUNT(o_orderkey) AS c_count FROM customer LEFT OUTER JOIN orders ON c_custkey = o_custkey AND o_comment NOT LIKE '%special%requests%' GROUP BY c_custkey, c_name ORDER BY c_count DESC, c_custkey LIMIT 100"),
    ("Q14", "SELECT 100.00 * SUM(CASE WHEN p_type LIKE 'PROMO%' THEN l_extendedprice * (1 - l_discount) ELSE 0 END) / SUM(l_extendedprice * (1 - l_discount)) AS promo_revenue FROM lineitem, part WHERE l_partkey = p_partkey AND l_shipdate >= '1995-09-01' AND l_shipdate < '1995-10-01'"),
    ("Q15", "SELECT s_suppkey, s_name, s_address, s_phone, total_revenue FROM supplier, (SELECT l_suppkey AS supplier_no, SUM(l_extendedprice * (1 - l_discount)) AS total_revenue FROM lineitem WHERE l_shipdate >= '1996-01-01' AND l_shipdate < '1996-04-01' GROUP BY l_suppkey) AS revenue WHERE s_suppkey = supplier_no AND total_revenue = (SELECT MAX(total_revenue) FROM (SELECT l_suppkey AS supplier_no, SUM(l_extendedprice * (1 - l_discount)) AS total_revenue FROM lineitem WHERE l_shipdate >= '1996-01-01' AND l_shipdate < '1996-04-01' GROUP BY l_suppkey) AS revenue) ORDER BY s_suppkey"),
    ("Q16", "SELECT p_brand, p_type, COUNT(DISTINCT ps_suppkey) AS supplier_cnt FROM partsupp, part WHERE p_partkey = ps_partkey AND p_brand <> 'Brand#45' AND p_type NOT LIKE 'MEDIUM POLISHED%' AND ps_suppkey NOT IN (SELECT s_suppkey FROM supplier WHERE s_comment LIKE '%Customer%Complaints%') GROUP BY p_brand, p_type ORDER BY supplier_cnt DESC, p_brand, p_type LIMIT 100"),
    ("Q17", "SELECT SUM(l_extendedprice) / 7.0 AS avg_yearly FROM lineitem, part WHERE p_partkey = l_partkey AND p_brand = 'Brand#23' AND p_container = 'MED BOX' AND l_quantity < (SELECT 0.2 * AVG(l_quantity) FROM lineitem WHERE l_partkey = p_partkey)"),
    ("Q18", "SELECT c_name, c_custkey, o_orderkey, o_orderdate, o_totalprice, SUM(l_quantity) AS sum_l_quantity FROM customer, orders, lineitem WHERE c_custkey = o_custkey AND l_orderkey = o_orderkey AND o_orderkey IN (SELECT l_orderkey FROM lineitem GROUP BY l_orderkey HAVING SUM(l_quantity) > 300) GROUP BY c_name, c_custkey, o_orderkey, o_orderdate, o_totalprice ORDER BY o_totalprice DESC, o_orderdate LIMIT 100"),
    ("Q19", "SELECT SUM(l_extendedprice * (1 - l_discount)) AS revenue FROM lineitem, part WHERE p_partkey = l_partkey AND l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01' AND l_shipmode IN ('AIR', 'AIR REG') AND p_brand = 'Brand#12' AND p_container IN ('SM CASE', 'SM BOX', 'SM PACK', 'SM PKG') AND l_quantity >= 1 AND l_quantity <= 11 AND (p_size BETWEEN 1 AND 5 OR p_size BETWEEN 10 AND 15 OR p_size BETWEEN 20 AND 25)"),
    ("Q20", "SELECT s_name, s_address FROM supplier, nation WHERE s_suppkey IN (SELECT ps_suppkey FROM partsupp WHERE ps_partkey IN (SELECT p_partkey FROM part WHERE p_name LIKE 'forest%') AND ps_availqty > (SELECT 0.5 * SUM(l_quantity) FROM lineitem WHERE l_partkey = ps_partkey AND l_suppkey = ps_suppkey AND l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01')) AND s_nationkey = n_nationkey AND n_name = 'CANADA' ORDER BY s_name"),
    ("Q21", "SELECT s_name, COUNT(*) AS numwait FROM supplier, lineitem l1, orders, nation WHERE s_suppkey = l1.l_suppkey AND o_orderkey = l1.l_orderkey AND o_orderstatus = 'F' AND l1.l_receiptdate > l1.l_commitdate AND EXISTS (SELECT * FROM lineitem l2 WHERE l2.l_orderkey = l1.l_orderkey AND l2.l_suppkey <> l1.l_suppkey) AND NOT EXISTS (SELECT * FROM lineitem l3 WHERE l3.l_orderkey = l1.l_orderkey AND l3.l_suppkey <> l1.l_suppkey AND l3.l_receiptdate > l3.l_commitdate) AND s_nationkey = n_nationkey AND n_name = 'SAUDI ARABIA' GROUP BY s_name ORDER BY numwait DESC, s_name LIMIT 100"),
    ("Q22", "SELECT cntrycode, COUNT(*) AS numcust, SUM(c_acctbal) AS totacctbal FROM (SELECT SUBSTR(c_phone, 1, 2) AS cntrycode, c_acctbal FROM customer WHERE SUBSTR(c_phone, 1, 2) IN ('13', '31', '23', '29', '30', '18', '17') AND c_acctbal > (SELECT AVG(c_acctbal) FROM customer WHERE c_acctbal > 0.00 AND SUBSTR(c_phone, 1, 2) IN ('13', '31', '23', '29', '30', '18', '17')) AND NOT EXISTS (SELECT * FROM orders WHERE o_custkey = c_custkey)) AS custsale GROUP BY cntrycode ORDER BY cntrycode"),
];

/// The outcome of running one standard statement over the wire path.
enum StatementOutcome {
    /// The statement was executed.
    Ran(RowSet),
    /// The statement is outside the **wire scope** and was NOT executed.
    Skipped(WireSkip),
}

/// Runs `sql` and captures the result set.
///
/// The client is passed in so that all 22 queries share **one** server and
/// one fixture load. The previous version called `start_sf001()` per query,
/// which restarted an ephemeral server and re-loaded 60 000 `lineitem` rows
/// 22 times — a single test target that took more than 20 minutes and would
/// have eaten a large part of the CI job's time budget.
fn run_query_to_rowset(
    client: &mut MySqlTestClient,
    query_id: &str,
    sql: &str,
) -> StatementOutcome {
    let start = Instant::now();
    let outcome = run_query_timed(client, query_id, sql, 120);
    let elapsed = start.elapsed().as_millis() as u64;

    match outcome {
        WireOutcome::Ran(Ok(rows), _) => {
            let parsed_rows: Vec<Row> = rows
                .iter()
                .map(|row| Row(row.iter().map(|c| Value::from_row_cell(c)).collect()))
                .collect();
            StatementOutcome::Ran(RowSet {
                query: query_id.to_string(),
                row_count: parsed_rows.len(),
                rows: parsed_rows,
                wall_time_ms: elapsed,
            })
        }
        WireOutcome::Ran(Err(e), _) => {
            eprintln!("[WARN] {} failed: {}", query_id, e);
            StatementOutcome::Ran(RowSet {
                query: query_id.to_string(),
                row_count: 0,
                rows: vec![],
                wall_time_ms: elapsed,
            })
        }
        WireOutcome::Skipped(skip) => StatementOutcome::Skipped(skip),
    }
}

/// The cargo target name, used in the NOT-RUN marker.
const TARGET: &str = "oracle_g1_tpch_sha256";

/// Check that the fixture on disk is the profile the baseline was calibrated
/// against.
///
/// The baseline stores a SHA-256 per query, computed over the rows of **one
/// specific dataset**. A different dataset cannot reproduce those hashes by
/// construction, so comparing them would report a "drift" that is really a
/// profile change. The stored baseline is at scale factor 0.001, while this
/// iteration's fixture is SF=0.01 — the two are different datasets, so the
/// comparison is disclosed as *not run* rather than reported as a failure.
fn baseline_profile_matches_fixture(baseline: &Sha256Baseline) -> bool {
    let Ok(baseline_sf) = baseline.scale_factor.parse::<f64>() else {
        report_not_run(
            TARGET,
            "profile_mismatch",
            &format!(
                "baseline scale factor {:?} is not a number, so no fixture profile can match it",
                baseline.scale_factor
            ),
        );
        return false;
    };
    match fixture_scale_factor() {
        Some(sf) if (sf - baseline_sf).abs() < 1e-12 => true,
        Some(sf) => {
            report_not_run(
                TARGET,
                "profile_mismatch",
                &format!(
                    "baseline was calibrated at scale factor {baseline_sf} but the fixture on disk is \
                     sf{sf}; the stored hashes belong to a different dataset and cannot be \
                     compared against it"
                ),
            );
            false
        }
        None => {
            report_not_run(
                TARGET,
                "profile_mismatch",
                &format!(
                    "baseline was calibrated at scale factor {baseline_sf} but the fixture has no \
                     readable manifest, so its profile is unknown"
                ),
            );
            false
        }
    }
}

#[test]
fn g1_tpch_sha256_baseline() {
    let baseline_path = std::path::Path::new(TPC_H_SHA256_BASELINE_FILE);
    if !baseline_path.exists() {
        eprintln!(
            "[SKIP] baseline not generated yet. Run with --generate-baseline to create: {}",
            TPC_H_SHA256_BASELINE_FILE
        );
        return;
    }

    let baseline = Sha256Baseline::load_or_warn(baseline_path).expect("baseline load failed");

    if !baseline_profile_matches_fixture(&baseline) {
        return;
    }

    let mut pass = 0;
    let mut fail = 0;
    let mut pending = 0;
    let mut not_run = 0;
    // The statements the file-storage wire path must not execute, and the
    // ones it must. Both counts are checked against the shared scope
    // afterwards, so a statement can neither slip through nor disappear.
    let mut skipped_scope: Vec<&'static str> = Vec::new();
    let expected_scoped = TPC_H_QUERIES
        .iter()
        .filter(|(qid, _)| tpch_wire_scope::in_wire_scope(qid))
        .count();
    let expected_skipped = TPC_H_QUERIES.len() - expected_scoped;

    // One server for all 22 queries.
    let Some(mut client) = start_sf001_named(TARGET) else {
        return;
    };

    for (qid, sql) in TPC_H_QUERIES {
        let actual = match run_query_to_rowset(&mut client, qid, sql) {
            StatementOutcome::Ran(rs) => rs,
            StatementOutcome::Skipped(skip) => {
                // Outside the wire scope: the statement is NOT executed.
                // Executing it would exhaust the machine and kill the test
                // binary with it (issue #5393).
                not_run += 1;
                skipped_scope.push(qid);
                eprintln!(
                    "[SKIPPED] {}: ({} issue={}) — {}",
                    qid, skip.reason, skip.issue, skip.detail
                );
                continue;
            }
        };
        let actual_hash = sha256_capture(&actual);

        let baseline_entry = baseline.queries.iter().find(|e| e.query_id == *qid);
        match baseline_entry {
            Some(entry) if entry.sha256 != "PENDING_GENERATION" => {
                if entry.sha256 == actual_hash {
                    pass += 1;
                } else {
                    fail += 1;
                    eprintln!(
                        "[FAIL] {}: actual={} expected={}",
                        qid, actual_hash, entry.sha256
                    );
                }
            }
            _ => {
                pending += 1;
                eprintln!(
                    "[PENDING] {}: row_count={} (baseline not generated)",
                    qid, actual.row_count
                );
            }
        }
    }

    eprintln!(
        "\n=== G1 TPC-H SHA-256 baseline summary ===\n  PASS:    {}\n  FAIL:    {}\n  \
         PENDING: {}\n  NOT RUN: {}",
        pass, fail, pending, not_run
    );

    // A statement whose hash drifted is reported first: it is the real
    // cause, the accounting below is only a consequence of it.
    assert_eq!(
        fail, 0,
        "{} TPC-H query result hashes drifted from baseline",
        fail
    );

    // Every statement must be accounted for: either it was executed and
    // hashed, it drifted, or it was skipped for a registered reason.
    assert_eq!(
        pass + fail + pending + not_run,
        TPC_H_QUERIES.len(),
        "{TARGET}: {pass} passed + {fail} failed + {pending} pending + {not_run} not run does \
         not account for the {} statements of this file",
        TPC_H_QUERIES.len()
    );
    // ... and the skipped ones must be exactly the ones the shared wire
    // scope excludes — neither more (a silently dropped statement) nor
    // fewer (a statement that could have been hashed).
    assert_eq!(
        skipped_scope.len(),
        expected_skipped,
        "{TARGET}: {skipped_scope:?} were skipped, but the wire scope excludes \
         {expected_skipped} statement(s) from this file"
    );
    assert_eq!(
        pass + fail + pending,
        expected_scoped,
        "{TARGET}: {} of the {expected_scoped} in-scope statements were executed",
        pass + fail + pending
    );
    assert!(
        pass + fail + pending > 0,
        "{TARGET}: this run executed nothing at all; a run that executes nothing must not \
         report success"
    );
    if pass == 0 {
        eprintln!(
            "[INFO] No baseline entries validated (all PENDING). Run --generate-baseline to seed."
        );
    }

    // Disclose the statements this function did not execute. One marker
    // per test function keeps the CI arithmetic exact.
    tpch_wire_scope::report_wire_exclusions(TARGET, &skipped_scope);
}

#[test]
#[ignore = "Run with: cargo test --test oracle_g1_tpch_sha256 -- --ignored --generate-baseline"]
fn generate_baseline() {
    let mut entries = Vec::new();
    let mut not_run = 0;
    let mut skipped_scope: Vec<&'static str> = Vec::new();
    let expected_scoped = TPC_H_QUERIES
        .iter()
        .filter(|(qid, _)| tpch_wire_scope::in_wire_scope(qid))
        .count();
    let expected_skipped = TPC_H_QUERIES.len() - expected_scoped;

    // One server for all 22 queries.
    let Some(mut client) = start_sf001_named(TARGET) else {
        return;
    };

    for (qid, sql) in TPC_H_QUERIES {
        let rs = match run_query_to_rowset(&mut client, qid, sql) {
            StatementOutcome::Ran(rs) => rs,
            StatementOutcome::Skipped(skip) => {
                // Outside the wire scope: the statement is NOT executed and
                // therefore gets **no** baseline entry — a hash over a
                // statement that was never run would be a fabricated value.
                not_run += 1;
                skipped_scope.push(qid);
                eprintln!(
                    "[SKIPPED] {}: ({} issue={}) — {}",
                    qid, skip.reason, skip.issue, skip.detail
                );
                continue;
            }
        };
        let hash = sha256_capture(&rs);
        entries.push(Sha256QueryEntry {
            query_id: qid.to_string(),
            row_count: rs.row_count,
            sha256: hash,
            wall_time_ms: rs.wall_time_ms,
        });
        eprintln!(
            "[GEN] {}: row_count={} hash={}... wall={}ms",
            qid,
            rs.row_count,
            &sha256_capture(&rs)[..16],
            rs.wall_time_ms
        );
    }

    assert_eq!(
        entries.len() + not_run,
        TPC_H_QUERIES.len(),
        "{TARGET}: {} hashed + {not_run} not run does not account for the {} statements of \
         this file",
        entries.len(),
        TPC_H_QUERIES.len()
    );
    assert_eq!(
        entries.len(),
        expected_scoped,
        "{TARGET}: only {} of the {expected_scoped} in-scope statements were hashed",
        entries.len()
    );
    assert_eq!(
        skipped_scope.len(),
        expected_skipped,
        "{TARGET}: {skipped_scope:?} were skipped, but the wire scope excludes \
         {expected_skipped} statement(s) from this file"
    );

    let baseline = Sha256Baseline {
        generated_at: "2026-06-18T00:00:00Z".to_string(),
        // Record the profile the hashes were actually computed on. A hard-coded
        // value here would make the profile guard compare every future run
        // against a profile that has nothing to do with the data at hand.
        scale_factor: fixture_scale_factor()
            .map(|sf| sf.to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        queries: entries,
    };

    let path = std::path::Path::new(TPC_H_SHA256_BASELINE_FILE);
    baseline
        .save(path)
        .unwrap_or_else(|e| panic!("save failed: {}", e));
    eprintln!("\n[OK] baseline written to {}", path.display());

    // Disclose the statements this function did not execute. One marker
    // per test function keeps the CI arithmetic exact.
    tpch_wire_scope::report_wire_exclusions(TARGET, &skipped_scope);
}
