//! TPC-H 22/22 cross-engine comparison at SF=0.01:
//!
//! * **sqlrustgo over the wire protocol** vs the **in-process SQLite
//!   oracle** — always available (`rusqlite` with the `bundled` feature),
//!   so this leg runs everywhere and is a hard gate;
//! * sqlrustgo vs **MariaDB** (`mysql` CLI) and vs **PostgreSQL**
//!   (`psql` CLI) — these need a live client binary *and* a live server
//!   holding the same fixture, which a CI runner has neither of.
//!
//! # What changed (issue #5379)
//!
//! Both external legs used to `eprintln!("MariaDB not available, skipping")`
//! and `return`. A test that skips and returns reports `test result: ok`:
//! the CI counted it as **passed**, and the log said nothing that a reader
//! could act on. The SQLite leg did not exist at all — the file only ever
//! compared against two external engines, so on CI it compared nothing and
//! still reported success.
//!
//! # What this file is now
//!
//! * The **SQLite leg runs unconditionally** and asserts what it actually
//!   compared: a run that compared nothing can never report success.
//! * Each external leg **probes its prerequisites first** and, when they
//!   are missing, reports a `TPCH-NOT-RUN` marker with a controlled reason
//!   code — `scale_layer_only` when the client binary or the server is not
//!   there, `profile_mismatch` when the external database does not hold
//!   the same fixture. NOT-RUN is disclosed separately by the CI summary
//!   and is never counted as a pass.
//! * A comparison that *was* performed and disagrees is a **failure**.
//!
//! # Prerequisites
//!
//! A TPC-H fixture, resolved at run time (`TPCH_DATA_DIR` first). A missing
//! fixture is disclosed as `TPCH-NOT-RUN` with the reason code
//! `fixture_missing` — never as a pass.
//!
//! # The external databases
//!
//! The MariaDB database is `tpch_sf01`, the PostgreSQL database is
//! `tpch_sf01_pgdate` on `localhost` — the local sandboxes this file has
//! always used. A leg only compares when the external database holds the
//! *same* fixture as the oracle; the row count of `lineitem` is checked
//! first, because comparing against a different dataset would produce
//! differences that say nothing about the engine.
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

use std::process::Command;
use std::sync::{Mutex, OnceLock};

use common::tpch_cli_harness::tpch_fixture::{reason, report_known_difference, report_not_run};
use common::tpch_wire_harness::{run_query_timed, start_sf01_named, WireOutcome};
use common::tpch_wire_scope;
use sqlite_oracle::{adapt_sql, diff_rows, SqliteOracle, NULL_CELL, TPC_H_ALL_TABLES};

/// Cargo target name — used in the NOT-RUN marker and in the `case_id` of
/// the known-difference registry.
const TARGET: &str = "tpch_sf01_22_vs_3engines";

/// Directory holding `q1.sql` … `q22.sql`, relative to the package root
/// (the working directory `cargo test` runs in).
const QUERIES_DIR: &str = "queries";

/// The TPC-H query set has exactly 22 statements.
const EXPECTED_QUERIES: usize = 22;

/// Per-statement timeout in seconds.
const TIMEOUT_S: u64 = 60;

/// Statements whose result is a single aggregate.
///
/// Their value is `NULL` when the filtered set is empty, and both the wire
/// protocol and the command line clients render a `NULL` as the empty
/// string — indistinguishable from a real empty string. Only the row count
/// is compared for them.
const ROW_COUNT_ONLY: [&str; 3] = ["Q6", "Q14", "Q17"];

/// The shared ephemeral server.
static CLIENT: OnceLock<Mutex<Option<common::MySqlTestClient>>> = OnceLock::new();

/// Whether the connection is still usable.
fn is_alive(client: &mut common::MySqlTestClient) -> bool {
    client.query_rows("SELECT 1").is_ok()
}

/// Run `f` with a live wire client.
///
/// The server is shared by the three tests of this target: loading the
/// SF=0.01 fixture into it costs about half a minute, and paying that
/// three times would make the target slower than the CI timeout. A
/// connection that died is replaced by a fresh server.
fn with_client<R>(f: impl FnOnce(&mut common::MySqlTestClient) -> R) -> R {
    let slot = CLIENT.get_or_init(|| Mutex::new(None));
    let mut guard = slot.lock().unwrap_or_else(|e| e.into_inner());
    let dead = match guard.as_mut() {
        Some(client) => !is_alive(client),
        None => true,
    };
    if dead {
        *guard = Some(
            start_sf01_named(TARGET)
                .expect("the fixture was resolved, so the ephemeral server must start"),
        );
    }
    f(guard.as_mut().expect("a client is present"))
}

