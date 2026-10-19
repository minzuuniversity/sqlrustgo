//! Q6 exact reproduction — run the Q6 `WHERE` clause predicate by predicate.
//!
//! # What changed (issue #5379)
//!
//! This file used to contain **zero assertions**. It probed the `WHERE`
//! clause of Q6 to find out whether the problem was in the clause parsing,
//! the predicate evaluation, the `BETWEEN` operator or the comparison logic
//! for dates — and printed every answer:
//!
//! ```text
//! eprintln!("shipdate = '1994-01-01' count = {}", r1.rows[0][0]);
//! eprintln!("discount BETWEEN 0.06 AND 0.08 count = {}", r4.rows[0][0]);
//! eprintln!("shipdate range: {:?}", r6.rows);
//! ```
//!
//! Nothing was asserted, so the file "passed" whatever the engine returned.
//! It also *panicked* in CI: it read `tests/data/tpch-sf01`, a symlink into
//! `/tmp` that resolves on exactly one machine. Fixture resolution now goes
//! through `tests/common/tpch_fixture.rs`; a missing fixture is reported as an
//! explicit `TPCH-NOT-RUN` marker (counted as `not_run`, never as `passed`).
//!
//! # What is asserted now
//!
//! * **Independent oracle** — every probe is also executed by an in-process
//!   SQLite loaded from the same fixture (`tests/common/sqlite_oracle.rs`).
//!   The expected values come from a *different engine*.
//! * **Data-independent invariants** — the properties the comparison logic
//!   must have whatever the data looks like: a comparison and its two
//!   strict/non-strict branches partition the table, `BETWEEN` is
//!   equivalent to the corresponding pair of comparisons, and the stored
//!   ship dates are well-formed ISO `YYYY-MM-DD` values that lie between the
//!   table's `MIN` and `MAX`.
//!
//! * **Anti-vacuity** — the probe date must actually occur in the data and
//!   the filters must actually reach rows. Without that, the partition
//!   invariants above would degenerate into `0 == 0` and would pass even for
//!   a completely broken engine.
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
const TARGET: &str = "diag_q6_where_parsed";

/// Tables this test needs.
const TABLES: [&str; 1] = ["lineitem"];

/// The `WHERE` predicate of the Q6 query.
const Q6_FILTER: &str = "l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01' \
                           AND l_discount BETWEEN 0.06 AND 0.08 \
                           AND l_quantity < 25";

/// The date the probes compare against.
const PROBE_DATE: &str = "1994-01-01";

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

/// Whether `s` is a well-formed ISO `YYYY-MM-DD` calendar date.
fn is_iso_date(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    if !b[..4].iter().all(|c| c.is_ascii_digit())
        || !b[5..7].iter().all(|c| c.is_ascii_digit())
        || !b[8..].iter().all(|c| c.is_ascii_digit())
    {
        return false;
    }
    let year: u32 = s[..4].parse().expect("the year is numeric");
    let month: u32 = s[5..7].parse().expect("the month is numeric");
    let day: u32 = s[8..].parse().expect("the day is numeric");
    year >= 1 && (1..=12).contains(&month) && (1..=31).contains(&day)
}

/// The text of one cell.
fn cell_text(v: &Value) -> String {
    match v {
        Value::Text(s) => s.clone(),
        v => v.to_sql_string(),
    }
}

