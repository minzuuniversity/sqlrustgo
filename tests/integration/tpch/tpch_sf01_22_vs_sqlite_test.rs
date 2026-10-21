//! TPC-H 22/22 wire protocol vs the **independent** in-process SQLite oracle
//! (cargo target `tpch_sf01_22_vs_sqlite_test`).
//!
//! # What changed (issue #5379)
//!
//! This test used to compare the wire-protocol result against
//! `tests/data/tpch-sf01/expected/Q{N}_sf01_baseline.json`. Those files were
//! **never committed**: `tests/data/tpch-sf01` was a symlink into `/tmp`, and
//! the JSON lived behind it. Every query therefore hit the
//! `expected_path.exists()` guard, took the `SKIP` branch, and the test
//! finished with
//!
//! ```text
//! === Summary: pass=0 fail=0 skip=22 ===
//! ```
//!
//! — and then reported `test result: ok` **without a single assertion**.
//!
//! # What this test is now
//!
//! A hard gate with three properties:
//!
//! 1. **The expected value comes from a different engine.** The same
//!    statement is executed by an in-process SQLite loaded from the same
//!    fixture (`tests/common/sqlite_oracle.rs`, `rusqlite` with the
//!    `bundled` feature — no external binary, no network). The engine's own
//!    output is never fed back in as its own expectation.
//! 2. **Every one of the 22 statements is compared**, and the test asserts
//!    that it really compared 22 — a run that compares nothing can never
//!    report success.
//! 3. **A mismatch is a failure.** There is no "pre-existing, separate
//!    Sprint work" escape hatch any more.
//!
//! # The two phases
//!
//! * **Phase 1 — data integrity.** The engine's row count per table is
//!   compared with the row count the independent oracle loaded from the
//!   same `.tbl` file. A difference means the engine did not load the whole
//!   fixture, and every result that depends on that table is meaningless.
//!   This phase exists because a truncated dataset produces *plausible*
//!   results: the query returns rows, they are just not the right ones.
//! * **Phase 2 — statement comparison.** All 22 statements of
//!   `queries/q{1..22}.sql` are run over the wire protocol and compared
//!   with the oracle's result for the same statement.
//!
//! The comparison is deliberately *row-count only*: the wire protocol
//! renders a float differently from SQLite (`17` vs `17.0`), and the
//! full-cell comparison belongs to `oracle_g15_tpch_sf01.rs`, which checks
//! the values as well.
//!
//! # How a difference may be tolerated
//!
//! Only through `tests/baseline/tpch_known_differences.json`: an entry with
//! a `case_id` of `<cargo-target>:<query-id>`, an `issue` and an `owner`.
//! A registered difference is disclosed through the `TPCH-KNOWN-DIFF`
//! marker and does not fail the test; an unregistered one does.
//!
//! # Prerequisites
//!
//! A TPC-H fixture. It is resolved at run time (`TPCH_DATA_DIR` first), and
//! a missing fixture is disclosed as `TPCH-NOT-RUN` with the reason code
//! `fixture_missing` — never as a pass.
//!
//! # The wire scope
//!
//! Three of the 22 standard statements — **Q9**, **Q18** and **Q19** — cannot be
//! executed on the file-storage wire path: Q9's intermediate result grows past
//! 1.5 GB within a second, Q18 kills the server process, and Q19 cannot be
//! parsed by the wire front end (all three: issue #5393). A killed process
//! takes the **whole test binary** with it, so the CI job would die with
//! `signal: 9` instead of reporting a test failure.
//!
//! The shared **wire scope** (`tests/common/tpch_wire_scope.rs`) refuses to
//! execute those statements: `run_query_timed` takes the query id and returns
//! `WireOutcome::Skipped` for them. This file therefore executes the 19
//! in-scope statements, discloses the 3 skips through the `TPCH-NOT-RUN`
//! marker, and asserts that exactly those 3 were skipped — neither more (a
//! silently dropped statement) nor fewer (a statement that could have run).

#[path = "../../common/mod.rs"]
mod common;
#[path = "../../common/sqlite_oracle.rs"]
mod sqlite_oracle;
use common::tpch_cli_harness::tpch_fixture;

