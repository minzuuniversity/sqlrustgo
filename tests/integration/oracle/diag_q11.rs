//! Q11 isolation — `HAVING` in a 3-way comma-list join.
//!
//! # What changed (issue #5379)
//!
//! This file used to contain **zero assertions**. It printed the expected
//! value and nothing else:
//!
//! ```text
//! eprintln!("Step 1: 3-way join + GERMANY (expect ~1060): {:?}", r1.rows);
//! eprintln!("Step 3: with HAVING > 10000 (expect 80): {}", r3.rows.len());
//! ```
//!
//! so the test "passed" even if every query returned garbage — and it
//! *panicked* in CI because it read `tests/data/tpch-sf01`, a symlink into
//! `/tmp` that resolves on exactly one machine.
//!
//! It now asserts twice over, with two independent sources of truth:
//!
//! * **Independent oracle** — the same query is executed by an in-process
//!   SQLite loaded from the same fixture (`tests/common/sqlite_oracle.rs`).
//!   The expected value therefore comes from a *different engine*, never from
//!   this engine's own output.
//! * **Data-independent invariants** — assertions whose expected value
//!   follows from the data's own consistency, not from a hard-coded row
//!   count: `HAVING > 0` must keep every group, a threshold above every
//!   aggregate must keep none, and the group count must equal
//!   `COUNT(DISTINCT ...)` over the same join.
//!
//! The historical numbers (1060 groups, 80 rows) belong to a dbgen SF=1
//! dataset that this repository has never contained; they are **not**
//! asserted here. Asserting them against a different dataset would be
//! asserting a number that has nothing to do with the data under test.

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
const TARGET: &str = "diag_q11";

/// The join predicate shared by every query in this file.
const JOIN: &str = "ps_suppkey = s_suppkey AND s_nationkey = n_nationkey AND n_name = 'GERMANY'";

/// Tables this test needs.
const TABLES: [&str; 3] = ["partsupp", "supplier", "nation"];

/// A threshold above every possible `SUM(ps_supplycost * ps_availqty)` in
/// the fixture — used to prove that `HAVING` can filter *everything* out.
const IMPOSSIBLE_THRESHOLD: i64 = 1_000_000_000_000;

#[test]
fn diag_q11_having_three_way() {
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

    // ---- Step 1: the 3-way join itself ----------------------------------
    let q_join = format!("SELECT COUNT(*) FROM partsupp, supplier, nation WHERE {JOIN}");
    let engine_join = engine.execute(&q_join).expect("executing the 3-way join");
    let oracle_join = oracle.query(&q_join).expect("querying the oracle");
    sqlite_oracle::check_against_oracle(
        "diag_q11:q11_join_count",
        &sqlite_oracle::engine_rows_to_strings(&engine_join.rows),
        &oracle_join,
    );

    // ---- Step 2: GROUP BY over the join ---------------------------------
    let q_group = format!(
        "SELECT ps_partkey, SUM(ps_supplycost * ps_availqty) AS part_value \
         FROM partsupp, supplier, nation WHERE {JOIN} GROUP BY ps_partkey"
    );
    let engine_group = engine
        .execute(&q_group)
        .expect("executing the grouped query");
    let groups = engine_group.rows.len();

    // Anti-vacuity: every invariant below is an equality between two counts.
    // If the join matched nothing, all of them would hold trivially and the
    // test would pass on an engine that returns nothing at all. The German
    // supplier filter must therefore match real data.
    assert!(
        groups > 0,
        "the 3-way join with n_name = 'GERMANY' matched no rows — the fixture's \
         partsupp.ps_suppkey may not reference the German suppliers, which would \
         make every count-based assertion below vacuously true"
    );

    // Invariant: the number of groups equals the number of distinct
    // `ps_partkey` values that survive the same join. Independent of any
    // hard-coded row count.
    let q_distinct =
        format!("SELECT COUNT(DISTINCT ps_partkey) FROM partsupp, supplier, nation WHERE {JOIN}");
    let engine_distinct = engine
        .execute(&q_distinct)
        .expect("executing COUNT(DISTINCT ...)");
    let distinct = engine_distinct.rows[0][0]
        .as_integer()
        .unwrap_or_else(|| panic!("COUNT(DISTINCT ...) did not return an integer"));
    assert_eq!(
        groups as i64, distinct,
        "the GROUP BY must produce one row per distinct ps_partkey of the joined rows"
    );

    // Oracle comparison for the grouped query.
    let oracle_group = oracle.query(&q_group).expect("querying the oracle");
    sqlite_oracle::check_against_oracle(
        "diag_q11:q11_group_by",
        &sqlite_oracle::engine_rows_to_strings(&engine_group.rows),
        &oracle_group,
    );

    // ---- Step 3: HAVING > 0 must keep every group -----------------------
    let q_having_zero = format!(
        "SELECT ps_partkey, SUM(ps_supplycost * ps_availqty) AS part_value \
         FROM partsupp, supplier, nation WHERE {JOIN} \
         GROUP BY ps_partkey HAVING SUM(ps_supplycost * ps_availqty) > 0"
    );
    let engine_having_zero = engine
        .execute(&q_having_zero)
        .expect("executing HAVING > 0");
    assert_eq!(
        engine_having_zero.rows.len(),
        groups,
        "HAVING with a threshold of 0 must keep every group of the GROUP BY"
    );
    let oracle_having_zero = oracle.query(&q_having_zero).expect("querying the oracle");
    sqlite_oracle::check_against_oracle(
        "diag_q11:q11_having_gt_zero",
        &sqlite_oracle::engine_rows_to_strings(&engine_having_zero.rows),
        &oracle_having_zero,
    );

    // ---- Step 4: an unreachable threshold must keep nothing --------------
    let q_having_none = format!(
        "SELECT ps_partkey, SUM(ps_supplycost * ps_availqty) AS part_value \
         FROM partsupp, supplier, nation WHERE {JOIN} \
         GROUP BY ps_partkey HAVING SUM(ps_supplycost * ps_availqty) > {IMPOSSIBLE_THRESHOLD}"
    );
    let engine_having_none = engine
        .execute(&q_having_none)
        .expect("executing the high threshold");
    assert_eq!(
        engine_having_none.rows.len(),
        0,
        "a HAVING threshold above every aggregate must keep 0 groups"
    );
    let oracle_having_none = oracle.query(&q_having_none).expect("querying the oracle");
    sqlite_oracle::check_against_oracle(
        "diag_q11:q11_having_above_all",
        &sqlite_oracle::engine_rows_to_strings(&engine_having_none.rows),
        &oracle_having_none,
    );

    // ---- Step 5: a real threshold keeps a subset of the groups -----------
    let q_having_10k = format!(
        "SELECT ps_partkey, SUM(ps_supplycost * ps_availqty) AS part_value \
         FROM partsupp, supplier, nation WHERE {JOIN} \
         GROUP BY ps_partkey HAVING SUM(ps_supplycost * ps_availqty) > 10000"
    );
    let engine_having_10k = engine
        .execute(&q_having_10k)
        .expect("executing HAVING > 10000");
    assert!(
        engine_having_10k.rows.len() <= groups,
        "a HAVING threshold can only narrow the group set, never widen it"
    );
    let oracle_having_10k = oracle.query(&q_having_10k).expect("querying the oracle");
    sqlite_oracle::check_against_oracle(
        "diag_q11:q11_having_gt_10000",
        &sqlite_oracle::engine_rows_to_strings(&engine_having_10k.rows),
        &oracle_having_10k,
    );
}
