//! TPC-H Q13 subquery repro test
//!
//! # What is under test
//!
//! Q13 puts a subquery on both sides of an `IN` / `NOT IN`:
//!
//! ```sql
//! SELECT o_custkey FROM orders WHERE o_comment LIKE '%special%requests%'
//! ```
//!
//! The historical finding (22-22-AUDIT-GATE-REPORT §4) was that `IN` and
//! `NOT IN` over that subquery returned the wrong customer sets.
//!
//! # Where the expected values come from
//!
//! From the **independent SQLite oracle** (`tests/common/sqlite_oracle.rs`),
//! never from this engine's own output. Hard-coding numbers would only freeze
//! whatever the engine happened to produce at the time.
//!
//! The previous version of this file did the opposite twice over:
//!
//! * it asserted numbers (`1..49`, `< 50`) calibrated on a *different*
//!   fixture — the Sprint 7 SF=0.001 data with 150 customers and 1 500
//!   orders — so the assertions could not hold against any other fixture;
//! * it loaded the fixture with one `INSERT` statement per row, 86 630 round
//!   trips through the SQL parser, which is why a single test target took
//!   more than 20 minutes and had to be killed by a timeout.
//!
//! The fixture is now loaded through the shared batch loader
//! (`tpch_fixture::load_into_storage`), which types the values instead of
//! quoting everything as text — the old loader stored `l_quantity` as the
//! *string* `'23'` in a `REAL` column.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::Arc;

#[path = "../../common/mod.rs"]
mod common;
#[path = "../../common/sqlite_oracle.rs"]
mod sqlite_oracle;
#[path = "../../common/tpch_fixture.rs"]
mod tpch_fixture;

use common::tpch_wire_harness::{report_not_run, SCHEMA_DDL};
use sqlite_oracle::{check_against_oracle, engine_rows_to_strings, SqliteOracle};
use tpch_fixture::Fixture;

/// The cargo target name, used in the NOT-RUN marker.
const TARGET: &str = "q13_subquery_repro";

/// The three TPC-H tables this test reads, and how many `.tbl` fields each
/// one contributes.
const TABLES: [(&str, usize); 8] = [
    ("region", 3),
    ("nation", 4),
    ("supplier", 7),
    ("customer", 8),
    ("part", 9),
    ("partsupp", 5),
    ("orders", 9),
    ("lineitem", 16),
];

/// The subquery both `IN` and `NOT IN` are built on.
const SUBQUERY: &str = "SELECT o_custkey FROM orders WHERE o_comment LIKE '%special%requests%'";

/// Resolve the fixture, or disclose the test as *not run*.
fn fixture() -> Option<Fixture> {
    match tpch_fixture::FixtureState::resolve() {
        tpch_fixture::FixtureState::Present(f) => Some(f),
        tpch_fixture::FixtureState::Missing { detail } => {
            report_not_run(TARGET, "fixture_missing", &detail);
            None
        }
        tpch_fixture::FixtureState::ProfileMismatch { detail, .. } => {
            report_not_run(TARGET, "profile_mismatch", &detail);
            None
        }
    }
}

/// Build an engine with the TPC-H schema and the fixture data loaded.
///
/// The rows are pushed straight into the storage in batches instead of being
/// replayed as `INSERT` statements.
fn make_engine(fixture: &Fixture) -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    for ddl in SCHEMA_DDL {
        engine
            .execute(ddl)
            .unwrap_or_else(|e| panic!("DDL failed: {ddl} - {e}"));
    }
    for (tbl, cols) in TABLES {
        let n = tpch_fixture::load_into_storage(&storage, fixture, tbl, cols)
            .unwrap_or_else(|e| panic!("cannot load {tbl}: {e}"));
        eprintln!("Loaded {tbl}: {n} rows");
    }
    engine
}

/// Run `sql` on the engine and return the rows as comparable strings.
fn engine_rows(engine: &mut ExecutionEngine<MemoryStorage>, sql: &str) -> Vec<Vec<String>> {
    let result = engine
        .execute(sql)
        .unwrap_or_else(|e| panic!("engine failed on `{sql}`: {e}"));
    engine_rows_to_strings(&result.rows)
}

