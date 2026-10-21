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
///
/// #5410: `partsupp` / `supplier` / `nation` were missing from this list.
/// The Q11 statement joins exactly those three tables, so it could never
/// execute: `run_count` returned `None`, the test took the "skipping"
/// branch, and the assertion at the bottom was **unreachable**. The test
/// passed without checking anything — a false green.
///
/// The three tables are loaded now, and Q11 is compared against an
/// independent in-process SQLite oracle.
///
/// Column counts are the TPC-H ones, matching
/// `tests/integration/sql/four_way_harness.rs::TABLE_COLS`.
const TABLES: [(&str, usize); 5] = [
    ("lineitem", 16),
    ("part", 9),
    ("partsupp", 5),
    ("supplier", 7),
    ("nation", 4),
];

/// The DDL for every table this test loads.
///
/// A failure to create a table is a **hard error**: silently ignoring it
/// (`let _ = engine.execute(ddl)`) is what let the Q11 branch disappear
/// into a "skip" in the first place.
const DDL: [&str; 5] = [
    "CREATE TABLE lineitem (l_orderkey INTEGER, l_partkey INTEGER, l_suppkey INTEGER, l_linenumber INTEGER, l_quantity REAL, l_extendedprice REAL, l_discount REAL, l_tax REAL, l_returnflag TEXT, l_linestatus TEXT, l_shipdate TEXT, l_commitdate TEXT, l_receiptdate TEXT, l_shipinstruct TEXT, l_shipmode TEXT, l_comment TEXT)",
    "CREATE TABLE part (p_partkey INTEGER, p_name TEXT, p_mfgr TEXT, p_brand TEXT, p_type TEXT, p_size INTEGER, p_container TEXT, p_retailprice REAL, p_comment TEXT)",
    "CREATE TABLE partsupp (ps_partkey INTEGER, ps_suppkey INTEGER, ps_availqty INTEGER, ps_supplycost REAL, ps_comment TEXT)",
    "CREATE TABLE supplier (s_suppkey INTEGER, s_name TEXT, s_address TEXT, s_nationkey INTEGER, s_phone TEXT, s_acctbal REAL, s_comment TEXT)",
    "CREATE TABLE nation (n_nationkey INTEGER, n_name TEXT, n_regionkey INTEGER, n_comment TEXT)",
];

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
    for ddl in DDL {
        engine
            .execute(ddl)
            .unwrap_or_else(|e| panic!("creating table for `{TARGET}` failed: {e}"));
    }
    for (table, ncols) in TABLES {
        let path = fixture.table_path(table);
        // #5410: a missing table used to be skipped (`continue`), which left
        // the table uncreated and turned the Q11 statement into an
        // unconditional failure that the test then "skipped" over. A missing
        // table in a present fixture is a **broken fixture**, not a reason to
        // silently run less.
        assert!(
            path.exists(),
            "fixture {} is present but table `{table}` is missing from it \
             (looked at {})",
            fixture.dir.display(),
            path.display()
        );
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => panic!("reading table `{table}` from {}: {e}", path.display()),
        };
        let rows = tpch_fixture::parse_tbl(&content);
        let _ = tpch_fixture::load_rows_into_storage(&storage, &rows, table, ncols)
            .unwrap_or_else(|e| panic!("loading table `{table}`: {e}"));
    }
    Some(Box::new(engine))
}

/// Why a statement did not produce a row count.
///
/// #5410: the distinction matters. The old `run_count` returned `None` for
/// *both* "the statement failed" and "the statement timed out", and the
/// caller then printed "skipping" and returned. A failed statement and a
/// timeout are both **failures** here — the tables the statements need are
/// loaded — so they must be reported as such, not folded into a skip.
#[derive(Debug)]
enum RunFailure {
    /// The statement returned an error.
    Execution(String),
    /// The statement panicked.
    Panic,
    /// The statement did not finish within the timeout.
    Timeout,
}

impl std::fmt::Display for RunFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunFailure::Execution(e) => write!(f, "the statement failed: {e}"),
            RunFailure::Panic => write!(f, "the statement panicked"),
            RunFailure::Timeout => write!(f, "the statement timed out"),
        }
    }
}

