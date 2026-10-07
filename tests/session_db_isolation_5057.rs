//! #5057: the SELECT path must resolve tables against the database the
//! CONNECTION selected, not against whatever another connection's `USE`
//! wrote to the shared storage most recently.
//!
//! # The defect this pins
//!
//! `ExecutionEngine` had no per-connection database. The SELECT path read
//! the storage's process-wide `current_db` at the top of each statement
//! (`engine_select.rs`), and every downstream lookup threaded that one
//! string through. `engine_dml.rs` took a statement-level snapshot of the
//! same shared value.
//!
//! The storage is shared by every connection — `mysql-server` hands the
//! same `Arc<RwLock<FileStorage>>` to each `handle_connection`. So with two
//! clients in different databases, the statement on one resolved its table
//! names against the other's database. Measured at 53.76% misdirected
//! statements under a concurrent switch (that figure was recorded while
//! the read still pointed at `storage.current_db`).
//!
//! # What the fix is
//!
//! One `ExecutionEngine` is constructed per connection, so
//! `ExecutionEngine::session_db` *is* the per-connection value. `USE`
//! writes it; SELECT and DML read it once at the top of the statement,
//! which also makes it a statement-level snapshot.
//!
//! `storage.current_db` is still written alongside it, because paths that
//! have not been parameterised yet resolve against it (`FileStorage::tbl`,
//! the DDL rename/truncate paths). The two are kept in agreement; this
//! batch does not switch that off.
//!
//! # What these pin
//!
//! 1. Two engines over one storage, each in its own database, see their
//!    own rows — the actual production shape.
//! 2. Interleaved `USE` on one connection never redirects the other.
//! 3. Concurrent SELECTs from both connections stay isolated.
//! 4. Control group: the shared `storage.current_db` is still kept in
//!    agreement with the session value, so un-parameterised paths do not
//!    drift.
//!
//! ---------------------------------------------------------------------
//! STATUS: 4 of these 5 tests FAIL on develop/v4.1.0 and are `#[ignore]`d.
//! ---------------------------------------------------------------------
//!
//! They are the acceptance criteria for the next #5057 batch, not a
//! regression in the current tree. Run them with:
//!
//!     cargo test --all-features --test session_db_isolation_5057 -- --ignored
//!
//! Measured on `develop/v4.1.0` (496531a00fe6): the four ignored tests fail
//! with `connection A (in d1) read the wrong database: left [2], right [1]`.
//! The fifth passes because the storage mirror does work — it is the
//! mirror being *consulted* that is the defect, which is why it stays live.
//!
//! Do not "fix" them by loosening what they assert. The failure output is
//! the specification. See `SESSION_DB_READPATH_5057_2026-10-07.md`.

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::{FileStorage, StorageEngine};
use std::path::PathBuf;
use std::sync::Arc;

