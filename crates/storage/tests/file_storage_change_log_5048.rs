//! #5048: `FileStorage` change capture — the prerequisite for a real
//! incremental backup.
//!
//! Before this, `FileStorage` had **no** versioned change capture at all:
//! no WAL, no CDC, no per-table change log. `StorageEngine::table_change_
//! stamp` was the only thing resembling one, and it is not usable here —
//! it answers "is my cached copy stale", not "which rows changed", its
//! contract says callers must treat the value as opaque, and only
//! `MemoryStorage` overrode it. `FileStorage` inherited the default `0`.
//!
//! Since the backup tool opens `FileStorage` and only `FileStorage`,
//! an incremental backup could not be produced from a real data
//! directory at all. These tests pin the capture on the engine itself,
//! against a real temp directory — not against a helper.

use sqlrustgo_storage::file_storage::{ChangeOp, FileStorage};
use sqlrustgo_storage::{ColumnDefinition, StorageEngine, TableInfo, Value};
use std::path::Path;

fn tbl(name: &str) -> TableInfo {
    TableInfo {
        name: name.to_string(),
        columns: vec![
            ColumnDefinition {
                name: "id".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: true,
                char_max_length: None,
                collation: None,
                default_value: None,
                auto_increment: false,
            },
            ColumnDefinition {
                name: "v".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                collation: None,
                default_value: None,
                auto_increment: false,
            },
        ],
        foreign_keys: vec![],
        unique_constraints: vec![],
        check_constraints: vec![],
        partition_info: None,
        collations: Default::default(),
        compression: None,
        original_sql: String::new(),
    }
}

fn db(dir: &Path) -> FileStorage {
    FileStorage::new(dir.to_path_buf()).expect("open FileStorage")
}

/// Without `enable_change_log` the engine must record nothing — and must
/// say so, so a caller can tell "nothing changed" from "nothing watched".
#[test]
fn issue_5048_change_log_is_off_by_default() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut s = db(tmp.path());

    assert!(!s.change_log_enabled(), "the log must be opt-in");
    assert_eq!(s.current_change_lsn(), 0);
    assert!(
        s.changes_since(0).is_empty(),
        "a disabled log must report no changes"
    );

    s.create_table(&tbl("t")).expect("create t");
    s.insert("t", vec![vec![Value::Integer(1), Value::Integer(10)]])
        .expect("insert");
    s.flush().expect("flush");

    assert!(s.changes_since(0).is_empty());
}

/// The core property: after enabling, a write is visible in the log.
#[test]
fn issue_5048_insert_is_captured() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut s = db(tmp.path());
    s.enable_change_log();
    s.create_table(&tbl("t")).expect("create t");

    s.insert(
        "t",
        vec![
            vec![Value::Integer(1), Value::Integer(10)],
            vec![Value::Integer(2), Value::Integer(20)],
        ],
    )
    .expect("insert");
    s.flush().expect("flush");

    let changes = s.changes_since(0);
    assert_eq!(
        changes.len(),
        2,
        "both rows must be captured, got {changes:?}"
    );
    for c in &changes {
        assert_eq!(c.op, ChangeOp::Insert);
        assert_eq!(c.table, "t");
        assert_eq!(c.key.len(), 1, "the key is the row's leading column");
    }
    assert_eq!(changes[0].key, vec![Value::Integer(1)]);
    assert_eq!(changes[1].key, vec![Value::Integer(2)]);
    assert_eq!(changes[0].row.as_ref().unwrap()[1], Value::Integer(10));
    assert_eq!(changes[1].row.as_ref().unwrap()[1], Value::Integer(20));
    // LSNs are strictly increasing, so a backup can slice by position.
    assert!(changes[0].lsn < changes[1].lsn, "lsns must increase");
}

/// An update must record the row **after** the change — that is what a
/// backup replays. Recording the pre-image would make a replay undo the
/// change.
#[test]
fn issue_5048_update_captures_the_post_image() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut s = db(tmp.path());
    s.enable_change_log();
    s.create_table(&tbl("t")).expect("create t");
    s.insert("t", vec![vec![Value::Integer(1), Value::Integer(10)]])
        .expect("insert");
    s.flush().expect("flush");

    let after_insert = s.current_change_lsn();
    let n = s
        .update("t", &[Value::Integer(1)], &[(1, Value::Integer(99))])
        .expect("update");
    assert_eq!(n, 1, "the update must match exactly one row");
    s.flush().expect("flush");

    let changes = s.changes_since(after_insert);
    assert_eq!(changes.len(), 1, "exactly one update, got {changes:?}");
    assert_eq!(changes[0].op, ChangeOp::Update);
    assert_eq!(changes[0].key, vec![Value::Integer(1)]);
    assert_eq!(
        changes[0].row.as_ref().unwrap(),
        &vec![Value::Integer(1), Value::Integer(99)],
        "the recorded row must be the post-update one, not the pre-image"
    );
}

#[test]
fn issue_5048_delete_is_captured() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut s = db(tmp.path());
    s.enable_change_log();
    s.create_table(&tbl("t")).expect("create t");
    s.insert(
        "t",
        vec![
            vec![Value::Integer(1), Value::Integer(10)],
            vec![Value::Integer(2), Value::Integer(20)],
        ],
    )
    .expect("insert");
    s.flush().expect("flush");

    let mark = s.current_change_lsn();
    let n = s.delete("t", &[Value::Integer(2)]).expect("delete");
    assert_eq!(n, 1);
    s.flush().expect("flush");

    let changes = s.changes_since(mark);
    assert_eq!(changes.len(), 1, "got {changes:?}");
    assert_eq!(changes[0].op, ChangeOp::Delete);
    assert_eq!(changes[0].key, vec![Value::Integer(2)]);
    assert!(changes[0].row.is_none(), "a delete has no row to replay");
}

