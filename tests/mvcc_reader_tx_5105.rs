//! #5105 regression: the engine's read path must carry BOTH the
//! database and the reading transaction.
//!
//! # What #5105 changed
//!
//! `scan_for_reader_with` took only the storage's shared `current_db` for
//! table resolution and passed `reader_tx` to `storage.scan_in`. Splitting
//! the database out per connection introduced
//! `scan_for_reader_in_db(storage, db, table)`, and it calls
//! `storage.scan_in_db(db, table)` — which takes **no transaction**.
//!
//! `reader_tx` is still computed on the line above and then thrown away:
//!
//! ```text
//! let reader_tx = self.reader_tx();
//! storage.scan_in_db(db, table)     // reader_tx unused
//! ```
//!
//! # Why that matters
//!
//! `scan_in_db`'s default is `self.scan(table)`, which resolves the
//! transaction from `inner.current_tx_id()` — one storage-wide value
//! holding whichever connection wrote last. `MvccStorage` overrides
//! `scan_in` precisely so reads do NOT go through that value
//! (`#4983`/`#4974`: an uncommitted write must not be visible to a reader
//! that did not make it). Routing the SELECT path around that override
//! puts production reads back on the shared value.
//!
//! The server wraps `FileStorage` in `MvccStorage`
//! (`mysql-server/src/lib.rs`, `V400-MVCC-ENABLE`), so this is the
//! production path, not a corner.
//!
//! # Note on the existing storage-layer tests
//!
//! `tx_isolation_and_escape_hatch_test.rs` calls `scan_in` on the trait
//! directly and stays green. That is exactly why the regression is easy
//! to miss: the isolation contract is still verified one layer below the
//! code that broke. These tests go through `ExecutionEngine`.
//!
//! # What these pin
//!
//! 1. A reader in its own transaction does not see another connection's
//!    uncommitted rows.
//! 2. A reader still sees committed rows.
//! 3. A connection with no transaction sees committed rows.
//! 4. The database half still works: the same transaction isolation holds
//!    in a non-default database, which is what `scan_in_tx_db` exists for.

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::{FileStorage, MvccStorage, StorageEngine};
use sqlrustgo_types::Value;
use std::path::PathBuf;
use std::sync::Arc;

