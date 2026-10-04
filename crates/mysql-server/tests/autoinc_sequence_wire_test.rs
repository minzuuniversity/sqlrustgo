//! #4945 / #4946 — autocommit INSERTs are acknowledged but never persisted.
//!
//! ## Root cause (located 2026-10-04 by instrumented probes)
//!
//! The production storage stack is
//! `ParallelWalStorage -> MvccStorage<FileStorage> -> FileStorage`, and
//! every autocommit INSERT lands in `FileStorage::insert_buffer` rather
//! than in `tables.rows`:
//!
//! ```text
//! FS_INSERT table=t in=1 in_tx=true buffer=true     x480   (probe output)
//! BUF_IN   table=t +1                               x480   (probe output)
//! BUF_FLUSH table=t                                       0   (probe output)
//! ```
//!
//! The buffer is only drained by `flush_buffer`, reached from
//! `flush_all_buffers` — and **`flush` is never called on the autocommit
//! path**. `crates/mysql-server/src/lib.rs` has exactly two `flush()`
//! call sites, `lib.rs:6251` and `lib.rs:6356`, both inside startup WAL
//! recovery. So the 480 acknowledged INSERTs stayed in `insert_buffer`,
//! which is pure memory: nothing reached disk, and a restart sees zero
//! rows. That is #4946 verbatim ("insert 480, kill, restart, COUNT(*)=0").
//!
//! There is a **second, independent** leg to this, which is why the
//! symptom looks like a partial loss rather than a total one.
//! `FileStorage::scan` does merge `insert_buffer` into its result
//! (`file_storage.rs:3782`), but the server reads through
//! `MvccStorage::scan` (`mvcc_storage.rs:272`), and that consults
//! `inner.scan()` **only when MVCC's key count has dropped** since the
//! last call — i.e. only when background GC may have evicted chains:
//!
//! ```rust
//! let needs = mvcc_count < cached_count || hit_count == 0;   // :293
//! ```
//!
//! Buffered rows that were never promoted into an MVCC chain are
//! therefore invisible unless that heuristic happens to fire. That is
//! why 341 of 480 rows are visible, with ids starting at 140 rather
//! than 1: the ids and the row count are the same defect seen from two
//! different read paths.
//!
//! ## Why the existing tests missed it
//!
//! | test | topology | result |
//! |---|---|---|
//! | `tests/blk1_auto_increment_concurrency_test.rs` | 8 threads, own engine, `MemoryStorage` | 480 rows, ids 1..480 — **passes** |
//! | `crates/storage/tests/b2_flush_...` | direct `FileStorage` + explicit `flush()` | passes |
//! | this file | 8 connections, autocommit, real wire | **fails** |
//!
//! Neither existing suite exercises the autocommit wire path against the
//! real backend, so BLK-1's "fix" was verified with a harness that
//! cannot observe this defect. #4950 §2.1's correction holds up: the
//! culprit is not `current_tx_id` (probe: the AUTO_INCREMENT allocator
//! issued 480 strictly increasing ids, 1..480) but the missing flush.

use sqlrustgo_mysql_client::{MySqlConnection, ResultSet};
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};

fn start() -> sqlrustgo_mysql_server::testing::EphemeralHandle {
    start_ephemeral(EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        bulk_insert_rows_per_flush: 10_000,
        load_infile_dir: None,
        server_threads: 8,
        storage: None,
        slow_query_log: None,
        metrics_port: None,
        wal_sync_mode_override: None,
    })
    .expect("ephemeral server starts")
}

fn connect(port: u16) -> MySqlConnection {
    let addr = format!("127.0.0.1:{}", port).parse().expect("addr");
    MySqlConnection::connect(&addr, "tester", "tester", "").expect("connect")
}

fn ids_of(conn: &mut MySqlConnection) -> Vec<i64> {
    match conn.execute("SELECT id FROM t").expect("select ids") {
        ResultSet::Select { rows, .. } => rows
            .iter()
            .filter_map(|r| r.first().and_then(|s| s.parse::<i64>().ok()))
            .collect(),
        other => panic!("expected select, got {:?}", other),
    }
}

fn count_of(conn: &mut MySqlConnection) -> i64 {
    match conn.execute("SELECT COUNT(*) FROM t").expect("count") {
        ResultSet::Select { rows, .. } => rows
            .first()
            .and_then(|r| r.first())
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(-1),
        other => panic!("expected select, got {:?}", other),
    }
}

/// Drive `threads` concurrent connections, each issuing `per` autocommit
/// INSERTs, and return (ids, client-side error count).
fn concurrent_autocommit_inserts(threads: usize, per: usize) -> (Vec<i64>, usize) {
    let handle = start();
    let port = handle.port;
    connect(port)
        .execute("CREATE TABLE t (id INTEGER AUTO_INCREMENT PRIMARY KEY, v INTEGER)")
        .expect("create table");

    let mut hs = Vec::new();
    for t in 0..threads {
        hs.push(std::thread::spawn(move || {
            let mut c = connect(port);
            let mut errs = 0usize;
            for i in 0..per {
                let sql = format!("INSERT INTO t (v) VALUES ({})", t * per + i);
                if c.execute(&sql).is_err() {
                    errs += 1;
                }
            }
            errs
        }));
    }
    let errs: usize = hs.into_iter().map(|h| h.join().expect("no panic")).sum();

    let mut c = connect(port);
    let ids = ids_of(&mut c);
    (ids, errs)
}