/// `update_if` and `delete_if` are the predicate-driven twins of
/// `update`/`delete`, and they are separate implementations — not a
/// delegation. Mutation W2 taught this the hard way: breaking only one
/// of the two update paths left every test green, because nothing
/// exercised the other. These pin both.
#[test]
fn issue_5048_update_if_and_delete_if_are_captured() {
    use sqlrustgo_storage::{RowFilter, RowMutation};

    let tmp = tempfile::tempdir().expect("tempdir");
    let mut s = db(tmp.path());
    s.enable_change_log();
    s.create_table(&tbl("t")).expect("create t");
    s.insert(
        "t",
        vec![
            vec![Value::Integer(1), Value::Integer(10)],
            vec![Value::Integer(2), Value::Integer(20)],
        ],
    )
    .expect("insert");
    s.flush().expect("flush");

    // update_if: bump every row whose v is 20.
    let mark = s.current_change_lsn();
    let only_twenties: RowFilter =
        Box::new(|r: &sqlrustgo_storage::Record| matches!(r.get(1), Some(Value::Integer(20))));
    let n = s
        .update_if(
            "t",
            &only_twenties,
            &RowMutation::new(vec![(1, Value::Integer(200))], 0),
        )
        .expect("update_if");
    assert_eq!(n, 1, "exactly one row has v = 20");
    s.flush().expect("flush");

    let upd = s.changes_since(mark);
    assert_eq!(upd.len(), 1, "update_if must be captured, got {upd:?}");
    assert_eq!(upd[0].op, ChangeOp::Update);
    assert_eq!(
        upd[0].row.as_ref().unwrap(),
        &vec![Value::Integer(2), Value::Integer(200)],
        "the post-image, not the pre-image"
    );

    // delete_if: remove the row whose v is now 200.
    let mark2 = s.current_change_lsn();
    let to_delete: RowFilter =
        Box::new(|r: &sqlrustgo_storage::Record| matches!(r.get(1), Some(Value::Integer(200))));
    let n = s.delete_if("t", &to_delete).expect("delete_if");
    assert_eq!(n, 1);
    s.flush().expect("flush");

    let del = s.changes_since(mark2);
    assert_eq!(del.len(), 1, "delete_if must be captured, got {del:?}");
    assert_eq!(del[0].op, ChangeOp::Delete);
    assert_eq!(del[0].key, vec![Value::Integer(2)]);
    assert!(del[0].row.is_none());
}

#[test]
fn issue_5048_changes_since_slices_by_lsn() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut s = db(tmp.path());
    s.enable_change_log();
    s.create_table(&tbl("t")).expect("create t");

    s.insert("t", vec![vec![Value::Integer(1), Value::Integer(10)]])
        .expect("insert 1");
    s.flush().expect("flush");
    // Take the mark *after* the first insert — this is what a full backup
    // does: snapshot the database, remember where it stopped.
    let after_first = s.current_change_lsn();

    s.insert("t", vec![vec![Value::Integer(2), Value::Integer(20)]])
        .expect("insert 2");
    s.flush().expect("flush");

    let delta = s.changes_since(after_first);
    assert_eq!(
        delta.len(),
        1,
        "a backup taken at the first insert must see only what came after, \
         got {delta:?}"
    );
    assert_eq!(delta[0].row.as_ref().unwrap()[0], Value::Integer(2));
}

/// A backup taken before *any* write must see everything — otherwise the
/// first delta after a fresh database would be empty and silent.
#[test]
fn issue_5048_a_mark_of_zero_sees_everything() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut s = db(tmp.path());
    s.enable_change_log();
    assert_eq!(s.current_change_lsn(), 0, "a fresh log starts at 0");

    s.create_table(&tbl("t")).expect("create t");
    for i in 1..=3i64 {
        s.insert("t", vec![vec![Value::Integer(i), Value::Integer(i * 10)]])
            .expect("insert");
        s.flush().expect("flush");
    }

    let all = s.changes_since(0);
    assert_eq!(all.len(), 3, "everything after LSN 0, got {}", all.len());
}

/// A change log that grows forever is not shippable; `changes_since`
/// combined with the LSN lets a backup advance without draining. This
/// pins that the slice stays correct as the log grows.
#[test]
fn issue_5048_repeated_deltas_each_slice_correctly() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut s = db(tmp.path());
    s.enable_change_log();
    s.create_table(&tbl("t")).expect("create t");

    let mut marks = Vec::new();
    for i in 1..=5i64 {
        s.insert("t", vec![vec![Value::Integer(i), Value::Integer(i * 10)]])
            .expect("insert");
        s.flush().expect("flush");
        // Mark *after* each write, the way a backup does.
        marks.push(s.current_change_lsn());
    }

    // A mark taken after write N must see only writes N+1..5. The first
    // mark was taken after write 1, so it sees 4; the last sees 0.
    for (n, mark) in marks.iter().enumerate() {
        let slice = s.changes_since(*mark);
        let expected = 4 - n;
        assert_eq!(
            slice.len(),
            expected,
            "mark after write {} should see {expected} further changes, got {slice:?}",
            n + 1
        );
        if let Some(first) = slice.first() {
            assert_eq!(
                first.row.as_ref().unwrap()[0],
                Value::Integer(n as i64 + 2),
                "the first change after a mark must be the next write"
            );
        }
    }
}
