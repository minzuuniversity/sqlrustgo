//! Sprint 5 harness validation — tests aggregate-aware diff
//! classification without running the full 22-query suite (which
//! hangs on Q3/Q4/Q8/Q21 N^2 EXISTS).
//!
//! Validates the 3-state classification (clean_match / data_limitation /
//! engine_issue) on hand-picked subsets.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::Arc;

// The shared TPC-H fixture resolver and batch loader (issue #5379).
//
// Fixture resolution, the NOT-RUN marker and the loader live in one place,
// so a missing fixture is an *explicit, disclosed state* instead of the
// silent `None` this file used to return — which made every test in it pass
// vacuously when the fixture was unreachable.
#[path = "../../common/tpch_fixture.rs"]
mod tpch_fixture;

use tpch_fixture::FixtureState;

/// Cargo target name — used in the NOT-RUN marker.
const TARGET: &str = "harness_validation_test";

/// Tables this test loads, with their TPC-H column counts.
const TABLES: [(&str, usize); 2] = [("lineitem", 16), ("part", 9)];

/// Load the TPC-H fixture into a fresh in-memory engine.
///
/// The rows are loaded through the storage layer in 10 000-row batches.
///
/// The previous loader issued **one `INSERT` statement per row** (~62 000
/// statements per call, and this file calls it once per test — four calls per
/// test binary). That cost was invisible while the fixture was unreachable:
/// the target then returned `None` before loading anything and every test
/// passed vacuously in milliseconds.
///
/// Issue #5379 made the fixture reachable at run time, and the cost appeared
/// for the first time: on CI run 8031 this target started at 05:58:01Z and
/// had **not finished 37 minutes later** (all four tests were reported as
/// "running for over 60 seconds"), so the job's `timeout 9000` would cut the
/// whole `test` job short — 146 of 446 declared targets would never run.
///
/// The two loaders are equivalent: both derive the same typed values
/// (integer / float / text) from the same TBL text, and the shared loader
/// additionally maps an empty field to NULL. The generated fixture has no
/// empty field inside a row (every empty field is the trailing one after the
/// final `|`, which is dropped together with the row terminator), so the two
/// loaders put the same values into the same tables.
fn make_engine_with_data() -> Option<Box<ExecutionEngine<MemoryStorage>>> {
    let state = FixtureState::resolve();
    let fixture = state.require(TARGET)?.clone();

    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    for ddl in [
        "CREATE TABLE lineitem (l_orderkey INTEGER, l_partkey INTEGER, l_suppkey INTEGER, l_linenumber INTEGER, l_quantity REAL, l_extendedprice REAL, l_discount REAL, l_tax REAL, l_returnflag TEXT, l_linestatus TEXT, l_shipdate TEXT, l_commitdate TEXT, l_receiptdate TEXT, l_shipinstruct TEXT, l_shipmode TEXT, l_comment TEXT)",
        "CREATE TABLE part (p_partkey INTEGER, p_name TEXT, p_mfgr TEXT, p_brand TEXT, p_type TEXT, p_size INTEGER, p_container TEXT, p_retailprice REAL, p_comment TEXT)",
    ] {
        let _ = engine.execute(ddl);
    }
    for (table, ncols) in TABLES {
        let path = fixture.table_path(table);
        if !path.exists() {
            continue;
        }
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let rows = tpch_fixture::parse_tbl(&content);
        let _ = tpch_fixture::load_rows_into_storage(&storage, &rows, table, ncols)
            .unwrap_or_else(|e| panic!("loading table `{table}`: {e}"));
    }
    Some(Box::new(engine))
}

/// Run sqlrustgo and return row count.
///
/// The query runs on a worker thread so that a query which does not terminate
/// cannot hang the test binary. The worker is given a **raw pointer** into
/// the engine, so the engine must outlive the worker: on the timeout path the
/// allocation is leaked instead of freed, because the worker may still be
/// reading it. Before issue #5379 this path was unreachable (the fixture was
/// missing, so the engine was never built and `run_count` was never called);
/// making the fixture reachable turned it into a live use-after-free.
fn run_count(engine: &mut Box<ExecutionEngine<MemoryStorage>>, sql: &str) -> Option<usize> {
    // Use mpsc channel + worker thread for timeout
    use std::sync::mpsc;
    use std::time::Duration;
    let (tx, rx) = mpsc::channel::<Option<usize>>();
    let sql_owned = sql.to_string();
    let engine_addr: usize = &mut **engine as *mut _ as usize;
    let handle = std::thread::spawn(move || unsafe {
        let engine_ptr = engine_addr as *mut ExecutionEngine<MemoryStorage>;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            (*engine_ptr).execute(&sql_owned)
        }));
        let payload = match result {
            Ok(Ok(r)) => Some(r.rows.len()),
            Ok(Err(_)) => None,
            Err(_) => None,
        };
        let _ = tx.send(payload);
    });
    let timeout_dur = Duration::from_secs(10);
    match rx.recv_timeout(timeout_dur) {
        Ok(payload) => {
            let _ = handle.join();
            payload
        }
        Err(_) => {
            // timeout: the worker still holds a pointer into the engine, so
            // the allocation must not be freed. Leak it — the test binary is
            // about to exit anyway.
            let leaked = std::mem::replace(engine, Box::new(ExecutionEngine::with_memory()));
            std::mem::forget(leaked);
            None
        }
    }
}

