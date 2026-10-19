//! Q11 isolated — what the `WHERE` filter actually removes.
//!
//! # What changed (issue #5379)
//!
//! This file used to contain **zero assertions**. It compared the
//! unfiltered 3-way cross product with the `WHERE`-filtered join and printed
//! both counts:
//!
//! ```text
//! eprintln!("Q11 3-way cross: rows.len() = {} (expected {})", r1.rows.len(), ...);
//! eprintln!("Q11 with WHERE: rows.len() = {} (expected 8000-ish)", r2.rows.len());
//! ```
//!
//! `8000-ish` is the row count the original author measured on **their own
//! dbgen SF=1 dataset** — a dataset this repository has never contained. It
//! is **not** asserted here.
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
//! * **Data-independent invariants** — the cross product of three tables is
//!   exactly the product of their row counts, filtering can only reduce the
//!   count, and the comma-list join and the explicit `INNER JOIN ... ON`
//!   syntax must agree on the surviving row count.
//! * **Anti-vacuity** — the `GERMANY` filter must actually select rows and
//!   must actually remove rows from the cross product. Without that, every
//!   cardinality invariant below would degenerate into `0 == 0` and would
//!   pass even for a completely broken engine.
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
const TARGET: &str = "diag_q11_where";

/// Tables this test needs.
const TABLES: [&str; 3] = ["partsupp", "supplier", "nation"];

/// The nation the original file filtered on.
const NATION: &str = "GERMANY";

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
fn diag_q11_where_only() {
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

    // ---- The row count of every single table ----------------------------
    let partsupp_rows = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q11_where:count_partsupp",
        "SELECT COUNT(*) FROM partsupp",
    );
    let supplier_rows = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q11_where:count_supplier",
        "SELECT COUNT(*) FROM supplier",
    );
    let nation_rows = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q11_where:count_nation",
        "SELECT COUNT(*) FROM nation",
    );
    let germany_rows = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q11_where:count_germany",
        &format!("SELECT COUNT(*) FROM nation WHERE n_name = '{NATION}'"),
    );

    // ---- Invariant 1: the loader loaded exactly what the tables hold ----
    assert_eq!(
        partsupp_rows + supplier_rows + nation_rows,
        loaded as i64,
        "the engine must see exactly the rows that were loaded into the storage"
    );

    // ---- Step 1: the 3-way cross product, without any WHERE --------------
    let q_cross = "SELECT COUNT(*) FROM partsupp, supplier, nation";
    let cross = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q11_where:cross_product",
        q_cross,
    );

    // ---- Invariant 2: a cross product is the product of the row counts ---
    assert_eq!(
        cross,
        partsupp_rows * supplier_rows * nation_rows,
        "the 3-way cross product must be exactly the product of the three \
         table row counts"
    );

    // ---- Step 2: with the WHERE filter ----------------------------------
    let q_where = format!(
        "SELECT COUNT(*) FROM partsupp, supplier, nation \
         WHERE ps_suppkey = s_suppkey AND s_nationkey = n_nationkey AND n_name = '{NATION}'"
    );
    let filtered = count_with_oracle(&mut engine, &oracle, "diag_q11_where:with_where", &q_where);

    // ---- Invariant 3: filtering can only reduce the row count -----------
    assert!(
        filtered <= cross,
        "adding a WHERE clause can only reduce the row count \
         (cross={cross}, filtered={filtered})"
    );
    assert!(
        filtered <= partsupp_rows,
        "each partsupp row matches at most one supplier and one nation, so the \
         filtered count can never exceed the number of partsupp rows \
         ({partsupp_rows})"
    );

    // ---- Step 3: the same filter written as an explicit JOIN -------------
    let q_join = format!(
        "SELECT COUNT(*) FROM partsupp INNER JOIN supplier ON ps_suppkey = s_suppkey \
         INNER JOIN nation ON s_nationkey = n_nationkey WHERE n_name = '{NATION}'"
    );
    let joined = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q11_where:explicit_join",
        &q_join,
    );

    // ---- Invariant 4: both join syntaxes agree --------------------------
    assert_eq!(
        joined, filtered,
        "the comma-list join and the equivalent INNER JOIN ... ON syntax must \
         select the same number of rows (comma={filtered}, explicit={joined})"
    );

    // ---- Invariant 5: filtering on the nation alone ---------------------
    // The cross product of the two other tables with the single German
    // nation row is exactly partsupp * supplier rows.
    let q_nation_only =
        format!("SELECT COUNT(*) FROM partsupp, supplier, nation WHERE n_name = '{NATION}'");
    let nation_only = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q11_where:nation_only",
        &q_nation_only,
    );
    assert_eq!(
        nation_only,
        partsupp_rows * supplier_rows * germany_rows,
        "the cross product restricted to the German nation must be exactly \
         partsupp * supplier * germany"
    );
    assert!(
        filtered <= nation_only,
        "adding the join predicates on top of the nation filter can only \
         reduce the row count (nation_only={nation_only}, filtered={filtered})"
    );

    // ---- Anti-vacuity: the filter must actually select and remove rows --
    // Without this, every cardinality invariant above would be `0 == 0` and
    // would pass even for a completely broken engine.
    assert!(
        germany_rows > 0,
        "the fixture must contain the `{NATION}` nation, otherwise the \
         filtered queries would be vacuously empty"
    );
    assert!(
        filtered > 0,
        "the `GERMANY` filter must select at least one joined row, otherwise \
         the cardinality invariants above would be vacuously true"
    );
    assert!(
        filtered < nation_only,
        "the join predicates must remove rows from the cross product \
         (nation_only={nation_only}, filtered={filtered})"
    );
}
