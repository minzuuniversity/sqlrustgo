//! Q14 isolated — just the `SUM` and the division.
//!
//! # What changed (issue #5379)
//!
//! This file used to contain **zero assertions**. It ran the Q14 expression
//! for two ship-date windows and printed the result (or the error message):
//!
//! ```text
//! eprintln!("Q14: rows = {}, first row = {:?}", result.rows.len(), result.rows.first());
//! eprintln!("Q14 1994-09 ERR: {}", e);
//! ```
//!
//! Nothing was asserted, so the file "passed" whatever the engine returned —
//! it even swallowed a query error into a printed message. It also
//! *panicked* in CI: it read `tests/data/tpch-sf01`, a symlink into `/tmp`
//! that resolves on exactly one machine. Fixture resolution now goes through
//! `tests/common/tpch_fixture.rs`; a missing fixture is reported as an
//! explicit `TPCH-NOT-RUN` marker (counted as `not_run`, never as `passed`).
//!
//! # What is asserted now
//!
//! * **Independent oracle** — every query is also executed by an in-process
//!   SQLite loaded from the same fixture (`tests/common/sqlite_oracle.rs`).
//!   The expected values come from a *different engine*.
//! * **Data-independent invariants** — the whole point of this file is the
//!   *decomposition* of the Q14 expression:
//!   - the promotional sum written as a `CASE` inside `SUM` and the same sum
//!     written as a `WHERE` filter must be equal;
//!   - the ratio the engine returns must be exactly
//!     `100 * promotional_sum / total_sum`;
//!   - the promotional revenue is a *share* of the total revenue, so it must
//!     lie in `[0, 100]`.
//!
//! * **Anti-vacuity** — the ship-date window must actually contain joined and
//!   promotional rows. Without that, the ratio invariants above would
//!   degenerate into `0 == 0` and would pass even for a completely broken
//!   engine.
//!
//! All queries in this file are plain SQL that SQLite runs as written
//! (`adapt_sql` reports neither adaptations nor unsupported constructs for
//! them), so **every** query is checked against the oracle.

#[path = "../../common/sqlite_oracle.rs"]
mod sqlite_oracle;
#[path = "../../common/tpch_fixture.rs"]
mod tpch_fixture;

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

use sqlite_oracle::SqliteOracle;
use tpch_fixture::FixtureState;

/// Cargo target name — used in the NOT-RUN marker and the known-difference
/// registry.
const TARGET: &str = "diag_q14_only";

/// Tables this test needs.
const TABLES: [&str; 2] = ["lineitem", "part"];

/// The two ship-date windows the original file used.
const WINDOWS: [(&str, &str, &str); 2] = [
    ("1995-09", "1995-09-01", "1995-10-01"),
    ("1994-09", "1994-09-01", "1994-10-01"),
];

/// Relative tolerance used when comparing the decomposed ratio with the
/// ratio the engine returns.
const RATIO_TOLERANCE: f64 = 1e-9;

/// Run `sql` on the independent SQLite oracle, after dialect adaptation.
///
/// Returns `None` when SQLite cannot run the query at all. That is never
/// silent: the reason is printed so the gap shows up in the CI log, and the
/// caller falls back to data-independent invariants for that query.
fn oracle_rows(oracle: &SqliteOracle, case_id: &str, sql: &str) -> Option<Vec<Vec<String>>> {
    let adapted = sqlite_oracle::adapt_sql(sql);
    if !adapted.unsupported.is_empty() {
        eprintln!(
            "TPCH-NO-ORACLE: {case_id} unsupported={:?} — this query is checked by \
             data-independent invariants only",
            adapted.unsupported
        );
        return None;
    }
    for a in &adapted.adaptations {
        eprintln!("TPCH-ORACLE-ADAPT: {case_id} {a}");
    }
    Some(
        oracle
            .query(&adapted.sql)
            .unwrap_or_else(|e| panic!("{case_id}: SQLite oracle query failed: {e}")),
    )
}

/// Read a cell as a number, whatever numeric type the engine produced.
fn as_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Integer(i) => Some(*i as f64),
        Value::Float(f) => Some(*f),
        _ => None,
    }
}

/// Whether two numbers agree within a relative tolerance.
fn close(a: f64, b: f64) -> bool {
    if a == b {
        return true;
    }
    let denom = a.abs().max(b.abs());
    if denom == 0.0 {
        return a == b;
    }
    (a - b).abs() / denom <= RATIO_TOLERANCE
}

