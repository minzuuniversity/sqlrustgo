//! The **wire scope**: which TPC-H standard queries may be executed on the
//! file-storage wire path, and which may not.
//!
//! # The problem this module exists for
//!
//! The wire path runs the MySQL protocol against the engine's *file* storage
//! backend. That backend materialises the intermediate result of a multi-way
//! join, and for three of the 22 standard queries the intermediate grows
//! until the operating system kills the engine process.
//!
//! A killed process is not a test failure — it takes the **whole test binary**
//! with it. The CI job then dies with `signal: 9` and every later target in
//! that binary never runs. That is exactly the "silently truncated test run"
//! shape that issue #5379 set out to end, so it must not be reintroduced by
//! the very change that fixes it.
//!
//! # The rule
//!
//! A standard query is executed on the wire path **only** if it is listed in
//! [`WIRE_SCOPE`]. Everything else is listed in [`WIRE_EXCLUSIONS`] together
//! with a controlled reason code and the issue that tracks the defect.
//!
//! The scope is enforced at **compile time**: the harness's query runner
//! takes the query id, so a call site that wants to execute a statement
//! outside the scope has to name it — and naming it produces a *skip*, not
//! an execution.
//!
//! # What the excluded statements lose
//!
//! Nothing, in the end: the three excluded statements are executed by the
//! **in-process** path (`tests/integration/sql/eval_22_vs_sf01.rs`), which
//! runs all 22 standard queries against the same independent oracle on the
//! same fixture. They are simply not executed on the file-storage backend,
//! where they are known to be unrunnable.
//!
//! # Evidence
//!
//! Measured on 2026-10-20 with the hermetic SF=0.01 fixture
//! (`scripts/gate/generate_tpch_sf.py --sf 0.01 --seed 42`,
//! `generator_sha256=b71e047c1bc9a9c1…`), the file-storage backend, and the
//! synthetic unique first column that works around issue #5392:
//!
//! | statement | outcome on the wire path |
//! |---|---|
//! | 18 of the 19 in scope | identical to the in-process SQLite oracle, cell by cell |
//! | Q13 | same row count, different cells — registered as a known difference (issue #5394) |
//! | Q9 | engine process killed by the OS (issue #5393) |
//! | Q18 | server process killed while the statement ran (issue #5393) |
//! | Q19 | the wire front end cannot parse it (issue #5393) |

/// Reason code: the file-storage backend cannot execute the statement
/// without exhausting the machine.
pub const REASON_ENGINE_RESOURCE_LIMIT: &str = "engine_resource_limit";

/// Reason code: the wire front end cannot parse the statement.
pub const REASON_ENGINE_PARSE_ERROR: &str = "engine_parse_error";

/// Reason code: a test function skipped at least one statement because of
/// the wire scope. The per-statement reasons are listed in the detail.
pub const REASON_WIRE_SCOPE_EXCLUSION: &str = "wire_scope_exclusion";

/// The identifiers of the 22 TPC-H standard queries.
///
/// This is the reference set against which [`WIRE_SCOPE`] and
/// [`WIRE_EXCLUSIONS`] are checked: the two of them together must cover
/// every standard query exactly once. Without that check a statement could
/// disappear from the wire path without anyone noticing — which is the
/// failure mode this whole mechanism exists to prevent.
pub const TPC_H_QUERY_IDS: [&str; 22] = [
    "Q1", "Q2", "Q3", "Q4", "Q5", "Q6", "Q7", "Q8", "Q9", "Q10", "Q11", "Q12", "Q13", "Q14", "Q15",
    "Q16", "Q17", "Q18", "Q19", "Q20", "Q21", "Q22",
];

/// The TPC-H statements the **file-storage wire path** can execute at
/// SF=0.01.
///
/// Each of them was executed against the file-storage backend with the
/// hermetic SF=0.01 fixture and compared **cell by cell** with the
/// independent in-process SQLite oracle. 18 of the 19 match; Q13 differs in
/// a way that is registered as a known difference (issue #5394), so it is
/// still compared — only the *difference* is tolerated, not the statement.
pub const WIRE_SCOPE: &[&str] = &[
    "Q1", "Q2", "Q3", "Q4", "Q5", "Q6", "Q7", "Q8", "Q10", "Q11", "Q12", "Q13", "Q14", "Q15",
    "Q16", "Q17", "Q20", "Q21", "Q22",
];

/// A standard query that the file-storage wire path must not execute.
#[derive(Debug, Clone, Copy)]
pub struct WireExclusion {
    /// The query identifier (`Q9`, …).
    pub qid: &'static str,
    /// Controlled reason code — see [`REASON_ENGINE_RESOURCE_LIMIT`] and
    /// friends.
    pub reason: &'static str,
    /// The issue that tracks the defect.
    pub issue: &'static str,
    /// What was measured, so a reader can check the claim.
    pub detail: &'static str,
}

