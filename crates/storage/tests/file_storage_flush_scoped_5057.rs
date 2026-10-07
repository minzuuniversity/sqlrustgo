//! #5057: `flush()` must persist each dirty table under the database it
//! belongs to — not under whichever database happened to be active.
//!
//! # The defect this pins
//!
//! `WriteState::dirty_tables` was a `HashSet<String>` of **bare table
//! names**, and the flush path resolved each name against `current_db` at
//! write time. A bare name cannot name a database, so two databases
//! holding a table of the same name collapsed into ONE set entry, and the
//! single write went to the active database's directory:
//!
//! ```text
//! d1.t <- 3 rows, d2.t <- 3 rows, current_db = d1
//! flush() -> Ok(())
//! d1/t.json  3 rows      <- correct
//! d2/t.json  0 rows      <- 3 rows silently gone, no error
//! ```
//!
//! Two things had to change for that to stop, and neither alone is
//! enough:
//!
//! 1. `dirty_tables` keyed by `(db, table)`, so an entry can say where it
//!    belongs, and `drain_dirty_windowed` returns that database instead of
//!    the caller substituting `current_db`.
//! 2. `flush_all_buffers` reaches every database, not just the active one.
//!    Until this, the other database's rows sat in `insert_buffer` and the
//!    drain saw an empty table and skipped it — so fix (1) alone still
//!    loses the rows, just via a different branch.
//!
//! `last_saved_row_count` and the `<table>.delta` path are scoped for the
//! same reason: sharing a "how many rows are on disk" counter between
//! `d1.t` and `d2.t` makes the second one look already-persisted, and a
//! shared delta file has one database's rows replayed into the other's
//! table at load time.
//!
//! # What these pin
//!
//! 1. Every database's dirty rows reach their own file, and survive a
//!    reopen.
//! 2. A delete in a non-active database is persisted too — the delete
//!    marker is scoped the same way the insert marker is.
//! 3. The two families stay distinct: naming a database routes there,
//!    omitting it still follows `current_db`. A refactor that merged them
//!    would let a caller who named `d2` write into the last `USE`.
//! 4. Delta files stay per-database.

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

fn row(n: i64) -> Vec<Value> {
    vec![Value::Integer(n)]
}

fn ids(records: &[Vec<Value>]) -> Vec<i64> {
    records
        .iter()
        .filter_map(|r| r.first())
        .filter_map(|v| match v {
            Value::Integer(n) => Some(*n),
            _ => None,
        })
        .collect()
}

/// A storage over a fresh temp directory, plus the directory itself so a
/// test can reopen from it.
fn fresh() -> (tempfile::TempDir, FileStorage) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let storage = FileStorage::new(tmp.path().to_path_buf()).expect("open");
    (tmp, storage)
}

/// `d1.t` and `d2.t`, three distinctly-numbered rows each.
fn two_databases(storage: &mut FileStorage) {
    storage.create_database("d1").expect("create d1");
    storage.create_database("d2").expect("create d2");
    storage
        .create_table_in_db("d1", &tbl("t"))
        .expect("create d1.t");
    storage
        .create_table_in_db("d2", &tbl("t"))
        .expect("create d2.t");
    storage
        .insert_in_db("d1", "t", vec![row(11), row(12), row(13)])
        .expect("insert d1.t");
    storage
        .insert_in_db("d2", "t", vec![row(21), row(22), row(23)])
        .expect("insert d2.t");
}

// ---------------------------------------------------------------------
// 1. The defect itself
// ---------------------------------------------------------------------

/// A flush from `d1` must persist `d2`'s rows into `d2`'s file.
///
/// This is the regression that lost data: before the fix `d2/t.json` was
/// written with an empty `rows` array and `flush()` returned `Ok(())`.
#[test]
fn issue_5057_flush_persists_rows_of_every_database() {
    let (tmp, mut storage) = fresh();
    two_databases(&mut storage);

    // Flush while d1 is the active database, so d2 is the one being
    // proved reachable.
    storage.set_current_db("d1").expect("use d1");
    storage.flush().expect("flush");

    // Read the files directly. `scan_in_db` merges the write buffer, so it
    // would report the in-memory rows and prove nothing about persistence.
    let d1 = std::fs::read_to_string(tmp.path().join("d1/t.json")).expect("read d1/t.json");
    let d2 = std::fs::read_to_string(tmp.path().join("d2/t.json")).expect("read d2/t.json");
    for (label, body, expect) in [
        ("d1", &d1, &[11, 12, 13][..]),
        ("d2", &d2, &[21, 22, 23][..]),
    ] {
        let json: serde_json::Value = serde_json::from_str(body).expect("valid json");
        let rows = json["rows"].as_array().expect("rows array");
        let got: Vec<i64> = rows
            .iter()
            .filter_map(|r| r.get(0)?.get("Integer")?.as_i64())
            .collect();
        assert_eq!(got, expect, "{label}/t.json holds the wrong rows");
    }
}

/// The rows must come back from a fresh storage, not merely exist as
/// bytes — this is what a restart actually sees.
#[test]
fn issue_5057_cross_db_flush_survives_reopen() {
    let (tmp, mut storage) = fresh();
    two_databases(&mut storage);
    storage.set_current_db("d1").expect("use d1");
    storage.flush().expect("flush");
    drop(storage);

    let reopened = FileStorage::new(tmp.path().to_path_buf()).expect("reopen");
    assert_eq!(
        ids(&reopened.scan_in_db("d1", "t").expect("scan d1.t")),
        vec![11, 12, 13],
        "d1.t did not come back"
    );
    assert_eq!(
        ids(&reopened.scan_in_db("d2", "t").expect("scan d2.t")),
        vec![21, 22, 23],
        "d2.t did not come back"
    );
}

