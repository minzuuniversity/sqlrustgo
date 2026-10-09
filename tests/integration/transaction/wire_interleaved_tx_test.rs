//! #5112: two connections interleaving BEGIN / DML / COMMIT / ROLLBACK
//! over the real MySQL wire protocol.
//!
//! # Why this file exists
//!
//! `concurrent_rollback_isolation_test.rs` drives `FileStorage` directly.
//! That is deliberate — it can pin the exact undo-log sequence — but it
//! bypasses the layers where #5112 was originally observed:
//!
//!   * the MySQL server hands every connection an `Arc` clone of **one
//!     shared** storage engine, and
//!   * each connection gets its own transaction identity through the wire.
//!
//! Driving storage directly cannot prove that a `ROLLBACK` arriving on
//! connection B leaves connection A's data alone *as the server sees it*.
//! This file closes that gap: two independent TCP connections, real
//! HandshakeResponse41, real COM_QUERY, one shared engine behind them.
//!
//! # The property under test
//!
//! A committed DELETE must survive a peer's ROLLBACK, and a rolled-back
//! DELETE must be restored. Connections operate on **distinct keys** within
//! a scenario, so no row lock is involved and the expected end state is
//! unambiguous — the same reasoning that governs the storage-level test.
//! Distinct keys are not a dodge: they are what makes "row N is present" a
//! sound oracle.
//!
//! Each scenario asserts on data read back through a **separate observer
//! connection**, so the verdict is about committed state rather than a
//! session's own view.

#[path = "../../common/mod.rs"]
mod common;

use common::MySqlTestClient;
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};
use std::thread;

// The ephemeral harness pre-creates a `tester` user whose password is also
// `tester` (see `EphemeralConfig`'s bootstrap in crates/mysql-server/src/lib.rs).
// `connect_at` takes the credentials explicitly, so they are spelled out here.
const USER: &str = "tester";
const PASSWORD: &str = "tester";

/// The ephemeral server's port. Every connection in a scenario dials this,
/// so they all land on the one shared engine.
struct WireServer {
    port: u16,
}

impl WireServer {
    fn start() -> Self {
        let handle = start_ephemeral(EphemeralConfig::default())
            .expect("ephemeral server should start for the wire scenario");
        // The handle is intentionally leaked into the port: `EphemeralHandle`
        // is not `Clone`, and dropping it would shut the server down. The
        // server thread ends with the test process, which is exactly the
        // lifetime these scenarios need.
        let port = handle.port;
        std::mem::forget(handle);
        Self { port }
    }

    fn connect(&self) -> MySqlTestClient {
        MySqlTestClient::connect_at(("127.0.0.1", self.port), USER, PASSWORD)
            .expect("connection to the ephemeral server should come up")
    }
}

/// Count the rows whose `id` equals `target`, as seen by `observer`.
fn count_id(observer: &mut MySqlTestClient, target: i64) -> i64 {
    let sql = format!("SELECT COUNT(*) FROM t WHERE id = {target}");
    let rows = observer.query_rows(&sql).expect("SELECT COUNT(*)");
    rows[0][0].parse::<i64>().expect("COUNT(*) is an integer")
}

fn seed(client: &mut MySqlTestClient) {
    client
        .exec("CREATE TABLE t (id INTEGER, k INTEGER)")
        .expect("CREATE TABLE");
    for id in 1..=50 {
        client
            .exec(&format!("INSERT INTO t VALUES ({id}, 0)"))
            .expect("INSERT seed row");
    }
}

/// The headline #5112 case, end to end: A commits a DELETE while B rolls
/// back one, on separate connections over one shared engine.
#[test]
fn committed_delete_survives_peer_rollback_over_wire() {
    let server = WireServer::start();
    let mut conn_a = server.connect();
    let mut conn_b = server.connect();
    seed(&mut conn_a);

    // A: BEGIN; DELETE 42; COMMIT
    conn_a.exec("BEGIN").expect("A BEGIN");
    conn_a
        .exec("DELETE FROM t WHERE id = 42")
        .expect("A DELETE");
    conn_a.exec("COMMIT").expect("A COMMIT");

    // B: BEGIN; DELETE 43; ROLLBACK
    conn_b.exec("BEGIN").expect("B BEGIN");
    conn_b
        .exec("DELETE FROM t WHERE id = 43")
        .expect("B DELETE");
    conn_b.exec("ROLLBACK").expect("B ROLLBACK");

    let mut observer = server.connect();

    assert_eq!(
        count_id(&mut observer, 42),
        0,
        "row 42 was DELETEd and COMMITted by connection A; a peer ROLLBACK must not \
         resurrect it"
    );
    assert_eq!(
        count_id(&mut observer, 43),
        1,
        "row 43 was DELETEd and ROLLBACKed by connection B; its DELETE must have been \
         undone, but an observer sees the row missing"
    );
}

