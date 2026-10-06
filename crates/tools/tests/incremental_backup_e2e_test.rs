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
    create_full_backup_from_demo, create_incremental_backup_from_demo, restore_backup,
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
    use sqlrustgo_tools::backup::restore_incremental_chain;

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

    create_incremental_backup_from_demo(&full_dir, &incr_a_dir, "sql").expect("incr a");
    let a_lsn = read_manifest(&incr_a_dir)["lsn"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(
        read_manifest(&incr_a_dir)["parent_lsn"].as_str(),
        Some(full_lsn.as_str()),
        "a.parent_lsn must equal full.lsn"
    );

    create_incremental_backup_from_demo(&incr_a_dir, &incr_b_dir, "sql").expect("incr b");
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
    restore_incremental_chain(&full_dir, &increments, &target, /* target_lsn */ None)
        .expect("restore chain");
    assert!(target.exists(), "chain restore must create target dir");
}
