//! Q11 isolated — the full 3-way join with `HAVING` and `ORDER BY`.
//!
//! # What changed (issue #5379)
//!
//! This file used to contain **zero assertions**. It ran the complete Q11
//! query and printed the row count together with the word "expected":
//!
//! ```text
//! eprintln!("Q11 full: rows.len() = {}, expected 80", r.rows.len());
//! ```
//!
//! Two things were wrong with that:
//!
//! 1. *Nothing was asserted.* The query could return garbage and the test
//!    would still pass — it only ever printed.
//! 2. *The expected value was a historical number.* `80` is the row count the
//!    original author measured on **their own dbgen SF=1 dataset**. This
//!    repository has never contained that dataset, so the number means
//!    nothing here. It is **not** asserted any more.
//!
//! The file also *panicked* in CI: it read `tests/data/tpch-sf01`, a symlink
//! into `/tmp` that resolves on exactly one machine. Fixture resolution now
//! goes through `tests/common/tpch_fixture.rs`, and a missing fixture is
//! reported as an explicit `TPCH-NOT-RUN` marker (counted as `not_run`, never
//! as `passed`).
//!
//! # What is asserted now
//!
//! * **Independent oracle** — the same queries run against an in-process
//!   SQLite loaded from the same fixture (`tests/common/sqlite_oracle.rs`).
//!   The expected values come from a *different engine*, never from this
//!   engine's own output.
//! * **Data-independent invariants** — the expected values follow from the
//!   data's own consistency: one row per distinct `ps_partkey`, `HAVING` can
//!   only narrow the group set, every returned row satisfies the `HAVING`
//!   predicate, and the result's column count matches the query's.
//! * **Anti-vacuity** — the `GERMANY` filter must actually select rows. If
//!   it selected none, every cardinality invariant below would degenerate
//!   into `0 == 0` and would pass even for a completely broken engine, so
//!   the file asserts non-emptiness explicitly.
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
const TARGET: &str = "diag_q11_3way";

/// Tables this test needs.
const TABLES: [&str; 3] = ["partsupp", "supplier", "nation"];

/// The join predicate shared by every query in this file.
const JOIN: &str = "ps_suppkey = s_suppkey AND s_nationkey = n_nationkey AND n_name = 'GERMANY'";

/// The `HAVING` threshold the original file used.
const HAVING_THRESHOLD: i64 = 10_000;

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