/// The inverse ordering: B rolls back **first**, then A commits. The
/// interleaving differs from the test above and the undo entries overlap
/// differently, so a defect that only shows on one ordering is caught.
#[test]
fn rollback_before_peer_commit_over_wire() {
    let server = WireServer::start();
    let mut conn_a = server.connect();
    let mut conn_b = server.connect();
    seed(&mut conn_a);

    conn_b.exec("BEGIN").expect("B BEGIN");
    conn_b
        .exec("DELETE FROM t WHERE id = 11")
        .expect("B DELETE");
    conn_b.exec("ROLLBACK").expect("B ROLLBACK");

    conn_a.exec("BEGIN").expect("A BEGIN");
    conn_a
        .exec("DELETE FROM t WHERE id = 12")
        .expect("A DELETE");
    conn_a.exec("COMMIT").expect("A COMMIT");

    let mut observer = server.connect();
    assert_eq!(count_id(&mut observer, 11), 1, "B's DELETE must be undone");
    assert_eq!(count_id(&mut observer, 12), 0, "A's DELETE must stand");
}

/// Two connections rolling back concurrently, each on its own key.
///
/// This is the ordering that exercises the shared undo log hardest: both
/// transactions have pending entries at the same time, so a ROLLBACK that
/// drains the log wholesale would leave the other connection unable to undo
/// its own work — while still reporting success. That is the exact shape of
/// the #5112 defect, reached through the wire instead of through storage.
#[test]
fn two_concurrent_rollbacks_over_wire() {
    const ROUNDS: usize = 20;
    let server = WireServer::start();
    let mut setup = server.connect();
    seed(&mut setup);

    let mut lost = 0usize;
    for round in 0..ROUNDS {
        // Fresh keys per round: a lost row from an earlier round must not be
        // able to mask a later one.
        let key_a = 1 + (2 * round) as i64;
        let key_b = 2 + (2 * round) as i64;

        let port_a = server.port;
        let port_b = server.port;
        let a = thread::spawn(move || {
            let mut c =
                MySqlTestClient::connect_at(("127.0.0.1", port_a), USER, PASSWORD).expect("conn A");
            c.exec("BEGIN").expect("A BEGIN");
            c.exec(&format!("DELETE FROM t WHERE id = {key_a}"))
                .expect("A DELETE");
            c.exec("ROLLBACK").expect("A ROLLBACK");
        });
        let b = thread::spawn(move || {
            let mut c =
                MySqlTestClient::connect_at(("127.0.0.1", port_b), USER, PASSWORD).expect("conn B");
            c.exec("BEGIN").expect("B BEGIN");
            c.exec(&format!("DELETE FROM t WHERE id = {key_b}"))
                .expect("B DELETE");
            c.exec("ROLLBACK").expect("B ROLLBACK");
        });
        a.join().expect("A thread");
        b.join().expect("B thread");

        let mut observer = server.connect();
        if count_id(&mut observer, key_a) != 1 || count_id(&mut observer, key_b) != 1 {
            lost += 1;
        }
    }

    assert_eq!(
        lost, 0,
        "{lost}/{ROUNDS} rounds left a row deleted after both connections reported a \
         successful ROLLBACK — one connection's rollback discarded another \
         connection's pending undo entries"
    );
}

/// INSERT-then-ROLLBACK across two connections: the row a rolled-back
/// transaction inserted must not become visible to anyone, and must not
/// disturb the other connection's committed rows.
#[test]
fn rolled_back_insert_is_invisible_to_peer_over_wire() {
    let server = WireServer::start();
    let mut committer = server.connect();
    let mut roller = server.connect();
    seed(&mut committer);

    // The committer adds a row and commits it.
    committer.exec("BEGIN").expect("committer BEGIN");
    committer
        .exec("INSERT INTO t VALUES (500, 7)")
        .expect("INSERT 500");
    committer.exec("COMMIT").expect("committer COMMIT");

    // The roller adds a different row and rolls it back.
    roller.exec("BEGIN").expect("roller BEGIN");
    roller
        .exec("INSERT INTO t VALUES (501, 8)")
        .expect("INSERT 501");
    roller.exec("ROLLBACK").expect("roller ROLLBACK");

    let mut observer = server.connect();
    assert_eq!(
        count_id(&mut observer, 500),
        1,
        "the committed row must be visible to an outside observer"
    );
    assert_eq!(
        count_id(&mut observer, 501),
        0,
        "the rolled-back INSERT must leave no trace, but an observer sees the row"
    );
}
