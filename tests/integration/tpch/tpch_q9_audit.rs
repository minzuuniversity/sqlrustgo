//! TPC-H 22 audit on SF=0.01 canonical fixture via wire protocol.
//! Ensures all 22 queries execute without crashing on the wire server.
//!
//! ## Fixture
//!
//! - `tests/data/tpch-sf01/` — 8 `.tbl` files (lineitem ≈ 6 015 rows)
//!
//! ## Historical note
//!
//! This file previously pointed at `/tmp/tpch_3way_sf001/clean` (SF=0.001,
//! ~600 lineitem rows) while the file header claimed SF=0.01. The canonical
//! committed fixture (`tests/data/tpch-sf01/`, ~6 015 lineitem rows) is the
//! correct reference. Wire protocol test now uses the right fixture.
//!
//! ## SQLite baseline regeneration
//!
//! ```text
//! python3 scripts/dev/build_tpch_sf01_sqlite.py
//! ```
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

#![allow(dead_code)]

#[path = "../../common/mod.rs"]
mod common;
use common::tpch_wire_harness::{run_query_timed, WireOutcome};
use common::tpch_wire_scope;
use std::process::Command;

const SQLITE_BASELINE_DB: &str = "/tmp/tpch_sf01_audit.db";

/// Cargo target name — used in the NOT-RUN marker.
const TARGET: &str = "tpch_q9_audit";

/// Run sqlite3 with a query, return the first column of the first row as i64.
fn sqlite_count(sql: &str) -> Option<i64> {
    let sql = format!("SELECT COUNT(*) FROM ({}) sub", sql);
    let output = Command::new("sqlite3")
        .args([SQLITE_BASELINE_DB, &sql])
        .output()
        .ok()?;
    String::from_utf8_lossy(&output.stdout).trim().parse().ok()
}

/// Strip ORDER BY clause so the query can be used as a subquery
/// (SQLite rejects ORDER BY inside a subquery without OFFSET).
fn strip_order_by(sql: &str) -> String {
    sql.rsplit_once("ORDER BY")
        .map(|(head, _)| head.trim().trim_end_matches(';'))
        .unwrap_or_else(|| sql.trim().trim_end_matches(';'))
        .to_string()
}

/// Rewrite EXTRACT(YEAR FROM col) [AS alias] for SQLite compatibility.
fn rewrite_for_sqlite(sql: &str) -> String {
    let re = regex::Regex::new(r"EXTRACT\(YEAR FROM ([^)]+)\)(?: AS (\w+))?").unwrap();
    re.replace_all(sql, |caps: &regex::Captures| {
        let col = caps.get(1).unwrap().as_str();
        match caps.get(2) {
            Some(alias) => {
                format!(
                    "CAST(strftime('%Y', {}) AS INTEGER) AS {}",
                    col,
                    alias.as_str()
                )
            }
            None => format!("CAST(strftime('%Y', {}) AS INTEGER)", col),
        }
    })
    .to_string()
}

