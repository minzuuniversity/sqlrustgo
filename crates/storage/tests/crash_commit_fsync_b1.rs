//! This test imports `sqlrustgo_storage::fault_injection`, which `crates/storage/src/lib.rs` gates
//! behind the `fault-injection` feature. An integration test links the library
//! without `cfg(test)`, so that item does not exist for it unless the
//! feature is on — and the test then fails to COMPILE rather than
//! failing at run time:
//!
//!     error[E0432]: unresolved import `sqlrustgo_storage::fault_injection`
//!
//! `cargo test --workspace` does not pass `--all-features`, so the
//! test is compiled out there instead of breaking the build. This is
//! deliberately NOT `#[ignore]`: an ignored test stays visible as
//! "N ignored" in the output, which is how a gate reports PASS while
//! the test never ran (ADR-008 Policy 2). A `cfg` gate removes the
//! test from the binary outright, and it IS built by the invocation
//! that enables the feature.
#![cfg(feature = "fault-injection")]
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
use std::time::Duration;

/// A committed transaction must survive `SIGKILL` taken at the
/// durability boundary.
#[test]
fn committed_transaction_survives_sigkill_at_commit_fsync() {
    let bin = require_server_bin();
    let my = mysql_bin();
    let scratch = Scratch::new("commit_fsync");
    let port = free_port();
    let arm = Arm::new(FaultPoint::AfterCommitFsync, &scratch, "commit_fsync")
        // #5556: occurrence 1 — the **first** commit that reaches the
        // point.
        //
        // Measured on this tree (macOS arm64): the scenario's two
        // statements that commit (the autocommit seed and the explicit
        // `COMMIT`) do **not** both reach `after_commit_fsync` — only the
        // autocommit one does. The explicit transaction's commit goes
        // through `FileStorage::commit_transaction_for`, which appends
        // the Commit entry and never reaches this point. See #5584.
        //
        // So occurrence 1 = the seed's commit, which is the boundary this
        // server can actually be crashed at.
        //
        // Before this, the server held at *every* commit: the script paid
        // 120 s per commit and the SIGKILL landed after the whole script
        // instead of at the boundary.
        .with_occurrence(1);

    let mut server = Server::start(&bin, &scratch.path, port, Some(arm.clone()))
        .unwrap_or_else(|e| panic!("start server: {e}"));
    let child_pid = server.pid();

    // The client runs in the background: the server holds at the boundary,
    // so this statement is never answered — the crash is supposed to
    // happen while it is in flight.
    let client = sql_background(
        &my,
        port,
        "CREATE DATABASE d1; USE d1; CREATE TABLE t(id INT PRIMARY KEY, v INT); \
         INSERT INTO t VALUES (1,10),(2,20),(3,30); \
         BEGIN; INSERT INTO t VALUES (4,40); COMMIT;",
    );

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
    let _ = client.recv_timeout(Duration::from_secs(5));

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
        expected_sum: 6,
        actual_sum: actual.clone(),
        outcome: if sum == 6 {
            "PASS".into()
        } else {
            "FAIL".into()
        },
    });

    assert!(
        reached,
        "signal named point {} but {} was armed",
        sig.point,
        arm.point.as_str()
    );
    // #5556: the crash now lands **at** the boundary (the seed's commit
    // fsync), not after the whole script — so the explicit transaction
    // that follows never commits, and its row must be **absent**.
    //
    // The expected sum is therefore 1+2+3 = 6, not 10:
    //
    //   * rows 1,2,3 — the autocommit seed, whose COMMIT reached the
    //     boundary — **must survive** the SIGKILL (that is the claim);
    //   * row 4 — the explicit transaction, which never committed —
    //     **must not** appear (a crash must not resurrect an
    //     uncommitted transaction).
    //
    // The old expectation was 10, and it was an artifact: the crash used
    // to land after the whole script (the harness could not react to the
    // boundary until the script was over), so both commits had already
    // happened. The test therefore never crashed at the boundary it
    // claimed — it only looked like it.
    assert_eq!(
        sum, 6,
        "the rows whose COMMIT reached the boundary must survive SIGKILL \
         (1+2+3=6), and the transaction that never committed must not \
         appear; got SUM(id)={actual}"
    );
}

