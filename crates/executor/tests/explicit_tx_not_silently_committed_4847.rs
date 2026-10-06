//! Issue #4847 follow-up (2026-10-07): an open EXPLICIT transaction must
//! never be silently committed by a second `BEGIN`.
//!
//! Defect
//! ------
//! `ExecutionEngine::begin_transaction` drained whatever transaction was
//! open before starting the new one, unconditionally. The drain exists
//! for a real reason — Issue #4519 / V312-85 added it so that the common
//! `INSERT ...; BEGIN; ...` script shape does not fail with "Transaction
//! already in progress" (the INSERT opened an implicit autocommit TX that
//! nobody committed).
//!
//! But the drain did not distinguish the two kinds of open transaction.
//! With an EXPLICIT transaction still open, the drain committed it. So:
//!
//! ```text
//! BEGIN; INSERT INTO t VALUES (1); BEGIN; ROLLBACK;
//! ```
//!
//! the second `BEGIN` committed the INSERT, and the `ROLLBACK` had
//! nothing left to undo. The user's rollback boundary was destroyed by a
//! statement that is a no-op or an error in every dialect this engine
//! targets (MySQL 1568 "Transaction already started", PostgreSQL
//! "there is already a transaction in progress", SQLite "cannot start a
//! transaction within a transaction"). No dialect commits an open
//! transaction because the user typed BEGIN again.
//!
//! Contributing factor: `TxSession::is_explicit_transaction` existed in
//! the struct and was initialised to `false` at all eight construction
//! sites, but was never assigned `true` and never read anywhere in
//! `src/`. The field was dead, and `tests/integration/sql/
//! v312_77_explicit_tx_semantics_test.rs` claimed Path D was "Fixed by
//! adding `is_explicit_transaction` flag". It was not — Path D was
//! actually handled by the separate `started_implicit` boolean.
//!
//! Fix
//! ---
//! `begin_transaction` now consults `is_explicit_transaction`. When an
//! explicit transaction is still open it returns
//! `ExecutionError("Transaction already in progress")` and commits
//! nothing. The flag is set on a successful explicit BEGIN and cleared by
//! COMMIT, ROLLBACK.
//!
//! Reference dialect behaviour was checked against the MySQL course
//! material this engine targets (`tests/compat/bustubx_edu_b_track/`);
//! refusing is the conservative choice because it never destroys work.

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::FileStorage;
use std::sync::Arc;

