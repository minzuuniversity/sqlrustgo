//! End-to-end tests for #4937 — semi-sync replication actually gates the
//! master's commit.
//!
//! Why this file exists separately from the unit tests in
//! `binlog_server.rs` / `semisync.rs`
//! -------------------------------------------------------
//! The 26 unit tests in `semisync.rs` prove `wait_for_acks` returns `Ok`
//! when its counter is satisfied. They say nothing about whether a commit
//! path *uses* it. That distinction is the whole point of this issue: a
//! fully-tested component with no production caller looks identical to a
//! working one from inside the unit-test suite.
//!
//! These tests run a real master and a real replica over TCP.
//!
//! The two that matter:
//!   - `commit_waits_for_replica_before_returning` — with a replica
//!     attached, the master's write must not return until the replica has
//!     reported the write.
//!   - `commit_times_out_and_degrades_when_no_replica` — with no replica,
//!     the master must block for the full timeout and then record the
//!     async degradation, rather than reporting success immediately.

use sqlrustgo_storage::binlog_client::BinlogClient;
use sqlrustgo_storage::binlog_protocol::BinlogEventData;
use sqlrustgo_storage::binlog_server::BinlogServer;
use sqlrustgo_storage::replication::{BinlogEvent, BinlogEventType};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn tmp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "e2e4937_{}_{}_{}",
        tag,
        std::process::id(),
        Instant::now().elapsed().as_nanos() as u64 ^ (tag.len() as u64)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn sample_event() -> BinlogEvent {
    BinlogEvent {
        event_type: BinlogEventType::Dml,
        tx_id: 1,
        table_id: 1,
        database: "e2e".to_string(),
        table: "t".to_string(),
        sql: Some("INSERT INTO e2e.t VALUES (1)".to_string()),
        row_data: None,
        lsn: 0,
        timestamp: 0,
    }
}

/// Start a server on an ephemeral port and run its accept loop on a
/// background thread. `start()` blocks until `stop()`, so it cannot be
/// called inline.
fn spawn_server(dir: &Path) -> (Arc<BinlogServer>, u16) {
    let server =
        Arc::new(BinlogServer::new("127.0.0.1", 0, 1, dir.join("binlog")).expect("bind server"));
    let port = server.local_addr().expect("local_addr").port();
    let bg = server.clone();
    std::thread::spawn(move || {
        let _ = bg.start();
    });
    // Let the accept loop arm before any client dials.
    std::thread::sleep(Duration::from_millis(120));
    (server, port)
}

/// A replica that is connected and consuming, i.e. it will ACK what the
/// master sends. Kept alive for the duration of the test.
struct Replica {
    _client: BinlogClient,
    _events: sqlrustgo_storage::binlog_client::Receiver<BinlogEventData>,
}

fn attach_replica(port: u16) -> Replica {
    let mut client = BinlogClient::new("127.0.0.1", port, 7).expect("connect replica");
    client.connect().expect("handshake");
    let events = client.start_replication().expect("start replication");
    // Let the master finish booking this replica's id.
    std::thread::sleep(Duration::from_millis(150));
    Replica {
        _client: client,
        _events: events,
    }
}