/// Compare the engine's rows with the other engine's rows.
///
/// `content` selects the comparison strength: `true` compares the cell
/// values (as a sorted multiset), `false` compares only the row count.
/// Returns `None` when the two sides agree.
fn compare_rows(
    engine_rows: &[Vec<String>],
    other_rows: &[Vec<String>],
    content: bool,
) -> Option<String> {
    if engine_rows.len() != other_rows.len() {
        return Some(format!(
            "row count differs: engine={} other={}",
            engine_rows.len(),
            other_rows.len()
        ));
    }
    if !content {
        return None;
    }
    // A command line client renders a SQL NULL as the empty string, which
    // cannot be told apart from a real empty string. The oracle's NULL
    // sentinel is therefore normalized to the empty string here; a
    // difference between a NULL and an empty string is *not* detectable in
    // this comparison, and the row count still is.
    let normalize = |rows: &[Vec<String>]| -> Vec<Vec<String>> {
        rows.iter()
            .map(|r| {
                r.iter()
                    .map(|c| {
                        if c.as_str() == NULL_CELL {
                            String::new()
                        } else {
                            c.clone()
                        }
                    })
                    .collect()
            })
            .collect()
    };
    let mut engine = normalize(engine_rows);
    let mut other = normalize(other_rows);
    engine.sort();
    other.sort();
    diff_rows(&engine, &other)
}

/// Run `sql` through the `mysql` CLI and return its rows.
fn run_mysql(sql: &str) -> Result<Vec<Vec<String>>, String> {
    let out = Command::new("mysql")
        .args(["-B", "-N", "tpch_sf01", "-e", sql])
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.split('\t').map(|c| c.to_string()).collect())
        .collect())
}

/// Run `sql` through the `psql` CLI and return its rows.
fn run_psql(sql: &str) -> Result<Vec<Vec<String>>, String> {
    let out = Command::new("env")
        .args([
            "PGPASSWORD=",
            "psql",
            "-h",
            "localhost",
            // #5500: no `-U <name>` here. The client takes the user from
            // `PGUSER` (libpq's own variable) and falls back to the
            // operating-system user — hardcoding one machine's user name
            // made this target unrunnable everywhere else.
            "-d",
            "tpch_sf01_pgdate",
            "-A",
            "-t",
            "-F",
            "|",
            "-c",
            sql,
        ])
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| {
            !l.is_empty() && !l.starts_with("---") && !l.starts_with("(") && !l.contains("?column?")
        })
        .map(|l| l.split('|').map(|c| c.to_string()).collect())
        .collect())
}

