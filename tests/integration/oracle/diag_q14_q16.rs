//! Q14 / Q16 isolation — the ship-date window and the `part` filters.
//!
//! # What changed (issue #5379)
//!
//! This file used to contain **zero assertions**. It ran the first steps of
//! Q14 and Q16 and printed the counts:
//!
//! ```text
//! eprintln!("Q14 step1 shipdate 1995-09: {:?}", r1.rows);
//! eprintln!("Q16 step1 part p_size IN: {:?}", r2.rows);
//! eprintln!("Q16 full (NOT IN subquery): {:?}", r5.rows);
//! ```
//!
//! Nothing was asserted, so the file "passed" whatever the engine returned.
//! It also *panicked* in CI: it read `tests/data/tpch-sf01`, a symlink into
//! `/tmp` that resolves on exactly one machine. Fixture resolution now goes
//! through `tests/common/tpch_fixture.rs`; a missing fixture is reported as an
//! explicit `TPCH-NOT-RUN` marker (counted as `not_run`, never as `passed`).
//!
//! # A stale comment in the original file
//!
//! The original file said:
//!
//! ```text
//! // This will be 0 because canonical SF=0.01 only has 1994 dates
//! ```
//!
//! That is **not** true of this repository's fixture generator: it draws
//! dates uniformly from 1992-01-01 to 1998-12-31, so the 1995-09 window is
//! not empty. The file therefore does not assert "0 rows" — it asserts the
//! properties that hold whatever the data looks like.
//!
//! # What is asserted now
//!
//! * **Independent oracle** — every query is also executed by an in-process
//!   SQLite loaded from the same fixture (`tests/common/sqlite_oracle.rs`).
//!   The expected values come from a *different engine*.
//! * **Data-independent invariants** — the properties that follow from the
//!   data's own consistency: an `IN` list is the disjoint union of its
//!   equality branches, a predicate and its complement partition the table,
//!   a `NOT IN` subquery removes exactly the rows an `IN` subquery keeps,
//!   and a narrower filter can never select more rows.
//!
//! * **Anti-vacuity** — the ship-date window and the `part` filters must
//!   actually select rows, and both branches of the `NOT IN` subquery must be
//!   hit. Without that, the partition and narrowing invariants above would
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
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::Arc;

use sqlite_oracle::SqliteOracle;
use tpch_fixture::FixtureState;

/// Cargo target name — used in the NOT-RUN marker and the known-difference
/// registry.
const TARGET: &str = "diag_q14_q16";

/// Tables this test needs.
const TABLES: [&str; 4] = ["lineitem", "part", "partsupp", "supplier"];

/// The `p_size` values of the Q16 query.
const Q16_SIZES: [i64; 8] = [49, 14, 23, 45, 19, 3, 36, 9];

/// The `p_brand` value the Q16 query excludes.
const Q16_BRAND: &str = "Brand#45";

/// The `p_type` prefix the Q16 query excludes.
const Q16_TYPE_PREFIX: &str = "MEDIUM POLISHED";

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