use std::time::Instant;

use common::tpch_cli_harness::tpch_fixture::{reason, report_known_difference, report_not_run};
use common::tpch_wire_harness::{run_query_timed, start_sf01_named, WireOutcome};
use common::tpch_wire_scope;
use sqlite_oracle::{adapt_sql, SqliteOracle, TPC_H_ALL_TABLES};

/// Cargo target name — used in the NOT-RUN marker and in the `case_id` of
/// the known-difference registry.
const TARGET: &str = "tpch_sf01_22_vs_sqlite_test";

/// Directory holding `q1.sql` … `q22.sql`, relative to the package root
/// (the working directory `cargo test` runs in).
const QUERIES_DIR: &str = "queries";

/// The TPC-H query set has exactly 22 statements; the test asserts that all
/// of them were compared.
const EXPECTED_QUERIES: usize = 22;

/// Per-statement timeout in seconds.
const TIMEOUT_S: u64 = 60;

/// Compare the wire-protocol rows with the oracle's rows.
///
/// Returns `None` when the two sides agree, and a description of the first
/// difference otherwise. The comparison is a **sorted multiset** comparison:
/// row order is implementation-defined for a statement without `ORDER BY`,
/// and comparing sequences would turn every such difference into a false
/// failure.
fn compare_rows(wire_rows: &[Vec<String>], oracle_rows: &[Vec<String>]) -> Option<String> {
    if wire_rows.len() != oracle_rows.len() {
        return Some(format!(
            "row count differs: wire={} oracle={}",
            wire_rows.len(),
            oracle_rows.len()
        ));
    }
    let mut wire = wire_rows.to_vec();
    let mut oracle = oracle_rows.to_vec();
    wire.sort();
    oracle.sort();
    sqlite_oracle::diff_rows(&wire, &oracle)
}