#[test]
fn diag_q6_where_parsed() {
    let state = FixtureState::resolve();
    let Some(fixture) = state.require(TARGET) else {
        // No fixture: the NOT-RUN marker was printed by `require`. This test
        // is reported as `not_run` by CI, never as `passed`.
        return;
    };

    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    engine
        .execute(
            "CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, \
             l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity REAL NOT NULL, \
             l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, \
             l_returnflag TEXT, l_linestatus TEXT, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, \
             l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT, l_shipmode TEXT, l_comment TEXT, \
             PRIMARY KEY (l_orderkey, l_linenumber))",
        )
        .unwrap();

    let loaded = tpch_fixture::load_into_storage(&storage, fixture, "lineitem", 16)
        .unwrap_or_else(|e| panic!("loading table `lineitem`: {e}"));
    assert!(loaded > 0, "table `lineitem` loaded 0 rows");

    // The independent oracle: same fixture, different engine.
    let oracle = SqliteOracle::open(fixture, &TABLES).expect("opening the SQLite oracle");

    // ---- The total number of rows ---------------------------------------
    let total = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q6_where_parsed:total",
        "SELECT COUNT(*) FROM lineitem",
    );
    assert_eq!(
        total, loaded as i64,
        "the engine must see exactly the rows that were loaded into the storage"
    );

    // ---- Test 1: a simple WHERE with a date string ------------------------
    let q_eq = format!("SELECT COUNT(*) FROM lineitem WHERE l_shipdate = '{PROBE_DATE}'");
    let eq_count = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q6_where_parsed:shipdate_eq",
        &q_eq,
    );

    let q_ge = format!("SELECT COUNT(*) FROM lineitem WHERE l_shipdate >= '{PROBE_DATE}'");
    let ge_count = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q6_where_parsed:shipdate_ge",
        &q_ge,
    );

    let q_lt_1995 = "SELECT COUNT(*) FROM lineitem WHERE l_shipdate < '1995-01-01'";
    let lt_1995 = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q6_where_parsed:shipdate_lt_1995",
        q_lt_1995,
    );

    // ---- The two strict branches of the `>=` comparison -------------------
    let q_gt = format!("SELECT COUNT(*) FROM lineitem WHERE l_shipdate > '{PROBE_DATE}'");
    let gt_count = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q6_where_parsed:shipdate_gt",
        &q_gt,
    );
    let q_lt = format!("SELECT COUNT(*) FROM lineitem WHERE l_shipdate < '{PROBE_DATE}'");
    let lt_count = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q6_where_parsed:shipdate_lt",
        &q_lt,
    );

    // ---- The complement of the 1995 upper bound ---------------------------
    let q_ge_1995 = "SELECT COUNT(*) FROM lineitem WHERE l_shipdate >= '1995-01-01'";
    let ge_1995 = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q6_where_parsed:shipdate_ge_1995",
        q_ge_1995,
    );

    // ---- Test 2: BETWEEN -------------------------------------------------
    let q_between = "SELECT COUNT(*) FROM lineitem WHERE l_discount BETWEEN 0.06 AND 0.08";
    let between = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q6_where_parsed:discount_between",
        q_between,
    );
    let q_comparisons =
        "SELECT COUNT(*) FROM lineitem WHERE l_discount >= 0.06 AND l_discount <= 0.08";
    let comparisons = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q6_where_parsed:discount_comparisons",
        q_comparisons,
    );

    // ---- The two branches of the `l_quantity < 25` comparison --------------
    let q_qty_lt = "SELECT COUNT(*) FROM lineitem WHERE l_quantity < 25";
    let qty_lt = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q6_where_parsed:quantity_lt",
        q_qty_lt,
    );
    let q_qty_ge = "SELECT COUNT(*) FROM lineitem WHERE l_quantity >= 25";
    let qty_ge = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q6_where_parsed:quantity_ge",
        q_qty_ge,
    );

    // ---- Test 3: the ship-date values at the head of the table -----------
    // The `ORDER BY` makes the sample deterministic; the `.tbl` file is in
    // `(l_orderkey, l_linenumber)` order, so this is the first three rows.
    let q_sample = "SELECT l_shipdate FROM lineitem ORDER BY l_orderkey, l_linenumber LIMIT 3";
    let engine_sample = engine
        .execute(q_sample)
        .expect("executing the ship-date sample");
    if let Some(expected) = oracle_rows(&oracle, "diag_q6_where_parsed:shipdate_sample", q_sample) {
        sqlite_oracle::check_against_oracle(
            "diag_q6_where_parsed:shipdate_sample",
            &sqlite_oracle::engine_rows_to_strings(&engine_sample.rows),
            &expected,
        );
    }

    // ---- Test 4: the actual range ----------------------------------------
    let q_range = "SELECT MIN(l_shipdate), MAX(l_shipdate) FROM lineitem";
    let engine_range = engine
        .execute(q_range)
        .expect("executing the ship-date range");
    if let Some(expected) = oracle_rows(&oracle, "diag_q6_where_parsed:shipdate_range", q_range) {
        sqlite_oracle::check_against_oracle(
            "diag_q6_where_parsed:shipdate_range",
            &sqlite_oracle::engine_rows_to_strings(&engine_range.rows),
            &expected,
        );
    }
    let min_date = cell_text(&engine_range.rows[0][0]);
    let max_date = cell_text(&engine_range.rows[0][1]);

    // ---- The full Q6 predicate, for reference ----------------------------
    let q_full = format!("SELECT COUNT(*) FROM lineitem WHERE {Q6_FILTER}");
    let full_count = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q6_where_parsed:full_q6_predicate",
        &q_full,
    );

    // ---- Anti-vacuity: the probes must actually reach data ---------------
    // Without this, the partition invariants below would degenerate into
    // `0 == 0` and would pass even for a completely broken engine.
    assert!(
        eq_count > 0,
        "the probe date '{PROBE_DATE}' must occur in the data, otherwise the \
         date-comparison invariants below would be vacuously true"
    );
    assert!(
        between > 0,
        "the `BETWEEN` filter must select rows, otherwise the `BETWEEN` \
         invariant below would be vacuously true"
    );
    assert!(
        full_count > 0,
        "the full Q6 predicate must select rows, otherwise the narrowing \
         invariant below would be vacuously true"
    );
    assert_eq!(
        engine_sample.rows.len(),
        3,
        "the ship-date sample must return 3 rows"
    );

    // ---- Invariant 1: `=`, `>` and `<` partition the table ----------------
    assert_eq!(
        eq_count + gt_count + lt_count,
        total,
        "`l_shipdate = '{PROBE_DATE}'`, `l_shipdate > '{PROBE_DATE}'` and \
         `l_shipdate < '{PROBE_DATE}'` must partition the table"
    );

    // ---- Invariant 2: `>=` is `=` plus `>` -------------------------------
    assert_eq!(
        ge_count,
        eq_count + gt_count,
        "`l_shipdate >= '{PROBE_DATE}'` must select the rows of \
         `l_shipdate = '{PROBE_DATE}'` plus the rows of \
         `l_shipdate > '{PROBE_DATE}'`"
    );

    // ---- Invariant 3: the two half-open ranges partition the table -------
    assert_eq!(
        lt_1995 + ge_1995,
        total,
        "`l_shipdate < '1995-01-01'` and `l_shipdate >= '1995-01-01'` must \
         partition the table"
    );

    // ---- Invariant 4: BETWEEN is equivalent to a pair of comparisons ------
    assert_eq!(
        between, comparisons,
        "`l_discount BETWEEN 0.06 AND 0.08` must select the same rows as \
         `l_discount >= 0.06 AND l_discount <= 0.08`"
    );

    // ---- Invariant 5: the quantity comparison partitions the table -------
    assert_eq!(
        qty_lt + qty_ge,
        total,
        "`l_quantity < 25` and `l_quantity >= 25` must partition the table"
    );

    // ---- Invariant 6: the stored ship dates are well-formed ISO dates -----
    assert!(
        is_iso_date(&min_date),
        "MIN(l_shipdate) must be a well-formed ISO date, got {min_date:?}"
    );
    assert!(
        is_iso_date(&max_date),
        "MAX(l_shipdate) must be a well-formed ISO date, got {max_date:?}"
    );
    assert!(
        min_date <= max_date,
        "MIN(l_shipdate) ({min_date}) must not be greater than MAX(l_shipdate) \
         ({max_date})"
    );
    for (i, row) in engine_sample.rows.iter().enumerate() {
        let d = cell_text(&row[0]);
        assert!(
            is_iso_date(&d),
            "row {i}: the ship date must be a well-formed ISO date, got {d:?}"
        );
        assert!(
            d >= min_date && d <= max_date,
            "row {i}: the ship date {d} must lie between MIN ({min_date}) and \
             MAX ({max_date})"
        );
    }

    // ---- Invariant 7: the full Q6 predicate narrows the table ------------
    assert!(
        full_count <= total,
        "the Q6 predicate can never select more rows than the table holds"
    );
    assert!(
        full_count <= between,
        "the Q6 predicate includes the `BETWEEN` filter, so it cannot select \
         more rows than the `BETWEEN` filter alone ({between} -> {full_count})"
    );
    assert!(
        full_count <= qty_lt,
        "the Q6 predicate includes `l_quantity < 25`, so it cannot select \
         more rows than that filter alone ({qty_lt} -> {full_count})"
    );
}
