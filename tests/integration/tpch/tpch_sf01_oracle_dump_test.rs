//! TPC-H SF=0.1 sqlrustgo Oracle Dump
//!
//! Runs all 22 TPC-H queries against the SF=0.1 fixture via the wire
//! protocol and dumps per-query TSVs + row counts to
//! `docs/releases/v3.12.0/evidence/tpch/cross_engine_sf01/sqlrustgo/`.
//!
//! The captured `.tsv` files can be diffed against the PostgreSQL
//! oracle at `cross_engine_sf01/postgres/` to validate sqlrustgo
//! semantics.
//!
//! Run:
//!   cargo test --test tpch_sf01_oracle_dump --all-features -- --nocapture
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

use common::tpch_wire_harness::{run_query_timed, start_sf01_named, WireOutcome};
use common::tpch_wire_scope;
use std::fs;
use std::path::PathBuf;

/// Cargo target name — used in the NOT-RUN marker.
const TARGET: &str = "tpch_sf01_oracle_dump";

const QUERIES: &[(&str, &str, u64)] = &[
    ("Q1", include_str!("../../../queries/q1.sql"), 60),
    ("Q2", include_str!("../../../queries/q2.sql"), 60),
    ("Q3", include_str!("../../../queries/q3.sql"), 60),
    ("Q4", include_str!("../../../queries/q4.sql"), 60),
    ("Q5", include_str!("../../../queries/q5.sql"), 60),
    ("Q6", include_str!("../../../queries/q6.sql"), 60),
    ("Q7", include_str!("../../../queries/q7.sql"), 60),
    ("Q8", include_str!("../../../queries/q8.sql"), 60),
    ("Q9", include_str!("../../../queries/q9.sql"), 60),
    ("Q10", include_str!("../../../queries/q10.sql"), 60),
    ("Q11", include_str!("../../../queries/q11.sql"), 60),
    ("Q12", include_str!("../../../queries/q12.sql"), 60),
    ("Q13", include_str!("../../../queries/q13.sql"), 60),
    ("Q14", include_str!("../../../queries/q14.sql"), 60),
    ("Q15", include_str!("../../../queries/q15.sql"), 60),
    ("Q16", include_str!("../../../queries/q16.sql"), 60),
    ("Q17", include_str!("../../../queries/q17.sql"), 60),
    ("Q18", include_str!("../../../queries/q18.sql"), 60),
    ("Q19", include_str!("../../../queries/q19.sql"), 60),
    ("Q20", include_str!("../../../queries/q20.sql"), 60),
    ("Q21", include_str!("../../../queries/q21.sql"), 60),
    ("Q22", include_str!("../../../queries/q22.sql"), 60),
];

#[test]
fn dump_sqlrustgo_sf01_oracle_tsvs() {
    let out_dir = PathBuf::from("docs/releases/v3.12.0/evidence/tpch/cross_engine_sf01/sqlrustgo");
    fs::create_dir_all(&out_dir).expect("create out dir");

    let Some(mut client) = start_sf01_named(TARGET) else {
        return;
    };
    println!("=== sqlrustgo SF=0.1 Oracle Dump ===");
    let mut compared = 0usize;
    let mut not_run = 0usize;
    let mut failed = 0usize;
    let mut errors = Vec::new();
    // The statements the file-storage wire path must not execute, and the
    // ones it must. Both counts are checked against the shared scope
    // afterwards, so a statement can neither slip through nor disappear.
    let mut skipped_scope: Vec<&'static str> = Vec::new();
    let expected_scoped = QUERIES
        .iter()
        .filter(|(qid, ..)| tpch_wire_scope::in_wire_scope(qid))
        .count();
    let expected_skipped = QUERIES.len() - expected_scoped;

    for (name, sql, timeout) in QUERIES {
        let sql = sql.trim();
        let qid: &'static str = name;
        let outcome = run_query_timed(&mut client, qid, sql, *timeout);
        print!("{:>3}: ", qid);
        let (rows, elapsed) = match outcome {
            WireOutcome::Ran(Ok(rows), elapsed) => (rows, elapsed),
            WireOutcome::Ran(Err(e), elapsed) => {
                failed += 1;
                println!("ERROR: {} ({:.2}s)", e, elapsed.as_secs_f64());
                errors.push((qid, e));
                continue;
            }
            WireOutcome::Skipped(skip) => {
                // Outside the wire scope: the statement is NOT executed.
                // Executing it would exhaust the machine and kill the test
                // binary with it (issue #5393).
                not_run += 1;
                skipped_scope.push(qid);
                println!(
                    "SKIPPED ({} issue={}) — {}",
                    skip.reason, skip.issue, skip.detail
                );
                continue;
            }
        };
        {
            let mut sorted = rows.clone();
            sorted.sort();
            let tsv: Vec<String> = sorted
                .iter()
                .map(|r| {
                    r.iter()
                        .map(|c| c.replace(['\t', '\n'], " "))
                        .collect::<Vec<_>>()
                        .join("\t")
                })
                .collect();
            let tsv_text = tsv.join("\n") + "\n";
            fs::write(
                out_dir.join(format!("{}.tsv", qid.to_lowercase())),
                tsv_text,
            )
            .expect("write tsv");
            println!("{} rows, {:.2}s", rows.len(), elapsed.as_secs_f64());
            compared += 1;
        }
    }
    println!("\n[OK] {compared} compared, {not_run} not run, {failed} failed (of 22)");

    // A statement that could not be executed is reported first: it is the
    // real cause, the accounting below is only a consequence of it.
    assert!(
        errors.is_empty(),
        "{} queries failed: {errors:?}",
        errors.len()
    );

    // Every statement must be accounted for: either it was executed and
    // dumped, it failed, or it was skipped for a registered reason.
    assert_eq!(
        compared + not_run + failed,
        QUERIES.len(),
        "{TARGET}: {compared} compared + {not_run} not run + {failed} failed does not account \
         for the {} statements of this file",
        QUERIES.len()
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
        "{TARGET}: {compared} of the {expected_scoped} in-scope statements were executed"
    );
    assert!(
        compared > 0,
        "{TARGET}: this run executed nothing at all; a run that executes nothing must not \
         report success"
    );

    // Disclose the statements this function did not execute. One marker
    // per test function keeps the CI arithmetic exact.
    tpch_wire_scope::report_wire_exclusions(TARGET, &skipped_scope);
}
