//! What type is `l_shipdate` actually in storage?
//!
//! # What changed (issue #5379)
//!
//! This file used to contain **zero assertions**. It printed the stored
//! ship dates, the counts of two comparisons and the raw storage value:
//!
//! ```text
//! eprintln!("first 5 shipdates: {:?}", r.rows);
//! eprintln!("shipdate >= '1994-01-01' (expect ~45000, got 60000): {:?}", r2.rows);
//! eprintln!("storage directly - l_shipdate value of row 0: {:?}", ...);
//! ```
//!
//! The `expect ~45000` / `got 60000` numbers are from the original author's
//! dbgen SF=1 dataset — a dataset this repository has never contained — and
//! the `got 60000` was a *complaint* about the engine, not an expectation.
//! Neither is asserted here.
//!
//! The file also *panicked* in CI: it read `tests/data/tpch-sf01`, a symlink
//! into `/tmp` that resolves on exactly one machine. Fixture resolution now
//! goes through `tests/common/tpch_fixture.rs`; a missing fixture is reported
//! as an explicit `TPCH-NOT-RUN` marker (counted as `not_run`, never as
//! `passed`).
//!
//! # What is asserted now
//!
//! * **Independent oracle** — the comparison counts are also produced by an
//!   in-process SQLite loaded from the same fixture
//!   (`tests/common/sqlite_oracle.rs`). The expected values come from a
//!   *different engine*.
//! * **Data-independent invariants** — the properties of the *stored*
//!   values, checked against the raw `.tbl` file as an independent source:
//!   `l_shipdate` must be stored as **text** (not as an integer, a float or
//!   a date), the multiset of the stored ship dates must be exactly the
//!   multiset of the ship-date fields in the `.tbl` file, every stored value
//!   must be a well-formed ISO `YYYY-MM-DD` calendar date, and a comparison
//!   and its two strict/non-strict branches must partition the table.
//!
//! * **Anti-vacuity** — the probe date must actually occur in the data.
//!   Without that, the comparison invariants above would degenerate into
//!   `0 == 0` and would pass even for a completely broken engine.
//!
//! All queries in this file are plain SQL that SQLite runs as written
//! (`adapt_sql` reports neither adaptations nor unsupported constructs for
//! them), so **every** query is checked against the oracle.

#[path = "../../common/sqlite_oracle.rs"]
mod sqlite_oracle;
#[path = "../../common/tpch_fixture.rs"]
mod tpch_fixture;

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine, Value};
use std::sync::Arc;

use sqlite_oracle::SqliteOracle;
use tpch_fixture::FixtureState;

/// Cargo target name — used in the NOT-RUN marker and the known-difference
/// registry.
const TARGET: &str = "diag_shipdate_type";

/// Tables this test needs.
const TABLES: [&str; 1] = ["lineitem"];

/// The zero-based index of `l_shipdate` in the TPC-H `lineitem` column
/// order.
const SHIPDATE_INDEX: usize = 10;

/// The date the comparisons probe against.
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