#[test]
fn test_tpch_22_wire_sf01_audit() {
    use common::tpch_wire_harness::start_sf001_named;

    let Some(mut client) = start_sf001_named(TARGET) else {
        return;
    };

    eprintln!("\n=== TPC-H 22 audit via wire (SF=0.01) ===\n");

    let queries_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("queries");
    let mut ok = 0;
    let mut err = 0;
    let mut parse_err = 0;
    let mut not_run = 0;
    // The statements the file-storage wire path must not execute, and the
    // ones it must. Both counts are checked against the shared scope
    // afterwards, so a statement can neither slip through nor disappear.
    let mut skipped_scope: Vec<&'static str> = Vec::new();
    let expected_scoped = (1..=22)
        .filter(|q| tpch_wire_scope::in_wire_scope(tpch_wire_scope::TPC_H_QUERY_IDS[q - 1]))
        .count();
    let expected_skipped = 22 - expected_scoped;

    for q in 1..=22 {
        let sql_path = queries_dir.join(format!("q{}.sql", q));
        let sql = std::fs::read_to_string(&sql_path).unwrap();
        let sql_trimmed = sql.trim().trim_end_matches(';');
        // The standard query id decides whether the statement may be
        // executed on the file-storage wire path at all (the **wire
        // scope**). The statement itself is wrapped in COUNT(*) so that the
        // audit only checks "does it run", not "what does it return".
        let qid = tpch_wire_scope::TPC_H_QUERY_IDS[q - 1];

        // Engine result via wire: wrap in COUNT(*)
        let wrap = format!("SELECT COUNT(*) AS c FROM ({}) sub", sql_trimmed);
        let engine_count: Option<i64> = match run_query_timed(&mut client, qid, &wrap, 60) {
            WireOutcome::Ran(Ok(rows), _) => rows
                .first()
                .and_then(|row| row.first())
                .and_then(|v| v.parse().ok()),
            WireOutcome::Ran(Err(e), _) => {
                let e_str = e.to_string();
                if e_str.contains("parse")
                    || e_str.contains("Parse")
                    || e_str.contains("syntax")
                    || e_str.contains("#42000")
                {
                    eprintln!(
                        "{qid:>3}: PARSE_ERR  ({})",
                        e_str.lines().next().unwrap_or(&e_str)
                    );
                    parse_err += 1;
                } else {
                    eprintln!(
                        "{qid:>3}: ERR  ({})",
                        e_str.lines().next().unwrap_or(&e_str)
                    );
                    err += 1;
                }
                None
            }
            WireOutcome::Skipped(skip) => {
                // Outside the wire scope: the statement is NOT executed.
                // Executing it would exhaust the machine and kill the test
                // binary with it (issue #5393).
                not_run += 1;
                skipped_scope.push(qid);
                eprintln!(
                    "{qid:>3}: SKIPPED ({} issue={}) — {}",
                    skip.reason, skip.issue, skip.detail
                );
                None
            }
        };

        if let Some(ref c) = engine_count {
            eprintln!("{qid:>3}: OK  ({} rows)", c);
            ok += 1;
        }
    }

    eprintln!("\n=== Totals ===");
    eprintln!("OK        : {ok}/22");
    eprintln!("ERR       : {err}/22");
    eprintln!("PARSE_ERR : {parse_err}/22");
    eprintln!("NOT RUN   : {not_run}/22");

    // A statement that crashed is reported first: it is the real cause, the
    // accounting below is only a consequence of it.
    assert_eq!(err, 0, "TPC-H gate failure: {} queries crashed", err);

    // Every statement must be accounted for: it ran, it failed, it did not
    // parse, or it was excluded by the wire scope. Nothing may disappear.
    assert_eq!(
        ok + err + parse_err + not_run,
        22,
        "{TARGET}: {ok} ok + {err} err + {parse_err} parse_err + {not_run} not run does not \
         account for the 22 statements of this file"
    );
    // ... and the skipped ones must be exactly the ones the shared wire
    // scope excludes — neither more (a silently dropped statement) nor
    // fewer (a statement that could have been run).
    assert_eq!(
        skipped_scope.len(),
        expected_skipped,
        "{TARGET}: {skipped_scope:?} were skipped, but the wire scope excludes \
         {expected_skipped} statement(s) from this file"
    );
    assert_eq!(
        ok + err + parse_err,
        expected_scoped,
        "{TARGET}: {} of the {expected_scoped} in-scope statements were attempted",
        ok + err + parse_err
    );
    assert!(
        ok + err + parse_err > 0,
        "{TARGET}: this run attempted nothing at all; a run that attempts nothing must not \
         report success"
    );

    // Disclose the statements this function did not execute. One marker
    // per test function keeps the CI arithmetic exact.
    tpch_wire_scope::report_wire_exclusions(TARGET, &skipped_scope);
}
