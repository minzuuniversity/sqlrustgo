//! Regression tests for issue #4847 — Transaction semantics across 3 paths.
//!
//! Issue body (2026-09-07):
//! Three transaction-related regressions were found in v3.12.0 GA:
//!
//! A. CLI `sqlite --batch`: `-- comment line` before `BEGIN;` made
//!    the #4626 pre-COMMIT workaround fail to fire (because the
//!    post-split logical statement started with `--`, not `BEGIN`).
//!    The engine then rejected BEGIN with "Transaction already in
//!    progress" and the subsequent ROLLBACK rolled back the INSERT
//!    in the implicit transaction — data loss.
//!
//! B. serve + soak: BEGIN after an INSERT in the same connection
//!    returned "Transaction already in progress"; the next ROLLBACK
//!    cleared the table (rolled back both the INSERT and the UPDATE).
//!
//! C. serve + cli (per-connection new): BEGIN; UPDATE; ROLLBACK;
//!    SELECT showed the UPDATE un-rolled-back, then the next
//!    BEGIN; UPDATE; COMMIT returned "transaction already committed".
//!
//! The CLI fix below addresses A. (the most reproducible path) by
//! scanning every line of the post-split logical statement for a
//! leading BEGIN/COMMIT/ROLLBACK token. B. and C. are out of scope
//! for this regression test (deferred to issue #4847 follow-up).
//!
//! ── Assertion corrected 2026-10-07 ────────────────────────────────────
//! This file previously asserted the *engine-level* symptom of A:
//!
//!     let err = engine.execute("BEGIN").unwrap_err();
//!     assert!(err.to_string().contains("Transaction already in progress"));
//!
//! i.e. it pinned "BEGIN directly after a bare INSERT must fail".
//! That behaviour was itself fixed by Issue #4519 (commit d3457e9c7c,
//! 2026-09-29, the V312-85 implicit-TX drain), which is why the test has
//! been failing on `develop` since then: `INSERT` in autocommit mode is
//! committed, so a following explicit `BEGIN` is legitimate and must
//! succeed under MySQL / PostgreSQL semantics.
//!
//! AFP lesson applied: the test encoded a *defect*, not a contract, and
//! the test's own header comment says as much ("the engine's
//! begin_transaction would then reject with ..." — describing the bug).
//! The contract worth pinning is the one that actually protects the user:
//! a ROLLBACK must undo the work done inside its transaction. That is
//! asserted below, and the "already in progress" refusal is asserted
//! where it genuinely belongs — on a *nested* explicit BEGIN, see
//! `explicit_tx_not_silently_committed_4847.rs`.
//!
//! Verified: the failing assertion was NOT caused by any v4.1.0 fix PR;
//! `d3457e9c7c` predates PR #5064 and is an ancestor of it.
//!
//! The fix lives in the CLI's `dispatch_one` function
//! (crates/sqlrustgo-cli/src/sqlite_mode.rs); the CLI-level side is
//! verified by the sqlrustgo-cli batch tests.

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::FileStorage;
use std::sync::Arc;

fn create_engine(dir: &std::path::Path) -> ExecutionEngine<FileStorage> {
    let storage = FileStorage::new_with_buffer_config(dir.to_path_buf(), 100, false)
        .expect("FileStorage::new");
    ExecutionEngine::new(Arc::new(RwLock::new(storage)))
}

fn values(e: &mut ExecutionEngine<FileStorage>) -> Vec<String> {
    e.execute("SELECT a FROM t")
        .expect("SELECT a FROM t")
        .rows
        .iter()
        .map(|r| format!("{:?}", r[0]))
        .collect()
}

/// The #4847 data-loss scenario, end to end at the engine level.
///
/// `INSERT` (implicit autocommit) → `BEGIN` → `INSERT` → `ROLLBACK`.
///
/// Before Issue #4519 the third statement errored and left the session
/// in the implicit transaction, so the ROLLBACK undid *both* inserts.
/// The `BEGIN` must now succeed and the ROLLBACK must undo only the
/// second insert — the first one was already committed by autocommit.
#[test]
fn test_issue_4847_implicit_tx_drained_by_begin_preserves_committed_insert() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = create_engine(dir.path());
    engine.execute("CREATE TABLE t(a INT)").unwrap();
    engine.execute("INSERT INTO t VALUES (1)").unwrap();

    // Must succeed: the preceding INSERT was an implicit autocommit TX.
    engine
        .execute("BEGIN")
        .expect("BEGIN after an implicit autocommit INSERT must succeed (#4519)");

    engine.execute("INSERT INTO t VALUES (2)").unwrap();
    assert_eq!(values(&mut engine), ["Integer(1)", "Integer(2)"]);

    engine.execute("ROLLBACK").unwrap();

    // The #4847 regression was `[]` here — the ROLLBACK took the
    // autocommitted row 1 with it.
    let after = values(&mut engine);
    assert_eq!(
        after,
        ["Integer(1)"],
        "ROLLBACK must undo only the in-transaction INSERT; the \
         autocommitted row must survive (issue #4847 data loss)"
    );
}

/// `COMMIT` inside the explicit transaction persists, proving the
/// transaction opened by the drain above is a real transaction and not a
/// silently-committed no-op.
#[test]
fn test_issue_4847_explicit_tx_after_drain_commits_normally() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = create_engine(dir.path());
    engine.execute("CREATE TABLE t(a INT)").unwrap();
    engine.execute("INSERT INTO t VALUES (1)").unwrap();

    engine
        .execute("BEGIN")
        .expect("BEGIN after implicit autocommit");
    engine.execute("INSERT INTO t VALUES (2)").unwrap();
    engine.execute("COMMIT").unwrap();

    assert_eq!(values(&mut engine), ["Integer(1)", "Integer(2)"]);
}