/// The control: the scenario issues exactly **one** commit — the
/// autocommit seed — so the fault point fires there and nowhere else.
/// After the crash, exactly the seeded rows must remain: not more, not
/// fewer.
///
/// Without this, the test above could pass on a harness that simply
/// never crashed anything and happened to see the right data.
///
/// #5375: the old version asserted the fault point must **not** fire
/// ("this scenario issues no COMMIT"). That premise is false: an
/// autocommit INSERT *is* a commit, so the point fires on the seed. The
/// product was right and the assertion was wrong — and it was wrong in
/// the direction that would have let a harness which never reached the
/// boundary look healthy.
///
/// What the control is actually for — proving the harness reached the
/// boundary at all — is now asserted directly: the signal must exist
/// and must name this run's id, this server's pid, and the armed point.
/// A leftover signal from an earlier run cannot stand in for this one.
#[test]
fn crash_without_new_commit_preserves_exactly_the_seeded_rows() {
    let bin = require_server_bin();
    let my = mysql_bin();
    let scratch = Scratch::new("control");
    let port = free_port();
    let arm = Arm::new(FaultPoint::AfterCommitFsync, &scratch, "control");

    let mut server = Server::start(&bin, &scratch.path, port, Some(arm.clone()))
        .unwrap_or_else(|e| panic!("start: {e}"));
    let child_pid = server.pid();

    // The client runs in the background: the server holds at the seed's
    // commit, so the statement is never answered.
    //
    // #5556: before this the client blocked until the script was over,
    // which meant the hold had to expire (120 s) before the harness even
    // looked at the signal — and the crash landed after the script.
    let client = sql_background(
        &my,
        port,
        "CREATE DATABASE d1; USE d1; CREATE TABLE t(id INT PRIMARY KEY, v INT); \
         INSERT INTO t VALUES (1,10),(2,20);",
    );

    // The seed's autocommit commit is the only commit this scenario
    // issues, so the fault point must have been reached there — and the
    // signal must be this run's and this server's.
    let sig = match await_signal(&arm, child_pid) {
        Ok(s) => s,
        Err(e) => panic!(
            "control: the fault point must be reached at the seed's commit \
             (an autocommit INSERT commits): {e}"
        ),
    };
    assert_eq!(
        sig.run_id, arm.run_id,
        "control: the signal is from a different run"
    );
    assert_eq!(
        sig.pid, child_pid,
        "control: the signal is from a different server"
    );
    assert_eq!(
        sig.point,
        FaultPoint::AfterCommitFsync.as_str(),
        "control: the signal names the wrong fault point"
    );

    server.kill();
    let _ = client.recv_timeout(Duration::from_secs(5));

    // #5556: the "the seeded rows are visible" check used to run **before**
    // the kill. It cannot run there any more: the server holds at the
    // commit boundary, so the transaction has not returned yet and the
    // rows are not visible. The same claim is checked after the restart
    // below, where it is meaningful (the commit was durable).
    let server = server.restart().expect("restart");
    await_queryable(&my, server.port(), BOOT_TIMEOUT).expect("queryable");

    let after = scalar(&my, server.port(), "SELECT IFNULL(SUM(id),0) FROM d1.t").unwrap();
    print_obs(&Observation {
        fault_point: "control-no-commit".into(),
        sync_mode: arm.sync_mode.clone(),
        run_id: arm.run_id.clone(),
        child_pid,
        reached: true,
        expected_sum: 3,
        actual_sum: after.clone(),
        outcome: "PASS".into(),
    });
    assert_eq!(after, "3", "control: seeded rows must survive intact");
}
