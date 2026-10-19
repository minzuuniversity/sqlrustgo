//! Q12 isolation — `CASE WHEN` inside a `SUM` aggregate with a 2-way join.
//!
//! # What changed (issue #5379)
//!
//! This file used to contain **zero assertions**. It walked Q12 from the
//! filtered `lineitem` count to the full grouped query and printed every
//! step:
//!
//! ```text
//! eprintln!("Step 1: filtered lineitem count (expect ~6200): {:?}", r1.rows);
//! eprintln!("Step 3: SUM(CASE) on orders (expect ~800): {:?}", r3.rows);
//! eprintln!("Step 4: full Q12: {:?}", r4.rows);
//! ```
//!
//! `~6200` and `~800` are counts the original author measured on **their own
//! dbgen SF=1 dataset** — a dataset this repository has never contained.
//! They are **not** asserted here; asserting them against a different
//! dataset would assert a number that has nothing to do with the data under
//! test.
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
//! * **Data-independent invariants** — the interesting property of Q12 is
//!   that the `CASE` expression must *partition* the joined rows: for every
//!   `l_shipmode` group, `high_line_count + low_line_count` must equal the
//!   number of joined rows in that group, the per-group `high_line_count`s
//!   must sum to the global high-priority count, and the number of groups
//!   must equal `COUNT(DISTINCT l_shipmode)` over the same join.
//!
//! * **Anti-vacuity** — the Q12 `WHERE` filter and the `GROUP BY` must
//!   actually select rows. Without that, the `CASE` invariants above would
//!   degenerate into `0 == 0` and would pass even for a completely broken
//!   engine.
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
const TARGET: &str = "diag_q12";

/// Tables this test needs.
const TABLES: [&str; 2] = ["orders", "lineitem"];

/// The `WHERE` predicate of the Q12 query.
const Q12_FILTER: &str = "l_shipmode IN ('MAIL', 'SHIP') \
                           AND l_commitdate < l_receiptdate \
                           AND l_shipdate < l_commitdate \
                           AND l_receiptdate >= '1994-01-01' \
                           AND l_receiptdate < '1995-01-01'";

/// The `CASE` expression that splits the joined rows into high / low
/// priority.
const HIGH_PRIORITY: &str = "o_orderpriority = '1-URGENT' OR o_orderpriority = '2-HIGH'";

/// The complementary `CASE` expression of the canonical TPC-H Q12.
const LOW_PRIORITY: &str = "o_orderpriority <> '1-URGENT' AND o_orderpriority <> '2-HIGH'";

/// The same Q12 query, but with a **data-derived** high-priority predicate.
///
/// The canonical TPC-H literals (`'1-URGENT'` / `'2-HIGH'`) do not occur in
/// this repository's fixture — its `o_orderpriority` values are `O-PRIO-1`
/// … `O-PRIO-5` — so on this data the canonical query's "high" side is
/// always 0 and the `CASE` partition would only ever be exercised on its
/// empty half. This variant picks the two most frequent priorities *from the
/// data* so that both halves of the `CASE` are non-empty and the partition
/// invariant has real content.
fn q12_with_priorities(p1: &str, p2: &str) -> String {
    format!(
        "SELECT l_shipmode, \
                SUM(CASE WHEN o_orderpriority = '{p1}' OR o_orderpriority = '{p2}' \
                    THEN 1 ELSE 0 END) AS high_line_count, \
                SUM(CASE WHEN o_orderpriority <> '{p1}' AND o_orderpriority <> '{p2}' \
                    THEN 1 ELSE 0 END) AS low_line_count \
         FROM orders, lineitem WHERE l_orderkey = o_orderkey AND {Q12_FILTER} \
         GROUP BY l_shipmode ORDER BY l_shipmode"
    )
}

/// The two most frequent `o_orderpriority` values, derived from the data.
fn data_derived_priorities(engine: &mut ExecutionEngine<MemoryStorage>) -> (String, String) {
    let result = engine
        .execute("SELECT o_orderpriority, COUNT(*) FROM orders GROUP BY o_orderpriority")
        .expect("querying the order priorities");
    let mut ranked: Vec<(i64, String)> = result
        .rows
        .iter()
        .map(|r| {
            let count = r[1]
                .as_integer()
                .unwrap_or_else(|| panic!("the priority count is not an integer"));
            let name = match &r[0] {
                Value::Text(s) => s.clone(),
                v => v.to_sql_string(),
            };
            (count, name)
        })
        .collect();
    // Most frequent first; ties broken by name so the choice is deterministic.
    ranked.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    assert!(
        ranked.len() >= 2,
        "the fixture must contain at least two distinct order priorities, \
         otherwise the `CASE` partition cannot be exercised on both halves"
    );
    (ranked[0].1.clone(), ranked[1].1.clone())
}

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

