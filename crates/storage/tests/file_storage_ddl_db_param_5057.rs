//! #5057 P1-c: `create_table` / `drop_table` must honour a named database.
//!
//! # What broke
//!
//! #5025 scoped tables by `scoped_key(db, table)` but left the DDL paths
//! resolving through `self.current_db`, so `create_table` always created in
//! whichever database the shared storage last selected — the same class of
//! defect as the `USE` no-op fixed in #5084.
//!
//! This file also pins an inconsistency that predates #5057 and that
//! `create_table_in_db` would otherwise inherit: `create_table` registered
//! its pre-created primary-key index under the **bare** table name, while
//! every other index key in the file (`create_index`, `drop_index`, the
//! startup loader, `update_pk_index`) is `scoped_key(db, table)`. The
//! pre-created index was therefore unreachable from every lookup — a dead
//! entry occupying memory.

use sqlrustgo_storage::{ColumnDefinition, FileStorage, StorageEngine, TableInfo, Value};

fn tbl(name: &str) -> TableInfo {
    TableInfo {
        name: name.to_string(),
        columns: vec![ColumnDefinition {
            name: "id".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: false,
            primary_key: true,
            char_max_length: None,
            collation: None,
            default_value: None,
            auto_increment: false,
        }],
        foreign_keys: vec![],
        unique_constraints: vec![],
        check_constraints: vec![],
        partition_info: None,
        collations: Default::default(),
        compression: None,
        original_sql: String::new(),
    }
}

fn fresh() -> (tempfile::TempDir, FileStorage) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let storage = FileStorage::new(tmp.path().to_path_buf()).expect("open");
    (tmp, storage)
}

/// A table created for a named database must exist there and not in the
/// storage's current database.
#[test]
fn issue_5057_create_table_in_db_creates_in_the_named_database() {
    let (_tmp, mut s) = fresh();
    s.create_database("d1").expect("create d1");
    s.create_database("d2").expect("create d2");

    // The storage's current database is `default` throughout — every
    // assertion below must hold without switching it.
    s.create_table_in_db("d2", &tbl("only_in_d2"))
        .expect("create in d2");

    assert!(
        s.has_table_in("d2", "only_in_d2"),
        "the table must exist in the database it was named for"
    );
    assert!(
        !s.has_table_in("default", "only_in_d2"),
        "the table must not also appear in current_db"
    );
}

/// The counterpart: without a database, `create_table` still follows
/// `current_db`. This is the control that would catch a refactor merging
/// the two families.
#[test]
fn issue_5057_create_table_without_a_database_still_follows_current_db() {
    let (_tmp, mut s) = fresh();
    s.create_database("d1").expect("create d1");

    s.set_current_db("d1").expect("switch to d1");
    s.create_table(&tbl("t")).expect("create");

    assert!(s.has_table_in("d1", "t"), "must land in d1");
}

/// Two databases, same table name: both must keep their own.
#[test]
fn issue_5057_same_table_name_coexists_across_databases() {
    let (_tmp, mut s) = fresh();
    s.create_database("d1").expect("create d1");
    s.create_database("d2").expect("create d2");

    s.create_table_in_db("d1", &tbl("shared"))
        .expect("create in d1");
    s.create_table_in_db("d2", &tbl("shared"))
        .expect("create in d2");

    s.insert_in_db("d1", "shared", vec![vec![Value::Integer(1)]])
        .expect("insert into d1");
    s.insert_in_db("d2", "shared", vec![vec![Value::Integer(2)]])
        .expect("insert into d2");

    assert_eq!(s.scan_in_db("d1", "shared").unwrap().len(), 1);
    assert_eq!(s.scan_in_db("d2", "shared").unwrap().len(), 1);
}

/// The index key fix: the pre-created primary-key index must be
/// reachable by the key every lookup uses.
///
/// `scan_with_index` resolves its index by `self.tbl(table)` — the scoped
/// key. Before the fix, `create_table` stored the index under the bare
/// name, so this lookup found nothing and fell back to a full scan.
#[test]
fn issue_5057_precreated_pk_index_is_reachable_by_its_scoped_key() {
    let (_tmp, mut s) = fresh();
    s.create_database("d2").expect("create d2");

    s.create_table_in_db("d2", &tbl("pk_probe"))
        .expect("create in d2");
    s.insert_in_db("d2", "pk_probe", vec![vec![Value::Integer(7)]])
        .expect("insert");

    // Current database is still `default`, so a `current_db`-based lookup
    // would not find a d2 index even if one existed. Name d2 explicitly.
    let rows = s
        .scan_with_index_in_db("d2", "pk_probe", "id", &Value::Integer(7))
        .expect("index lookup must succeed");

    assert_eq!(
        rows.len(),
        1,
        "the index `create_table` pre-created must be reachable; a miss \
         here means its key does not match what `scan_with_index` looks up"
    );
    assert_eq!(rows[0], vec![Value::Integer(7)]);
}

/// `drop_table_in_db` must remove the named database's table and leave
/// the same-named table in another database alone.
#[test]
fn issue_5057_drop_table_in_db_removes_only_the_named_databases_table() {
    let (_tmp, mut s) = fresh();
    s.create_database("d1").expect("create d1");
    s.create_database("d2").expect("create d2");

    s.create_table_in_db("d1", &tbl("shared"))
        .expect("create in d1");
    s.create_table_in_db("d2", &tbl("shared"))
        .expect("create in d2");
    s.insert_in_db("d1", "shared", vec![vec![Value::Integer(1)]])
        .expect("d1 row");
    s.insert_in_db("d2", "shared", vec![vec![Value::Integer(2)]])
        .expect("d2 row");

    s.drop_table_in_db("d2", "shared").expect("drop d2's table");

    assert!(
        !s.has_table_in("d2", "shared"),
        "the named database's table must be gone"
    );
    assert!(
        s.has_table_in("d1", "shared"),
        "d1's identically-named table must survive"
    );
    assert_eq!(
        s.scan_in_db("d1", "shared").unwrap().len(),
        1,
        "d1's row must survive"
    );
}

/// Without a database, `drop_table` still follows `current_db`.
#[test]
fn issue_5057_drop_table_without_a_database_still_follows_current_db() {
    let (_tmp, mut s) = fresh();
    s.create_database("d1").expect("create d1");
    s.set_current_db("d1").expect("switch to d1");
    s.create_table(&tbl("t")).expect("create");

    s.drop_table("t").expect("drop");

    assert!(!s.has_table_in("d1", "t"), "must drop from d1");
}
