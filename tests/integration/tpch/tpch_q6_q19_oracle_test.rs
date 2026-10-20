//! TPC-H Q6/Q19 cell-level oracle — issue #3288.
//!
//! Sprint 1.5 cell-diff oracle reported Q6/Q19 as
//! row_count_mismatch (sqlrustgo=1, PG=0). The investigation
//! (docs/audit/status/2026-06-07-q6-q19-investigation-3288.md)
//! showed that BOTH versions agree on row_count=1 against the
//! simplified SF=0.001 fixture that sqlrustgo + SQLite + MariaDB
//! share. The PG "0 rows" is a fixture schema artifact (PG uses a
//! different lineitem shape on the simplified data).
//!
//! This test exercises Q6 + Q19 in-process and asserts the values that
//! Sprint 5 oracle (#3284) should treat as the authoritative ground truth.
//!
//! # What changed (issue #5379)
//!
//! Three defects in this file were found while the TPC-H fixture was made
//! reachable at run time. All three are fixed here; the file's *assertions*
//! are not weakened — they are re-based on an independent oracle.
//!
//! 1. **The loader never loaded anything.** Every row was inserted with
//!    `INSERT INTO lineitem (c0, c1, ..., c15) VALUES (...)` — positional
//!    names that the table does not have. The engine does **not** reject an
//!    unknown column name in an INSERT column list: it inserts a row of
//!    NULLs instead (verified with the built binary:
//!    `INSERT INTO t (c0, c1) VALUES (1, 2)` on a table `(a, b)` produces the
//!    row `NULL | NULL`). The tables therefore held ~62 000 rows of NULLs,
//!    every filter matched nothing, and the assertions below passed on
//!    garbage.
//!
//! 2. **The expected values were hard-coded and profile-bound.** `Q19` was
//!    expected to count `0` rows — true only for the hand-built "simplified
//!    SF=0.001" fixture that this repository has never contained. On the
//!    generated fixture the same query matches a different number of rows,
//!    so a hard-coded number is not an expectation, it is a leftover.
//!    The expectations now come from the **independent in-process SQLite
//!    oracle** (`tests/common/sqlite_oracle.rs`), loaded from the same
//!    fixture — which is what this file's own header always promised
//!    ("oracle just needs to use SQLite as the truth source for these two
//!    queries").
//!
//! 3. **The load cost the CI `test` job its budget.** One INSERT statement
//!    per row, three times over, is ~186 000 statement executions per run.
//!    The rows are now loaded through the storage layer in 10 000-row
//!    batches.
//!
//! The data-independent invariants stay: an aggregate without a `GROUP BY`
//! produces exactly one row, and the cell is a finite number.
//!
//! Z440-runnable: no PG, no MySQL. Uses the in-process engine plus the
//! in-process SQLite oracle.
//!
//! Refs: issue #3288, issue #5379, SPRINT4_MASTER_PLAN.md §1 root cause D,
//! docs/audit/status/2026-06-07-q6-q19-investigation-3288.md

#[path = "../../common/sqlite_oracle.rs"]
mod sqlite_oracle;
#[path = "../../common/tpch_fixture.rs"]
mod tpch_fixture;

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value as SqlValue};
use std::sync::Arc;

use sqlite_oracle::SqliteOracle;
use tpch_fixture::FixtureState;

/// Cargo target name — used in the NOT-RUN marker.
const TARGET: &str = "tpch_q6_q19_oracle_test";

/// Tables this test loads, with their TPC-H column counts.
///
/// The `part` table has **nine** columns. The previous DDL in this file had
/// only seven (it dropped `p_retailprice` and moved `p_comment` into its
/// place), which did not match the TPC-H schema the fixture is written in —
/// nor the schema the SQLite oracle uses.
const TABLES: [(&str, usize); 2] = [("lineitem", 16), ("part", 9)];

const SCHEMA_SQL: &[&str] = &[
    "CREATE TABLE lineitem (\
        l_orderkey INTEGER, l_partkey INTEGER, l_suppkey INTEGER, l_linenumber INTEGER, \
        l_quantity REAL, l_extendedprice REAL, l_discount REAL, l_tax REAL, \
        l_returnflag TEXT, l_linestatus TEXT, l_shipdate TEXT, l_commitdate TEXT, \
        l_receiptdate TEXT, l_shipinstruct TEXT, l_shipmode TEXT, l_comment TEXT\
    )",
    "CREATE TABLE part (\
        p_partkey INTEGER, p_name TEXT, p_mfgr TEXT, p_brand TEXT, \
        p_type TEXT, p_size INTEGER, p_container TEXT, p_retailprice REAL, \
        p_comment TEXT\
    )",
];

const Q6_CANONICAL: &str = "SELECT SUM(l_extendedprice * l_discount) AS revenue \
    FROM lineitem \
    WHERE l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01' \
      AND l_discount BETWEEN 0.06 AND 0.08 \
      AND l_quantity < 25";

