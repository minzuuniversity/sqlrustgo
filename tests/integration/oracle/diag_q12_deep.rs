//! Q12 deeper isolation — the `WHERE` predicates of Q12, one at a time.
//!
//! # What changed (issue #5379)
//!
//! This file used to contain **zero assertions**. It added one `WHERE`
//! predicate at a time and printed the resulting count:
//!
//! ```text
//! eprintln!("shipmode IN: {:?}", r1.rows);
//! eprintln!("+ receipt >= 1994: {:?}", r5.rows);
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
//! * **Independent oracle** — every step is also executed by an in-process
//!   SQLite loaded from the same fixture (`tests/common/sqlite_oracle.rs`).
//!   The expected values come from a *different engine*.
//! * **Data-independent invariants** — the properties of a `WHERE` clause
//!   that follow from the data's own consistency: an `IN` list is the
//!   disjoint union of its equality branches, a comparison and its negation
//!   partition the table, a range and its complement partition the table,
//!   and adding a conjunct can only narrow the result.
//!
//! * **Anti-vacuity** — every filter step must actually select rows. Without
//!   that, the partition and narrowing invariants above would degenerate into
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
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::Arc;

use sqlite_oracle::SqliteOracle;
use tpch_fixture::FixtureState;

/// Cargo target name — used in the NOT-RUN marker and the known-difference
/// registry.
const TARGET: &str = "diag_q12_deep";

/// Tables this test needs.
const TABLES: [&str; 1] = ["lineitem"];

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