/// The full Q12 query.
fn q12() -> String {
    format!(
        "SELECT l_shipmode, \
                SUM(CASE WHEN {HIGH_PRIORITY} THEN 1 ELSE 0 END) AS high_line_count, \
                SUM(CASE WHEN {LOW_PRIORITY} THEN 1 ELSE 0 END) AS low_line_count \
         FROM orders, lineitem WHERE l_orderkey = o_orderkey AND {Q12_FILTER} \
         GROUP BY l_shipmode ORDER BY l_shipmode"
    )
}

#[test]
fn diag_q12_case_when() {
    let state = FixtureState::resolve();
    let Some(fixture) = state.require(TARGET) else {
        // No fixture: the NOT-RUN marker was printed by `require`. This test
        // is reported as `not_run` by CI, never as `passed`.
        return;
    };

    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    engine
        .execute("CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus TEXT NOT NULL, o_totalprice REAL NOT NULL, o_orderdate TEXT NOT NULL, o_orderpriority TEXT, o_clerk TEXT, o_shippriority INTEGER, o_comment TEXT)")
        .unwrap();
    engine
        .execute("CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity REAL NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT, l_linestatus TEXT, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT, l_shipmode TEXT, l_comment TEXT, PRIMARY KEY (l_orderkey, l_linenumber))")
        .unwrap();

    for (table, ncols) in [("orders", 9), ("lineitem", 16)] {
        let loaded = tpch_fixture::load_into_storage(&storage, fixture, table, ncols)
            .unwrap_or_else(|e| panic!("loading table `{table}`: {e}"));
        assert!(loaded > 0, "table `{table}` loaded 0 rows");
    }

    // The independent oracle: same fixture, different engine.
    let oracle = SqliteOracle::open(fixture, &TABLES).expect("opening the SQLite oracle");

    // ---- Step 1: count the lineitem rows passing the date/shipmode filter
    let q1 = format!("SELECT COUNT(*) FROM lineitem WHERE {Q12_FILTER}");
    let filtered = count_with_oracle(&mut engine, &oracle, "diag_q12:step1_filtered", &q1);

    // ---- Step 2: the same filter with the join --------------------------
    let q2 = format!(
        "SELECT COUNT(*) FROM orders, lineitem \
         WHERE l_orderkey = o_orderkey AND {Q12_FILTER}"
    );
    let joined = count_with_oracle(&mut engine, &oracle, "diag_q12:step2_joined", &q2);

    // ---- Step 2b: the unfiltered join -----------------------------------
    // This is the reason step 1 and step 2 must agree: the join is 1:1 and
    // every lineitem row finds its order.
    let q2b = "SELECT COUNT(*) FROM orders, lineitem WHERE l_orderkey = o_orderkey";
    let unfiltered_join = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q12:step2b_unfiltered_join",
        &q2b,
    );
    let lineitem_rows = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q12:step2b_lineitem_total",
        "SELECT COUNT(*) FROM lineitem",
    );

    // ---- Step 3: a `CASE WHEN` aggregate over `orders` alone ------------
    let q3 = "SELECT SUM(CASE WHEN o_orderpriority = '1-URGENT' THEN 1 ELSE 0 END) FROM orders";
    let urgent_only = count_with_oracle(&mut engine, &oracle, "diag_q12:step3_sum_case", &q3);
    let orders_rows = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q12:step3_orders_total",
        "SELECT COUNT(*) FROM orders",
    );

    // ---- Step 3b: the high-priority count over `orders` alone -----------
    // This is a data fact about the whole table. It is *not* the value the
    // per-group counters must sum to: the Q12 groups only cover the
    // **joined and filtered** rows, which is a strict subset of `orders`.
    let q3b = format!("SELECT SUM(CASE WHEN {HIGH_PRIORITY} THEN 1 ELSE 0 END) FROM orders");
    let global_high = count_with_oracle(&mut engine, &oracle, "diag_q12:step3b_global_high", &q3b);

    // ---- Step 3c: the high-priority count over the *joined* rows --------
    // This is the value the per-group `high_line_count`s must sum to.
    let q3c = format!(
        "SELECT SUM(CASE WHEN {HIGH_PRIORITY} THEN 1 ELSE 0 END) \
         FROM orders, lineitem WHERE l_orderkey = o_orderkey AND {Q12_FILTER}"
    );
    let joined_high = count_with_oracle(&mut engine, &oracle, "diag_q12:step3c_joined_high", &q3c);

    // ---- Anti-vacuity: the Q12 filter must actually select rows ----------
    // Without this, every `CASE` invariant below would degenerate into
    // `0 == 0` and would pass even for a completely broken engine.
    assert!(
        filtered > 0,
        "the Q12 `WHERE` filter must select at least one lineitem row, \
         otherwise the `CASE` invariants below would be vacuously true"
    );
    assert!(
        joined > 0,
        "the Q12 `WHERE` filter must select at least one joined row, \
         otherwise the `CASE` invariants below would be vacuously true"
    );
    assert!(
        unfiltered_join > 0,
        "the orders ⋈ lineitem join must produce rows"
    );

    // ---- Step 4: the full Q12 --------------------------------------------
    let q4 = q12();
    let engine_q12 = engine.execute(&q4).expect("executing the full Q12 query");
    if let Some(expected) = oracle_rows(&oracle, "diag_q12:step4_full_q12", &q4) {
        sqlite_oracle::check_against_oracle(
            "diag_q12:step4_full_q12",
            &sqlite_oracle::engine_rows_to_strings(&engine_q12.rows),
            &expected,
        );
    }

    // ---- Step 5: the joined row count per shipmode ----------------------
    let q5 = format!(
        "SELECT l_shipmode, COUNT(*) FROM orders, lineitem \
         WHERE l_orderkey = o_orderkey AND {Q12_FILTER} GROUP BY l_shipmode"
    );
    let engine_per_mode = engine
        .execute(&q5)
        .expect("executing the per-shipmode count");
    if let Some(expected) = oracle_rows(&oracle, "diag_q12:step5_per_shipmode", &q5) {
        sqlite_oracle::check_against_oracle(
            "diag_q12:step5_per_shipmode",
            &sqlite_oracle::engine_rows_to_strings(&engine_per_mode.rows),
            &expected,
        );
    }

    // ---- Step 6: how many distinct shipmodes the join produces ----------
    let q6 = format!(
        "SELECT COUNT(DISTINCT l_shipmode) FROM orders, lineitem \
         WHERE l_orderkey = o_orderkey AND {Q12_FILTER}"
    );
    let distinct_modes = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q12:step6_distinct_shipmodes",
        &q6,
    );

    // ---- Anti-vacuity: the Q12 GROUP BY must produce groups --------------
    assert!(
        !engine_q12.rows.is_empty(),
        "the Q12 GROUP BY must produce at least one `l_shipmode` group, \
         otherwise the per-group `CASE` invariants below would be vacuously true"
    );
    assert!(
        !engine_per_mode.rows.is_empty(),
        "the per-shipmode count must produce at least one group"
    );
    assert!(
        distinct_modes > 0,
        "the Q12 filter must reach at least one distinct `l_shipmode`"
    );

    // ---- Invariant 1: the join is 1:1, so it removes no lineitem row ---
    assert_eq!(
        unfiltered_join, lineitem_rows,
        "every lineitem row must find exactly one order row"
    );
    assert_eq!(
        joined, filtered,
        "the 1:1 join to orders must not change the filtered lineitem count \
         (before={filtered}, after={joined})"
    );
    assert!(
        joined <= lineitem_rows,
        "the joined count can never exceed the number of lineitem rows"
    );

    // ---- Invariant 2: the CASE expression partitions the joined rows ----
    let mut sum_high = 0i64;
    let mut sum_low = 0i64;
    for (i, row) in engine_q12.rows.iter().enumerate() {
        assert_eq!(
            row.len(),
            3,
            "row {i} must have exactly 3 cells (l_shipmode, high_line_count, low_line_count)"
        );
        let high = as_f64(&row[1]).unwrap_or_else(|| {
            panic!("row {i}: high_line_count is not numeric: {:?}", row[1]);
        });
        let low = as_f64(&row[2]).unwrap_or_else(|| {
            panic!("row {i}: low_line_count is not numeric: {:?}", row[2]);
        });
        assert!(
            high >= 0.0 && low >= 0.0,
            "row {i}: the CASE counters must not be negative (high={high}, low={low})"
        );
        sum_high += high as i64;
        sum_low += low as i64;

        // The group's counters must add up to the number of joined rows of
        // that shipmode.
        let mode = match &row[0] {
            Value::Text(s) => s.clone(),
            v => v.to_sql_string(),
        };
        let group_rows = engine_per_mode
            .rows
            .iter()
            .find(|r| match &r[0] {
                Value::Text(s) => s == &mode,
                v => &v.to_sql_string() == &mode,
            })
            .unwrap_or_else(|| {
                panic!(
                    "Q12 produced the shipmode {mode:?} which the per-shipmode count does not have"
                )
            });
        let group_count = group_rows[1]
            .as_integer()
            .unwrap_or_else(|| panic!("the per-shipmode count is not an integer"));
        assert_eq!(
            (high + low) as i64,
            group_count,
            "the CASE counters of the shipmode {mode:?} must add up to the \
             number of joined rows of that shipmode"
        );
    }

    // ---- Invariant 3: the per-group counters sum to the global counts ---
    assert_eq!(
        sum_high + sum_low,
        joined,
        "the per-shipmode CASE counters must add up to the number of joined rows"
    );
    assert_eq!(
        sum_high, joined_high,
        "the per-shipmode high_line_counts must add up to the number of \
         *joined* high-priority rows"
    );
    assert_eq!(
        sum_low,
        joined - joined_high,
        "the per-shipmode low_line_counts must add up to the number of joined \
         rows that are not high-priority"
    );
    assert!(
        joined_high <= global_high,
        "the joined high-priority rows cannot outnumber the high-priority \
         orders of the whole table ({joined_high} > {global_high})"
    );

    // ---- Invariant 4: the GROUP BY has one row per distinct shipmode ----
    assert_eq!(
        engine_q12.rows.len() as i64,
        distinct_modes,
        "the GROUP BY must produce exactly one row per distinct l_shipmode of \
         the joined rows"
    );
    assert_eq!(
        engine_q12.columns.len(),
        3,
        "the Q12 result must declare exactly 3 columns"
    );

    // ---- Invariant 5: the `CASE` aggregate over `orders` alone -----------
    assert!(
        urgent_only <= orders_rows,
        "the number of urgent orders can never exceed the number of orders \
         ({urgent_only} > {orders_rows})"
    );
    assert!(
        global_high <= orders_rows,
        "the number of high-priority orders can never exceed the number of \
         orders ({global_high} > {orders_rows})"
    );
    assert!(
        global_high >= urgent_only,
        "the high-priority set contains the urgent-only set, so its count \
         cannot be smaller"
    );

    // ---- The same Q12, with a data-derived high-priority predicate ------
    // The canonical literals do not occur in this fixture, so the canonical
    // query's "high" side is always 0. This variant uses the two most
    // frequent priorities *from the data*, so both halves of the `CASE`
    // are non-empty and the partition invariant has real content.
    let (p1, p2) = data_derived_priorities(&mut engine);
    eprintln!("diag_q12: data-derived high priorities = {p1}, {p2}");

    let q_native = q12_with_priorities(&p1, &p2);
    let engine_native = engine
        .execute(&q_native)
        .expect("executing the data-derived Q12 query");
    if let Some(expected) = oracle_rows(&oracle, "diag_q12:step7_data_derived_q12", &q_native) {
        sqlite_oracle::check_against_oracle(
            "diag_q12:step7_data_derived_q12",
            &sqlite_oracle::engine_rows_to_strings(&engine_native.rows),
            &expected,
        );
    }

    let q_native_global = format!(
        "SELECT SUM(CASE WHEN o_orderpriority = '{p1}' OR o_orderpriority = '{p2}' \
                THEN 1 ELSE 0 END) FROM orders, lineitem \
         WHERE l_orderkey = o_orderkey AND {Q12_FILTER}"
    );
    let native_global_high = count_with_oracle(
        &mut engine,
        &oracle,
        "diag_q12:step7_data_derived_joined_high",
        &q_native_global,
    );

    // ---- Invariant 6: the data-derived `CASE` partitions the rows -------
    let mut native_high = 0i64;
    let mut native_low = 0i64;
    for (i, row) in engine_native.rows.iter().enumerate() {
        assert_eq!(
            row.len(),
            3,
            "row {i} of the data-derived Q12 must have exactly 3 cells"
        );
        let high = as_f64(&row[1]).unwrap_or_else(|| {
            panic!("row {i}: high_line_count is not numeric: {:?}", row[1]);
        });
        let low = as_f64(&row[2]).unwrap_or_else(|| {
            panic!("row {i}: low_line_count is not numeric: {:?}", row[2]);
        });
        native_high += high as i64;
        native_low += low as i64;
    }
    assert_eq!(
        native_high + native_low,
        joined,
        "the data-derived `CASE` counters must add up to the number of joined rows"
    );
    assert_eq!(
        native_high, native_global_high,
        "the per-shipmode high_line_counts must add up to the global count of \
         high-priority orders"
    );
    assert!(
        native_high > 0,
        "the data-derived high-priority predicate must select orders, \
         otherwise the `CASE` partition would only be exercised on its empty half"
    );
    assert!(
        native_low > 0,
        "the data-derived low-priority side must select orders, \
         otherwise the `CASE` partition would only be exercised on its empty half"
    );
}