const Q19_CANONICAL: &str = "SELECT COUNT(*) AS cnt \
    FROM lineitem, part \
    WHERE p_partkey = l_partkey \
      AND p_brand = 'Brand#12' \
      AND p_container IN ('SM CASE', 'SM BOX', 'SM PACK', 'SM PKG') \
      AND l_quantity >= 1 AND l_quantity <= 11 \
      AND p_size BETWEEN 1 AND 5 \
      AND l_shipmode IN ('AIR', 'AIR REG') \
      AND l_shipinstruct = 'DELIVER IN PERSON'";

/// The engine under test plus the independent oracle, both loaded from the
/// same fixture.
struct Ctx {
    engine: ExecutionEngine<MemoryStorage>,
    oracle: SqliteOracle,
}

/// Build the engine and the independent oracle from the resolved fixture.
///
/// Returns `None` when the fixture is missing — the shared resolver has
/// already printed the NOT-RUN marker in that case (issue #5379: a missing
/// fixture is an explicit, disclosed state, never a panic and never a silent
/// pass).
fn setup() -> Option<Ctx> {
    let state = FixtureState::resolve();
    let fixture = state.require(TARGET)?.clone();

    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    for ddl in SCHEMA_SQL {
        engine
            .execute(ddl)
            .unwrap_or_else(|e| panic!("DDL failed: {ddl} - {e}"));
    }
    for (table, ncols) in TABLES {
        let loaded = tpch_fixture::load_into_storage(&storage, &fixture, table, ncols)
            .unwrap_or_else(|e| panic!("loading table `{table}`: {e}"));
        assert!(loaded > 0, "table `{table}` loaded 0 rows");
    }

    // The independent oracle: same fixture, different engine.
    let oracle =
        SqliteOracle::open(&fixture, &["lineitem", "part"]).expect("opening the SQLite oracle");

    Some(Ctx { engine, oracle })
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

/// Execute `sql` on the engine, compare the result with the independent
/// oracle, and return the engine's rows.
fn run_and_compare(ctx: &mut Ctx, case_id: &str, sql: &str) -> Vec<Vec<SqlValue>> {
    let result = ctx
        .engine
        .execute(sql)
        .unwrap_or_else(|e| panic!("{case_id}: {e}"));
    if let Some(expected) = oracle_rows(&ctx.oracle, case_id, sql) {
        sqlite_oracle::check_against_oracle(
            case_id,
            &sqlite_oracle::engine_rows_to_strings(&result.rows),
            &expected,
        );
    }
    result.rows
}

#[test]
fn test_tpch_q6_cell_value_against_sqlite_ground_truth() {
    let Some(mut ctx) = setup() else {
        return;
    };

    let rows = run_and_compare(&mut ctx, "tpch:Q6", Q6_CANONICAL);
    assert_eq!(
        rows.len(),
        1,
        "Q6 should return exactly 1 row (per SQLite ground truth), got {}",
        rows.len()
    );
    // The expected *value* comes from the independent oracle (checked above).
    // What is asserted here is the data-independent shape: the cell is a
    // finite number.
    let v = &rows[0][0];
    let s = format!("{:?}", v);
    eprintln!("Q6 returned: {}", s);
    let parsed: Result<f64, _> = s.trim_start_matches("Float(").trim_end_matches(')').parse();
    assert!(
        parsed.is_ok(),
        "Q6 first cell should be parseable as f64, got: {}",
        s
    );
    let val = parsed.unwrap();
    assert!(val.is_finite(), "Q6 revenue should be finite, got {}", val);
    eprintln!("Q6 cell value = {}", val);
}

#[test]
fn test_tpch_q19_cell_value_against_sqlite_ground_truth() {
    let Some(mut ctx) = setup() else {
        return;
    };

    let rows = run_and_compare(&mut ctx, "tpch:Q19", Q19_CANONICAL);
    assert_eq!(
        rows.len(),
        1,
        "Q19 should return exactly 1 row (per SQLite ground truth), got {}",
        rows.len()
    );
    // The expected count comes from the independent oracle. What is asserted
    // here is the data-independent shape: the cell is an integer.
    let v = &rows[0][0];
    let s = format!("{:?}", v);
    eprintln!("Q19 returned: {}", s);
    let parsed: Result<i64, _> = s
        .trim_start_matches("Integer(")
        .trim_end_matches(')')
        .parse();
    assert!(
        parsed.is_ok(),
        "Q19 first cell should be parseable as i64, got: {}",
        s
    );
}

#[test]
fn test_tpch_q6_v2_discount_05_07_quantity_24() {
    // Per tpch_value_test_v2.rs line 191 (the version used in Sprint 1.5
    // cell-diff oracle). Different discount/quantity range from canonical.
    // This is a SECOND oracle value, kept for backwards compat with the
    // cell-diff JSON snapshot.
    const Q6_V2: &str = "SELECT SUM(l_extendedprice) AS revenue \
        FROM lineitem \
        WHERE l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01' \
          AND l_discount BETWEEN 0.05 AND 0.07 \
          AND l_quantity < 24";

    let Some(mut ctx) = setup() else {
        return;
    };

    let rows = run_and_compare(&mut ctx, "tpch:Q6-v2", Q6_V2);
    assert_eq!(rows.len(), 1, "Q6 v2 should return 1 row");
    eprintln!("Q6 v2 returned: {:?}", rows[0][0]);
}
