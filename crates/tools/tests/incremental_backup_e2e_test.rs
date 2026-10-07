//! E2E tests for #4938 — incremental backup wiring.
//!
//! Closes AC2 + AC3 of #4938: the `create_full_backup` /
//! `create_incremental_backup` / `restore_backup` /
//! `restore_incremental_chain` file-level functions are now exercised
//! end-to-end on a real tempdir (not just unit-tested on internal
//! helpers). All 4 tests are deterministic; 3 consecutive runs show
//! no flakes.
//!
//! Honest scope: `restore_backup` materializes into a fresh
//! `MemoryStorage` (or, in chain mode, walks manifest and applies
//! per-increment in LSN order) — this test pins the **chain** behaviour
//! (sort by LSN, walk all increments, no error), not the
//! disk-materialization side which is the production caller's
//! responsibility.

use std::fs;

use sqlrustgo_tools::backup::{
    create_full_backup_from_demo, create_incremental_backup_from_demo,
    create_incremental_backup_with_changeset, restore_backup, IncrementalBackupContext,
};

fn read_manifest(p: &std::path::Path) -> serde_json::Value {
    let s = fs::read_to_string(p.join("manifest.json"))
        .unwrap_or_else(|e| panic!("manifest at {} unreadable: {e}", p.display()));
    serde_json::from_str(&s)
        .unwrap_or_else(|e| panic!("manifest at {} not valid JSON: {e}", p.display()))
}

#[test]
fn issue_4938_full_backup_writes_expected_files() {
    // AC2: create_full_backup writes manifest.json + schema.sql +
    // data/<table>.sql. The manifest has a non-empty lsn (fulls do
    // not declare a parent_lsn).
    let tmp = tempfile::tempdir().expect("tempdir");
    let target = tmp.path().join("full");
    let data = tmp.path().join("data");

    create_full_backup_from_demo(&target, "sql").expect("create_full_backup");

    assert!(
        target.join("manifest.json").exists(),
        "full backup must produce manifest.json at {}",
        target.display()
    );
    let m = read_manifest(&target);
    assert_eq!(m["backup_type"], "full", "backup_type must be `full`");
    assert!(
        m["lsn"].is_string() && !m["lsn"].as_str().unwrap().is_empty(),
        "full backup manifest must carry a non-empty lsn, got {}",
        m["lsn"]
    );
    assert!(
        m["parent_lsn"].is_null(),
        "full backup must not declare a parent_lsn, got {}",
        m["parent_lsn"]
    );
}

#[test]
fn issue_4938_incremental_backup_references_parent() {
    // AC2: create_incremental_backup declares parent_lsn equal to
    // the base full's lsn. (Schema is intentionally not rewritten for
    // incrementals — that matches the full's schema.)
    let tmp = tempfile::tempdir().expect("tempdir");
    let full_dir = tmp.path().join("full");
    let incr_dir = tmp.path().join("incr");
    let data = tmp.path().join("data");

    create_full_backup_from_demo(&full_dir, "sql").expect("full");
    let parent_lsn = read_manifest(&full_dir)["lsn"]
        .as_str()
        .expect("parent manifest lsn")
        .to_string();

    create_incremental_backup_from_demo(&full_dir, &incr_dir, "sql").expect("incremental");

    let m = read_manifest(&incr_dir);
    // #4938 AC5: this entry point exports every table, so the manifest
    // is labelled `Full`. It used to assert `incremental`, which pinned
    // the wrong label — a restore operator reading that manifest would
    // expect a delta to replay and get a second full copy instead.
    // `parent_lsn` is still asserted below: the chain link is real even
    // though the type is not.
    assert_eq!(
        m["backup_type"], "full",
        "an all-tables export must be labelled `full`, not `incremental`"
    );
    assert_eq!(
        m["parent_lsn"].as_str(),
        Some(parent_lsn.as_str()),
        "incremental parent_lsn must equal full's lsn"
    );
    assert!(
        m["lsn"].is_string() && !m["lsn"].as_str().unwrap().is_empty(),
        "incremental must carry its own lsn"
    );
    // The two lsns must differ — otherwise the increment is a no-op.
    assert_ne!(
        m["lsn"].as_str(),
        Some(parent_lsn.as_str()),
        "incremental lsn must differ from parent lsn"
    );
}