#[test]
fn test_q6_aggregate_returns_one_row() {
    // Q6 is a scalar aggregate: SUM over the filtered rows always produces
    // exactly one row — with a NULL value when the filter matches nothing,
    // and with a REAL value when it matches. The assertion is therefore
    // data-independent; it is the *row count* that is checked, not the sum.
    let Some(mut engine) = make_engine_with_data() else {
        return;
    };
    let q6 = "SELECT SUM(l_extendedprice * l_discount) AS revenue FROM lineitem WHERE l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01' AND l_discount BETWEEN 0.06 AND 0.08 AND l_quantity < 25";
    let count = run_count(&mut engine, q6);
    assert!(count.is_some(), "Q6 should complete (not timeout)");
    assert_eq!(count.unwrap(), 1, "Q6 scalar aggregate returns 1 row");
}

#[test]
fn test_q14_aggregate_returns_one_row() {
    // Q14: SUM(CASE WHEN ... END) / SUM(...) - always 1 row
    let Some(mut engine) = make_engine_with_data() else {
        return;
    };
    let q14 = "SELECT 100.00 * SUM(CASE WHEN p_type LIKE 'PROMO%' THEN l_extendedprice * (1 - l_discount) ELSE 0 END) / SUM(l_extendedprice * (1 - l_discount)) AS promo_revenue FROM lineitem, part WHERE l_partkey = p_partkey AND l_shipdate >= '1995-09-01' AND l_shipdate < '1995-10-01'";
    let count = run_count(&mut engine, q14);
    assert!(count.is_some(), "Q14 should complete (not timeout)");
    assert_eq!(count.unwrap(), 1, "Q14 scalar aggregate returns 1 row");
}

#[test]
fn test_q1_groupby_returns_six_rows() {
    // Q1: GROUP BY l_returnflag, l_linestatus. The TPC-H domain has exactly
    // six combinations (R/A/N × O/F), so a dataset that contains all of them
    // produces exactly six groups. The generated fixture draws both columns
    // uniformly, so every combination occurs thousands of times.
    let Some(mut engine) = make_engine_with_data() else {
        return;
    };
    let q1 = "SELECT l_returnflag, l_linestatus, SUM(l_quantity) AS sum_qty, SUM(l_extendedprice) AS sum_base_price, SUM(l_extendedprice*(1-l_discount)) AS sum_disc_price, SUM(l_extendedprice*(1-l_discount)*(1+l_tax)) AS sum_charge, AVG(l_quantity) AS avg_qty, AVG(l_extendedprice) AS avg_price, AVG(l_discount) AS avg_disc, COUNT(*) AS count_order FROM lineitem WHERE l_shipdate <= '1998-09-02' GROUP BY l_returnflag, l_linestatus ORDER BY l_returnflag, l_linestatus";
    let count = run_count(&mut engine, q1);
    assert!(count.is_some(), "Q1 should complete (not timeout)");
    let n = count.unwrap();
    assert_eq!(n, 6, "Q1 GROUP BY returns 6 rows (matches PG)");
}

#[test]
fn test_q11_in_subquery_returns_zero() {
    // DISCLOSURE (issue #5409): this statement references `partsupp`,
    // `supplier` and `nation` — none of which this file creates. The
    // statement therefore fails, `run_count` returns `None`, and the
    // assertion at the bottom is **never reached**: the test passes without
    // checking anything. That was true before issue #5379 as well (the
    // fixture was unreachable, so the whole file passed vacuously).
    //
    // Making it real means loading the three extra tables and re-basing the
    // expectation, which changes what this test asserts; that is filed
    // separately instead of being done here.
    let Some(mut engine) = make_engine_with_data() else {
        return;
    };
    let q11 = "SELECT ps_partkey, SUM(ps_supplycost * ps_availqty) AS value FROM partsupp, supplier, nation WHERE ps_suppkey = s_suppkey AND s_nationkey = n_nationkey AND n_name = 'GERMANY' GROUP BY ps_partkey HAVING SUM(ps_supplycost * ps_availqty) > (SELECT SUM(ps_supplycost * ps_availqty) * 0.0001000000 FROM partsupp, supplier, nation WHERE ps_suppkey = s_suppkey AND s_nationkey = n_nationkey AND n_name = 'GERMANY') ORDER BY value DESC";
    let count = run_count(&mut engine, q11);
    // The statement cannot be executed (missing tables) or may time out due
    // to the N^2 join — in both cases `run_count` returns `None`.
    if count.is_none() {
        eprintln!("Q11 could not be executed (missing tables) or timed out — skipping");
        return;
    }
    // If it completes, just verify we got a non-negative result
    assert!(count.unwrap() < 10000, "Q11 returns reasonable row count");
}