/// Run sqlrustgo and return the produced rows.
///
/// The query runs on a worker thread so that a query which does not terminate
/// cannot hang the test binary. The worker is given a **raw pointer** into
/// the engine, so the engine must outlive the worker: on the timeout path the
/// allocation is leaked instead of freed, because the worker may still be
/// reading it. Before issue #5379 this path was unreachable (the fixture was
/// missing, so the engine was never built and `run_count` was never called);
/// making the fixture reachable turned it into a live use-after-free.
fn run_query_rows(
    engine: &mut Box<ExecutionEngine<MemoryStorage>>,
    sql: &str,
    timeout: std::time::Duration,
) -> Result<Vec<Vec<sqlrustgo_types::Value>>, RunFailure> {
    // Use mpsc channel + worker thread for timeout
    use std::sync::mpsc;
    let (tx, rx) = mpsc::channel::<Result<Vec<Vec<sqlrustgo_types::Value>>, RunFailure>>();
    let sql_owned = sql.to_string();
    let engine_addr: usize = &mut **engine as *mut _ as usize;
    let handle = std::thread::spawn(move || unsafe {
        let engine_ptr = engine_addr as *mut ExecutionEngine<MemoryStorage>;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            (*engine_ptr).execute(&sql_owned)
        }));
        let payload = match result {
            Ok(Ok(r)) => Ok(r.rows),
            Ok(Err(e)) => Err(RunFailure::Execution(e.to_string())),
            Err(_) => Err(RunFailure::Panic),
        };
        let _ = tx.send(payload);
    });
    match rx.recv_timeout(timeout) {
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
            Err(RunFailure::Timeout)
        }
    }
}

/// Run sqlrustgo and return the row count.
fn run_query(
    engine: &mut Box<ExecutionEngine<MemoryStorage>>,
    sql: &str,
    timeout: std::time::Duration,
) -> Result<usize, RunFailure> {
    run_query_rows(engine, sql, timeout).map(|rows| rows.len())
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
    let count = run_query(&mut engine, q6, std::time::Duration::from_secs(10))
        .expect("Q6 should complete (not timeout, not error)");
    assert_eq!(count, 1, "Q6 scalar aggregate returns 1 row");
}

#[test]
fn test_q14_aggregate_returns_one_row() {
    // Q14: SUM(CASE WHEN ... END) / SUM(...) - always 1 row
    let Some(mut engine) = make_engine_with_data() else {
        return;
    };
    let q14 = "SELECT 100.00 * SUM(CASE WHEN p_type LIKE 'PROMO%' THEN l_extendedprice * (1 - l_discount) ELSE 0 END) / SUM(l_extendedprice * (1 - l_discount)) AS promo_revenue FROM lineitem, part WHERE l_partkey = p_partkey AND l_shipdate >= '1995-09-01' AND l_shipdate < '1995-10-01'";
    let count = run_query(&mut engine, q14, std::time::Duration::from_secs(10))
        .expect("Q14 should complete (not timeout, not error)");
    assert_eq!(count, 1, "Q14 scalar aggregate returns 1 row");
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
    let n = run_query(&mut engine, q1, std::time::Duration::from_secs(10))
        .expect("Q1 should complete (not timeout, not error)");
    assert_eq!(n, 6, "Q1 GROUP BY returns 6 rows (matches PG)");
}

// =========================================================================
// Q11 — the statement that used to be unreachable (#5410)
// =========================================================================

/// The Q11 statement, verbatim.
///
/// It joins `partsupp`, `supplier` and `nation` — the three tables that
/// were missing from [`TABLES`], which is why the statement could never
/// execute and the assertion below was unreachable.
const Q11: &str = "SELECT ps_partkey, SUM(ps_supplycost * ps_availqty) AS value \
     FROM partsupp, supplier, nation \
     WHERE ps_suppkey = s_suppkey AND s_nationkey = n_nationkey AND n_name = 'GERMANY' \
     GROUP BY ps_partkey \
     HAVING SUM(ps_supplycost * ps_availqty) > (SELECT SUM(ps_supplycost * ps_availqty) * 0.0001000000 \
       FROM partsupp, supplier, nation \
       WHERE ps_suppkey = s_suppkey AND s_nationkey = n_nationkey AND n_name = 'GERMANY') \
     ORDER BY value DESC";