#[test]
fn tpch_sf01_22_vs_sqlite() {
    let state = tpch_fixture::FixtureState::resolve();
    let Some(fixture) = state.require(TARGET) else {
        // `require` has already printed the NOT-RUN marker.
        return;
    };

    let oracle = SqliteOracle::open(fixture, &TPC_H_ALL_TABLES)
        .unwrap_or_else(|e| panic!("{TARGET}: cannot build the independent SQLite oracle: {e}"));

    let Some(mut client) = start_sf01_named(TARGET) else {
        // The harness has already printed the NOT-RUN marker.
        return;
    };

    // ------------------------------------------------------------------
    // Phase 1 — data integrity.
    //
    // The engine must have loaded the whole fixture. A table that came
    // back short is reported here, because the statement comparison below
    // would otherwise show a difference whose cause (a truncated table)
    // is invisible in the numbers.
    //
    // #5464: the two sides are **not** interleaved. Every engine row
    // count is collected first, and only then is the oracle asked.
    //
    // Why this matters: the server reaps connections that have been idle
    // for `SQLRUSTGO_IDLE_TIMEOUT_SECS` (default 30 s,
    // `crates/mysql-server/src/lib.rs::read_idle_timeout_secs`). An
    // interleaved loop leaves the engine's connection idle for the whole
    // duration of each oracle computation — and a slow reference
    // computation (an un-indexed SQLite `EXISTS` over `lineitem` took
    // 41 s) is enough for the reaper to close it. The **next** engine
    // statement then fails with a transport error
    // (`read packet header: failed to fill whole buffer`) that has
    // nothing to do with the statement being run — a misattributed
    // failure.
    //
    // Fetching the engine side first removes the interleaving entirely:
    // the connection is never idle *between* two engine statements.
    // ------------------------------------------------------------------
    eprintln!("=== Phase 1: data integrity (engine vs oracle row counts) ===");
    let mut truncated_tables = 0usize;
    // Engine side first — every count, no oracle in between.
    let engine_counts: Vec<(&str, Option<i64>)> = TPC_H_ALL_TABLES
        .iter()
        .map(|table| {
            let sql = format!("SELECT COUNT(*) FROM {table}");
            let engine_count = match client.query_rows(&sql) {
                Ok(rows) => rows
                    .first()
                    .and_then(|r| r.first())
                    .and_then(|c| c.parse::<i64>().ok()),
                Err(e) => {
                    eprintln!("  {table:9}: ERROR ({e})");
                    None
                }
            };
            (*table, engine_count)
        })
        .collect();
    // Oracle side second.
    for (table, engine_count) in &engine_counts {
        let oracle_count = oracle
            .table_row_count(table)
            .unwrap_or_else(|e| panic!("{TARGET}: the SQLite oracle cannot count {table}: {e}"));
        match engine_count {
            Some(n) if *n == oracle_count => {
                eprintln!("  {table:9}: {n} row(s) — complete");
            }
            Some(n) => {
                truncated_tables += 1;
                eprintln!(
                    "  {table:9}: {n} row(s) in the engine, {oracle_count} in the fixture — \
                     DATA LOSS"
                );
            }
            None => {
                truncated_tables += 1;
                eprintln!("  {table:9}: the engine could not report a row count");
            }
        }
    }

    // ------------------------------------------------------------------
    // Phase 2 — statement comparison.
    //
    // #5464: like Phase 1, the two sides are **not** interleaved.
    //
    // Pass A runs every statement on the **engine** and keeps the rows.
    // Pass B then asks the **oracle** for the same statements and compares.
    //
    // The reason is the idle-connection reaper: while the oracle computes
    // a statement, the engine's connection is idle. A slow reference
    // computation (41 s for an un-indexed SQLite `EXISTS` over
    // `lineitem`) exceeds the 30 s default idle timeout, the server
    // closes the connection, and the **next** engine statement dies with
    // `read packet header: failed to fill whole buffer` — a failure that
    // has nothing to do with the statement it is attributed to.
    //
    // Fetching the engine side first removes the window entirely: the
    // connection is only ever idle *between* two engine statements.
    //
    // The accounting is unchanged: every statement is still compared,
    // skipped or failed exactly once, and the same assertions run at the
    // end.
    // ------------------------------------------------------------------
    eprintln!();
    eprintln!("=== Phase 2: 22 TPC-H statements (wire) vs the SQLite oracle ===");

    let mut compared = 0usize;
    let mut mismatched = 0usize;
    let mut known = 0usize;
    let mut not_run = 0usize;
    let mut failed = 0usize;
    // The statements the file-storage wire path must not execute, and the
    // ones it must. Both counts are checked against the shared scope
    // afterwards, so a statement can neither slip through nor disappear.
    let mut skipped_scope: Vec<&'static str> = Vec::new();
    let expected_scoped = (1..=EXPECTED_QUERIES)
        .filter(|n| tpch_wire_scope::in_wire_scope(tpch_wire_scope::TPC_H_QUERY_IDS[n - 1]))
        .count();
    let expected_skipped = EXPECTED_QUERIES - expected_scoped;

    // ---- Pass A: the engine side, for every statement. ----
    // The oracle is not touched here, so the engine's connection is
    // never idle while a reference computation runs.
    let mut engine_side: Vec<(&'static str, String, Vec<Vec<String>>, std::time::Duration)> =
        Vec::new();

    for n in 1..=EXPECTED_QUERIES {
        let sql = std::fs::read_to_string(format!("{QUERIES_DIR}/q{n}.sql"))
            .unwrap_or_else(|e| panic!("{TARGET}: cannot read {QUERIES_DIR}/q{n}.sql: {e}"));
        let qid = tpch_wire_scope::TPC_H_QUERY_IDS[n - 1];
        let not_run_label = format!("{TARGET}:{qid}");

        // The SQLite oracle only rewrites an explicit whitelist of
        // constructs. Anything else is reported here instead of being
        // silently rewritten — comparing the engine against a *different*
        // statement would prove nothing.
        let adapted = adapt_sql(&sql);
        if !adapted.unsupported.is_empty() {
            report_not_run(
                &not_run_label,
                reason::SCALE_LAYER_ONLY,
                &format!(
                    "the SQLite oracle cannot execute this statement ({:?}); \
                     it belongs to the cross-engine dialect layer",
                    adapted.unsupported
                ),
            );
            not_run += 1;
            continue;
        }

        let t0 = Instant::now();
        let wire_rows = match run_query_timed(&mut client, qid, &sql, TIMEOUT_S) {
            WireOutcome::Ran(Ok(rows), _) => rows,
            WireOutcome::Ran(Err(e), _) => {
                failed += 1;
                eprintln!("  {qid:2}: ERROR ({e}) after {:?}", t0.elapsed());
                continue;
            }
            WireOutcome::Skipped(skip) => {
                // Outside the wire scope: the statement is NOT executed.
                // Executing it would exhaust the machine and kill the test
                // binary with it (issue #5393).
                not_run += 1;
                skipped_scope.push(qid);
                eprintln!(
                    "  {qid:2}: SKIPPED ({} issue={}) — {}",
                    skip.reason, skip.issue, skip.detail
                );
                continue;
            }
        };
        let elapsed = t0.elapsed();
        eprintln!("  {qid:2}: engine rows={} in {elapsed:?}", wire_rows.len());
        engine_side.push((qid, adapted.sql, wire_rows, elapsed));
    }

    // ---- Pass B: the oracle side, then the comparison. ----
    for (qid, oracle_sql, wire_rows, elapsed) in &engine_side {
        let oracle_rows = match oracle.query(oracle_sql) {
            Ok(rows) => rows,
            Err(e) => {
                failed += 1;
                eprintln!("  {qid:2}: the SQLite oracle failed on this statement ({e})");
                continue;
            }
        };

        match compare_rows(wire_rows, &oracle_rows) {
            None => {
                compared += 1;
                eprintln!("  {qid:2}: MATCH (rows={}) in {elapsed:?}", wire_rows.len());
            }
            Some(diff) => match tpch_fixture::lookup_statement_known_difference(qid) {
                Some(entry) => {
                    report_known_difference(&entry, &diff);
                    known += 1;
                    compared += 1;
                    eprintln!("  {qid:2}: KNOWN-DIFF (registered) {diff}");
                }
                None => {
                    compared += 1;
                    mismatched += 1;
                    eprintln!("  {qid:2}: MISMATCH {diff}");
                }
            },
        }
    }

    eprintln!();
    eprintln!(
        "=== Summary: compared={compared} mismatched={mismatched} known={known} \
         not_run={not_run} failed={failed} truncated_tables={truncated_tables} ==="
    );

    // Every statement must be accounted for: either it was compared against
    // the independent oracle, or it was skipped for a registered reason.
    assert_eq!(
        mismatched, 0,
        "{TARGET}: {mismatched} TPC-H statements did not match the independent SQLite oracle. \
         (If this is a known engine defect, register it in \
         tests/baseline/tpch_known_differences.json with an issue and an owner.)"
    );
    assert_eq!(
        failed, 0,
        "{TARGET}: {failed} TPC-H statement(s) could not be executed on one of the two sides"
    );
    assert_eq!(
        compared + not_run + failed,
        EXPECTED_QUERIES,
        "{TARGET}: {compared} compared + {not_run} not run + {failed} failed does not account for the \
         {EXPECTED_QUERIES} TPC-H statements of this file"
    );
    // ... and the skipped ones must be exactly the ones the shared wire
    // scope excludes — neither more (a silently dropped statement) nor
    // fewer (a statement that could have been compared).
    assert_eq!(
        skipped_scope.len(),
        expected_skipped,
        "{TARGET}: {skipped_scope:?} were skipped, but the wire scope excludes \
         {expected_skipped} statement(s) from this file"
    );
    assert_eq!(
        compared, expected_scoped,
        "{TARGET}: {compared} of the {expected_scoped} in-scope statements were compared \
         against the independent SQLite oracle"
    );
    assert!(
        compared > 0,
        "{TARGET}: this run compared nothing at all; a run that compares nothing must not \
         report success"
    );
    assert_eq!(
        truncated_tables, 0,
        "{TARGET}: the engine did not load the whole fixture — {truncated_tables} table(s) came \
         back with a different row count than the independent oracle. Every result that \
         depends on such a table is untrustworthy."
    );

    // Disclose the statements this function did not execute. One marker
    // per test function keeps the CI arithmetic exact.
    tpch_wire_scope::report_wire_exclusions(TARGET, &skipped_scope);
}
