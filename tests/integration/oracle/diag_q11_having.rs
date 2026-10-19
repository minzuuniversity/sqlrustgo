//! Q11 `HAVING` isolated — three `HAVING` thresholds over the same GROUP BY.
//!
//! # What changed (issue #5379)
//!
//! This file used to contain **zero assertions**. It ran three variants of
//! Q11 and printed the row counts:
//!
//! ```text
//! eprintln!("HAVING > 0: rows.len()={} (expected 80)", r1.rows.len());
//! eprintln!("HAVING > 10000000: rows.len()={} (expected 0)", r2.rows.len());
//! eprintln!("Full Q11: rows.len()={} (expected 80)", r3.rows.len());
//! ```
//!
//! `80` is the row count the original author measured on **their own dbgen
//! SF=1 dataset** — a dataset this repository has never contained. Asserting
//! it here would assert a number that has nothing to do with the data under
//! test, so it is **not** asserted. The `expected 0` for the very high
//! threshold is a different kind of claim: it only holds when the threshold
//! really is above every group aggregate, which is a property of *this*
//! dataset, not of the SF=1 one. It is therefore replaced by a threshold
//! that is **derived from the data** (the maximum group value plus one) and
//! asserted to filter everything out.
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
//! * **Data-independent invariants** — `HAVING > 0` keeps every group (once
//!   every group value is shown to be positive), a higher threshold keeps a
//!   subset, a threshold above the maximum group value keeps nothing, and
//!   every returned row satisfies the `HAVING` predicate.
//! * **Anti-vacuity** — the `GERMANY` filter must actually select rows and
//!   the `HAVING` thresholds must actually keep some. Without that, every
//!   invariant below would degenerate into `0 == 0` and would pass even for
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
const TARGET: &str = "diag_q11_having";

/// Tables this test needs.
const TABLES: [&str; 3] = ["partsupp", "supplier", "nation"];

/// The join predicate shared by every query in this file.
const JOIN: &str = "ps_suppkey = s_suppkey AND s_nationkey = n_nationkey AND n_name = 'GERMANY'";

/// The `HAVING` threshold the original file used for the "full Q11" query.
const HAVING_THRESHOLD: i64 = 10_000;

/// The very high `HAVING` threshold the original file used.
const HIGH_THRESHOLD: i64 = 10_000_000;

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

/// Build the `GROUP BY ... HAVING <threshold>` query of this file.
fn having_query(threshold: &str) -> String {
    format!(
        "SELECT ps_partkey, SUM(ps_supplycost * ps_availqty) AS part_value \
         FROM partsupp, supplier, nation WHERE {JOIN} \
         GROUP BY ps_partkey HAVING SUM(ps_supplycost * ps_availqty) > {threshold}"
    )
}