/// Q11 is a three-way join with a correlated `HAVING` subquery — materially
/// heavier than Q1/Q6/Q14. It gets its own budget instead of borrowing the
/// 10 s one, because a timeout here has to be a *failure* (see below) and
/// a too-tight budget would make it a failure for the wrong reason.
const Q11_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);

/// The three tables Q11 needs, with the SQLite column kinds used to load
/// them into the oracle.
const Q11_TABLES: [(&str, usize, &[&str]); 3] = [
    ("partsupp", 5, &["int", "int", "int", "float", "text"]),
    (
        "supplier",
        7,
        &["int", "text", "text", "int", "text", "float", "text"],
    ),
    ("nation", 4, &["int", "text", "int", "text"]),
];

/// The oracle's schema for the three tables.
const Q11_SQLITE_DDL: [&str; 3] = [
    "CREATE TABLE partsupp (ps_partkey INTEGER, ps_suppkey INTEGER, ps_availqty INTEGER, ps_supplycost REAL, ps_comment TEXT)",
    "CREATE TABLE supplier (s_suppkey INTEGER, s_name TEXT, s_address TEXT, s_nationkey INTEGER, s_phone TEXT, s_acctbal REAL, s_comment TEXT)",
    "CREATE TABLE nation (n_nationkey INTEGER, n_name TEXT, n_regionkey INTEGER, n_comment TEXT)",
];

/// Load one fixture table into the SQLite oracle.
fn load_sqlite_table(
    conn: &rusqlite::Connection,
    table: &str,
    kinds: &[&str],
    rows: &[Vec<String>],
) {
    let ncols = kinds.len();
    let placeholders: Vec<String> = (1..=ncols).map(|i| format!("?{i}")).collect();
    let sql = format!("INSERT INTO {table} VALUES ({})", placeholders.join(", "));
    let mut stmt = conn.prepare(&sql).unwrap_or_else(|e| panic!("{sql}: {e}"));
    for row in rows {
        let values: Vec<rusqlite::types::Value> = (0..ncols)
            .map(|i| {
                let raw = row.get(i).map(|s: &String| s.as_str()).unwrap_or("");
                match kinds[i] {
                    "int" => rusqlite::types::Value::Integer(raw.parse().unwrap_or(0)),
                    "float" => rusqlite::types::Value::Real(raw.parse().unwrap_or(0.0)),
                    _ => rusqlite::types::Value::Text(raw.to_string()),
                }
            })
            .collect();
        stmt.execute(rusqlite::params_from_iter(values.iter()))
            .unwrap_or_else(|e| panic!("inserting into {table}: {e}"));
    }
}

/// Run Q11 on an **independent** in-process SQLite oracle.
///
/// SQLite is a separate implementation with no shared code path with
/// sqlrustgo, so its result is independent evidence — not "the expected
/// value computed by the code under test". `rusqlite` is used with the
/// `bundled` feature, so the SQLite C sources are compiled from the
/// repository: no external binary, no network, no PATH lookup.
fn sqlite_q11_rows(fixture: &tpch_fixture::Fixture) -> Vec<(i64, f64)> {
    use rusqlite::Connection;

    let conn = Connection::open_in_memory().expect("open in-memory SQLite");
    for ddl in Q11_SQLITE_DDL {
        conn.execute(ddl, [])
            .unwrap_or_else(|e| panic!("{ddl}: {e}"));
    }
    for (table, ncols, kinds) in Q11_TABLES {
        let content = fixture
            .read_table(table)
            .unwrap_or_else(|e| panic!("reading {table} for the oracle: {e}"));
        let rows = tpch_fixture::parse_tbl(&content);
        load_sqlite_table(&conn, table, kinds, &rows);
        let _ = ncols;
    }

    let mut stmt = conn.prepare(Q11).expect("prepare Q11 for the oracle");
    let mut rows = stmt.query([]).expect("run Q11 on the oracle");
    let mut out = Vec::new();
    while let Some(row) = rows.next().expect("step Q11 on the oracle") {
        let partkey: i64 = row.get(0).expect("read ps_partkey");
        let value: f64 = row.get(1).expect("read value");
        out.push((partkey, value));
    }
    out
}