fn engine(dir: &std::path::Path) -> ExecutionEngine<FileStorage> {
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

fn seeded() -> (tempfile::TempDir, ExecutionEngine<FileStorage>) {
    let dir = tempfile::tempdir().unwrap();
    let mut e = engine(dir.path());
    e.execute("CREATE TABLE t(a INT)").unwrap();
    (dir, e)
}

/// Same, but with a PRIMARY KEY so a duplicate INSERT fails.
///
/// Why a *failing* DML is needed: a successful implicit DML calls
/// `commit_implicit_dml_tx`, which resets `current_tx_id` to `None`
/// before it returns. `begin_transaction` therefore sees
/// `current_tx_id == None` and short-circuits on `(false, _) => true`
/// without ever reading `is_explicit_transaction`. A *failed* implicit
/// DML returns through `?` without that commit, leaving `current_tx_id`
/// set — that is the only way a stale `is_explicit_transaction` flag
/// becomes observable, and it is also the situation the Issue #4519
/// drain was originally written for.
fn seeded_pk() -> (tempfile::TempDir, ExecutionEngine<FileStorage>) {
    let dir = tempfile::tempdir().unwrap();
    let mut e = engine(dir.path());
    e.execute("CREATE TABLE t(a INT PRIMARY KEY)").unwrap();
    (dir, e)
}

/// Fail a duplicate-key INSERT. Returns the error text so callers can
/// assert the leak-inducing failure actually happened.
fn leak_implicit_tx(e: &mut ExecutionEngine<FileStorage>, key: &str) -> String {
    let err = e
        .execute(&format!("INSERT INTO t VALUES ({})", key))
        .expect_err("duplicate PRIMARY KEY INSERT must fail");
    err.to_string()
}

/// The core regression: `ROLLBACK` after a nested `BEGIN` must still undo
/// the work. Before the fix this returned `[Integer(1)]`; the INSERT had
/// been committed by the second BEGIN.
#[test]
fn nested_begin_must_not_commit_the_open_explicit_transaction() {
    let (_dir, mut e) = seeded();

    e.execute("BEGIN").unwrap();
    e.execute("INSERT INTO t VALUES (1)").unwrap();

    // The nested BEGIN is refused, and it commits nothing.
    let err = e
        .execute("BEGIN")
        .expect_err("nested explicit BEGIN must be refused, not silently committed");
    assert!(
        err.to_string().contains("Transaction already in progress"),
        "unexpected error text: {}",
        err
    );

    e.execute("ROLLBACK").unwrap();

    let after = values(&mut e);
    assert_eq!(
        after,
        Vec::<String>::new(),
        "ROLLBACK must undo the INSERT that the refused nested BEGIN left open"
    );
}

/// The refusal must not corrupt the session: after the error the same
/// transaction is still usable and a later COMMIT still works.
#[test]
fn transaction_survives_a_refused_nested_begin() {
    let (_dir, mut e) = seeded();

    e.execute("BEGIN").unwrap();
    assert!(e.execute("BEGIN").is_err(), "nested BEGIN must error");

    // Still inside the original transaction.
    e.execute("INSERT INTO t VALUES (1)").unwrap();
    e.execute("COMMIT").unwrap();

    assert_eq!(values(&mut e), ["Integer(1)"]);
}

/// The drain must still fire for an IMPLICIT autocommit transaction.
/// Without this the fix would be "just start rejecting again", i.e. a
/// straight regression of Issue #4519 / V312-85.
#[test]
fn implicit_autocommit_transaction_is_still_drained() {
    let (_dir, mut e) = seeded();

    e.execute("INSERT INTO t VALUES (1)").unwrap();
    e.execute("BEGIN")
        .expect("BEGIN after an implicit autocommit INSERT must succeed (#4519)");
    e.execute("INSERT INTO t VALUES (2)").unwrap();
    e.execute("ROLLBACK").unwrap();

    // Row 1 was autocommitted before BEGIN and must survive.
    assert_eq!(values(&mut e), ["Integer(1)"]);
}

/// COMMIT clears the explicit flag even when a subsequent *failed*
/// implicit DML leaks a transaction.
///
/// Mutation evidence: neither "COMMIT then BEGIN" nor "COMMIT then a
/// *successful* implicit DML then BEGIN" catches a missing reset — in
/// both cases `current_tx_id` is `None` at BEGIN time and the `(false,
/// _)` arm short-circuits before the flag is ever read. Measured
/// (`zz_probe_leak`): with the COMMIT reset removed, this exact sequence
/// makes `BEGIN` return
/// `ExecutionError("Transaction already in progress")`, which is wrong —
/// the open transaction is implicit, so the #4519 drain applies.
#[test]
fn begin_after_commit_plus_leaked_tx_is_not_treated_as_nested() {
    let (_dir, mut e) = seeded_pk();
    e.execute("INSERT INTO t VALUES (1)").unwrap();

    e.execute("BEGIN").unwrap();
    e.execute("INSERT INTO t VALUES (2)").unwrap();
    e.execute("COMMIT").unwrap();

    assert!(
        leak_implicit_tx(&mut e, "1").contains("Duplicate entry"),
        "the leak-inducing failure must actually fail"
    );

    e.execute("BEGIN")
        .expect("BEGIN after COMMIT + a leaked IMPLICIT tx must drain, not be refused as nested");
    e.execute("INSERT INTO t VALUES (3)").unwrap();
    e.execute("COMMIT").unwrap();

    assert_eq!(values(&mut e), ["Integer(1)", "Integer(2)", "Integer(3)"]);
}

/// Symmetric to the COMMIT case: ROLLBACK must clear the flag too.
#[test]
fn begin_after_rollback_plus_leaked_tx_is_not_treated_as_nested() {
    let (_dir, mut e) = seeded_pk();
    e.execute("INSERT INTO t VALUES (1)").unwrap();

    e.execute("BEGIN").unwrap();
    e.execute("INSERT INTO t VALUES (2)").unwrap();
    e.execute("ROLLBACK").unwrap();

    assert!(leak_implicit_tx(&mut e, "1").contains("Duplicate entry"));

    e.execute("BEGIN")
        .expect("BEGIN after ROLLBACK + a leaked IMPLICIT tx must drain, not be refused as nested");
    e.execute("INSERT INTO t VALUES (3)").unwrap();
    e.execute("COMMIT").unwrap();

    assert_eq!(values(&mut e), ["Integer(1)", "Integer(3)"]);
}

/// DML inside an explicit transaction must not flip the flag: the
/// implicit-DML wrapper opens and commits its own TX around the
/// statement, and it must leave the explicit one alone.
#[test]
fn implicit_dml_inside_explicit_tx_does_not_clear_the_flag() {
    let (_dir, mut e) = seeded();

    e.execute("BEGIN").unwrap();
    e.execute("INSERT INTO t VALUES (1)").unwrap();
    e.execute("UPDATE t SET a = 99 WHERE a = 1").unwrap();

    // Still explicit → still refused.
    let err = e
        .execute("BEGIN")
        .expect_err("nested explicit BEGIN must be refused");
    assert!(
        err.to_string().contains("Transaction already in progress"),
        "unexpected error text: {}",
        err
    );

    e.execute("ROLLBACK").unwrap();
    assert_eq!(values(&mut e), Vec::<String>::new());
}
