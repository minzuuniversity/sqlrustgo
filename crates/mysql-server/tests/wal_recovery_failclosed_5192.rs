//! #5192 (IAIA-410 F9): startup must fail closed when WAL recovery drops
//! entries belonging to committed transactions.
//!
//! # The defect
//!
//! `INSERT ... COMMIT` returns OK to the client, so the write is
//! acknowledged as durable. If recovery then cannot replay it — as
//! happened whenever the table lived in a non-default database, because
//! WAL entries carry no database identifier — the data is gone.
//!
//! Startup used to log one `WARN` and continue. The server printed
//! "Ready to accept connections" and took new writes on top of a
//! silently truncated database.
//!
//! # What this file does and does not cover
//!
//! The decision (`guard_unrecoverable_commits`) is a pure function of
//! the recovery report, so it is tested directly. The end-to-end half —
//! that `run_server_v2` actually consults it on both recovery paths — is
//! pinned by asserting both call sites are still wired, using the same
//! technique `call_site_routes_skipped_into_the_ok_packet` uses elsewhere
//! in this crate for the LOAD DATA counter.
//!
//! This does NOT test that recovery can replay a multi-database WAL. That
//! is the separate root cause (WAL entries carry no database identifier)
//! and is still open; this file only stops its failure being silent.

use sqlrustgo_mysql_server::guard_unrecoverable_commits;
use sqlrustgo_storage::recovery_engine::RecoveryReport;

fn report(
    entries_total: usize,
    committed: usize,
    inserted: usize,
    skipped: usize,
) -> RecoveryReport {
    RecoveryReport {
        entries_total,
        committed_txns: committed,
        rows_inserted: inserted,
        skipped_entries: skipped,
        ..Default::default()
    }
}

/// The case that matters most: a guard that fired on clean recovery
/// would make the database unstartable for everyone.
#[test]
fn clean_recovery_starts() {
    let r = report(12, 3, 12, 0);
    assert!(
        guard_unrecoverable_commits(&r).is_ok(),
        "a clean recovery must never block startup"
    );
}

/// The exact numbers observed in the #5192 reproduction.
#[test]
fn skipped_committed_entries_block_startup() {
    let r = report(5, 1, 0, 2);
    let err = guard_unrecoverable_commits(&r).expect_err(
        "#5192: acknowledged writes that recovery dropped must block \
         startup, not merely be logged",
    );
    let msg = err.to_string();
    assert!(
        msg.contains("acknowledged") && msg.contains("lost"),
        "the refusal must say acknowledged data was lost, got: {msg}"
    );
}

/// Skipped entries with no committed transaction still mean the WAL did
/// not fully apply. The report cannot say whether those entries were
/// ever acknowledged, so the guard refuses and leaves the call to the
/// operator rather than deciding for them.
#[test]
fn skipped_entries_without_commits_also_block() {
    let r = report(4, 0, 0, 4);
    assert!(guard_unrecoverable_commits(&r).is_err());
}

/// A WAL with nothing in it must not block startup — an empty database
/// is not a damaged one.
#[test]
fn empty_wal_starts() {
    let r = report(0, 0, 0, 0);
    assert!(guard_unrecoverable_commits(&r).is_ok());
}

/// The escape hatch is opt-in. This asserts the default, so a test run
/// that happens to export the variable cannot silently redefine "safe".
#[test]
fn escape_hatch_defaults_to_off() {
    assert!(
        !matches!(
            std::env::var("SQLRUSTGO_ALLOW_DATA_LOSS").as_deref(),
            Ok("1")
        ),
        "SQLRUSTGO_ALLOW_DATA_LOSS must not be set while running these tests"
    );
}

/// Both recovery call sites must remain wired to the guard. A guard
/// consulted on only one of the two startup paths still loses data on
/// the other.
#[test]
fn both_recovery_call_sites_are_guarded() {
    let src = include_str!("../src/lib.rs");
    let guarded = src.matches("guard_unrecoverable_commits(&report)?").count();
    assert_eq!(
        guarded, 2,
        "#5192: both WAL recovery call sites must call \
         guard_unrecoverable_commits — found {guarded}"
    );
}