/// The Q11 result, as `(ps_partkey, value)` pairs.
///
/// The engine returns `Value`s, so the pair is normalised here: the key is
/// an integer, the value is a float.
fn normalize_q11_rows(rows: &[Vec<sqlrustgo_types::Value>]) -> Vec<(i64, f64)> {
    rows.iter()
        .map(|r| {
            let key = match r.first() {
                Some(sqlrustgo_types::Value::Integer(i)) => *i,
                Some(sqlrustgo_types::Value::Float(f)) => *f as i64,
                other => panic!("Q11 ps_partkey must be numeric, got {other:?}"),
            };
            let value = match r.get(1) {
                Some(sqlrustgo_types::Value::Float(f)) => *f,
                Some(sqlrustgo_types::Value::Integer(i)) => *i as f64,
                other => panic!("Q11 value must be numeric, got {other:?}"),
            };
            (key, value)
        })
        .collect()
}

/// Relative tolerance for comparing the engine's aggregates against the
/// oracle's.
///
/// Both sides sum the same products in floating point, but in a different
/// order, so the last bits differ. A relative tolerance of 1e-9 is far
/// tighter than any semantic difference and far looser than the noise.
const Q11_REL_TOLERANCE: f64 = 1e-9;

fn values_match(a: f64, b: f64) -> bool {
    if a == b {
        return true;
    }
    let denom = a.abs().max(b.abs());
    if denom == 0.0 {
        return a == b;
    }
    (a - b).abs() / denom <= Q11_REL_TOLERANCE
}

#[test]
fn test_q11_in_subquery_returns_zero() {
    // #5410: this test used to be a hole in the suite. The statement
    // referenced three tables the file never created, so it always failed;
    // `run_count` returned `None`; the branch printed "skipping" and
    // returned — and the assertion underneath was never reached. The test
    // was green without checking anything.
    //
    // The three tables are loaded now, and the result is compared against an
    // independent in-process SQLite oracle. A statement that fails or times
    // out is a **failure**, not a reason to skip: the data it needs is
    // present.
    let state = FixtureState::resolve();
    let Some(fixture) = state.require(TARGET).cloned() else {
        return;
    };
    let Some(mut engine) = make_engine_with_data() else {
        return;
    };

    let rows = run_query_rows(&mut engine, Q11, Q11_TIMEOUT).unwrap_or_else(|e| {
        panic!("Q11 must execute — partsupp, supplier and nation are all loaded: {e}")
    });
    let engine_rows = normalize_q11_rows(&rows);
    let oracle_rows = sqlite_q11_rows(&fixture);

    // The statement must produce something. An empty result on a fixture
    // that has GERMANY suppliers and their partsupp rows means the join or
    // the HAVING filter is broken.
    assert!(
        !engine_rows.is_empty(),
        "Q11 returned no rows; the fixture has GERMANY suppliers with partsupp rows"
    );
    assert!(
        !oracle_rows.is_empty(),
        "the SQLite oracle returned no rows for Q11 — the fixture or the \
         statement is wrong, so the comparison would be meaningless"
    );

    // Same set of partsupp keys.
    let engine_keys: Vec<i64> = engine_rows.iter().map(|(k, _)| *k).collect();
    let oracle_keys: Vec<i64> = oracle_rows.iter().map(|(k, _)| *k).collect();
    assert_eq!(
        engine_keys, oracle_keys,
        "Q11 must select the same partsupp keys as the independent SQLite oracle"
    );

    // Same aggregates for each key.
    for ((ek, ev), (ok, ov)) in engine_rows.iter().zip(oracle_rows.iter()) {
        assert_eq!(ek, ok, "Q11 rows must be aligned with the oracle rows");
        assert!(
            values_match(*ev, *ov),
            "Q11 aggregate for ps_partkey={ek}: engine {ev} vs SQLite oracle {ov}"
        );
    }
}
