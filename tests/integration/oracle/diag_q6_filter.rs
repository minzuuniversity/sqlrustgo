//! Q6 filter diagnostic — do `WHERE` filters actually work for 1-table
//! queries?
//!
//! # What changed (issue #5379)
//!
//! This file used to contain **zero assertions**. It added one `WHERE`
//! predicate at a time and printed the resulting count:
//!
//! ```text
//! eprintln!("Step 3: shipdate + discount = {}", both_count);
//! eprintln!("  expected from Q6: 1");
//! eprintln!("Step 5: full Q6 SUM = {:?}", r5.rows);
//! ```
//!
//! `1` is the row count the original author measured on **their own dbgen
//! SF=1 dataset** — a dataset this repository has never contained. It is
//! **not** asserted here.
//!
//! The file also *panicked* in CI: it read `tests/data/tpch-sf01`, a symlink
//! into `/tmp` that resolves on exactly one machine. Fixture resolution now
//! goes through `tests/common/tpch_fixture.rs`; a missing fixture is reported
//! as an explicit `TPCH-NOT-RUN` marker (counted as `not_run`, never as
//! `passed`).
//!
//! # What is asserted now
//!
//! * **Independent oracle** — every query is also executed by an in-process
//!   SQLite loaded from the same fixture (`tests/common/sqlite_oracle.rs`).
//!   The expected values come from a *different engine*.
//! * **Data-independent invariants** — the properties a `WHERE` clause must
//!   have whatever the data looks like: adding a conjunct can only narrow
//!   the result, the order of the conjuncts does not matter, `BETWEEN` is
//!   equivalent to the corresponding pair of comparisons, and the `SUM` of
//!   the filtered rows is a subset of the `SUM` of the whole table.
//!
//! * **Anti-vacuity** — every filter step must actually select rows. Without
//!   that, the narrowing and `SUM` invariants above would degenerate into
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
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

use sqlite_oracle::SqliteOracle;
use tpch_fixture::FixtureState;

/// Cargo target name — used in the NOT-RUN marker and the known-difference
/// registry.
const TARGET: &str = "diag_q6_filter";

/// Tables this test needs.
const TABLES: [&str; 1] = ["lineitem"];

/// The `WHERE` predicate of the Q6 query.
const Q6_FILTER: &str = "l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01' \
                           AND l_discount BETWEEN 0.06 AND 0.08 \
                           AND l_quantity < 25";

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

/// Execute `sql` on the engine, compare it with the oracle, and return the
/// engine's single cell as a number (`None` for SQL NULL).
fn optional_sum_with_oracle(
    engine: &mut ExecutionEngine<MemoryStorage>,
    oracle: &SqliteOracle,
    case_id: &str,
    sql: &str,
) -> Option<f64> {
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
    match &result.rows[0][0] {
        Value::Null => None,
        Value::Integer(i) => Some(*i as f64),
        Value::Float(f) => Some(*f),
        v => panic!("{case_id}: the aggregate did not return a number: {v:?}"),
    }
}