#[test]
fn diag_q11_having() {
    let state = FixtureState::resolve();
    let Some(fixture) = state.require(TARGET) else {
        // No fixture: the NOT-RUN marker was printed by `require`. This test
        // is reported as `not_run` by CI, never as `passed`.
        return;
    };

    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    engine
        .execute("CREATE TABLE partsupp (ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, ps_comment TEXT, PRIMARY KEY (ps_partkey, ps_suppkey))")
        .unwrap();
    engine
        .execute("CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)")
        .unwrap();
    engine
        .execute("CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, n_regionkey INTEGER NOT NULL, n_comment TEXT)")
        .unwrap();

    for (table, ncols) in [("partsupp", 5), ("supplier", 7), ("nation", 4)] {
        let loaded = tpch_fixture::load_into_storage(&storage, fixture, table, ncols)
            .unwrap_or_else(|e| panic!("loading table `{table}`: {e}"));
        assert!(loaded > 0, "table `{table}` loaded 0 rows");
    }

    // The independent oracle: same fixture, different engine.
    let oracle = SqliteOracle::open(fixture, &TABLES).expect("opening the SQLite oracle");

    // ---- The GROUP BY without HAVING: the group set every threshold
    //      filters down from ----------------------------------------------
    let q_group = format!(
        "SELECT ps_partkey, SUM(ps_supplycost * ps_availqty) AS part_value \
         FROM partsupp, supplier, nation WHERE {JOIN} GROUP BY ps_partkey"
    );
    let engine_group = engine
        .execute(&q_group)
        .expect("executing the grouped query without HAVING");
    if let Some(expected) = oracle_rows(&oracle, "diag_q11_having:q11_group_by", &q_group) {
        sqlite_oracle::check_against_oracle(
            "diag_q11_having:q11_group_by",
            &sqlite_oracle::engine_rows_to_strings(&engine_group.rows),
            &expected,
        );
    }
    let groups = engine_group.rows.len();
    let group_values: Vec<f64> = engine_group
        .rows
        .iter()
        .map(|r| {
            as_f64(&r[1]).unwrap_or_else(|| {
                panic!("part_value is not numeric: {:?}", r[1]);
            })
        })
        .collect();

    // ---- Anti-vacuity: the nation filter must actually select rows -------
    // Without this, every invariant below would be `0 == 0` and would pass
    // even for a completely broken engine.
    assert!(
        groups > 0,
        "the `GERMANY` filter must select at least one partsupp row, otherwise \
         every `HAVING` invariant in this file would be vacuously true"
    );

    // ---- Invariant 1: every group value is positive ----------------------
    // This is the data fact that makes `HAVING > 0` a no-op below. It is
    // asserted explicitly so that invariant 2 cannot be reached with a
    // hidden assumption.
    for (i, v) in group_values.iter().enumerate() {
        assert!(
            *v > 0.0,
            "group {i} has a non-positive part_value {v}; the `HAVING > 0` \
             invariant below only holds when every group value is positive"
        );
    }

    // ---- HAVING > 0 ------------------------------------------------------
    let q_having_zero = having_query("0");
    let engine_having_zero = engine
        .execute(&q_having_zero)
        .expect("executing HAVING > 0");
    if let Some(expected) = oracle_rows(
        &oracle,
        "diag_q11_having:q11_having_gt_zero",
        &q_having_zero,
    ) {
        sqlite_oracle::check_against_oracle(
            "diag_q11_having:q11_having_gt_zero",
            &sqlite_oracle::engine_rows_to_strings(&engine_having_zero.rows),
            &expected,
        );
    }

    // ---- Invariant 2: `HAVING > 0` keeps exactly the same rows -----------
    assert_eq!(
        engine_having_zero.rows.len(),
        groups,
        "HAVING with a threshold of 0 must keep every group of the GROUP BY"
    );
    let mut zero_rows = sqlite_oracle::engine_rows_to_strings(&engine_having_zero.rows);
    let mut group_rows = sqlite_oracle::engine_rows_to_strings(&engine_group.rows);
    zero_rows.sort();
    group_rows.sort();
    assert_eq!(
        zero_rows, group_rows,
        "HAVING > 0 must return the same rows as the GROUP BY without HAVING \
         (every group value is positive)"
    );

    // ---- HAVING > 10000000 -----------------------------------------------
    let q_having_high = having_query(&HIGH_THRESHOLD.to_string());
    let engine_having_high = engine
        .execute(&q_having_high)
        .expect("executing HAVING > 10000000");
    if let Some(expected) = oracle_rows(
        &oracle,
        "diag_q11_having:q11_having_gt_high",
        &q_having_high,
    ) {
        sqlite_oracle::check_against_oracle(
            "diag_q11_having:q11_having_gt_high",
            &sqlite_oracle::engine_rows_to_strings(&engine_having_high.rows),
            &expected,
        );
    }

    // ---- Invariant 3: a higher threshold keeps a subset ------------------
    assert!(
        engine_having_high.rows.len() <= engine_having_zero.rows.len(),
        "a higher HAVING threshold can only narrow the result, never widen it \
         ({} rows for > {HIGH_THRESHOLD}, {} rows for > 0)",
        engine_having_high.rows.len(),
        engine_having_zero.rows.len()
    );
    let mut high_rows = sqlite_oracle::engine_rows_to_strings(&engine_having_high.rows);
    high_rows.sort();
    for row in &high_rows {
        assert!(
            zero_rows.binary_search(row).is_ok(),
            "row {row:?} survived `HAVING > {HIGH_THRESHOLD}` but not `HAVING > 0`"
        );
    }

    // ---- Invariant 4: a threshold above the maximum group value keeps 0 ---
    // The threshold is derived from the *oracle's* group values, so the
    // expectation ("nothing survives") is not derived from the engine's own
    // output.
    let group_values_oracle = oracle
        .query(&q_group)
        .expect("querying the oracle for the group values");
    assert_eq!(
        group_values_oracle.len(),
        groups,
        "the oracle and the engine must agree on the number of groups"
    );
    let oracle_max = group_values_oracle
        .iter()
        .map(|r| r[1].parse::<f64>().expect("oracle group value is numeric"))
        .fold(f64::NEG_INFINITY, f64::max);
    assert!(
        oracle_max.is_finite(),
        "the oracle must report a finite maximum group value"
    );
    let impossible = oracle_max + 1.0;
    let q_having_impossible = having_query(&format!("{impossible}"));
    let engine_having_impossible = engine
        .execute(&q_having_impossible)
        .expect("executing the data-derived impossible threshold");
    assert_eq!(
        engine_having_impossible.rows.len(),
        0,
        "a HAVING threshold above every group value ({impossible}) must keep 0 groups"
    );
    if let Some(expected) = oracle_rows(
        &oracle,
        "diag_q11_having:q11_having_above_max",
        &q_having_impossible,
    ) {
        sqlite_oracle::check_against_oracle(
            "diag_q11_having:q11_having_above_max",
            &sqlite_oracle::engine_rows_to_strings(&engine_having_impossible.rows),
            &expected,
        );
    }

    // ---- The full Q11 ---------------------------------------------------
    let q_full = format!(
        "{} ORDER BY part_value DESC",
        having_query(&HAVING_THRESHOLD.to_string())
    );
    let engine_full = engine
        .execute(&q_full)
        .expect("executing the full Q11 query");
    if let Some(expected) = oracle_rows(&oracle, "diag_q11_having:q11_full", &q_full) {
        sqlite_oracle::check_against_oracle(
            "diag_q11_having:q11_full",
            &sqlite_oracle::engine_rows_to_strings(&engine_full.rows),
            &expected,
        );
    }

    // ---- Anti-vacuity: the `HAVING > 10000` filter must keep some rows ---
    assert!(
        !engine_full.rows.is_empty(),
        "the `HAVING > {HAVING_THRESHOLD}` filter must keep at least one group, \
         otherwise the narrowing invariants above would be vacuously true"
    );

    // ---- Invariant 5: the full Q11 is a narrowing of the group set ------
    assert!(
        engine_full.rows.len() <= groups,
        "the full Q11 must not return more rows than the GROUP BY it filters"
    );
    let mut full_rows = sqlite_oracle::engine_rows_to_strings(&engine_full.rows);
    full_rows.sort();
    for row in &full_rows {
        assert!(
            group_rows.binary_search(row).is_ok(),
            "row {row:?} is not a member of the GROUP BY result"
        );
    }

    // ---- Invariant 6: every returned row satisfies the HAVING predicate --
    for (i, row) in engine_full.rows.iter().enumerate() {
        let part_value = as_f64(&row[1])
            .unwrap_or_else(|| panic!("row {i}: part_value is not numeric: {:?}", row[1]));
        assert!(
            part_value > HAVING_THRESHOLD as f64,
            "row {i} was returned by a HAVING > {HAVING_THRESHOLD} query but its \
             part_value is {part_value}"
        );
    }
}
