//! Q11 deeper isolation — the intermediate join steps of the 3-way join.
//!
//! # What changed (issue #5379)
//!
//! This file used to contain **zero assertions**. It walked the Q11 join
//! from one table to three and printed each step's row count:
//!
//! ```text
//! eprintln!("Step1 1-join: rows.len()={}, first={:?}", r1.rows.len(), r1.rows.first());
//! eprintln!("Step4 group_by: rows.len()={}, first={:?}", r4.rows.len(), r4.rows.first());
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
//! * **Data-independent invariants** — the interesting property of this file
//!   is that the *syntax* of the join must not change its cardinality:
//!   a comma-list join and an explicit `INNER JOIN ... ON` over the same
//!   predicates must produce the same number of rows, adding a join can
//!   only narrow the result, and a `GROUP BY` must produce exactly one row
//!   per distinct group key.
//! * **Anti-vacuity** — the join steps and the `GERMANY` filter must
//!   actually select rows. Without that, every cardinality invariant below
//!   would degenerate into `0 == 0` and would pass even for a completely
//!   broken engine.
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
const TARGET: &str = "diag_q11_steps";

/// Tables this test needs.
const TABLES: [&str; 3] = ["partsupp", "supplier", "nation"];

/// The join predicate of the full Q11.
const JOIN: &str = "ps_suppkey = s_suppkey AND s_nationkey = n_nationkey AND n_name = 'GERMANY'";

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
fn diag_q11_steps() {
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

    let mut loaded = 0usize;
    for (table, ncols) in [("partsupp", 5), ("supplier", 7), ("nation", 4)] {
        let n = tpch_fixture::load_into_storage(&storage, fixture, table, ncols)
            .unwrap_or_else(|e| panic!("loading table `{table}`: {e}"));
        assert!(n > 0, "table `{table}` loaded 0 rows");
        loaded += n;
    }

    // The independent oracle: same fixture, different engine.
    let oracle = SqliteOracle::open(fixture, &TABLES).expect("opening the SQLite oracle");

    // ---- Step 1: just join supplier (1st JOIN) --------------------------
    let q1 = "SELECT COUNT(*) FROM partsupp INNER JOIN supplier ON ps_suppkey = s_suppkey";
    let step1 = count_with_oracle(&mut engine, &oracle, "diag_q11_steps:step1_one_join", q1);

    // ---- Step 2: full 3-way, explicit JOIN syntax -----------------------
    let q2 = "SELECT COUNT(*) FROM partsupp INNER JOIN supplier ON ps_suppkey = s_suppkey INNER JOIN nation ON s_nationkey = n_nationkey";
    let step2 = count_with_oracle(&mut engine, &oracle, "diag_q11_steps:step2_two_joins", q2);

    // ---- Step 3: 3-way comma-list ----------------------------------------
    let q3 = "SELECT COUNT(*) FROM partsupp, supplier, nation WHERE ps_suppkey = s_suppkey AND s_nationkey = n_nationkey";
    let step3 = count_with_oracle(&mut engine, &oracle, "diag_q11_steps:step3_comma_join", q3);

    // ---- Step 4: add GROUP BY without HAVING ----------------------------
    let q4 = format!(
        "SELECT ps_partkey, SUM(ps_supplycost * ps_availqty) AS part_value \
         FROM partsupp, supplier, nation WHERE {JOIN} GROUP BY ps_partkey"
    );
    let engine_group = engine.execute(&q4).expect("executing the grouped query");
    if let Some(expected) = oracle_rows(&oracle, "diag_q11_steps:step4_group_by", &q4) {
        sqlite_oracle::check_against_oracle(
            "diag_q11_steps:step4_group_by",
            &sqlite_oracle::engine_rows_to_strings(&engine_group.rows),
            &expected,
        );
    }
    let groups = engine_group.rows.len();

    // ---- Step 5: how many distinct parts survive the same join ----------
    let q5 =
        format!("SELECT COUNT(DISTINCT ps_partkey) FROM partsupp, supplier, nation WHERE {JOIN}");
    let distinct_parts = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q11_steps:step5_distinct_parts",
        &q5,
    );

    // ---- Step 6: does every supplier match exactly one nation? -----------
    let q6 = "SELECT COUNT(*) FROM supplier, nation WHERE s_nationkey = n_nationkey";
    let supplier_nation = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q11_steps:step6_supplier_nation",
        q6,
    );
    let suppliers = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q11_steps:step6_supplier_total",
        "SELECT COUNT(*) FROM supplier",
    );

    // ---- Anti-vacuity: every step must actually produce rows ------------
    // Without this, every cardinality invariant below would be `0 == 0` and
    // would pass even for a completely broken engine.
    assert!(
        step1 > 0,
        "the partsupp ⋈ supplier join must produce rows, otherwise the \
         cardinality invariants below would be vacuously true"
    );
    assert!(
        step2 > 0 && step3 > 0,
        "the 3-way join must produce rows, otherwise the cardinality \
         invariants below would be vacuously true"
    );
    assert!(
        groups > 0,
        "the `GERMANY` filter must select at least one partsupp row, otherwise \
         the `GROUP BY` invariants below would be vacuously true"
    );
    assert!(
        distinct_parts > 0,
        "the `GERMANY` filter must reach at least one distinct part"
    );

    // ---- Invariant 1: the nation join is not restrictive ------------------
    // Every supplier row matches exactly one nation row, so adding the
    // nation join cannot remove a joined row.
    assert_eq!(
        supplier_nation, suppliers,
        "every supplier must match exactly one nation (the join to nation is not restrictive)"
    );
    assert_eq!(
        step2, step1,
        "adding the nation join must not change the row count \
         (step1={step1}, step2={step2})"
    );

    // ---- Invariant 2: the comma-list join and the explicit JOIN agree ---
    assert_eq!(
        step3, step2,
        "a comma-list join and the equivalent INNER JOIN ... ON syntax must \
         produce the same number of rows (comma={step3}, explicit={step2})"
    );

    // ---- Invariant 3: a join can only narrow -----------------------------
    assert!(
        step2 <= step1,
        "adding a join can only narrow the result (step1={step1}, step2={step2})"
    );
    assert!(
        step3 <= loaded as i64,
        "a joined row count can never exceed the number of loaded rows ({loaded})"
    );

    // ---- Invariant 4: the GROUP BY has one row per distinct part ---------
    assert_eq!(
        groups as i64, distinct_parts,
        "the GROUP BY must produce exactly one row per distinct ps_partkey of the joined rows"
    );
}