/// A delete in the non-active database must be persisted too. The delete
/// path marks the table dirty through a different code path than insert,
/// and it used the `scoped_key` spelling while insert used the bare name —
/// one set, two conventions.
#[test]
fn issue_5057_cross_db_delete_is_persisted() {
    let (tmp, mut storage) = fresh();
    two_databases(&mut storage);

    storage
        .delete_in_db("d2", "t", &[Value::Integer(22)])
        .expect("delete from d2.t");

    storage.set_current_db("d1").expect("use d1");
    storage.flush().expect("flush");
    drop(storage);

    let reopened = FileStorage::new(tmp.path().to_path_buf()).expect("reopen");
    assert_eq!(
        ids(&reopened.scan_in_db("d2", "t").expect("scan d2.t")),
        vec![21, 23],
        "the deletion in the non-active database was not persisted"
    );
    assert_eq!(
        ids(&reopened.scan_in_db("d1", "t").expect("scan d1.t")),
        vec![11, 12, 13],
        "the deletion leaked into the active database"
    );
}

/// Second flush must not re-append the same rows. `last_saved_row_count`
/// is scoped per `(db, table)` now; if it were shared, flushing twice
/// would either skip or duplicate depending on which side moved.
#[test]
fn issue_5057_second_flush_does_not_duplicate_rows() {
    let (_tmp, mut storage) = fresh();
    two_databases(&mut storage);
    storage.set_current_db("d1").expect("use d1");

    storage.flush().expect("first flush");
    storage.flush().expect("second flush");
    storage
        .insert_in_db("d2", "t", vec![row(24)])
        .expect("insert into d2.t");
    storage.flush().expect("third flush");

    assert_eq!(
        ids(&storage.scan_in_db("d1", "t").expect("scan d1.t")),
        vec![11, 12, 13],
        "d1.t changed across repeated flushes"
    );
    assert_eq!(
        ids(&storage.scan_in_db("d2", "t").expect("scan d2.t")),
        vec![21, 22, 23, 24],
        "d2.t lost or duplicated rows across flushes"
    );
}

// ---------------------------------------------------------------------
// 2. Control group: the two families must stay distinct
// ---------------------------------------------------------------------

/// Control group. Inserting without naming a database still follows
/// `current_db`.
///
/// If a refactor routed the database-less family through the `(db, table)`
/// plumbing incorrectly — or, worse, made the named family fall back to
/// `current_db` — this fails first. The named-database assertions above
/// would still pass, because they name their target on every call.
#[test]
fn issue_5057_unnamed_family_still_follows_current_db() {
    let (tmp, mut storage) = fresh();
    storage.create_database("d1").expect("create d1");
    storage.create_database("d2").expect("create d2");
    storage.set_current_db("d2").expect("use d2");
    storage.create_table(&tbl("t")).expect("create t in d2");
    storage.insert("t", vec![row(31), row(32)]).expect("insert");

    assert_eq!(
        ids(&storage.scan_in_db("d2", "t").expect("scan d2.t")),
        vec![31, 32],
        "the database-less family stopped following current_db"
    );

    storage.flush().expect("flush");
    let body = std::fs::read_to_string(tmp.path().join("d2/t.json")).expect("read d2/t.json");
    let json: serde_json::Value = serde_json::from_str(&body).expect("valid json");
    assert_eq!(
        json["rows"].as_array().expect("rows").len(),
        2,
        "the database-less family's rows were not persisted under current_db"
    );
}

// ---------------------------------------------------------------------
// 3. Delta files stay per-database
// ---------------------------------------------------------------------

/// Delta files must not be shared between databases.
///
/// Two databases appending to one `<table>.delta` means the loader
/// replays both databases' rows into whichever table it finds under that
/// name — cross-database row contamination, not just a misplaced file.
#[test]
fn issue_5057_delta_files_are_per_database() {
    let (tmp, mut storage) = fresh();
    two_databases(&mut storage);
    storage.set_current_db("d1").expect("use d1");
    storage.flush().expect("flush");

    let d1_delta = tmp.path().join("d1/t.delta");
    let d2_delta = tmp.path().join("d2/t.delta");

    // Whether a delta exists at all depends on which branch the window
    // took (first persist writes a full snapshot, later ones append).
    // What must hold is that neither database's delta mentions the
    // other's rows.
    for (label, path, mine, theirs) in [("d1", &d1_delta, 11, 21), ("d2", &d2_delta, 21, 11)] {
        let Ok(body) = std::fs::read_to_string(path) else {
            continue;
        };
        assert!(
            !body.contains(&format!("{theirs}")),
            "{label}'s delta file contains rows belonging to the other database:\n{body}"
        );
        let _ = mine;
    }

    // The loaded result is the real assertion: contamination shows up as
    // rows in the wrong table.
    drop(storage);
    let reopened = FileStorage::new(tmp.path().to_path_buf()).expect("reopen");
    assert_eq!(
        ids(&reopened.scan_in_db("d1", "t").expect("d1")),
        vec![11, 12, 13]
    );
    assert_eq!(
        ids(&reopened.scan_in_db("d2", "t").expect("d2")),
        vec![21, 22, 23]
    );
}