/// Two engines over one `MvccStorage<FileStorage>` — the production stack
/// (`V400-MVCC-ENABLE`).
fn two_connections() -> (
    ExecutionEngine<MvccStorage<FileStorage>>,
    ExecutionEngine<MvccStorage<FileStorage>>,
    PathBuf,
) {
    let dir = std::env::temp_dir().join(format!(
        "mvcc_reader_tx_{}_{}",
        std::process::id(),
        next_seq()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let inner = MvccStorage::new(FileStorage::new(dir.clone()).unwrap());
    inner.rebuild_from_inner().expect("rebuild MVCC chain");
    let storage = Arc::new(RwLock::new(inner));
    (
        ExecutionEngine::new(Arc::clone(&storage)),
        ExecutionEngine::new(storage),
        dir,
    )
}

/// A per-call counter: naming every directory after the process id makes
/// parallel tests delete each other's data. (This is the defect #5105
/// fixed in its own copy of this setup.)
fn next_seq() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    SEQ.fetch_add(1, Ordering::SeqCst)
}

fn seed(x: &mut ExecutionEngine<MvccStorage<FileStorage>>) {
    x.execute("CREATE DATABASE d1").unwrap();
    x.execute("USE d1").unwrap();
    x.execute("CREATE TABLE t (id INT PRIMARY KEY)").unwrap();
    x.execute("INSERT INTO t VALUES (1)").unwrap();
}

/// Put B in the same database as the table.
///
/// Required, not incidental: a connection that has not run `USE d1` cannot
/// resolve `t` to `d1.t`, and must not be able to read it. Omitting this
/// made these tests pass for the wrong reason — the data side was reading
/// through the inner storage's shared `current_db`, which happened to name
/// `d1` because A had selected it. The read was therefore correct by
/// accident, and the test proved nothing about isolation.
fn join(x: &mut ExecutionEngine<MvccStorage<FileStorage>>) {
    x.execute("USE d1").unwrap();
}

fn count(x: &mut ExecutionEngine<MvccStorage<FileStorage>>) -> usize {
    let rows = x.execute("SELECT COUNT(*) FROM t").unwrap().rows;
    match &rows[0][0] {
        Value::Integer(n) => *n as usize,
        other => panic!("expected a count, got {other:?}"),
    }
}

// ---------------------------------------------------------------------
// The regression
// ---------------------------------------------------------------------

/// An uncommitted write must not be visible to another connection.
///
/// A opens a transaction and inserts; B has none. Before #5105, B's read
/// went through `MvccStorage::scan_in(B.reader_tx)` and was filtered to
/// versions B may see. After it, B's read goes through `scan_in_db` →
/// `scan` → `inner.current_tx_id()`, which after A's write names A's
/// transaction.
#[test]
fn uncommitted_write_is_invisible_to_another_connection() {
    let (mut a, mut b, dir) = two_connections();
    seed(&mut a);
    join(&mut b);

    a.execute("BEGIN").unwrap();
    a.execute("INSERT INTO t VALUES (2)").unwrap();

    assert_eq!(
        count(&mut b),
        1,
        "connection B saw A's UNCOMMITTED row — the engine read path is no \
         longer carrying reader_tx"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The reader must still see committed rows.
///
/// The other half of the contract: a fix that carried `reader_tx` but
/// dropped committed data would pass the test above.
#[test]
fn committed_write_is_visible_to_another_connection() {
    let (mut a, mut b, dir) = two_connections();
    seed(&mut a);
    join(&mut b);

    a.execute("BEGIN").unwrap();
    a.execute("INSERT INTO t VALUES (2)").unwrap();
    a.execute("COMMIT").unwrap();

    assert_eq!(
        count(&mut b),
        2,
        "connection B did not see A's committed row"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A connection with no transaction of its own reads committed data.
///
/// `reader_tx()` returns 0 when `tx_session.current_tx_id` is `None`. That
/// value must reach the storage as an argument rather than being replaced
/// by whatever the storage last saw.
#[test]
fn autocommit_reader_sees_committed_rows() {
    let (mut a, mut b, dir) = two_connections();
    seed(&mut a);
    join(&mut b);

    a.execute("BEGIN").unwrap();
    a.execute("INSERT INTO t VALUES (2)").unwrap();
    a.execute("COMMIT").unwrap();

    assert_eq!(count(&mut a), 2, "the writer should see its own commit");
    assert_eq!(count(&mut b), 2, "the autocommit reader missed the commit");
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------
// The other half: the database still has to work
// ---------------------------------------------------------------------

/// Same isolation, in a non-default database.
///
/// `scan_in_tx_db`'s own doc comment says `MvccStorage` needs both — `db`
/// picks the table namespace, `reader_tx` picks which versions are
/// visible — but its default implementation does `let _ = db;` and no
/// storage overrides it. This pins the half that does work today, so a
/// fix for the transaction cannot quietly break the database.
#[test]
fn isolation_holds_inside_a_non_default_database() {
    let (mut a, mut b, dir) = two_connections();
    seed(&mut a);
    join(&mut b);

    a.execute("BEGIN").unwrap();
    a.execute("INSERT INTO t VALUES (2)").unwrap();

    assert_eq!(
        count(&mut b),
        1,
        "in d1, connection B saw A's uncommitted row"
    );
    a.execute("COMMIT").unwrap();
    assert_eq!(count(&mut b), 2, "in d1, connection B missed A's commit");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Two databases, same table name, different rows — and the read must not
/// mix them.
///
/// This exists because the other three tests all live in `d1`, where the
/// inner engine's shared `current_db` happens to name the right database.
/// That coincidence made them blind to the `db` half of the fix: mutation
/// M-H (reading the inner engine by bare name, ignoring `db`) survived all
/// of them.
///
/// Here `B` selects `d2` last, so the inner engine's `current_db` is `d2`
/// for the whole test. If `A` — who is in `d1` — still reads `d1.t`, then
/// every row of the inner read is attributed to the right database.
#[test]
fn same_table_name_in_two_databases_does_not_mix() {
    let (mut a, mut b, dir) = two_connections();

    // d1.t holds one row, d2.t holds two, so "3" can only mean a merge.
    a.execute("CREATE DATABASE d1").unwrap();
    a.execute("USE d1").unwrap();
    a.execute("CREATE TABLE t (id INT PRIMARY KEY)").unwrap();
    a.execute("INSERT INTO t VALUES (1)").unwrap();

    a.execute("CREATE DATABASE d2").unwrap();
    a.execute("USE d2").unwrap();
    a.execute("CREATE TABLE t (id INT PRIMARY KEY)").unwrap();
    a.execute("INSERT INTO t VALUES (2)").unwrap();
    a.execute("INSERT INTO t VALUES (3)").unwrap();

    // Put both connections in place, then let B select LAST so the inner
    // engine's shared `current_db` names d2. A must still read d1.t.
    // (`USE` writes both the session value and the storage mirror, so
    // whoever runs it last decides the mirror — hence B last.)
    a.execute("USE d1").unwrap();
    b.execute("USE d2").unwrap();

    assert_eq!(count(&mut a), 1, "A (in d1) did not see exactly d1.t");
    assert_eq!(count(&mut b), 2, "B (in d2) did not see exactly d2.t");
    let _ = std::fs::remove_dir_all(&dir);
}