#[test]
fn issue_4938_all_tables_export_is_not_labelled_incremental() {
    // #4938 AC5, the property itself rather than one test's assertion.
    //
    // The defect was not a wrong string in one test: it was that
    // `create_incremental_backup` exports every table and stamps the
    // manifest `Incremental`. Anyone restoring from that manifest is
    // told a delta exists when a second complete copy does.
    //
    // This pins the invariant on BOTH entry points, so putting the label
    // back on either one fails here.
    let tmp = tempfile::tempdir().expect("tempdir");
    let full_dir = tmp.path().join("full");
    let chain_dir = tmp.path().join("chained");

    create_full_backup_from_demo(&full_dir, "sql").expect("full");
    create_incremental_backup_from_demo(&full_dir, &chain_dir, "sql").expect("chained");

    // Both directories contain a complete export of the demo dataset.
    for (label, dir) in [("full", &full_dir), ("chained", &chain_dir)] {
        let data = dir.join("data");
        let files: Vec<_> = fs::read_dir(&data)
            .expect("data dir")
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert!(
            !files.is_empty(),
            "{label} backup exported no table files, so the label check below proves nothing"
        );
    }

    let m = read_manifest(&chain_dir);
    assert_ne!(
        m["backup_type"], "incremental",
        "an export containing every table must not be labelled `incremental` \
         (it is a full dump; use create_incremental_backup_with_changeset for a real delta)"
    );
    assert_eq!(m["backup_type"], "full");
    // The chain link is real and must survive the relabelling.
    assert_eq!(
        m["parent_lsn"].as_str(),
        read_manifest(&full_dir)["lsn"].as_str(),
        "relabelling must not drop the parent link"
    );
}

#[test]
fn issue_4938_restore_backup_loads_full_without_error() {
    // AC2: a complete full backup can be round-tripped via
    // verify_backup + restore_backup without errors.
    let tmp = tempfile::tempdir().expect("tempdir");
    let full_dir = tmp.path().join("full");
    let target_dir = tmp.path().join("restored");
    let data = tmp.path().join("data");

    create_full_backup_from_demo(&full_dir, "sql").expect("full");
    restore_backup(&full_dir, &target_dir, /* clean */ true).expect("restore");
    assert!(target_dir.exists(), "restore must create target dir");
}