/// Run `sql` on the independent SQLite oracle.
fn oracle_rows(oracle: &SqliteOracle, sql: &str) -> Vec<Vec<String>> {
    oracle
        .query(sql)
        .unwrap_or_else(|e| panic!("SQLite oracle failed on `{sql}`: {e}"))
}

#[test]
fn q13_bare_subquery_row_count() {
    let Some(fixture) = fixture() else { return };
    let mut engine = make_engine(&fixture);
    let oracle = SqliteOracle::open(&fixture, &["orders", "customer"]).expect("open SQLite oracle");

    let engine_rows = engine_rows(&mut engine, SUBQUERY);
    let expected = oracle_rows(&oracle, SUBQUERY);

    check_against_oracle("q13_subquery_repro:bare_subquery", &engine_rows, &expected);

    // The subquery must actually select something: a fixture in which no
    // order comment matches would make the two `IN` / `NOT IN` tests below
    // degenerate into "everything" / "nothing".
    assert!(
        !expected.is_empty(),
        "no order comment matches '%special%requests%' in this fixture — \
         the Q13 subquery would be empty and the IN / NOT IN assertions below \
         would prove nothing"
    );
    eprintln!("bare subquery returned {} rows", engine_rows.len());
}

#[test]
fn q13_in_subquery_filters() {
    let Some(fixture) = fixture() else { return };
    let mut engine = make_engine(&fixture);
    let oracle = SqliteOracle::open(&fixture, &["orders", "customer"]).expect("open SQLite oracle");

    let sql = format!("SELECT c_custkey FROM customer WHERE c_custkey IN ({SUBQUERY})");
    let engine_rows = engine_rows(&mut engine, &sql);
    let expected = oracle_rows(&oracle, &sql);

    check_against_oracle("q13_subquery_repro:in_subquery", &engine_rows, &expected);

    // `IN` has to be *selective*: it must return some customers and leave
    // others out.
    let total = oracle_rows(&oracle, "SELECT c_custkey FROM customer").len();
    assert!(
        !expected.is_empty(),
        "IN (subquery) matched no customer — the filter is degenerate"
    );
    assert!(
        expected.len() < total,
        "IN (subquery) matched every customer ({expected_len}/{total}) — \
         the filter is not selective",
        expected_len = expected.len()
    );
    eprintln!(
        "IN (subquery) returned {} of {total} customers",
        engine_rows.len()
    );
}

#[test]
fn q13_not_in_subquery_excludes() {
    let Some(fixture) = fixture() else { return };
    let mut engine = make_engine(&fixture);
    let oracle = SqliteOracle::open(&fixture, &["orders", "customer"]).expect("open SQLite oracle");

    let sql = format!("SELECT c_custkey FROM customer WHERE c_custkey NOT IN ({SUBQUERY})");
    let engine_rows = engine_rows(&mut engine, &sql);
    let expected = oracle_rows(&oracle, &sql);

    check_against_oracle(
        "q13_subquery_repro:not_in_subquery",
        &engine_rows,
        &expected,
    );

    // `IN` and `NOT IN` partition the customer set: together they must
    // account for every customer exactly once. This catches a `NOT IN` that
    // silently drops the customers it should keep — the very defect the
    // original test was written to catch, now stated as an invariant that
    // does not depend on any fixture-specific number.
    let total = oracle_rows(&oracle, "SELECT c_custkey FROM customer").len();
    let in_rows = oracle_rows(
        &oracle,
        &format!("SELECT c_custkey FROM customer WHERE c_custkey IN ({SUBQUERY})"),
    );
    assert!(
        !expected.is_empty(),
        "NOT IN (subquery) matched no customer — the filter is degenerate"
    );
    assert_eq!(
        in_rows.len() + expected.len(),
        total,
        "IN and NOT IN must partition the {total} customers, but they account for {}",
        in_rows.len() + expected.len()
    );
    eprintln!(
        "NOT IN (subquery) returned {} of {total} customers",
        engine_rows.len()
    );
}