#[test]
fn diag_q11_3way() {
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

    // ---- Query 1: the full Q11 ------------------------------------------
    let q_full = format!(
        "SELECT ps_partkey, SUM(ps_supplycost * ps_availqty) AS part_value \
         FROM partsupp, supplier, nation WHERE {JOIN} \
         GROUP BY ps_partkey HAVING SUM(ps_supplycost * ps_availqty) > {HAVING_THRESHOLD} \
         ORDER BY part_value DESC"
    );
    let engine_full = engine
        .execute(&q_full)
        .expect("executing the full Q11 query");
    if let Some(expected) = oracle_rows(&oracle, "diag_q11_3way:q11_full", &q_full) {
        sqlite_oracle::check_against_oracle(
            "diag_q11_3way:q11_full",
            &sqlite_oracle::engine_rows_to_strings(&engine_full.rows),
            &expected,
        );
    }

    // ---- Query 2: the same GROUP BY without HAVING ------------------------
    let q_group = format!(
        "SELECT ps_partkey, SUM(ps_supplycost * ps_availqty) AS part_value \
         FROM partsupp, supplier, nation WHERE {JOIN} GROUP BY ps_partkey"
    );
    let engine_group = engine
        .execute(&q_group)
        .expect("executing the grouped query without HAVING");
    if let Some(expected) = oracle_rows(&oracle, "diag_q11_3way:q11_group_by", &q_group) {
        sqlite_oracle::check_against_oracle(
            "diag_q11_3way:q11_group_by",
            &sqlite_oracle::engine_rows_to_strings(&engine_group.rows),
            &expected,
        );
    }
    let groups = engine_group.rows.len();

    // ---- Query 3: the number of distinct parts in the same join ----------
    let q_distinct =
        format!("SELECT COUNT(DISTINCT ps_partkey) FROM partsupp, supplier, nation WHERE {JOIN}");
    let engine_distinct = engine
        .execute(&q_distinct)
        .expect("executing COUNT(DISTINCT ps_partkey)");
    if let Some(expected) = oracle_rows(&oracle, "diag_q11_3way:q11_distinct_parts", &q_distinct) {
        sqlite_oracle::check_against_oracle(
            "diag_q11_3way:q11_distinct_parts",
            &sqlite_oracle::engine_rows_to_strings(&engine_distinct.rows),
            &expected,
        );
    }
    let distinct_parts = engine_distinct.rows[0][0]
        .as_integer()
        .unwrap_or_else(|| panic!("COUNT(DISTINCT ps_partkey) did not return an integer"));

    // ---- Anti-vacuity: the nation filter must actually select rows -------
    // Without this, every invariant below would be `0 == 0` and would pass
    // even for a completely broken engine.
    assert!(
        groups > 0,
        "the `GERMANY` filter must select at least one partsupp row, otherwise \
         every cardinality invariant in this file would be vacuously true"
    );
    assert!(
        distinct_parts > 0,
        "the `GERMANY` filter must reach at least one distinct part"
    );
    assert!(
        !engine_full.rows.is_empty(),
        "the `HAVING > {HAVING_THRESHOLD}` filter must keep at least one group, \
         otherwise the `HAVING` invariants below would be vacuously true"
    );

    // ---- Invariant 1: the GROUP BY has one row per distinct part ---------
    assert_eq!(
        groups as i64, distinct_parts,
        "the GROUP BY must produce exactly one row per distinct ps_partkey of the joined rows"
    );

    // ---- Invariant 2: HAVING can only narrow the group set ---------------
    assert!(
        engine_full.rows.len() <= groups,
        "a HAVING clause can only remove groups, never add them \
         (engine returned {} rows for HAVING > {HAVING_THRESHOLD}, the GROUP BY has {groups})",
        engine_full.rows.len()
    );

    // ---- Invariant 3: every returned row satisfies the HAVING predicate ---
    for (i, row) in engine_full.rows.iter().enumerate() {
        assert_eq!(
            row.len(),
            2,
            "row {i} must have exactly 2 cells (ps_partkey, part_value)"
        );
        let part_value = as_f64(&row[1])
            .unwrap_or_else(|| panic!("row {i}: part_value is not numeric: {:?}", row[1]));
        assert!(
            part_value > HAVING_THRESHOLD as f64,
            "row {i} was returned by a HAVING > {HAVING_THRESHOLD} query but its \
             part_value is {part_value}"
        );
    }

    // ---- Invariant 4: the result has one row per part --------------------
    let mut full_keys: Vec<String> = engine_full
        .rows
        .iter()
        .map(|r| sqlite_oracle::engine_cell_to_string(&r[0]))
        .collect();
    let unique_keys = {
        let mut k = full_keys.clone();
        k.sort();
        k.dedup();
        k.len()
    };
    assert_eq!(
        unique_keys,
        engine_full.rows.len(),
        "the GROUP BY key ps_partkey must be unique in the result"
    );
    full_keys.sort();
    let mut group_keys: Vec<String> = engine_group
        .rows
        .iter()
        .map(|r| sqlite_oracle::engine_cell_to_string(&r[0]))
        .collect();
    group_keys.sort();
    for key in &full_keys {
        assert!(
            group_keys.binary_search(key).is_ok(),
            "row {key:?} survived the HAVING filter but it is not a member of \
             the GROUP BY result"
        );
    }

    // ---- Invariant 5: the result's shape matches the query ----------------
    assert_eq!(
        engine_full.columns.len(),
        2,
        "the result must declare exactly 2 columns (ps_partkey, part_value)"
    );
    assert_eq!(
        engine_group.columns.len(),
        2,
        "the result must declare exactly 2 columns (ps_partkey, part_value)"
    );

    // ---- Invariant 6: `ORDER BY part_value DESC` really is descending -----
    for pair in engine_full.rows.windows(2) {
        let prev = as_f64(&pair[0][1]).expect("part_value is numeric");
        let next = as_f64(&pair[1][1]).expect("part_value is numeric");
        assert!(
            prev >= next,
            "the result is ordered by part_value DESC but {prev} comes before {next}"
        );
    }
}
