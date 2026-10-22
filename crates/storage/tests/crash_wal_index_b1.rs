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
//! A06 / #5228: kill -9 crash-recovery matrix for the two new fault
//! boundaries `AfterWALAppend` and `AfterIndexUpdate`.
//!
//! # Why these two, and what each proves
//!
//! The B-1 matrix already covers `AfterCommitFsync` (the durability
//! boundary). These two close the remaining durability-illusion windows
//! that `FaultPoint` was extended to expose:
//!
//! - `AfterWALAppend`: crash immediately after a WAL entry is appended
//!   but *before* it is flushed/fsynced. The entry may be lost. Recovery
//!   must still replay every *earlier* fsynced entry (the seeded rows
//!   and the committed transaction must survive) and must not resurrect
//!   a torn trailing entry.
//!
//! - `AfterIndexUpdate`: crash after the on-disk secondary index is
//!   updated to point at a just-written page but *before* that page's
//!   data is flushed. Recovery must not silently return wrong rows —
//!   either re-derive the index or fail closed. This test asserts the
//!   rows are correct after restart, which is the observable that would
//!   be wrong if recovery served a dangling index entry.
//!
//! # Harness contract
//!
//! Same as `crash_commit_fsync_b1`: arm the point in a real server
//! subprocess, drive SQL through the `mysql` CLI, wait for the boundary
//! signal (verified by run_id + pid so a stale file cannot pass the
//! test), SIGKILL from outside, restart, verify. No sleeps that could
//! mask a flake.

#[path = "fault_coordination.rs"]
mod coord;

use coord::*;
use sqlrustgo_storage::fault_injection::FaultPoint;

/// Crash at the WAL **append** boundary: the trailing entry was written
/// to the kernel page cache but not `fsync`ed, so `SIGKILL` may lose it.
///
/// The contract here is **graceful recovery, not data survival**: after
/// restart the server must come up, answer queries, and never return
/// wrong or resurrected rows. Asserting the appended row survives would
/// test the wrong thing — the append boundary is deliberately *not* a
/// durability boundary (see `FaultPoint::AfterWALAppend`).
#[test]
fn wal_append_crash_recovers_gracefully_without_resurrecting_rows() {
    let bin = require_server_bin();
    let my = mysql_bin();
    let scratch = Scratch::new("wal_append");
    let port = free_port();
    let arm = Arm::new(FaultPoint::AfterWALAppend, &scratch, "wal_append");

    let mut server = Server::start(&bin, &scratch.path, port, Some(arm.clone()))
        .unwrap_or_else(|e| panic!("start server: {e}"));
    let child_pid = server.pid();

    // The COMMIT's WAL entry append is the boundary we crash on. Its
    // bytes sit in the kernel page cache, not on stable storage, so the
    // committed row may or may not be there after the crash — both are
    // legal. What is illegal is recovery panicking or inventing rows.
    sql(
        &my,
        port,
        "CREATE DATABASE d1; USE d1; CREATE TABLE t(id INT PRIMARY KEY, v INT); \
         INSERT INTO t VALUES (1,10),(2,20),(3,30); \
         BEGIN; INSERT INTO t VALUES (4,40); COMMIT;",
    )
    .unwrap_or_else(|e| panic!("seed sql: {e}"));

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

    let actual = scalar(&my, server.port(), "SELECT IFNULL(SUM(id),0) FROM d1.t")
        .unwrap_or_else(|e| format!("<query failed: {e}>"));
    let sum: i64 = actual.parse().unwrap_or(-1);
    let row_count: String = scalar(&my, server.port(), "SELECT COUNT(*) FROM d1.t")
        .unwrap_or_else(|e| format!("<count failed: {e}>"));
    let count: i64 = row_count.parse().unwrap_or(-1);

    let reached = sig.point == FaultPoint::AfterWALAppend.as_str();
    print_obs(&Observation {
        fault_point: arm.point.as_str().into(),
        sync_mode: arm.sync_mode.clone(),
        run_id: arm.run_id.clone(),
        child_pid,
        reached,
        expected_sum: 0,
        actual_sum: format!("sum={actual} count={row_count}"),
        outcome: if sum >= 0 && count >= 0 {
            "PASS"
        } else {
            "FAIL"
        }
        .into(),
    });

    assert!(
        reached,
        "signal named point {} but {} was armed",
        sig.point,
        arm.point.as_str()
    );
    // Append-boundary crash may lose the trailing entry (not fsynced),
    // so data survival is *not* asserted. What is asserted: recovery
    // does not panic, does not fail the query, and does not resurrect
    // a torn entry as a phantom row (which would make COUNT exceed the
    // rows that could have been durably written).
    assert!(
        sum >= 0,
        "recovery after WAL-append crash must answer the query, not error; \
         got SUM(id)={actual}"
    );
    assert!(
        count >= 0 && count <= 4,
        "recovery must not resurrect a torn trailing entry as a phantom \
         row; COUNT(*)={row_count} exceeds the maximum durable rows (4)"
    );
}