/// The full Q14 query for a ship-date window.
fn q14(from: &str, to: &str) -> String {
    format!(
        "SELECT 100.00 * SUM(CASE WHEN p_type LIKE 'PROMO%' \
                   THEN l_extendedprice * (1 - l_discount) ELSE 0 END) \
                / SUM(l_extendedprice * (1 - l_discount)) AS promo_revenue \
         FROM lineitem, part WHERE l_partkey = p_partkey \
           AND l_shipdate >= '{from}' AND l_shipdate < '{to}'"
    )
}

/// The promotional sum, written as a `CASE` inside the aggregate.
fn promo_sum_case(from: &str, to: &str) -> String {
    format!(
        "SELECT SUM(CASE WHEN p_type LIKE 'PROMO%' \
                   THEN l_extendedprice * (1 - l_discount) ELSE 0 END) \
         FROM lineitem, part WHERE l_partkey = p_partkey \
           AND l_shipdate >= '{from}' AND l_shipdate < '{to}'"
    )
}

/// The same promotional sum, written as a `WHERE` filter.
fn promo_sum_where(from: &str, to: &str) -> String {
    format!(
        "SELECT SUM(l_extendedprice * (1 - l_discount)) \
         FROM lineitem, part WHERE l_partkey = p_partkey \
           AND l_shipdate >= '{from}' AND l_shipdate < '{to}' \
           AND p_type LIKE 'PROMO%'"
    )
}

/// The denominator of the ratio, on its own.
fn total_sum(from: &str, to: &str) -> String {
    format!(
        "SELECT SUM(l_extendedprice * (1 - l_discount)) \
         FROM lineitem, part WHERE l_partkey = p_partkey \
           AND l_shipdate >= '{from}' AND l_shipdate < '{to}'"
    )
}

/// Execute `sql` on the engine, compare it with the oracle, and return the
/// engine's single cell as a number.
fn scalar_with_oracle(
    engine: &mut ExecutionEngine<MemoryStorage>,
    oracle: &SqliteOracle,
    case_id: &str,
    sql: &str,
) -> f64 {
    let result = engine
        .execute(sql)
        .unwrap_or_else(|e| panic!("{case_id}: {e}"));
    if let Some(expected) = oracle_rows(oracle, case_id, sql) {
        sqlite_oracle::check_against_oracle(
            case_id,
            &sqlite_oracle::engine_rows_to_strings(&result.rows),
            &expected,
        );
    }
    assert_eq!(
        result.rows.len(),
        1,
        "{case_id}: an aggregate without a GROUP BY must produce exactly one row"
    );
    as_f64(&result.rows[0][0]).unwrap_or_else(|| {
        panic!(
            "{case_id}: the aggregate did not return a number: {:?}",
            result.rows[0][0]
        )
    })
}

/// Execute `sql` on the engine, compare it with the oracle, and return the
/// engine's first cell as an integer.
fn count_with_oracle(
    engine: &mut ExecutionEngine<MemoryStorage>,
    oracle: &SqliteOracle,
    case_id: &str,
    sql: &str,
) -> i64 {
    let result = engine
        .execute(sql)
        .unwrap_or_else(|e| panic!("{case_id}: {e}"));
    if let Some(expected) = oracle_rows(oracle, case_id, sql) {
        sqlite_oracle::check_against_oracle(
            case_id,
            &sqlite_oracle::engine_rows_to_strings(&result.rows),
            &expected,
        );
    }
    result.rows[0][0]
        .as_integer()
        .unwrap_or_else(|| panic!("{case_id}: COUNT(*) did not return an integer"))
}