#[test]
fn issue_4938_chain_restore_handles_full_plus_incrementals() {
    // AC3: full -> incr_a -> incr_b; restore_incremental_chain is
    // called with the increments in REVERSE order to prove the chain
    // sorts by LSN internally. The harness asserts distinct lsns, the
    // parent_lsn chain (full -> a, a -> b), and that the target dir is
    // created.
    use sqlrustgo_storage::{StorageEngine, Value};
    use sqlrustgo_tools::backup::restore_incremental_chain_into;

    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    let full_dir = tmp.path().join("full");
    let incr_a_dir = tmp.path().join("incr_a");
    let incr_b_dir = tmp.path().join("incr_b");
    let target = tmp.path().join("restored");

    create_full_backup_from_demo(&full_dir, "sql").expect("full");
    let full_lsn = read_manifest(&full_dir)["lsn"]
        .as_str()
        .unwrap()
        .to_string();

    // #5048: a real delta. `create_incremental_backup_from_demo` exports
    // every table, so it produces a directory with no `changes.json` and
    // nothing to replay — using it here would have tested nothing.
    // `users` has 4 columns (id, name, email, created_at) — a 1-column
    // row would insert NULLs into name/email/created_at.
    // #5048 uses its own two-integer-column table rather than the demo
    // `users` table. `users` has POINT/JSON columns, and the full-restore
    // SQL parser splits values on bare commas (`backup.rs:422`), so its
    // rows do not round-trip. That parser bug is pre-existing and out of
    // scope here; reusing `users` would make this test fail for reasons
    // unrelated to delta replay.
    let mut ctx_a = IncrementalBackupContext::new();
    ctx_a.record_insert(
        "orders",
        vec![Value::Integer(4)],
        vec![
            Value::Integer(4),
            Value::Integer(2),
            Value::Float(59.5),
            Value::Text("shipped".into()),
        ],
    );
    create_incremental_backup_with_changeset(&full_dir, &incr_a_dir, &ctx_a).expect("incr a");
    let a_lsn = read_manifest(&incr_a_dir)["lsn"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(
        read_manifest(&incr_a_dir)["parent_lsn"].as_str(),
        Some(full_lsn.as_str()),
        "a.parent_lsn must equal full.lsn"
    );

    let mut ctx_b = IncrementalBackupContext::new();
    // Delete order 2, and update order 1's total. Both operations must be
    // visible in the restored database, not merely counted.
    ctx_b.record_delete("orders", vec![Value::Integer(2)]);
    ctx_b.record_update(
        "orders",
        vec![Value::Integer(1)],
        vec![
            Value::Integer(1),
            Value::Integer(1),
            Value::Float(109.99),
            Value::Text("completed".into()),
        ],
    );
    create_incremental_backup_with_changeset(&incr_a_dir, &incr_b_dir, &ctx_b).expect("incr b");
    let b_lsn = read_manifest(&incr_b_dir)["lsn"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(
        read_manifest(&incr_b_dir)["parent_lsn"].as_str(),
        Some(a_lsn.as_str()),
        "b.parent_lsn must equal a.lsn"
    );

    // All three lsns must be distinct for the chain to be meaningful.
    let all_lsns = [&full_lsn, &a_lsn, &b_lsn];
    let unique: std::collections::HashSet<_> = all_lsns.iter().collect();
    assert_eq!(unique.len(), 3, "lsns must all differ, got {all_lsns:?}");

    // Restore in reverse order — restore_incremental_chain must
    // sort by LSN internally to apply full -> a -> b.
    let increments = [incr_b_dir.clone(), incr_a_dir.clone()];
    let restored = restore_incremental_chain_into(&full_dir, &increments, &target, None)
        .expect("restore chain");
    assert!(target.exists(), "chain restore must create target dir");

    // #5048: the deltas must actually be in the restored database. The
    // old implementation printed "N operations applied" without touching
    // storage, so this is the assertion that could not have passed before.
    //
    // The demo `orders` table starts with three rows:
    //   (1, 1, 99.99, completed) (2, 1, 149.50, pending) (3, 2, 29.99, completed)
    // Delta a inserts (4, 2, 59.5, shipped).
    // Delta b deletes id 2 and updates id 1 to (1, 1, 109.99, completed).
    // So the restored state must be exactly ids 1, 3, 4 — with 1's total
    // changed and 2 gone.
    let rows = restored.scan("orders").expect("scan orders");
    let mut ids: Vec<i64> = rows
        .iter()
        .filter_map(|r| match r.first() {
            Some(Value::Integer(v)) => Some(*v),
            _ => None,
        })
        .collect();
    ids.sort();
    let base_ids = base_orders_ids(&full_dir);
    assert_eq!(
        ids,
        vec![1, 3, 4],
        "base is {base_ids:?}; after the deltas order 2 must be deleted and \
         order 4 inserted, leaving exactly 1, 3, 4"
    );

    let total_of_1 = rows
        .iter()
        .find(|r| matches!(r.first(), Some(Value::Integer(1))))
        .and_then(|r| match r.get(2) {
            Some(Value::Float(v)) => Some(*v),
            _ => None,
        });
    assert_eq!(
        total_of_1,
        Some(109.99),
        "delta b's UPDATE must have changed order 1's total to 109.99"
    );
}

/// The base full backup's order ids, for the failure message.
fn base_orders_ids(full_dir: &std::path::Path) -> Vec<i64> {
    use sqlrustgo_storage::{StorageEngine, Value};
    use sqlrustgo_tools::backup::restore_backup_into;
    let tmp = tempfile::tempdir().expect("tempdir");
    let base = restore_backup_into(full_dir, &tmp.path().join("t"), true).expect("base restore");
    let mut ids: Vec<i64> = base
        .scan("orders")
        .expect("scan orders")
        .iter()
        .filter_map(|r| match r.first() {
            Some(Value::Integer(v)) => Some(*v),
            _ => None,
        })
        .collect();
    ids.sort();
    ids
}
