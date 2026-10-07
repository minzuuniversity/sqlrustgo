//! #5113: every engine read path names the database, not just the
//! transaction.
//!
//! # The shape of the remaining gap
//!
//! `scan_for_reader*` was migrated to carry `reader_tx` (#4974/#4983) and
//! then to carry the connection's database (#5057/#5105) — but only at the
//! call sites that were convenient at the time. Fourteen remained without a
//! database, and they were not all equivalent to leaving it out:
//!
//! * `execute_single_join` built `(rows, info)` in one tuple where the
//!   `info` line named `current_db` and the `rows` line did not. The right
//!   table's COLUMNS came from the right database and its ROWS from
//!   another — a shape mismatch reported far from its cause.
//! * `table_status_row` had a `Some(db)` branch that resolved against the
//!   named database and a `None` branch that did not resolve against
//!   anything.
//! * The rest were plain reads in `&self` methods that could have called
//!   `self.session_db()` all along.
//!
//! # What these pin
//!
//! One case per path that had a distinct failure mode, plus a control
//! group: the whole point is that *every* read resolves against the
//! connection, so a test that passes only for the paths already wired
//! proves nothing.
//!
//! # Which of these actually discriminate
//!
//! Established by mutation, not by assertion-counting: reverting `src/` to
//! the baseline and re-running, then separately forcing every
//! `session_db()` in the tree to a database that does not exist.
//!
//! | test | baseline | forced-bad-db | verdict |
//! |------|----------|---------------|---------|
//! | `join_reads_rows_and_schema_from_the_same_database` | FAIL | pass | **discriminates** |
//! | `exists_subquery_reads_the_connections_table` | pass | FAIL | **discriminates** |
//! | `show_table_status_counts_the_connections_table` | pass | pass | inert |
//! | `insert_pk_cache_is_built_from_the_connections_table` | pass | pass | inert |
//! | `ordinary_statements_work_in_a_single_database` | pass | pass | control group |
//!
//! The two inert ones are `#[ignore]`d with the reason inline rather than
//! left as assertions that pass for reasons unrelated to what they claim to
//! pin.

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::{FileStorage, StorageEngine};
use sqlrustgo_types::Value;
use std::path::PathBuf;
use std::sync::Arc;

type Conn = ExecutionEngine<FileStorage>;

fn next_seq() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    SEQ.fetch_add(1, Ordering::SeqCst)
}

