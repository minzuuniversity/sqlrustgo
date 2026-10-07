//! #5106 incidental finding: multi-table `UPDATE` is a no-op that reports
//! success.
//!
//! Not part of the database-scoping work this file was written for; kept
//! because the scoping tests had to be arranged around it, and the
//! arrangement should not outlive the reason.
//!
//! Measured on `develop/v4.1.0` (`f39684d6e1`), single database, no
//! transactions, `FileStorage`:
//!
//! ```text
//! INSERT INTO t1 VALUES (1, 10); INSERT INTO t2 VALUES (1, 10);
//!
//! UPDATE t1 SET k = 55                -> affected=1   t1.k becomes 55   OK
//! UPDATE t1, t2 SET k = 99            -> affected=1   t1.k stays  55   WRONG
//! UPDATE t1, t2 SET k = 77 WHERE id=1 -> affected=1   both stay        WRONG
//! DELETE t1, t2 FROM t1, t2           -> affected=2   both emptied    OK
//! ```
//!
//! The UPDATE reports one affected row and changes nothing, so a caller
//! checking `affected_rows` concludes the statement ran. That is worse
//! than an error: a silent no-op is indistinguishable from a WHERE clause
//! that matched nothing, which is a normal outcome.

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::{FileStorage, StorageEngine};
use std::sync::Arc;

#[test]
fn probe_multi_table_update() {
    let dir = std::env::temp_dir().join("probe_mt_update");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let st = Arc::new(RwLock::new(FileStorage::new(dir).unwrap()));
    let mut x = ExecutionEngine::new(st);

    x.execute("CREATE DATABASE d1").unwrap();
    x.execute("USE d1").unwrap();
    x.execute("CREATE TABLE t1 (id INT PRIMARY KEY, k INT)")
        .unwrap();
    x.execute("CREATE TABLE t2 (id INT PRIMARY KEY, k INT)")
        .unwrap();
    x.execute("INSERT INTO t1 VALUES (1, 10)").unwrap();
    x.execute("INSERT INTO t2 VALUES (1, 10)").unwrap();

    for sql in [
        "SELECT k FROM t1",
        "UPDATE t1 SET k = 55", // single table
        "SELECT k FROM t1",
        "UPDATE t1, t2 SET k = 99", // multi table, no WHERE
        "SELECT k FROM t1",
        "SELECT k FROM t2",
        "UPDATE t1, t2 SET k = 77 WHERE id = 1", // multi table with WHERE
        "SELECT k FROM t1",
        "SELECT k FROM t2",
        "DELETE t1, t2 FROM t1, t2", // multi table delete
        "SELECT COUNT(*) FROM t1",
        "SELECT COUNT(*) FROM t2",
    ] {
        match x.execute(sql) {
            Ok(r) => println!(
                "{sql:<42} -> rows={:?} affected={}",
                r.rows, r.affected_rows
            ),
            Err(e) => println!("{sql:<42} -> ERR {e}"),
        }
    }
    let _ = std::fs::remove_dir_all(std::env::temp_dir().join("probe_mt_update"));
}