#[test]
fn diag_q14_only() {
    let state = FixtureState::resolve();
    let Some(fixture) = state.require(TARGET) else {
        // No fixture: the NOT-RUN marker was printed by `require`. This test
        // is reported as `not_run` by CI, never as `passed`.
        return;
    };

    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    engine
        .execute("CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity REAL NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT, l_linestatus TEXT, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT, l_shipmode TEXT, l_comment TEXT, PRIMARY KEY (l_orderkey, l_linenumber))")
        .unwrap();
    engine
        .execute("CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT)")
        .unwrap();

    for (table, ncols) in [("lineitem", 16), ("part", 9)] {
        let loaded = tpch_fixture::load_into_storage(&storage, fixture, table, ncols)
            .unwrap_or_else(|e| panic!("loading table `{table}`: {e}"));
        assert!(loaded > 0, "table `{table}` loaded 0 rows");
    }

    // The independent oracle: same fixture, different engine.
    let oracle = SqliteOracle::open(fixture, &TABLES).expect("opening the SQLite oracle");

    for (label, from, to) in WINDOWS {
        // ---- The full Q14 ------------------------------------------------
        let ratio = scalar_with_oracle(
            &mut engine,
            &oracle,
            &format!("diag_q14_only:q14_{label}"),
            &q14(from, to),
        );

        // ---- The two spellings of the promotional sum -------------------
        let promo_case = scalar_with_oracle(
            &mut engine,
            &oracle,
            &format!("diag_q14_only:promo_sum_case_{label}"),
            &promo_sum_case(from, to),
        );
        let promo_where = scalar_with_oracle(
            &mut engine,
            &oracle,
            &format!("diag_q14_only:promo_sum_where_{label}"),
            &promo_sum_where(from, to),
        );

        // ---- The denominator ---------------------------------------------
        let total = scalar_with_oracle(
            &mut engine,
            &oracle,
            &format!("diag_q14_only:total_sum_{label}"),
            &total_sum(from, to),
        );

        // ---- The row counts behind the sums ------------------------------
        let joined_rows = count_with_oracle(
            &mut engine,
            &oracle,
            &format!("diag_q14_only:joined_rows_{label}"),
            &format!(
                "SELECT COUNT(*) FROM lineitem, part WHERE l_partkey = p_partkey \
                   AND l_shipdate >= '{from}' AND l_shipdate < '{to}'"
            ),
        );
        let promo_rows = count_with_oracle(
            &mut engine,
            &oracle,
            &format!("diag_q14_only:promo_rows_{label}"),
            &format!(
                "SELECT COUNT(*) FROM lineitem, part WHERE l_partkey = p_partkey \
                   AND l_shipdate >= '{from}' AND l_shipdate < '{to}' \
                   AND p_type LIKE 'PROMO%'"
            ),
        );

        // ---- Anti-vacuity: the window and the promotional rows must exist -
        // Without this, every ratio invariant below would degenerate into
        // `0 == 0` and would pass even for a completely broken engine.
        assert!(
            joined_rows > 0,
            "the {label} window must contain joined rows, otherwise the \
             promotional revenue is undefined"
        );
        assert!(
            promo_rows > 0,
            "the {label} window must contain promotional rows, otherwise the \
             promotional revenue would trivially be 0"
        );

        // ---- Invariant 1: the two spellings of the promotional sum agree --
        assert!(
            close(promo_case, promo_where),
            "the promotional sum written as `SUM(CASE WHEN ... END)` ({promo_case}) \
             and the same sum written as a `WHERE` filter ({promo_where}) must be equal"
        );

        // ---- Invariant 2: the window is not empty ------------------------
        assert!(
            joined_rows > 0,
            "the {label} window must contain joined rows, otherwise the \
             promotional revenue is undefined"
        );
        assert!(
            promo_rows <= joined_rows,
            "the promotional rows of the {label} window ({promo_rows}) cannot \
             outnumber the joined rows ({joined_rows})"
        );

        // ---- Invariant 3: the denominator is positive ---------------------
        assert!(
            total > 0.0,
            "the denominator of the {label} ratio must be positive, got {total}"
        );

        // ---- Invariant 4: the promotional sum is a subset -----------------
        assert!(
            promo_case <= total,
            "the promotional sum ({promo_case}) is a subset of the total sum \
             ({total}) and can never be larger"
        );
        assert!(
            promo_case >= 0.0,
            "the promotional sum of the {label} window must not be negative, got {promo_case}"
        );

        // ---- Invariant 5: the ratio is a share, so it lies in [0, 100] ----
        assert!(
            (0.0..=100.0).contains(&ratio),
            "the promotional revenue of the {label} window is a share of the \
             total revenue and must lie in [0, 100], got {ratio}"
        );

        // ---- Invariant 6: the ratio is exactly promo / total --------------
        let expected_ratio = 100.0 * promo_case / total;
        assert!(
            close(ratio, expected_ratio),
            "the promotional revenue of the {label} window must be \
             100 * promo_sum / total_sum = {expected_ratio}, but the engine \
             returned {ratio}"
        );
    }
}