/// Two connections over one storage; `d1` and `d2` hold identically-named
/// tables whose rows are distinguishable by value.
fn two_connections() -> (Conn, Conn, PathBuf) {
    let dir = std::env::temp_dir().join(format!(
        "read_path_db_{}_{}",
        std::process::id(),
        next_seq()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let storage = Arc::new(RwLock::new(FileStorage::new(dir.clone()).unwrap()));
    (
        ExecutionEngine::new(Arc::clone(&storage)),
        ExecutionEngine::new(storage),
        dir,
    )
}

fn seed(x: &mut Conn) {
    x.execute("CREATE DATABASE d1").unwrap();
    x.execute("CREATE DATABASE d2").unwrap();
    for (db, tag) in [("d1", 1), ("d2", 2)] {
        x.execute(&format!("USE {db}")).unwrap();
        x.execute("CREATE TABLE t (id INT PRIMARY KEY, k INT)")
            .unwrap();
        x.execute(&format!("CREATE TABLE u (id INT PRIMARY KEY, k INT)"))
            .unwrap();
        x.execute(&format!("INSERT INTO t VALUES ({tag}, {tag})"))
            .unwrap();
        x.execute(&format!("INSERT INTO u VALUES ({tag}, {tag})"))
            .unwrap();
    }
}

fn rows(x: &mut Conn, sql: &str) -> Vec<Vec<Value>> {
    x.execute(sql).unwrap_or_else(|e| panic!("{sql}: {e}")).rows
}

fn int(v: &Value) -> i64 {
    match v {
        Value::Integer(n) => *n,
        other => panic!("expected integer, got {other:?}"),
    }
}

// ---------------------------------------------------------------------
// 1. execute_single_join — schema and rows from the same database
// ---------------------------------------------------------------------

/// A JOIN must not take its right-hand rows from another database.
///
/// The schema line in this tuple already named `current_db`. With the rows
/// line not naming it, `d1.t JOIN u` could return `d1.t`'s columns filled
/// with `d2.u`'s rows. The column count matches here (both tables have
/// two columns), so the join succeeds and returns values from the wrong
/// table — no error, wrong answer.
#[test]
fn join_reads_rows_and_schema_from_the_same_database() {
    let (mut a, mut b, dir) = two_connections();
    seed(&mut a);
    a.execute("USE d1").unwrap();
    b.execute("USE d2").unwrap();

    let out = rows(&mut a, "SELECT t.k FROM t JOIN u ON t.id = u.id");

    assert_eq!(out.len(), 1, "expected one joined row");
    assert_eq!(
        int(&out[0][0]),
        1,
        "the join returned a row from the other database (d2 holds k=2)"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------
// 2. SHOW TABLE STATUS — the unnamed branch
// ---------------------------------------------------------------------

/// `SHOW TABLE STATUS` without `FROM` counts the connection's table.
///
/// `#[ignore]`d: mutation-tested, this assertion does not discriminate.
/// Replacing every `session_db()` in the tree with a database that does not
/// exist leaves it passing, so it is not exercising the `None` branch of
/// `table_status_row` at all — the statement is served elsewhere. The fix
/// there is kept (it is the same argument threading as everywhere else),
/// but this test cannot witness it. See the file header.
#[test]
#[ignore = "does not reach table_status_row's None branch (mutation-proven)"]
fn show_table_status_counts_the_connections_table() {
    let (mut a, mut b, dir) = two_connections();
    seed(&mut a);
    a.execute("USE d1").unwrap();
    b.execute("USE d2").unwrap();

    let out = rows(&mut a, "SHOW TABLE STATUS LIKE 't'");
    assert!(
        !out.is_empty(),
        "SHOW TABLE STATUS returned nothing for the connection's own table"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------
// 3. A subquery / EXISTS path
// ---------------------------------------------------------------------

/// `EXISTS` must evaluate against the connection's table.
#[test]
fn exists_subquery_reads_the_connections_table() {
    let (mut a, mut b, dir) = two_connections();
    seed(&mut a);
    a.execute("USE d1").unwrap();
    b.execute("USE d2").unwrap();

    // d1.t has id=1; if the subquery ran against d2.t it would find id=2
    // and wrongly report "exists".
    let out = rows(
        &mut a,
        "SELECT k FROM t WHERE EXISTS (SELECT 1 FROM t WHERE id = 1)",
    );
    assert_eq!(
        int(&out[0][0]),
        1,
        "the subquery resolved against another database"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------
// 4. INSERT's primary-key cache
// ---------------------------------------------------------------------

/// The PK cache must be seeded from the connection's table.
///
/// `#[ignore]`d: mutation-tested, this assertion does not discriminate.
/// Forcing every read to a nonexistent database leaves it passing, so the
/// INSERT path here is not reaching `scan_for_reader_in_db` — the cache is
/// probably being bypassed and the duplicate check falling back to a scan
/// that already names the database. The `#[ignore]` keeps the scenario
/// ready for when that changes. See the file header.
#[test]
#[ignore = "does not reach the PK-cache scan (mutation-proven)"]
fn insert_pk_cache_is_built_from_the_connections_table() {
    let (mut a, mut b, dir) = two_connections();
    seed(&mut a);

    // Both databases now hold a row with id=1, so the caches collide only
    // if one is read from the other's table.
    a.execute("USE d1").unwrap();
    b.execute("USE d2").unwrap();
    a.execute("INSERT INTO t VALUES (7, 7)").unwrap();

    // The duplicate check must see d1.t's ids {1} — not d2.t's.
    let r = a.execute("INSERT INTO t VALUES (7, 8)");
    assert!(
        r.is_err(),
        "INSERT of a duplicate primary key was accepted — the cache was built \
         from another database"
    );

    // And a fresh id must still be accepted.
    a.execute("INSERT INTO t VALUES (8, 8)")
        .expect("a non-duplicate insert must succeed");
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------
// 5. Control group
// ---------------------------------------------------------------------

/// Ordinary statements on a single database still behave.
///
/// Every isolation test above has a `current_db` that happens to name a
/// real database, so a change that quietly fell back to it would pass all
/// of them. This one pins the ordinary case on both sides.
#[test]
fn ordinary_statements_work_in_a_single_database() {
    let (mut a, _b, dir) = two_connections();
    seed(&mut a);
    a.execute("USE d1").unwrap();

    assert_eq!(int(&rows(&mut a, "SELECT COUNT(*) FROM t")[0][0]), 1);
    assert_eq!(
        rows(&mut a, "SELECT t.k FROM t JOIN u ON t.id = u.id").len(),
        1
    );
    assert_eq!(
        int(&rows(&mut a, "SELECT k FROM t WHERE EXISTS (SELECT 1 FROM t)")[0][0]),
        1
    );
    a.execute("INSERT INTO t VALUES (9, 9)").unwrap();
    assert_eq!(int(&rows(&mut a, "SELECT COUNT(*) FROM t")[0][0]), 2);
    let _ = std::fs::remove_dir_all(&dir);
}