#[test]
fn diag_q12_deep() {
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

    // ---- The total number of lineitem rows -------------------------------
    let total = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q12_deep:total",
        "SELECT COUNT(*) FROM lineitem",
    );
    assert_eq!(
        total, loaded as i64,
        "the engine must see exactly the rows that were loaded into the storage"
    );

    // ---- Step 1: the `IN` clause -----------------------------------------
    let q1 = "SELECT COUNT(*) FROM lineitem WHERE l_shipmode IN ('MAIL', 'SHIP')";
    let in_clause = count_with_oracle(&mut engine, &oracle, "diag_q12_deep:shipmode_in", q1);

    // ---- Step 2: the `<` test on dates -----------------------------------
    let q2 = "SELECT COUNT(*) FROM lineitem WHERE l_commitdate < l_receiptdate";
    let commit_lt_receipt =
        count_with_oracle(&mut engine, &oracle, "diag_q12_deep:commit_lt_receipt", q2);

    // ---- Step 3: the AND chain -------------------------------------------
    let q3 = "SELECT COUNT(*) FROM lineitem WHERE l_shipmode IN ('MAIL', 'SHIP') AND l_commitdate < l_receiptdate";
    let chain = count_with_oracle(&mut engine, &oracle, "diag_q12_deep:chain", q3);

    // ---- Step 4: more filters --------------------------------------------
    let q4 = "SELECT COUNT(*) FROM lineitem WHERE l_shipmode IN ('MAIL', 'SHIP') AND l_commitdate < l_receiptdate AND l_shipdate < l_commitdate";
    let plus_ship_lt_commit = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q12_deep:plus_ship_lt_commit",
        q4,
    );

    // ---- Step 5: the full Q12 predicate ----------------------------------
    let q5 = "SELECT COUNT(*) FROM lineitem WHERE l_shipmode IN ('MAIL', 'SHIP') AND l_commitdate < l_receiptdate AND l_shipdate < l_commitdate AND l_receiptdate >= '1994-01-01' AND l_receiptdate < '1995-01-01'";
    let full_filter = count_with_oracle(&mut engine, &oracle, "diag_q12_deep:full_filter", q5);

    // ---- Step 6: just the date filters -----------------------------------
    let q6 = "SELECT COUNT(*) FROM lineitem WHERE l_receiptdate >= '1994-01-01' AND l_receiptdate < '1995-01-01'";
    let receipt_1994 =
        count_with_oracle(&mut engine, &oracle, "diag_q12_deep:receipt_1994_only", q6);

    // ---- Step 7/8: the two branches of the `IN` clause -------------------
    let q7 = "SELECT COUNT(*) FROM lineitem WHERE l_shipmode = 'MAIL'";
    let mail_only = count_with_oracle(&mut engine, &oracle, "diag_q12_deep:shipmode_mail", q7);
    let q8 = "SELECT COUNT(*) FROM lineitem WHERE l_shipmode = 'SHIP'";
    let ship_only = count_with_oracle(&mut engine, &oracle, "diag_q12_deep:shipmode_ship", q8);

    // ---- Step 9: the negation of the `<` test ----------------------------
    let q9 = "SELECT COUNT(*) FROM lineitem WHERE l_commitdate >= l_receiptdate";
    let commit_ge_receipt =
        count_with_oracle(&mut engine, &oracle, "diag_q12_deep:commit_ge_receipt", q9);

    // ---- Step 10: the complement of the 1994 receipt-date range ----------
    let q10 = "SELECT COUNT(*) FROM lineitem WHERE l_receiptdate < '1994-01-01' OR l_receiptdate >= '1995-01-01'";
    let outside_1994 = count_with_oracle(&mut engine, &oracle, "diag_q12_deep:outside_1994", q10);

    // ---- Step 11: the complement of the `IN` clause ----------------------
    let q11 = "SELECT COUNT(*) FROM lineitem WHERE l_shipmode <> 'MAIL' AND l_shipmode <> 'SHIP'";
    let not_in_clause =
        count_with_oracle(&mut engine, &oracle, "diag_q12_deep:shipmode_not_in", q11);

    // ---- Anti-vacuity: every *filter* step must actually select rows ----
    // The `l_commitdate >= l_receiptdate` step is the **negation** of a
    // comparison and is legitimately allowed to be empty — it is excluded
    // from this check on purpose.
    assert!(
        in_clause > 0,
        "`l_shipmode IN ('MAIL', 'SHIP')` must select rows, otherwise the \
         `IN` invariant below would be vacuously true"
    );
    assert!(
        mail_only > 0 && ship_only > 0,
        "both branches of the `IN` list must select rows, otherwise the \
         `IN` invariant below would be vacuously true"
    );
    assert!(
        commit_lt_receipt > 0,
        "`l_commitdate < l_receiptdate` must select rows"
    );
    assert!(
        chain > 0 && plus_ship_lt_commit > 0 && full_filter > 0,
        "each step of the narrowing chain must select at least one row, \
         otherwise the narrowing invariant below would be vacuously true \
         (chain={chain}, plus_ship_lt_commit={plus_ship_lt_commit}, \
         full_filter={full_filter})"
    );
    assert!(
        receipt_1994 > 0,
        "the 1994 receipt-date range must select rows, otherwise the \
         partition invariant below would be vacuously true"
    );
    assert!(
        not_in_clause > 0,
        "the complement of the `IN` filter must select rows, otherwise the \
         partition invariant below would be vacuously true"
    );

    // ---- Invariant 1: `IN` is the disjoint union of its branches ---------
    assert_eq!(
        in_clause,
        mail_only + ship_only,
        "`l_shipmode IN ('MAIL', 'SHIP')` must select exactly the rows of \
         `l_shipmode = 'MAIL'` plus the rows of `l_shipmode = 'SHIP'`"
    );

    // ---- Invariant 2: a comparison and its negation partition the table --
    assert_eq!(
        commit_lt_receipt + commit_ge_receipt,
        total,
        "`l_commitdate < l_receiptdate` and `l_commitdate >= l_receiptdate` \
         must partition the table"
    );

    // ---- Invariant 3: a range and its complement partition the table -----
    assert_eq!(
        receipt_1994 + outside_1994,
        total,
        "the 1994 receipt-date range and its complement must partition the table"
    );

    // ---- Invariant 4: adding a conjunct can only narrow ------------------
    assert!(
        chain <= in_clause,
        "adding a conjunct can only narrow the result ({in_clause} -> {chain})"
    );
    assert!(
        chain <= commit_lt_receipt,
        "adding a conjunct can only narrow the result ({commit_lt_receipt} -> {chain})"
    );
    assert!(
        plus_ship_lt_commit <= chain,
        "adding a conjunct can only narrow the result ({chain} -> {plus_ship_lt_commit})"
    );
    assert!(
        full_filter <= plus_ship_lt_commit,
        "adding a conjunct can only narrow the result \
         ({plus_ship_lt_commit} -> {full_filter})"
    );
    assert!(
        full_filter <= receipt_1994,
        "the full filter implies the 1994 receipt-date range, so it cannot \
         select more rows than the range alone ({receipt_1994} -> {full_filter})"
    );

    // ---- Invariant 5: the filter cannot select more than the table ------
    assert!(
        full_filter <= total,
        "a WHERE clause can never select more rows than the table holds"
    );

    // ---- Invariant 6: the `IN` filter and its complement partition the
    //      table -------------------------------------------------------
    assert_eq!(
        in_clause + not_in_clause,
        total,
        "`l_shipmode IN ('MAIL', 'SHIP')` and its complement must partition \
         the table"
    );
}