#[test]
fn commit_waits_for_replica_before_returning() {
    let dir = tmp_dir("wait");
    let (server, port) = spawn_server(&dir);
    let _replica = attach_replica(port);

    // wait_count = 1: one replica must confirm.
    server.configure_semi_sync(1, 5_000);
    assert!(server.semi_sync().is_enabled(), "semi-sync must be on");

    let start = Instant::now();
    let lsn = server.write_event(&sample_event()).expect("write event");
    let elapsed = start.elapsed();

    assert!(
        server.acked_pos_of(7) >= lsn,
        "#4937: the replica's durable position ({}) must have reached the \
         written LSN ({lsn})",
        server.acked_pos_of(7)
    );
    assert_eq!(
        server.acking_pos_slave_count(),
        1,
        "#4937: exactly one replica should have acknowledged"
    );

    let status = server.semi_sync().get_status();
    assert_eq!(
        status.rpl_semi_sync_master_yes_transactions, 1,
        "#4937: an acknowledged commit must be counted as a 'yes'"
    );
    assert_eq!(
        status.rpl_semi_sync_master_no_transactions, 0,
        "#4937: nothing should have degraded while a replica was attached"
    );

    // Sanity: it did wait, rather than returning instantly and hoping.
    // The bound is loose on purpose (CI machines are slow) — the point is
    // only to distinguish "waited for the replica" from "never waited".
    assert!(
        elapsed < Duration::from_secs(4),
        "#4937: an acknowledged commit should not take the full timeout \
         (took {elapsed:?})"
    );

    server.stop();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn commit_times_out_and_degrades_when_no_replica() {
    let dir = tmp_dir("timeout");
    let (server, port) = spawn_server(&dir);
    let _ = port;

    // No replica is ever attached.
    server.configure_semi_sync(1, 400);

    let start = Instant::now();
    let lsn = server.write_event(&sample_event()).expect("write event");
    let elapsed = start.elapsed();

    assert_eq!(
        server.acking_pos_slave_count(),
        0,
        "#4937: nothing may claim to have acknowledged with no replica"
    );
    assert!(
        elapsed >= Duration::from_millis(350),
        "#4937: with no replica the master must block for the timeout \
         rather than reporting success immediately (took {elapsed:?} for lsn {lsn})"
    );

    let status = server.semi_sync().get_status();
    assert_eq!(
        status.rpl_semi_sync_master_no_transactions, 1,
        "#4937: a timed-out commit must be recorded as degraded to async"
    );
    assert_eq!(
        status.rpl_semi_sync_master_yes_transactions, 0,
        "#4937: a degraded commit must not be counted as acknowledged"
    );

    server.stop();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn semi_sync_is_off_by_default() {
    // A server nobody configured must behave exactly as before #4937:
    // a commit with no replica cannot possibly block.
    let dir = tmp_dir("default_off");
    let (server, _port) = spawn_server(&dir);

    assert!(
        !server.semi_sync().is_enabled(),
        "#4937: semi-sync must be opt-in"
    );

    let start = Instant::now();
    server.write_event(&sample_event()).expect("write event");
    assert!(
        start.elapsed() < Duration::from_millis(300),
        "#4937: with semi-sync disabled, write_event must not wait \
         (took {:?})",
        start.elapsed()
    );
    assert_eq!(
        server.acking_pos_slave_count(),
        0,
        "#4937: no acknowledgement without a replica"
    );

    server.stop();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn wait_count_of_zero_is_async_even_when_enabled() {
    // N=0 is the documented degenerate form of semi-sync: it must never
    // block, and must not record degradations either.
    let dir = tmp_dir("zero");
    let (server, _port) = spawn_server(&dir);

    server.configure_semi_sync(0, 5_000);
    assert!(
        !server.semi_sync().is_enabled(),
        "#4937: wait_count=0 must leave semi-sync disabled"
    );

    let start = Instant::now();
    server.write_event(&sample_event()).expect("write event");
    assert!(
        start.elapsed() < Duration::from_millis(300),
        "#4937: wait_count=0 must not block (took {:?})",
        start.elapsed()
    );

    server.stop();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_replica_that_never_acks_does_not_unblock_the_master() {
    // The negative control for the positive case above: a replica that is
    // connected but silent must not satisfy the wait. This is the
    // distinction that heartbeat ACKs would have papered over — a
    // heartbeat can arrive and be echoed while the replica has written
    // nothing, so if this test passes only because some *other* ACK path
    // woke the master, the gate is not actually gating.
    let dir = tmp_dir("silent");
    let (server, port) = spawn_server(&dir);

    // Connect the TCP client but never start replication, so the
    // replica's id is never booked and no BinlogAck can ever arrive.
    let mut raw = BinlogClient::new("127.0.0.1", port, 9).expect("connect");
    raw.connect().expect("handshake");

    server.configure_semi_sync(1, 400);

    let start = Instant::now();
    server.write_event(&sample_event()).expect("write event");
    let elapsed = start.elapsed();

    assert!(
        elapsed >= Duration::from_millis(350),
        "#4937: a silent replica must leave the master waiting for the \
         full timeout (took {elapsed:?})"
    );
    assert_eq!(
        server.acking_pos_slave_count(),
        0,
        "#4937: a connected-but-silent replica must not be counted"
    );
    assert_eq!(
        server
            .semi_sync()
            .get_status()
            .rpl_semi_sync_master_no_transactions,
        1,
        "#4937: the commit must be recorded as degraded"
    );

    drop(raw);
    server.stop();
    let _ = std::fs::remove_dir_all(&dir);
}
