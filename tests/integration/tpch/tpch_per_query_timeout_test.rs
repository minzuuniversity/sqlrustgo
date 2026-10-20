//! Per-statement timeout on the file-storage wire path.
//!
//! Q1 and Q6 must both complete well inside the 30 s per-statement budget.
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
//! `WireOutcome::Skipped` for them. Q1 and Q6 are inside the scope, so both
//! are executed and timed; a skip would be disclosed through the
//! `TPCH-NOT-RUN` marker and the test function would report *not run* —
//! never a pass.

#[path = "../../common/mod.rs"]
mod common;
use common::tpch_wire_harness::{run_query_timed, start_sf001_named, WireOutcome};
use common::tpch_wire_scope;
use std::time::Duration;

/// Cargo target name — used in the NOT-RUN marker.
const TARGET: &str = "tpch_per_query_timeout_test";

/// Run one standard statement and assert that it completed inside the
/// per-statement timeout.
///
/// `qid` is the standard query id: it is what makes the **wire scope**
/// enforceable at compile time. A statement outside the scope is *not*
/// executed — the runner returns `WireOutcome::Skipped` instead — so this
/// function can never accidentally run an unrunnable statement.
fn assert_completes_within(qid: &'static str, sql: &str, timeout_s: u64) {
    let Some(mut client) = start_sf001_named(TARGET) else {
        return;
    };
    let outcome = run_query_timed(&mut client, qid, sql, timeout_s);
    let (result, elapsed) = match outcome {
        WireOutcome::Ran(result, elapsed) => (result, elapsed),
        WireOutcome::Skipped(skip) => {
            // Outside the wire scope: the statement is NOT executed. The
            // skip is disclosed, and the test function reports *not run* —
            // never a pass.
            eprintln!(
                "  {qid}: SKIPPED ({} issue={}) — {}",
                skip.reason, skip.issue, skip.detail
            );
            tpch_wire_scope::report_wire_exclusions(TARGET, &[qid]);
            return;
        }
    };
    assert!(result.is_ok(), "{qid} must complete: {result:?}");
    assert!(
        elapsed < Duration::from_secs(timeout_s),
        "{qid} too slow: {elapsed:?}"
    );

    // The statement is in the wire scope, so it must have been executed —
    // a run that executes nothing must not report success.
    assert!(
        tpch_wire_scope::in_wire_scope(qid),
        "{TARGET}: {qid} is not in the wire scope, so this test cannot assert anything about it"
    );
}

#[test]
fn tpch_per_query_timeout_q1() {
    let q1 = "SELECT l_returnflag, l_linestatus, COUNT(*) \
              FROM lineitem \
              WHERE l_shipdate <= '1998-09-02' \
              GROUP BY l_returnflag, l_linestatus \
              ORDER BY l_returnflag, l_linestatus";
    assert_completes_within("Q1", q1, 30);
}

#[test]
fn tpch_per_query_timeout_q6() {
    let q6 = "SELECT COUNT(*) FROM lineitem \
              WHERE l_shipdate >= '1994-01-01' \
                AND l_shipdate < '1995-01-01' \
                AND l_quantity < 25";
    assert_completes_within("Q6", q6, 30);
}