/// The `p_size IN (...)` list of the Q16 query.
fn size_list() -> String {
    Q16_SIZES
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

/// The `part` filters of the Q16 query, without the `NOT IN` subquery.
fn q16_part_filters() -> String {
    format!(
        "p_brand <> '{Q16_BRAND}' AND p_type NOT LIKE '{Q16_TYPE_PREFIX}%' \
         AND p_size IN ({})",
        size_list()
    )
}

#[test]
fn diag_q14_q16() {
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
    engine
        .execute("CREATE TABLE partsupp (ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, ps_comment TEXT, PRIMARY KEY (ps_partkey, ps_suppkey))")
        .unwrap();
    engine
        .execute("CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)")
        .unwrap();

    for (table, ncols) in [
        ("lineitem", 16),
        ("part", 9),
        ("partsupp", 5),
        ("supplier", 7),
    ] {
        let loaded = tpch_fixture::load_into_storage(&storage, fixture, table, ncols)
            .unwrap_or_else(|e| panic!("loading table `{table}`: {e}"));
        assert!(loaded > 0, "table `{table}` loaded 0 rows");
    }

    // The independent oracle: same fixture, different engine.
    let oracle = SqliteOracle::open(fixture, &TABLES).expect("opening the SQLite oracle");

    // ---- Q14 step 1: the 1995-09 ship-date window -----------------------
    let q14_window = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q14_q16:q14_shipdate_1995_09",
        "SELECT COUNT(*) FROM lineitem WHERE l_shipdate >= '1995-09-01' AND l_shipdate < '1995-10-01'",
    );
    let q14_from = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q14_q16:q14_shipdate_from_1995_09",
        "SELECT COUNT(*) FROM lineitem WHERE l_shipdate >= '1995-09-01'",
    );
    let q14_to = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q14_q16:q14_shipdate_from_1995_10",
        "SELECT COUNT(*) FROM lineitem WHERE l_shipdate >= '1995-10-01'",
    );
    let lineitem_total = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q14_q16:lineitem_total",
        "SELECT COUNT(*) FROM lineitem",
    );

    // ---- Invariant 1: the ship-date window is a slice of the table ------
    assert_eq!(
        q14_window,
        q14_from - q14_to,
        "the [1995-09-01, 1995-10-01) window must be the difference of the \
         two half-open ranges it is built from"
    );
    assert!(
        q14_window <= q14_from && q14_from <= lineitem_total,
        "each narrower ship-date range must select no more rows than the \
         wider one"
    );

    // ---- Q16 step 1: p_size IN -------------------------------------------
    let q16_size = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q14_q16:q16_p_size_in",
        &format!(
            "SELECT COUNT(*) FROM part WHERE p_size IN ({})",
            size_list()
        ),
    );

    // ---- The size histogram, for the disjoint-union invariant -----------
    let q16_size_hist = "SELECT p_size, COUNT(*) FROM part GROUP BY p_size";
    let engine_hist = engine
        .execute(q16_size_hist)
        .expect("executing the p_size histogram");
    if let Some(expected) = oracle_rows(&oracle, "diag_q14_q16:q16_p_size_hist", q16_size_hist) {
        sqlite_oracle::check_against_oracle(
            "diag_q14_q16:q16_p_size_hist",
            &sqlite_oracle::engine_rows_to_strings(&engine_hist.rows),
            &expected,
        );
    }
    let size_sum: i64 = engine_hist
        .rows
        .iter()
        .filter(|r| Q16_SIZES.contains(&r[0].as_integer().unwrap_or(-1)))
        .map(|r| {
            r[1].as_integer()
                .expect("the histogram count is an integer")
        })
        .sum();

    // ---- Invariant 2: `IN` is the disjoint union of its branches ---------
    assert_eq!(
        q16_size,
        size_sum,
        "`p_size IN ({})` must select exactly the rows of the sizes it lists",
        size_list()
    );

    // ---- Q16 step 2: p_brand <> -----------------------------------------
    let q16_brand = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q14_q16:q16_p_brand_ne",
        &format!("SELECT COUNT(*) FROM part WHERE p_brand <> '{Q16_BRAND}'"),
    );
    let q16_brand_eq = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q14_q16:q16_p_brand_eq",
        &format!("SELECT COUNT(*) FROM part WHERE p_brand = '{Q16_BRAND}'"),
    );
    let part_total = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q14_q16:part_total",
        "SELECT COUNT(*) FROM part",
    );

    // ---- Invariant 3: a predicate and its complement partition the table -
    assert_eq!(
        q16_brand + q16_brand_eq,
        part_total,
        "`p_brand <> '{Q16_BRAND}'` and `p_brand = '{Q16_BRAND}'` must \
         partition the part table"
    );

    // ---- Q16 step 3: p_type NOT LIKE ------------------------------------
    let q16_type = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q14_q16:q16_p_type_not_like",
        &format!("SELECT COUNT(*) FROM part WHERE p_type NOT LIKE '{Q16_TYPE_PREFIX}%'"),
    );
    let q16_type_like = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q14_q16:q16_p_type_like",
        &format!("SELECT COUNT(*) FROM part WHERE p_type LIKE '{Q16_TYPE_PREFIX}%'"),
    );

    // ---- Invariant 4: the LIKE complement partitions the table -----------
    assert_eq!(
        q16_type + q16_type_like,
        part_total,
        "`p_type NOT LIKE '{Q16_TYPE_PREFIX}%'` and \
         `p_type LIKE '{Q16_TYPE_PREFIX}%'` must partition the part table"
    );

    // ---- Q16: the joined rows without the subquery filter ----------------
    let q16_joined = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q14_q16:q16_joined",
        "SELECT COUNT(*) FROM partsupp, part WHERE p_partkey = ps_partkey",
    );
    let q16_filtered = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q14_q16:q16_filtered_no_subquery",
        &format!(
            "SELECT COUNT(*) FROM partsupp, part WHERE p_partkey = ps_partkey AND {}",
            q16_part_filters()
        ),
    );
    // The number of *distinct* parts the filtered join touches. The join
    // itself may multiply rows (a part has several `partsupp` rows), so the
    // bound below is on the parts, not on the joined rows.
    let q16_filtered_parts = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q14_q16:q16_filtered_distinct_parts",
        &format!(
            "SELECT COUNT(DISTINCT p_partkey) FROM partsupp, part \
             WHERE p_partkey = ps_partkey AND {}",
            q16_part_filters()
        ),
    );

    // ---- Q16 full: with the `NOT IN` subquery ----------------------------
    let q16_full = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q14_q16:q16_full",
        &format!(
            "SELECT COUNT(*) FROM partsupp, part WHERE p_partkey = ps_partkey AND {} \
             AND ps_suppkey NOT IN (SELECT s_suppkey FROM supplier WHERE s_comment LIKE '%bad%deals%')",
            q16_part_filters()
        ),
    );

    // ---- Anti-vacuity: every filter must actually select rows -------------
    // `p_brand = 'Brand#45'` is the **complement** of a filter and is
    // legitimately allowed to be empty (this fixture has no such brand) —
    // it is excluded from this check on purpose.
    assert!(
        q14_window > 0,
        "the 1995-09 ship-date window must contain lineitem rows, otherwise \
         the ship-date invariants above would be vacuously true"
    );
    assert!(
        q16_size > 0,
        "`p_size IN (...)` must select parts, otherwise the `IN` invariant \
         above would be vacuously true"
    );
    assert!(
        q16_brand > 0 && q16_type > 0,
        "the `p_brand` and `p_type` filters must select parts, otherwise the \
         partition invariants above would be vacuously true"
    );
    assert!(
        q16_filtered > 0 && q16_full > 0,
        "the Q16 join with its filters must select rows, otherwise the \
         narrowing invariants below would be vacuously true \
         (filtered={q16_filtered}, full={q16_full})"
    );

    // ---- Invariant 5: the filters can only narrow ------------------------
    assert!(
        q16_filtered <= q16_joined,
        "the `part` filters can only narrow the joined rows \
         ({q16_joined} -> {q16_filtered})"
    );
    assert!(
        q16_filtered_parts <= q16_size,
        "the parts the filtered join touches cannot outnumber the parts the \
         `p_size IN` filter alone selects ({q16_size} -> {q16_filtered_parts})"
    );
    assert!(
        q16_full <= q16_filtered,
        "the `NOT IN` subquery can only remove rows \
         ({q16_filtered} -> {q16_full})"
    );

    // ---- The `NOT IN` subquery, spelled the other way round --------------
    // `s_suppkey <= 10` matches real suppliers, so this pair of queries
    // gives the `NOT IN` subquery real coverage.
    let q16_in = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q14_q16:q16_subquery_in",
        &format!(
            "SELECT COUNT(*) FROM partsupp, part WHERE p_partkey = ps_partkey AND {} \
             AND ps_suppkey IN (SELECT s_suppkey FROM supplier WHERE s_suppkey <= 10)",
            q16_part_filters()
        ),
    );
    let q16_not_in = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q14_q16:q16_subquery_not_in",
        &format!(
            "SELECT COUNT(*) FROM partsupp, part WHERE p_partkey = ps_partkey AND {} \
             AND ps_suppkey NOT IN (SELECT s_suppkey FROM supplier WHERE s_suppkey <= 10)",
            q16_part_filters()
        ),
    );

    // ---- Anti-vacuity: both branches of the `NOT IN` subquery must be hit -
    // The `NOT IN` subquery is only meaningful if it actually removes rows
    // (and if its complement is not empty either).
    assert!(
        q16_in > 0 && q16_not_in > 0,
        "both branches of the `NOT IN` subquery must select rows, otherwise \
         the partition invariant below would be vacuously true \
         (in={q16_in}, not_in={q16_not_in})"
    );

    // ---- Invariant 6: `IN` and `NOT IN` partition the joined rows --------
    assert_eq!(
        q16_in + q16_not_in,
        q16_filtered,
        "the `IN` and the `NOT IN` variant of the subquery filter must \
         partition the filtered joined rows"
    );
}
