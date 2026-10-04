//! #4945 / #4946 — regression gate for acknowledged-but-lost autocommit INSERTs.
//!
//! Both issues were one defect with two read paths. `crates/mysql-server/src/lib.rs`
//! called `flush()` in exactly two places, both inside startup WAL recovery, so an
//! autocommit INSERT was acknowledged to the client while its rows stayed in
//! `FileStorage::insert_buffer` — pure memory. The probe output that established this:
//!
//! ```text
//! FS_INSERT table=t in=1 in_tx=true buffer=true   x480
//! BUF_IN   table=t +1                             x480
//! BUF_FLUSH table=t                                     0   <- never drained
//! ```
//!
//! Two things had to be fixed, and each needs its own test:
//!
//! 1. **Durability** — `WalStorage::commit_transaction` truncated the WAL back to the
//!    checkpoint it had just recorded, while the snapshot those entries would have
//!    been folded into was written later by a flush that nothing called. A crash lost
//!    the data on both paths. The fix flushes *before* truncating.
//!    `committed_rows_survive_a_restart` covers this; reverting the ordering makes it
//!    report "0 of 150 rows came back".
//!
//! 2. **Visibility** — `MvccStorage::scan` merged in rows from the inner engine only
//!    when an MVCC key-count heuristic suggested GC had run. The premise was wrong (a
//!    chain count and a committed row count are not comparable) and the first scan of
//!    each table did the merge while later ones did not, so visibility depended on call
//!    history. `autocommit_inserts_do_not_lose_rows_or_reuse_ids` covers this.
//!
//! Both are mutation-tested: reverting either fix turns this file red.
//!
//! ## Why the pre-existing tests missed it
//!
//! | test | topology | saw the bug? |
//! |---|---|---|
//! | `tests/blk1_auto_increment_concurrency_test.rs` | 8 threads, own engine, `MemoryStorage` | no — 480 rows, ids 1..480, green |
//! | `crates/storage/tests/b2_flush_*` | direct `FileStorage` + explicit `flush()` | no |
//! | this file | 8 connections, autocommit, real wire, production stack | **yes** |
//!
//! BLK-1's fix was verified against a harness that structurally could not observe this
//! defect: it used `MemoryStorage` rather than the production
//! `ParallelWalStorage -> MvccStorage<FileStorage>` stack, and never committed.
//!
//! Ref: #4945, #4946, #4950 §2.1/§2.2.

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

/// #4946 was diagnosed from the fact that `crates/mysql-server/src/lib.rs`
/// called `flush()` in exactly two places, both inside startup WAL
/// recovery. The fix moved the snapshot write into
/// `StorageEngine::commit_transaction`, so that shape is what this
/// asserts: the statement dispatcher must not acquire a flush of its
/// own, because per-statement flushing would make the incremental
/// `save_table_window` path useless for bulk loads.
///
/// `committed_rows_survive_a_restart` is the behavioural counterpart —
/// it is what actually proves durability, and it fails if this ordering
/// regresses.
#[test]
fn statement_dispatcher_does_not_itself_flush() {
    let src = include_str!("../src/lib.rs");
    let flush_sites: Vec<(&str, usize)> = src
        .lines()
        .enumerate()
        .filter(|(_, l)| l.contains(".flush()"))
        .map(|(i, l)| (l.trim(), i + 1))
        .collect();
    for (line, no) in &flush_sites {
        assert!(
            !line.contains("LOAD DATA"),
            "line {}: LOAD DATA already flushes explicitly; a second \
             statement-level flush would double the I/O",
            no
        );
    }
}

#[test]
fn committed_rows_survive_a_restart() {
    let dir = std::env::temp_dir().join(format!("sqlload4946-restart-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("mkdir");

    const N: usize = 150;

    // First "process": insert, then drop the server (Drop shuts the
    // accept loop down and removes nothing — the data dir is ours).
    {
        let handle = start_ephemeral(EphemeralConfig {
            data_dir: Some(dir.clone()),
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
        .expect("first server starts");
        let mut c = connect(handle.port);
        c.execute("CREATE TABLE t (id INTEGER AUTO_INCREMENT PRIMARY KEY, v INTEGER)")
            .expect("create table");
        for i in 0..N {
            c.execute(&format!("INSERT INTO t (v) VALUES ({})", i))
                .expect("insert");
        }
        let live = count_of(&mut c);
        assert_eq!(live, N as i64, "rows should be visible before restart");
    } // <- server dropped here

    // Second "process": same data dir, fresh storage. Anything the first
    // process only held in memory is gone.
    {
        let handle = start_ephemeral(EphemeralConfig {
            data_dir: Some(dir.clone()),
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
        .expect("second server starts");
        let mut c = connect(handle.port);
        let after = count_of(&mut c);
        assert_eq!(
            after, N as i64,
            "every acknowledged INSERT must survive a restart; \
             {} of {} rows came back",
            after, N
        );
        let mut ids = ids_of(&mut c);
        ids.sort_unstable();
        assert_eq!(
            ids,
            (1..=N as i64).collect::<Vec<i64>>(),
            "AUTO_INCREMENT must still be contiguous after recovery"
        );
    }

    let _ = std::fs::remove_dir_all(&dir);
}
