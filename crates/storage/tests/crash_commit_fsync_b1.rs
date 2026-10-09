//! B-1.1: the minimal closed loop — one fault point, one crash, one
//! verification that the committed transaction survived.
//!
//! Proves the harness end to end before any more fault points are added:
//! arm `after_commit_fsync`, run a transaction, wait for the boundary,
//! SIGKILL from outside, restart, check the data.
//!
//! # Why this must fail loudly when it does not reach the boundary
//!
//! A test that cannot reach its fault point must not pass by falling
//! back to "the server probably just restarted fine". The harness
//! verifies the signal's run id and pid against the live child, so a
//! leftover file from an earlier run cannot stand in for this one.

#[path = "fault_coordination.rs"]
mod coord;

use coord::*;
use sqlrustgo_storage::fault_injection::FaultPoint;

/// A committed transaction must survive `SIGKILL` taken at the
/// durability boundary.
#[test]
fn committed_transaction_survives_sigkill_at_commit_fsync() {
    let bin = server_bin();
    let my = mysql_bin();
    let scratch = Scratch::new("commit_fsync");
    let port = free_port();
    let arm = Arm::new(FaultPoint::AfterCommitFsync, &scratch, "commit_fsync");

    let mut server = Server::start(&bin, &scratch.path, port, Some(arm.clone()))
        .unwrap_or_else(|e| panic!("start server: {e}"));
    let child_pid = server.pid();

    sql(
        &my,
        port,
        "CREATE DATABASE d1; USE d1; CREATE TABLE t(id INT PRIMARY KEY, v INT); \
         INSERT INTO t VALUES (1,10),(2,20),(3,30); \
         BEGIN; INSERT INTO t VALUES (4,40); COMMIT;",
    )
    .unwrap_or_else(|e| panic!("seed sql: {e}"));

    // The boundary: WAL Commit entry fsync'd, client not yet answered.
    let sig = match await_signal(&arm, child_pid) {
        Ok(s) => s,
        Err(e) => {
            print_obs(&Observation {
                fault_point: arm.point.as_str().into(),
                sync_mode: arm.sync_mode.clone(),
                run_id: arm.run_id.clone(),
                child_pid,
                reached: false,
                expected_sum: 10,
                actual_sum: "<never reached>".into(),
                outcome: format!("FAIL: {e}"),
            });
            panic!("{e}");
        }
    };

    server.kill();

    let server = server.restart().unwrap_or_else(|e| panic!("restart: {e}"));
    await_queryable(&my, server.port(), BOOT_TIMEOUT)
        .unwrap_or_else(|e| panic!("server never became queryable: {e}"));

    // ids 1+2+3+4 — the committed transaction is part of the sum.
    let actual = scalar(&my, server.port(), "SELECT IFNULL(SUM(id),0) FROM d1.t")
        .unwrap_or_else(|e| format!("<query failed: {e}>"));
    let sum: i64 = actual.parse().unwrap_or(-1);

    let reached = sig.point == FaultPoint::AfterCommitFsync.as_str();
    print_obs(&Observation {
        fault_point: arm.point.as_str().into(),
        sync_mode: arm.sync_mode.clone(),
        run_id: arm.run_id.clone(),
        child_pid,
        reached,
        expected_sum: 10,
        actual_sum: actual.clone(),
        outcome: if sum == 10 { "PASS".into() } else { "FAIL".into() },
    });

    assert!(
        reached,
        "signal named point {} but {} was armed",
        sig.point,
        arm.point.as_str()
    );
    assert_eq!(
        sum, 10,
        "a transaction that reached COMMIT's fsync must survive SIGKILL; \
         got SUM(id)={actual}"
    );
}

/// The control: the same crash with **no** committed transaction beyond
/// the seeded rows must leave exactly those rows — not more, not fewer.
///
/// Without this, the test above could pass on a harness that simply
/// never crashed anything and happened to see the right data.
#[test]
fn crash_without_new_commit_preserves_exactly_the_seeded_rows() {
    let bin = server_bin();
    let my = mysql_bin();
    let scratch = Scratch::new("control");
    let port = free_port();
    let arm = Arm::new(FaultPoint::AfterCommitFsync, &scratch, "control");

    let mut server = Server::start(&bin, &scratch.path, port, Some(arm.clone()))
        .unwrap_or_else(|e| panic!("start: {e}"));
    let child_pid = server.pid();

    sql(
        &my,
        port,
        "CREATE DATABASE d1; USE d1; CREATE TABLE t(id INT PRIMARY KEY, v INT); \
         INSERT INTO t VALUES (1,10),(2,20);",
    )
    .expect("seed");

    // The fault point only fires on a COMMIT, and this scenario issues
    // none, so it must never be reached — and the test says so rather
    // than blocking for the timeout.
    std::thread::sleep(std::time::Duration::from_millis(500));
    assert!(
        !arm.signal_file().exists(),
        "control: the fault point fired without any COMMIT, so the hook is \
         not where this test assumes it is"
    );
    assert_eq!(
        scalar(&my, server.port(), "SELECT IFNULL(SUM(id),0) FROM d1.t").unwrap(),
        "3",
        "control: ids 1+2"
    );

    server.kill();
    let server = server.restart().expect("restart");
    await_queryable(&my, server.port(), BOOT_TIMEOUT).expect("queryable");

    let after = scalar(&my, server.port(), "SELECT IFNULL(SUM(id),0) FROM d1.t").unwrap();
    print_obs(&Observation {
        fault_point: "control-no-commit".into(),
        sync_mode: arm.sync_mode.clone(),
        run_id: arm.run_id.clone(),
        child_pid,
        reached: false,
        expected_sum: 3,
        actual_sum: after.clone(),
        outcome: "PASS".into(),
    });
    assert_eq!(after, "3", "control: seeded rows must survive intact");
}