/// The regression in its reported form: 8 x 60 = 480 autocommit inserts
/// across 8 connections. Every one of them was acknowledged as
/// successful, yet rows go missing and the id range starts part-way up
/// instead of at 1.
#[test]
fn autocommit_inserts_do_not_lose_rows_or_reuse_ids() {
    let threads = 8usize;
    let per = 60usize;
    let total = threads * per;

    let (ids, errs) = concurrent_autocommit_inserts(threads, per);

    assert_eq!(
        errs, 0,
        "the field report had 0 errors — rows were acknowledged then lost, \
         which is worse than an error"
    );

    assert_eq!(
        ids.len(),
        total,
        "every acknowledged INSERT must be durable: expected {} rows, got {}",
        total,
        ids.len()
    );

    let mut sorted = ids.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        ids.len(),
        "AUTO_INCREMENT must not hand out the same id twice"
    );

    let expected: Vec<i64> = (1..=total as i64).collect();
    assert_eq!(
        sorted,
        expected,
        "AUTO_INCREMENT must be monotonic across connections: 480 inserts \
         must occupy 1..=480, but the range starts at {:?} and covers only \
         {} distinct values",
        sorted.first(),
        sorted.len()
    );
}

/// `COUNT(*)` must agree with the rows actually present.
///
/// #4946 recorded a `COUNT(*)` of 257 for 480 inserts, a number that
/// recurred across unrelated scales (480 rows, 25000 sysbench rows) and
/// looked like a fixed read-path cap rather than a storage limit. Here
/// we assert the two views agree; if this fails on its own, the two
/// symptoms of #4945/#4946 are not yet one root cause.
#[test]
fn count_star_agrees_with_selected_rows() {
    let handle = start();
    let port = handle.port;
    let mut c = connect(port);
    c.execute("CREATE TABLE t (id INTEGER AUTO_INCREMENT PRIMARY KEY, v INTEGER)")
        .expect("create table");

    for i in 0..200 {
        c.execute(&format!("INSERT INTO t (v) VALUES ({})", i))
            .expect("insert");
    }

    let ids = ids_of(&mut c);
    let count = count_of(&mut c);
    assert_eq!(
        count,
        ids.len() as i64,
        "COUNT(*) returned {} but {} rows are visible",
        count,
        ids.len()
    );
}

/// A single connection issuing many autocommit INSERTs must also produce
/// a contiguous range — this isolates the failure to the multi-connection
/// case, so a future regression can be attributed to whichever side
/// breaks.
#[test]
fn single_connection_autocommit_sequence_is_contiguous() {
    let handle = start();
    let port = handle.port;
    let mut c = connect(port);
    c.execute("CREATE TABLE t (id INTEGER AUTO_INCREMENT PRIMARY KEY, v INTEGER)")
        .expect("create table");

    let n = 200usize;
    for i in 0..n {
        c.execute(&format!("INSERT INTO t (v) VALUES ({})", i))
            .expect("insert");
    }
    let mut ids = ids_of(&mut c);
    ids.sort_unstable();
    assert_eq!(
        ids,
        (1..=n as i64).collect::<Vec<i64>>(),
        "one connection must still get 1..={}",
        n
    );
}

/// Source-level guard for the two `flush()` call sites that #4946
/// recorded.
///
/// The defect is an *absence*: `crates/mysql-server/src/lib.rs` calls
/// `flush()` only from startup WAL recovery (`lib.rs:6251` and
/// `lib.rs:6356`), never after a committed DML statement. No behavioural
/// test can assert on an absence, so this pins the fact that made the
/// absence survive review: both call sites sit inside the WAL-recovery
/// block.
///
/// When #4946 is fixed this test must be revisited — it documents the
/// pre-fix state on purpose, so that a future change to those line
/// numbers has to be a deliberate edit rather than an accident.
#[test]
fn the_only_flush_call_sites_are_wal_recovery() {
    let src = include_str!("../src/lib.rs");
    let lines: Vec<&str> = src.lines().collect();
    let hits: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.contains("file_storage.flush()"))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(
        hits.len(),
        2,
        "expected exactly 2 flush() call sites (both WAL recovery); found {} at lines {:?}",
        hits.len(),
        hits.iter().map(|i| i + 1).collect::<Vec<_>>()
    );
    for i in hits {
        // Look back far enough to leave the enclosing block. A flush
        // outside WAL recovery would mean the autocommit path started
        // persisting, which is what #4946 asks for.
        let start = i.saturating_sub(40);
        let ctx = lines[start..=i].join("\n");
        assert!(
            ctx.contains("recovery") || ctx.contains("recover") || ctx.contains("WAL"),
            "a flush() call appeared outside WAL recovery (line {}) — the \
             autocommit path may now be flushing, which is what #4946 asks \
             for; update this test when fixing it. Context:\n{}",
            i + 1,
            ctx
        );
    }
}