#[test]
fn diag_q6_filter() {
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

    // ---- Step 1: total count --------------------------------------------
    let total = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q6_filter:step1_total",
        "SELECT COUNT(*) FROM lineitem",
    );
    assert_eq!(
        total, loaded as i64,
        "the engine must see exactly the rows that were loaded into the storage"
    );

    // ---- Step 2: count after only the shipdate filter --------------------
    let shipdate_count = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q6_filter:step2_shipdate",
        "SELECT COUNT(*) FROM lineitem WHERE l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01'",
    );

    // ---- Step 3: shipdate + discount BETWEEN -----------------------------
    let both_count = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q6_filter:step3_discount_between",
        "SELECT COUNT(*) FROM lineitem WHERE l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01' AND l_discount BETWEEN 0.06 AND 0.08",
    );

    // ---- Step 4: shipdate + discount + quantity --------------------------
    let all_count = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q6_filter:step4_quantity",
        "SELECT COUNT(*) FROM lineitem WHERE l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01' AND l_discount BETWEEN 0.06 AND 0.08 AND l_quantity < 25",
    );

    // ---- Step 5: the full Q6 with the SUM -------------------------------
    let full_sum = optional_sum_with_oracle(
        &mut engine,
        &oracle,
        "diag_q6_filter:step5_full_q6_sum",
        &format!("SELECT SUM(l_extendedprice * l_discount) FROM lineitem WHERE {Q6_FILTER}"),
    );

    // ---- Step 6: the same filter with the conjuncts reordered ------------
    // The order of the conjuncts must not change the result.
    let reordered = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q6_filter:step6_reordered_conjuncts",
        "SELECT COUNT(*) FROM lineitem WHERE l_discount BETWEEN 0.06 AND 0.08 AND l_quantity < 25 AND l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01'",
    );

    // ---- Step 7: BETWEEN written as a pair of comparisons -----------------
    let between = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q6_filter:step7_between_whole_table",
        "SELECT COUNT(*) FROM lineitem WHERE l_discount BETWEEN 0.06 AND 0.08",
    );
    let comparisons = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q6_filter:step7_comparisons_whole_table",
        "SELECT COUNT(*) FROM lineitem WHERE l_discount >= 0.06 AND l_discount <= 0.08",
    );

    // ---- Step 8: the SUM over the whole table ----------------------------
    let whole_sum = optional_sum_with_oracle(
        &mut engine,
        &oracle,
        "diag_q6_filter:step8_whole_table_sum",
        "SELECT SUM(l_extendedprice * l_discount) FROM lineitem",
    );

    // ---- Anti-vacuity: every filter step must actually select rows -------
    // Without this, the narrowing invariant below would degenerate into
    // `0 == 0` and would pass even for a completely broken engine.
    assert!(
        shipdate_count > 0,
        "the ship-date filter must select rows, otherwise the narrowing \
         invariant below would be vacuously true"
    );
    assert!(
        both_count > 0,
        "the ship-date + discount filter must select rows, otherwise the \
         narrowing invariant below would be vacuously true"
    );
    assert!(
        all_count > 0,
        "the full Q6 filter must select rows, otherwise the `SUM` invariant \
         below would be vacuously true"
    );
    assert!(
        between > 0,
        "the `BETWEEN` filter must select rows over the whole table"
    );

    // ---- Invariant 1: adding a conjunct can only narrow ------------------
    assert!(
        shipdate_count <= total,
        "a filter can never select more rows than the table holds \
         ({total} -> {shipdate_count})"
    );
    assert!(
        both_count <= shipdate_count,
        "adding a conjunct can only narrow the result ({shipdate_count} -> {both_count})"
    );
    assert!(
        all_count <= both_count,
        "adding a conjunct can only narrow the result ({both_count} -> {all_count})"
    );

    // ---- Invariant 2: the order of the conjuncts does not matter ---------
    assert_eq!(
        reordered, all_count,
        "a conjunction must select the same rows whatever order its conjuncts \
         are written in ({all_count} vs {reordered})"
    );

    // ---- Invariant 3: BETWEEN is equivalent to a pair of comparisons ------
    assert_eq!(
        between, comparisons,
        "`l_discount BETWEEN 0.06 AND 0.08` must select the same rows as \
         `l_discount >= 0.06 AND l_discount <= 0.08`"
    );

    // ---- Invariant 4: the SUM tracks the filtered row count ---------------
    // Every term of `l_extendedprice * l_discount` is strictly positive in
    // the filtered set, so the sum is zero exactly when no row survives.
    match full_sum {
        Some(v) => {
            assert!(
                v > 0.0,
                "the filtered SUM must be positive when the filter selects \
                 rows, got {v} for {all_count} rows"
            );
            assert!(
                all_count > 0,
                "the filtered SUM is {v} but the filter selects no row at all"
            );
        }
        None => assert_eq!(
            all_count, 0,
            "the filtered SUM is NULL, so the filter must select no row at all"
        ),
    }

    // ---- Invariant 5: the filtered SUM is a subset of the whole SUM ------
    if let (Some(filtered), Some(whole)) = (full_sum, whole_sum) {
        assert!(
            filtered <= whole,
            "the filtered SUM ({filtered}) is a subset of the whole-table SUM \
             ({whole}) and can never be larger"
        );
    }
}