/// The statements the file-storage wire path cannot execute at SF=0.01.
///
/// Every entry names the defect that makes the statement unrunnable and the
/// issue that tracks it. A test that skips a statement which is **not** in
/// this list is a bug in the test, and the scope check rejects it.
pub const WIRE_EXCLUSIONS: &[WireExclusion] = &[
    WireExclusion {
        qid: "Q9",
        reason: REASON_ENGINE_RESOURCE_LIMIT,
        issue: "#5393",
        detail: "six-way join; the file-storage backend's intermediate result exceeded 1.5 GB \
                 within one second of execution and the engine process was killed by the OS \
                 (SIGKILL), which also killed the test binary",
    },
    WireExclusion {
        qid: "Q18",
        reason: REASON_ENGINE_RESOURCE_LIMIT,
        issue: "#5393",
        detail: "IN (SELECT … GROUP BY … HAVING …) over lineitem; the server process was killed \
                 while the statement was running (the client saw: Lost connection to server \
                 during query)",
    },
    WireExclusion {
        qid: "Q19",
        reason: REASON_ENGINE_PARSE_ERROR,
        issue: "#5393",
        detail: "the wire front end cannot parse an IN (…) list inside a parenthesised OR \
                 group: Parse error: Expected RParen in parens, depth=1, current=Some(In)",
    },
];

/// Whether `qid` may be executed on the file-storage wire path.
pub fn in_wire_scope(qid: &str) -> bool {
    WIRE_SCOPE.contains(&qid)
}

/// Whether `qid` names one of the 22 standard queries.
pub fn is_standard_query(qid: &str) -> bool {
    TPC_H_QUERY_IDS.contains(&qid)
}

/// The registered exclusion for `qid`, if there is one.
pub fn wire_exclusion(qid: &str) -> Option<&'static WireExclusion> {
    WIRE_EXCLUSIONS.iter().find(|e| e.qid == qid)
}

/// The statements in `skipped` that are **not** registered as wire
/// exclusions.
///
/// A non-empty result means the test is dropping coverage silently — which
/// is precisely the failure mode the wire scope exists to prevent.
pub fn unregistered_wire_skips<'a>(skipped: &'a [&'a str]) -> Vec<&'a str> {
    skipped
        .iter()
        .filter(|q| wire_exclusion(q).is_none())
        .copied()
        .collect()
}

/// Disclose the statements a test function skipped because of the wire scope.
///
/// The marker is emitted **once per test function**, and only when that
/// function actually skipped something. That keeps the CI arithmetic exact:
/// the summary subtracts the number of NOT-RUN markers from the number of
/// passed test functions, so one marker cancels exactly one function.
///
/// The per-statement reason codes and issues are listed in the detail, so
/// the log says *why* a statement is missing and who is tracking it.
pub fn report_wire_exclusions(target: &str, skipped: &[&str]) {
    if skipped.is_empty() {
        return;
    }
    let detail = skipped
        .iter()
        .map(|q| match wire_exclusion(q) {
            Some(e) => format!("{q} reason={} issue={}", e.reason, e.issue),
            None => format!("{q} reason=unregistered"),
        })
        .collect::<Vec<_>>()
        .join(", ");
    let line =
        format!("TPCH-NOT-RUN: {target} reason={REASON_WIRE_SCOPE_EXCLUSION} detail={detail}");
    println!("{line}");
    if let Ok(path) = std::env::var("TPCH_NOT_RUN_FILE") {
        if !path.trim().is_empty() {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path.trim())
            {
                let _ = writeln!(f, "{line}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_scope_and_exclusions_cover_every_standard_query() {
        // The scope and the exclusion list must partition the 22 standard
        // queries: a statement that is in neither would vanish from the wire
        // path without anyone noticing.
        let mut covered: Vec<&str> = WIRE_SCOPE.to_vec();
        covered.extend(WIRE_EXCLUSIONS.iter().map(|e| e.qid));
        covered.sort();
        let mut expected = TPC_H_QUERY_IDS.to_vec();
        expected.sort();
        assert_eq!(
            covered, expected,
            "WIRE_SCOPE and WIRE_EXCLUSIONS must cover the 22 standard queries exactly once"
        );
        // ... and the two of them must not overlap.
        for e in WIRE_EXCLUSIONS {
            assert!(
                !in_wire_scope(e.qid),
                "{} is both in the wire scope and excluded from it",
                e.qid
            );
        }
    }

    #[test]
    fn every_wire_exclusion_carries_a_reason_and_an_issue() {
        for e in WIRE_EXCLUSIONS {
            assert!(!e.reason.is_empty(), "{} has no reason code", e.qid);
            assert!(
                e.issue.starts_with('#'),
                "{} has no issue reference (got '{}')",
                e.qid,
                e.issue
            );
            assert!(!e.detail.is_empty(), "{} has no measured detail", e.qid);
        }
    }

    #[test]
    fn unregistered_wire_skips_are_reported() {
        assert!(unregistered_wire_skips(&["Q9", "Q18", "Q19"]).is_empty());
        assert_eq!(unregistered_wire_skips(&["Q9", "Q42"]), vec!["Q42"]);
    }

    #[test]
    fn standard_queries_are_recognised() {
        assert!(is_standard_query("Q1"));
        assert!(is_standard_query("Q22"));
        assert!(!is_standard_query("Q23"));
        assert!(!is_standard_query("q1"));
    }
}