/// Whether a command line client binary exists in this environment.
fn binary_available(binary: &str) -> bool {
    Command::new(binary)
        .arg("--version")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

/// Compare the engine's result with the other engine's result for all 22
/// statements.
///
/// `run` executes one statement on the other engine. The result is
/// `None` when the other engine could not run it at all.
fn compare_against_other_engine(
    leg: &str,
    mut run: impl FnMut(&str) -> Result<Vec<Vec<String>>, String>,
) {
    with_client(|client| {
        let mut compared = 0usize;
        let mut mismatched = 0usize;
        let mut known = 0usize;
        let mut not_run = 0usize;
        let mut failed = 0usize;
        // The statements the file-storage wire path must not execute, and
        // the ones it must. Both counts are checked against the shared scope
        // afterwards, so a statement can neither slip through nor disappear.
        let mut skipped_scope: Vec<&'static str> = Vec::new();
        let expected_scoped = (1..=EXPECTED_QUERIES)
            .filter(|n| tpch_wire_scope::in_wire_scope(tpch_wire_scope::TPC_H_QUERY_IDS[n - 1]))
            .count();
        let expected_skipped = EXPECTED_QUERIES - expected_scoped;

        for n in 1..=EXPECTED_QUERIES {
            let sql = std::fs::read_to_string(format!("{QUERIES_DIR}/q{n}.sql"))
                .unwrap_or_else(|e| panic!("{TARGET}: cannot read {QUERIES_DIR}/q{n}.sql: {e}"));
            let qid = tpch_wire_scope::TPC_H_QUERY_IDS[n - 1];
            let not_run_label = format!("{TARGET}:{leg}:{qid}");

            let adapted = adapt_sql(&sql);
            if !adapted.unsupported.is_empty() {
                let detail = format!(
                    "the SQLite oracle cannot execute this statement ({:?}); \
                     it belongs to the cross-engine dialect layer",
                    adapted.unsupported
                );
                report_not_run(&not_run_label, reason::SCALE_LAYER_ONLY, &detail);
                not_run += 1;
                continue;
            }

            let t0 = std::time::Instant::now();
            let engine_rows = match run_query_timed(&mut *client, qid, &sql, TIMEOUT_S) {
                WireOutcome::Ran(Ok(rows), _) => rows,
                WireOutcome::Ran(Err(e), _) => {
                    failed += 1;
                    eprintln!(
                        "  {qid:2}: the engine failed ({e}) after {:?}",
                        t0.elapsed()
                    );
                    continue;
                }
                WireOutcome::Skipped(skip) => {
                    // Outside the wire scope: the statement is NOT executed.
                    // Executing it would exhaust the machine and kill the
                    // test binary with it (issue #5393). The other engine is
                    // not asked either — there would be nothing to compare.
                    not_run += 1;
                    skipped_scope.push(qid);
                    eprintln!(
                        "  {qid:2}: SKIPPED ({} issue={}) — {}",
                        skip.reason, skip.issue, skip.detail
                    );
                    continue;
                }
            };
            let other_rows = match run(&sql) {
                Ok(rows) => rows,
                Err(e) => {
                    failed += 1;
                    eprintln!("  {qid:2}: {leg} failed ({e})");
                    continue;
                }
            };

            let content = !ROW_COUNT_ONLY.contains(&qid);
            match compare_rows(&engine_rows, &other_rows, content) {
                None => {
                    compared += 1;
                    eprintln!("  {qid:2}: MATCH (rows={})", engine_rows.len());
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
            "=== Summary ({leg}): compared={compared} mismatched={mismatched} known={known} \
             not_run={not_run} failed={failed} ==="
        );

        // Every statement must be accounted for: either it was compared, or
        // it was skipped for a registered reason. Nothing may disappear.
        assert_eq!(
            mismatched, 0,
            "{TARGET}: {mismatched} statement(s) disagree with {leg}. (If this is a known engine \
             defect, register it in tests/baseline/tpch_known_differences.json with an issue and \
             an owner.)"
        );
        assert_eq!(
            failed, 0,
            "{TARGET}: {failed} statement(s) could not be executed on one of the two sides"
        );
        assert_eq!(
            compared + not_run + failed,
            EXPECTED_QUERIES,
            "{TARGET}: the {leg} leg compared {compared} statements, did not run {not_run} and failed on {failed}; \
             that does not account for the {EXPECTED_QUERIES} statements of this file"
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
            "{TARGET}: the {leg} leg compared {compared} of the {expected_scoped} in-scope \
             statements"
        );
        assert!(
            compared > 0,
            "{TARGET}: the {leg} leg compared nothing at all; a run that compares nothing must \
             not report success"
        );

        // Disclose the statements this function did not execute. One marker
        // per test function keeps the CI arithmetic exact.
        tpch_wire_scope::report_wire_exclusions(&format!("{TARGET}:{leg}"), &skipped_scope);
    });
}

/// The SQLite leg — the in-process oracle. Always available.
#[test]
fn tpch_sf01_22_vs_sqlite_oracle() {
    let state = tpch_fixture::FixtureState::resolve();
    let Some(fixture) = state.require(TARGET) else {
        // `require` has already printed the NOT-RUN marker.
        return;
    };
    let oracle = SqliteOracle::open(fixture, &TPC_H_ALL_TABLES)
        .unwrap_or_else(|e| panic!("{TARGET}: cannot build the independent SQLite oracle: {e}"));

    with_client(|client| {
        let mut compared = 0usize;
        let mut mismatched = 0usize;
        let mut known = 0usize;
        let mut not_run = 0usize;
        let mut failed = 0usize;
        // The statements the file-storage wire path must not execute, and
        // the ones it must. Both counts are checked against the shared scope
        // afterwards, so a statement can neither slip through nor disappear.
        let mut skipped_scope: Vec<&'static str> = Vec::new();
        let expected_scoped = (1..=EXPECTED_QUERIES)
            .filter(|n| tpch_wire_scope::in_wire_scope(tpch_wire_scope::TPC_H_QUERY_IDS[n - 1]))
            .count();
        let expected_skipped = EXPECTED_QUERIES - expected_scoped;

        for n in 1..=EXPECTED_QUERIES {
            let sql = std::fs::read_to_string(format!("{QUERIES_DIR}/q{n}.sql"))
                .unwrap_or_else(|e| panic!("{TARGET}: cannot read {QUERIES_DIR}/q{n}.sql: {e}"));
            let qid = tpch_wire_scope::TPC_H_QUERY_IDS[n - 1];
            let not_run_label = format!("{TARGET}:sqlite:{qid}");

            let adapted = adapt_sql(&sql);
            if !adapted.unsupported.is_empty() {
                let detail = format!(
                    "the SQLite oracle cannot execute this statement ({:?}); \
                     it belongs to the cross-engine dialect layer",
                    adapted.unsupported
                );
                report_not_run(&not_run_label, reason::SCALE_LAYER_ONLY, &detail);
                not_run += 1;
                continue;
            }

            let t0 = std::time::Instant::now();
            let engine_rows = match run_query_timed(&mut *client, qid, &sql, TIMEOUT_S) {
                WireOutcome::Ran(Ok(rows), _) => rows,
                WireOutcome::Ran(Err(e), _) => {
                    failed += 1;
                    eprintln!(
                        "  {qid:2}: the engine failed ({e}) after {:?}",
                        t0.elapsed()
                    );
                    continue;
                }
                WireOutcome::Skipped(skip) => {
                    // Outside the wire scope: the statement is NOT executed.
                    // Executing it would exhaust the machine and kill the
                    // test binary with it (issue #5393).
                    not_run += 1;
                    skipped_scope.push(qid);
                    eprintln!(
                        "  {qid:2}: SKIPPED ({} issue={}) — {}",
                        skip.reason, skip.issue, skip.detail
                    );
                    continue;
                }
            };
            let oracle_rows = match oracle.query(&adapted.sql) {
                Ok(rows) => rows,
                Err(e) => {
                    failed += 1;
                    eprintln!("  {qid:2}: the SQLite oracle failed ({e})");
                    continue;
                }
            };

            let content = !ROW_COUNT_ONLY.contains(&qid);
            match compare_rows(&engine_rows, &oracle_rows, content) {
                None => {
                    compared += 1;
                    eprintln!("  {qid:2}: MATCH (rows={})", engine_rows.len());
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
            "=== Summary (sqlite): compared={compared} mismatched={mismatched} known={known} \
             not_run={not_run} failed={failed} ==="
        );

        // Every statement must be accounted for: either it was compared, or
        // it was skipped for a registered reason. Nothing may disappear.
        assert_eq!(
            mismatched, 0,
            "{TARGET}: {mismatched} statement(s) disagree with the independent SQLite oracle. \
             (If this is a known engine defect, register it in \
             tests/baseline/tpch_known_differences.json with an issue and an owner.)"
        );
        assert_eq!(
            failed, 0,
            "{TARGET}: {failed} statement(s) could not be executed on one of the two sides"
        );
        assert_eq!(
            compared + not_run + failed,
            EXPECTED_QUERIES,
            "{TARGET}: the SQLite leg compared {compared} statements, did not run {not_run} and failed on {failed}; \
             that does not account for the {EXPECTED_QUERIES} statements of this file"
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
            "{TARGET}: the SQLite leg compared {compared} of the {expected_scoped} in-scope \
             statements"
        );
        assert!(
            compared > 0,
            "{TARGET}: the SQLite leg compared nothing at all; a run that compares nothing must \
             not report success"
        );

        // Disclose the statements this function did not execute. One marker
        // per test function keeps the CI arithmetic exact.
        tpch_wire_scope::report_wire_exclusions(TARGET, &skipped_scope);
    });
}

/// The MariaDB leg — needs the `mysql` client and a live `tpch_sf01`
/// database holding the same fixture.
#[test]
fn tpch_sf01_22_vs_mariadb_cell() {
    if !binary_available("mysql") {
        report_not_run(
            &format!("{TARGET}:mariadb"),
            reason::SCALE_LAYER_ONLY,
            "the `mysql` client binary is not installed in this environment; the cross-engine \
             comparison against MariaDB belongs to the external scale layer",
        );
        return;
    }
    if let Err(e) = run_mysql("SELECT 1") {
        report_not_run(
            &format!("{TARGET}:mariadb"),
            reason::SCALE_LAYER_ONLY,
            &format!(
                "the `mysql` client is installed but the `tpch_sf01` database is not reachable \
                 ({e}); the cross-engine comparison against MariaDB belongs to the external \
                 scale layer"
            ),
        );
        return;
    }

    let state = tpch_fixture::FixtureState::resolve();
    let Some(fixture) = state.require(TARGET) else {
        return;
    };
    let oracle = SqliteOracle::open(fixture, &TPC_H_ALL_TABLES)
        .unwrap_or_else(|e| panic!("{TARGET}: cannot build the independent SQLite oracle: {e}"));

    // The external database must hold the same fixture, otherwise a
    // difference would say nothing about the engine.
    let expected = oracle
        .table_row_count("lineitem")
        .unwrap_or_else(|e| panic!("{TARGET}: the SQLite oracle cannot count lineitem: {e}"));
    match run_mysql("SELECT COUNT(*) FROM lineitem") {
        Ok(rows) => {
            let actual: i64 = rows
                .first()
                .and_then(|r| r.first())
                .and_then(|c| c.parse::<i64>().ok())
                .unwrap_or(-1);
            if actual != expected {
                report_not_run(
                    &format!("{TARGET}:mariadb"),
                    reason::PROFILE_MISMATCH,
                    &format!(
                        "the MariaDB database holds {actual} lineitem row(s) but the fixture \
                         holds {expected}; the cross-engine comparison needs the same dataset"
                    ),
                );
                return;
            }
        }
        Err(e) => {
            report_not_run(
                &format!("{TARGET}:mariadb"),
                reason::SCALE_LAYER_ONLY,
                &format!("the MariaDB database could not be queried ({e})"),
            );
            return;
        }
    }

    compare_against_other_engine("mariadb", run_mysql);
}

/// The PostgreSQL leg — needs the `psql` client and a live
/// `tpch_sf01_pgdate` database holding the same fixture.
#[test]
fn tpch_sf01_22_vs_postgresql_pgdate_cell() {
    if !binary_available("psql") {
        report_not_run(
            &format!("{TARGET}:postgresql"),
            reason::SCALE_LAYER_ONLY,
            "the `psql` client binary is not installed in this environment; the cross-engine \
             comparison against PostgreSQL belongs to the external scale layer",
        );
        return;
    }
    if let Err(e) = run_psql("SELECT 1") {
        report_not_run(
            &format!("{TARGET}:postgresql"),
            reason::SCALE_LAYER_ONLY,
            &format!(
                "the `psql` client is installed but the `tpch_sf01_pgdate` database is not \
                 reachable ({e}); the cross-engine comparison against PostgreSQL belongs to the \
                 external scale layer"
            ),
        );
        return;
    }

    let state = tpch_fixture::FixtureState::resolve();
    let Some(fixture) = state.require(TARGET) else {
        return;
    };
    let oracle = SqliteOracle::open(fixture, &TPC_H_ALL_TABLES)
        .unwrap_or_else(|e| panic!("{TARGET}: cannot build the independent SQLite oracle: {e}"));

    let expected = oracle
        .table_row_count("lineitem")
        .unwrap_or_else(|e| panic!("{TARGET}: the SQLite oracle cannot count lineitem: {e}"));
    match run_psql("SELECT COUNT(*) FROM lineitem") {
        Ok(rows) => {
            let actual: i64 = rows
                .first()
                .and_then(|r| r.first())
                .and_then(|c| c.parse::<i64>().ok())
                .unwrap_or(-1);
            if actual != expected {
                report_not_run(
                    &format!("{TARGET}:postgresql"),
                    reason::PROFILE_MISMATCH,
                    &format!(
                        "the PostgreSQL database holds {actual} lineitem row(s) but the fixture \
                         holds {expected}; the cross-engine comparison needs the same dataset"
                    ),
                );
                return;
            }
        }
        Err(e) => {
            report_not_run(
                &format!("{TARGET}:postgresql"),
                reason::SCALE_LAYER_ONLY,
                &format!("the PostgreSQL database could not be queried ({e})"),
            );
            return;
        }
    }

    compare_against_other_engine("postgresql", run_psql);
}