#[test]
fn test_shipdate_actual_value() {
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

    let loaded = tpch_fixture::load_into_storage(&storage, fixture, "lineitem", 16)
        .unwrap_or_else(|e| panic!("loading table `lineitem`: {e}"));
    assert!(loaded > 0, "table `lineitem` loaded 0 rows");

    // The independent oracle: same fixture, different engine.
    let oracle = SqliteOracle::open(fixture, &TABLES).expect("opening the SQLite oracle");

    // ---- The total number of rows ---------------------------------------
    let total = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_shipdate_type:total",
        "SELECT COUNT(*) FROM lineitem",
    );
    assert_eq!(
        total, loaded as i64,
        "the engine must see exactly the rows that were loaded into the storage"
    );

    // ---- The stored ship dates, straight out of the storage --------------
    let stored_rows = {
        let s = storage.read();
        s.scan("lineitem")
            .unwrap_or_else(|e| panic!("scanning the stored lineitem rows: {e}"))
    };
    assert_eq!(
        stored_rows.len() as i64,
        total,
        "the storage must hold exactly the rows that were loaded"
    );

    // ---- Invariant 1: `l_shipdate` is stored as TEXT ---------------------
    // The `lineitem` schema declares `l_shipdate TEXT`, and the `.tbl` file
    // stores it as a `YYYY-MM-DD` string. The loader must not silently turn
    // it into a number or a date.
    let mut stored_dates: Vec<String> = Vec::with_capacity(stored_rows.len());
    for (i, row) in stored_rows.iter().enumerate() {
        let cell = row
            .get(SHIPDATE_INDEX)
            .unwrap_or_else(|| panic!("stored row {i} has no l_shipdate column"));
        assert!(
            matches!(cell, Value::Text(_)),
            "stored row {i}: l_shipdate must be stored as TEXT, got {cell:?}"
        );
        let s = match cell {
            Value::Text(s) => s.clone(),
            v => v.to_sql_string(),
        };
        stored_dates.push(s);
    }

    // ---- Invariant 2: the stored dates are the file's dates --------------
    // The raw `.tbl` file is the independent source: the loader must have
    // preserved every ship date exactly.
    let raw_rows = fixture
        .load_rows("lineitem")
        .unwrap_or_else(|e| panic!("reading the raw lineitem rows: {e}"));
    let mut raw_dates: Vec<String> = raw_rows
        .iter()
        .map(|r| {
            r.get(SHIPDATE_INDEX)
                .unwrap_or_else(|| panic!("a raw lineitem row has no l_shipdate column"))
                .clone()
        })
        .collect();
    assert_eq!(
        raw_dates.len() as i64,
        total,
        "the raw `.tbl` file must hold exactly the rows that were loaded"
    );
    raw_dates.sort();
    let mut sorted_stored = stored_dates.clone();
    sorted_stored.sort();
    assert_eq!(
        sorted_stored, raw_dates,
        "the stored ship dates must be exactly the ship-date fields of the \
         raw `.tbl` file"
    );

    // ---- Invariant 3: every stored date is a well-formed ISO date -------
    for (i, d) in stored_dates.iter().enumerate() {
        assert!(
            is_iso_date(d),
            "stored row {i}: l_shipdate must be a well-formed ISO date, got {d:?}"
        );
    }

    // ---- The ship dates the engine returns -------------------------------
    // The `ORDER BY` makes the sample deterministic; the `.tbl` file is in
    // `(l_orderkey, l_linenumber)` order, so this is the first five rows.
    let q_sample = "SELECT l_shipdate FROM lineitem ORDER BY l_orderkey, l_linenumber LIMIT 5";
    let engine_sample = engine
        .execute(q_sample)
        .expect("executing the ship-date sample");
    if let Some(expected) = oracle_rows(&oracle, "diag_shipdate_type:shipdate_sample", q_sample) {
        sqlite_oracle::check_against_oracle(
            "diag_shipdate_type:shipdate_sample",
            &sqlite_oracle::engine_rows_to_strings(&engine_sample.rows),
            &expected,
        );
    }

    // ---- Invariant 4: the engine returns the stored text -----------------
    for (i, row) in engine_sample.rows.iter().enumerate() {
        match &row[0] {
            Value::Text(s) => assert!(
                is_iso_date(s),
                "row {i}: the returned ship date must be a well-formed ISO date, got {s:?}"
            ),
            v => panic!("row {i}: the returned ship date must be TEXT, got {v:?}"),
        }
        assert!(
            stored_dates.contains(&row[0].to_sql_string()),
            "row {i}: the returned ship date {:?} is not one of the stored ship dates",
            row[0]
        );
    }

    // ---- The comparison behaviour ----------------------------------------
    let q_ge = format!("SELECT COUNT(*) FROM lineitem WHERE l_shipdate >= '{PROBE_DATE}'");
    let ge_count = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_shipdate_type:shipdate_ge",
        &q_ge,
    );
    let q_lt = format!("SELECT COUNT(*) FROM lineitem WHERE l_shipdate < '{PROBE_DATE}'");
    let lt_count = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_shipdate_type:shipdate_lt",
        &q_lt,
    );
    let q_eq = format!("SELECT COUNT(*) FROM lineitem WHERE l_shipdate = '{PROBE_DATE}'");
    let eq_count = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_shipdate_type:shipdate_eq",
        &q_eq,
    );
    let q_gt = format!("SELECT COUNT(*) FROM lineitem WHERE l_shipdate > '{PROBE_DATE}'");
    let gt_count = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_shipdate_type:shipdate_gt",
        &q_gt,
    );
    let q_tautology = format!(
        "SELECT COUNT(*) FROM lineitem WHERE l_shipdate < '{PROBE_DATE}' \
         OR l_shipdate >= '{PROBE_DATE}'"
    );
    let tautology = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_shipdate_type:shipdate_tautology",
        &q_tautology,
    );

    // ---- Anti-vacuity: the probes must actually reach data ---------------
    // Without this, the partition invariants below would degenerate into
    // `0 == 0` and would pass even for a completely broken engine.
    assert!(
        eq_count > 0,
        "the probe date '{PROBE_DATE}' must occur in the data, otherwise the \
         date-comparison invariants below would be vacuously true"
    );
    assert_eq!(
        engine_sample.rows.len(),
        5,
        "the ship-date sample must return 5 rows"
    );
    assert!(
        !stored_dates.is_empty(),
        "the storage must hold ship dates, otherwise the type invariants \
         above would be vacuously true"
    );

    // ---- Invariant 5: the comparison branches partition the table ---------
    assert_eq!(
        lt_count + eq_count + gt_count,
        total,
        "`l_shipdate < '{PROBE_DATE}'`, `l_shipdate = '{PROBE_DATE}'` and \
         `l_shipdate > '{PROBE_DATE}'` must partition the table"
    );
    assert_eq!(
        ge_count,
        eq_count + gt_count,
        "`l_shipdate >= '{PROBE_DATE}'` must select the rows of \
         `l_shipdate = '{PROBE_DATE}'` plus the rows of \
         `l_shipdate > '{PROBE_DATE}'`"
    );

    // ---- Invariant 6: a tautology selects every row ----------------------
    assert_eq!(
        tautology, total,
        "a comparison and its complement must together select every row"
    );
}
