//! TPC-H Q15 + Q16 isolated check (sqlrustgo in-process vs in-process SQLite).
//!
//! Q15: subquery in FROM (view). Q16: NOT IN subquery.
//!
//! #5404: this file used to have **zero assertions**. Both tests only
//! `println!`ed their result, and the error branches only printed
//! `"✗ Q15 failed to execute"` — so the engine could return an error, zero
//! rows, or entirely wrong data and the target still reported success.
//!
//! The comparison against PostgreSQL that the header comment promised was
//! also never performed: the `Err` branch of a PG call is indistinguishable
//! from "PG is not installed" in the old code path, and the result was only
//! printed.
//!
//! ## What it does now
//!
//! The truth source is an **independent in-process SQLite** (bundled
//! `rusqlite`, so no external binary and no network), loaded from the same
//! fixture as the engine. The engine's rows and the oracle's rows are
//! compared **cell by cell**, and a statement that fails is a **failure**,
//! not a printout.
//!
//! The statements themselves still come from the repository's authoritative
//! `queries/q15.sql` / `queries/q16.sql`, located via `CARGO_MANIFEST_DIR`
//! rather than the current working directory.

#[path = "../../common/tpch_fixture.rs"]
mod tpch_fixture;

#[path = "../../common/tpch_oracle.rs"]
mod tpch_oracle;

use std::path::PathBuf;

/// Cargo target name — used in the NOT-RUN marker.
const TARGET: &str = "_q15_q16_check";

/// Read one of the repository's canonical TPC-H statements.
///
/// `CARGO_MANIFEST_DIR` is used instead of a relative path: `cargo test`
/// happens to run with the package root as the working directory, but a
/// test that depends on the caller's working directory is one more
/// machine-specific assumption.
fn canonical_query(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("queries/{name}.sql"));
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    raw.trim().trim_end_matches(';').to_string()
}

/// Load the given tables and run `sql` on the engine and on the oracle.
///
/// Returns the engine's rows together with the number of rows loaded per
/// table, so the caller can state an upper bound that is derived from the
/// data rather than from a hard-coded number.
///
/// `None` means **the target did not run** — the TPC-H fixture is not
/// available. `require` has already written the NOT-RUN record in that
/// case, and the caller must return without asserting anything: the
/// fixture being absent is an *environment* fact, not a product failure.
///
/// #5515: this used to return `(vec![], vec![])` for the missing-fixture
/// case, and the callers then asserted "the query must return at least one
/// row" — so a missing fixture turned into a **red test** even though the
/// NOT-RUN disclosure had already been written. The disclosure and the
/// verdict contradicted each other: the target was reported as both
/// "did not run" and "failed". Every other fixture-gated target in the
/// tree (20+ of them, e.g. `tpch_value_correctness_test.rs`) uses
/// `else { return; }` — this file was the only one that didn't.
fn compare_with_oracle(
    tables: &[&str],
    sql: &str,
    label: &str,
) -> Option<(Vec<Vec<String>>, Vec<(String, usize)>)> {
    let state = tpch_fixture::FixtureState::resolve();
    let Some(fixture) = state.require(TARGET) else {
        // `require` has already written the NOT-RUN record: the target did
        // not run, and must not be counted as passed.
        return None;
    };

    let storage = std::sync::Arc::new(parking_lot::RwLock::new(sqlrustgo::MemoryStorage::new()));
    let mut engine = sqlrustgo::ExecutionEngine::new(storage.clone());
    for ddl in tpch_oracle::engine_ddl(tables) {
        engine
            .execute(&ddl)
            .unwrap_or_else(|e| panic!("{label}: creating the table failed: {e}"));
    }
    let mut loaded = Vec::new();
    for table in tables {
        let path = fixture.table_path(table);
        assert!(
            path.exists(),
            "{label}: the fixture is present but table `{table}` is missing from it \
             (looked at {})",
            path.display()
        );
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{label}: reading {table} failed: {e}"));
        let rows = tpch_oracle::parse_tbl(&content);
        let ncols = tpch_oracle::table_spec(table)
            .map(|(_, cols, _)| cols)
            .unwrap_or(0);
        let n = tpch_oracle::load_rows_into_storage(&storage, &rows, table, ncols)
            .unwrap_or_else(|e| panic!("{label}: loading `{table}` failed: {e}"));
        loaded.push((table.to_string(), n));
    }

    let conn = tpch_oracle::load_into_sqlite(&fixture.dir, tables)
        .unwrap_or_else(|e| panic!("{label}: loading the SQLite oracle failed: {e}"));

    let ours = tpch_oracle::engine_rows(&mut engine, sql)
        .unwrap_or_else(|e| panic!("{label}: the statement must execute — {e}"));
    let theirs = tpch_oracle::sqlite_rows(&conn, sql)
        .unwrap_or_else(|e| panic!("{label}: the SQLite oracle must execute the statement — {e}"));

    tpch_oracle::assert_rows_match(label, &ours, &theirs);
    Some((ours, loaded))
}

#[test]
fn test_q15_subquery_from() {
    let sql = canonical_query("q15");

    let Some((rows, loaded)) = compare_with_oracle(&["supplier", "lineitem"], &sql, "Q15") else {
        // The fixture is not available: `require` has already written the
        // NOT-RUN record. Nothing to assert — the target did not run.
        return;
    };

    assert!(
        !rows.is_empty(),
        "Q15 must return at least one row: the fixture has lineitem rows in \
         the Q15 date window and suppliers to join them to"
    );

    // Data-independent invariant: Q15 groups by `l_suppkey` and then joins
    // the result to `supplier` on `s_suppkey`, so every output row
    // corresponds to exactly one supplier and the result can never have
    // more rows than there are suppliers.
    let suppliers = loaded
        .iter()
        .find(|(t, _)| t == "supplier")
        .map(|(_, n)| *n)
        .unwrap_or(0);
    assert!(
        rows.len() <= suppliers,
        "Q15 returned {} rows but the fixture only has {suppliers} suppliers",
        rows.len()
    );
}

#[test]
fn test_q16_not_in_subquery() {
    let sql = canonical_query("q16");

    let Some((rows, loaded)) = compare_with_oracle(&["partsupp", "part", "supplier"], &sql, "Q16")
    else {
        // The fixture is not available: `require` has already written the
        // NOT-RUN record. Nothing to assert — the target did not run.
        return;
    };

    assert!(
        !rows.is_empty(),
        "Q16 must return at least one row: the fixture has partsupp rows for \
         parts whose brand/type/size pass the filter"
    );

    // Data-independent invariant: Q16 groups by (p_brand, p_type, p_size),
    // so the result can never have more rows than there are partsupp rows.
    let partsupp = loaded
        .iter()
        .find(|(t, _)| t == "partsupp")
        .map(|(_, n)| *n)
        .unwrap_or(0);
    assert!(
        rows.len() <= partsupp,
        "Q16 returned {} rows but the fixture only has {partsupp} partsupp rows",
        rows.len()
    );
}