/// Two engines over ONE storage — the production topology.
///
/// The old tests each built their own engine, so they could not express the
/// defect: with a single engine, `USE` and the statements that follow it
/// always agree with each other.
fn two_connections() -> (
    ExecutionEngine<FileStorage>,
    ExecutionEngine<FileStorage>,
    Arc<RwLock<FileStorage>>,
    PathBuf,
) {
    let dir = std::env::temp_dir().join(format!("session_db_5057_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let storage = Arc::new(RwLock::new(FileStorage::new(dir.clone()).unwrap()));
    (
        ExecutionEngine::new(Arc::clone(&storage)),
        ExecutionEngine::new(Arc::clone(&storage)),
        storage,
        dir,
    )
}

/// Seed `d1.t` and `d2.t`, each with rows whose ids identify the database.
fn seed(a: &mut ExecutionEngine<FileStorage>) {
    a.execute("CREATE DATABASE d1").unwrap();
    a.execute("CREATE DATABASE d2").unwrap();
    for (db, id) in [("d1", 1), ("d2", 2)] {
        a.execute(&format!("USE {db}")).unwrap();
        a.execute("CREATE TABLE t (id INT PRIMARY KEY)").unwrap();
        a.execute(&format!("INSERT INTO t VALUES ({id})")).unwrap();
    }
}

fn ids(x: &mut ExecutionEngine<FileStorage>) -> Vec<sqlrustgo_types::Value> {
    let mut out = Vec::new();
    for row in x.execute("SELECT id FROM t").unwrap().rows {
        out.push(row[0].clone());
    }
    out
}

// ---------------------------------------------------------------------
// 1. The defect: two connections, one shared storage
// ---------------------------------------------------------------------

/// Each connection must read its own database's table.
///
/// Before the fix, connection B's `USE d2` wrote the shared
/// `storage.current_db`, so connection A's next SELECT resolved `t`
/// against `d2` and returned d2's row.
#[test]
#[ignore = "#5057 next batch: executor still reads the shared storage.current_db"]
fn select_reads_the_connections_own_database() {
    let (mut a, mut b, storage, dir) = two_connections();
    seed(&mut a);

    a.execute("USE d1").unwrap();
    b.execute("USE d2").unwrap();

    assert_eq!(
        ids(&mut a),
        vec![sqlrustgo_types::Value::Integer(1)],
        "connection A (in d1) read the wrong database"
    );
    assert_eq!(
        ids(&mut b),
        vec![sqlrustgo_types::Value::Integer(2)],
        "connection B (in d2) read the wrong database"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Interleaved `USE` on one connection must never redirect the other.
///
/// This is the alternation that a single shared value cannot survive: the
/// pattern repeats the read/switch cycle rather than switching once and
/// reading once.
#[test]
#[ignore = "#5057 next batch: executor still reads the shared storage.current_db"]
fn interleaved_use_does_not_redirect_the_other_connection() {
    let (mut a, mut b, storage, dir) = two_connections();
    seed(&mut a);

    a.execute("USE d1").unwrap();
    b.execute("USE d2").unwrap();

    for round in 0..3 {
        assert_eq!(
            ids(&mut a),
            vec![sqlrustgo_types::Value::Integer(1)],
            "round {round}: connection A was redirected"
        );
        // B switches away and back; A must not notice either move.
        b.execute("USE d1").unwrap();
        b.execute("USE d2").unwrap();
        assert_eq!(
            ids(&mut a),
            vec![sqlrustgo_types::Value::Integer(1)],
            "round {round}: connection A followed B's USE"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------
// 2. Concurrent statements from both connections
// ---------------------------------------------------------------------

/// Two threads, one connection each, reading at the same time.
///
/// The storage lock serialises the two reads; what is being pinned is that
/// serialisation does not also share the *answer*. If the SELECT path read
/// the shared `current_db`, one thread's statement would resolve against
/// the other's database.
#[test]
#[ignore = "#5057 next batch: executor still reads the shared storage.current_db"]
fn concurrent_reads_stay_isolated() {
    let (mut a, mut b, storage, dir) = two_connections();
    seed(&mut a);
    a.execute("USE d1").unwrap();
    b.execute("USE d2").unwrap();

    let a = std::thread::spawn(move || {
        let mut seen = Vec::new();
        for _ in 0..50 {
            seen.push(ids(&mut a));
        }
        seen
    });
    let b = std::thread::spawn(move || {
        let mut seen = Vec::new();
        for _ in 0..50 {
            seen.push(ids(&mut b));
        }
        seen
    });

    let want_a = vec![sqlrustgo_types::Value::Integer(1)];
    let want_b = vec![sqlrustgo_types::Value::Integer(2)];
    for r in a.join().unwrap() {
        assert_eq!(r, want_a, "connection A saw another database's rows");
    }
    for r in b.join().unwrap() {
        assert_eq!(r, want_b, "connection B saw another database's rows");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------
// 3. Control group
// ---------------------------------------------------------------------

/// The shared `storage.current_db` must still agree with each
/// connection's session value.
///
/// Not an end in itself — it is what keeps the paths that have not been
/// parameterised yet (`FileStorage::tbl`, DDL rename/truncate) pointing at
/// the right database. If a future change made `USE` update only
/// `session_db`, those paths would silently start resolving against
/// whichever connection wrote last, and every SELECT test here would still
/// pass because they go through `*_in_db`.
#[test]
fn shared_current_db_still_agrees_with_the_last_use() {
    let (mut a, mut b, storage, dir) = two_connections();
    seed(&mut a);

    a.execute("USE d1").unwrap();
    assert_eq!(
        storage.read().current_db(),
        "d1",
        "connection A's USE did not reach the storage mirror"
    );

    b.execute("USE d2").unwrap();
    assert_eq!(
        storage.read().current_db(),
        "d2",
        "connection B's USE did not reach the storage mirror"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The mirror reflects the LAST `USE`, and a connection still reads its
/// own database regardless of which was last.
///
/// Both halves matter: the first is what un-parameterised paths need, the
/// second is the actual isolation guarantee. Asserting only the first
/// would pass with the defect in place.
#[test]
#[ignore = "#5057 next batch: executor still reads the shared storage.current_db"]
fn last_use_wins_on_the_mirror_but_not_on_the_connection() {
    let (mut a, mut b, storage, dir) = two_connections();
    seed(&mut a);
    a.execute("USE d1").unwrap();
    b.execute("USE d2").unwrap(); // B writes the mirror last

    assert_eq!(
        storage.read().current_db(),
        "d2",
        "the mirror should hold the last USE"
    );
    assert_eq!(
        ids(&mut a),
        vec![sqlrustgo_types::Value::Integer(1)],
        "connection A must not follow the mirror"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