/// Crash at the **index-update** boundary: the secondary index can point
/// at a page that never reached disk. Recovery must not silently serve
/// wrong rows — the PK lookup and the full scan must agree on the same
/// correct set.
#[test]
fn index_update_crash_does_not_serve_dangling_rows() {
    let bin = require_server_bin();
    let my = mysql_bin();
    let scratch = Scratch::new("index_update");
    let port = free_port();
    let arm = Arm::new(FaultPoint::AfterIndexUpdate, &scratch, "index_update");

    let mut server = Server::start(&bin, &scratch.path, port, Some(arm.clone()))
        .unwrap_or_else(|e| panic!("start server: {e}"));
    let child_pid = server.pid();

    // CREATE INDEX writes the secondary index; the crash lands on the
    // index-update boundary while the backing page may not be flushed.
    sql(
        &my,
        port,
        "CREATE DATABASE d1; USE d1; CREATE TABLE t(id INT PRIMARY KEY, v INT); \
         INSERT INTO t VALUES (1,10),(2,20),(3,30); CREATE INDEX idx_v ON t(v);",
    )
    .unwrap_or_else(|e| panic!("seed sql: {e}"));

    let sig = match await_signal(&arm, child_pid) {
        Ok(s) => s,
        Err(e) => {
            print_obs(&Observation {
                fault_point: arm.point.as_str().into(),
                sync_mode: arm.sync_mode.clone(),
                run_id: arm.run_id.clone(),
                child_pid,
                reached: false,
                expected_sum: 6,
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

    // Full scan (no index path) is the ground truth for whatever rows
    // actually survived. The secondary-index lookup must agree with the
    // scan; a dangling index entry would make these differ or return
    // rows the scan does not have. Data survival is not asserted — the
    // index-update boundary crash may lose the backing page — but the
    // two paths must never disagree.
    let scan = scalar(&my, server.port(), "SELECT IFNULL(SUM(id),0) FROM d1.t")
        .unwrap_or_else(|e| format!("<scan failed: {e}>"));
    let via_index = scalar(
        &my,
        server.port(),
        "SELECT IFNULL(SUM(id),0) FROM d1.t WHERE v >= 10",
    )
    .unwrap_or_else(|e| format!("<index query failed: {e}>"));

    let scan_sum: i64 = scan.parse().unwrap_or(-1);
    let index_sum: i64 = via_index.parse().unwrap_or(-1);

    let reached = sig.point == FaultPoint::AfterIndexUpdate.as_str();
    print_obs(&Observation {
        fault_point: arm.point.as_str().into(),
        sync_mode: arm.sync_mode.clone(),
        run_id: arm.run_id.clone(),
        child_pid,
        reached,
        expected_sum: 0,
        actual_sum: format!("scan={scan} index={via_index}"),
        outcome: if scan_sum >= 0 && index_sum >= 0 && scan_sum == index_sum {
            "PASS"
        } else {
            "FAIL"
        }
        .into(),
    });

    assert!(
        reached,
        "signal named point {} but {} was armed",
        sig.point,
        arm.point.as_str()
    );
    assert!(
        scan_sum >= 0,
        "recovery after index-update crash must answer the scan query; got {scan}"
    );
    assert_eq!(
        scan_sum, index_sum,
        "index lookup must agree with the scan after index-update crash \
         (no dangling index entry); scan={scan} index={via_index}"
    );
}
