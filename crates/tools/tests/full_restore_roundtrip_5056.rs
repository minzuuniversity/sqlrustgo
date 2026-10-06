//! #5056: full-backup restore produced rows one column off.
//!
//! `DataRestorer::restore_sql` fed `parse_insert` a slice that started at
//! the `VALUES` keyword, so the first "column" of every restored row was
//! the literal text `VALUES (1`. Restoring the demo `orders` table gave:
//!
//! ```text
//! ["Text(\"VALUES (1\")", Int(1), Float(99.99), Text("completed")]
//! ```
//!
//! — a text primary key where an integer belonged. It went unnoticed
//! because `restore_backup` returned `()` and the only assertion anywhere
//! was that the target directory existed.
//!
//! These tests assert on restored **content**, which is the whole point.

use sqlrustgo_storage::{StorageEngine, Value};
use sqlrustgo_tools::backup::{create_full_backup_from_demo, restore_backup_into};

/// The demo `orders` table is
/// `(1, 1, 99.99, completed) (2, 1, 149.50, pending) (3, 2, 29.99, completed)`.
/// All four columns must come back with their original types and values.
#[test]
fn issue_5056_full_restore_preserves_column_count_and_types() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let full = tmp.path().join("full");
    create_full_backup_from_demo(&full, "sql").expect("full backup");

    let restored = restore_backup_into(&full, &tmp.path().join("t"), true).expect("restore");
    let rows = restored.scan("orders").expect("scan orders");

    assert_eq!(
        rows.len(),
        3,
        "orders must come back with 3 rows, got {}",
        rows.len()
    );

    for r in &rows {
        assert_eq!(
            r.len(),
            4,
            "every restored row must have 4 columns, got {r:?} — a leading \
             `VALUES (n` column means parse_insert sliced from the keyword"
        );
    }

    let mut ids: Vec<i64> = rows
        .iter()
        .map(|r| match r.first() {
            Some(Value::Integer(v)) => *v,
            other => {
                panic!("id column must be Integer, got {other:?} — the value was parsed as text")
            }
        })
        .collect();
    ids.sort();
    assert_eq!(ids, vec![1, 2, 3], "ids must be integers 1..3, got {ids:?}");

    let first = &rows[0];
    assert_eq!(
        first.get(2),
        Some(&Value::Float(99.99)),
        "total column must survive as Float"
    );
    assert_eq!(
        first.get(3),
        Some(&Value::Text("completed".to_string())),
        "status column must survive as Text"
    );
}

/// `value_to_sql` escapes a literal quote by doubling it (`''`), and a
/// comma inside a quoted string is not a column separator. The bare
/// `split(',')` this replaces turned `'a,b'` into two columns.
#[test]
fn issue_5056_commas_inside_text_values_do_not_split_columns() {
    use sqlrustgo_storage::DataRestorer;

    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("t.sql");
    // One row, two columns; the first value contains a comma AND a
    // doubled quote.
    std::fs::write(&data, "INSERT INTO t VALUES (1, 'a,b');").expect("write");

    let mut storage = sqlrustgo_storage::MemoryStorage::new();
    let n = DataRestorer::restore_from_backup(
        &mut storage,
        &data,
        sqlrustgo_storage::BackupFormat::Sql,
    )
    .expect("restore");
    assert_eq!(n, 1, "one INSERT statement must restore one row");

    let rows = storage.scan("t").expect("scan");
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].len(),
        2,
        "the comma must not create a third column: {rows:?}"
    );
    assert_eq!(rows[0][0], Value::Integer(1));
    assert_eq!(
        rows[0][1],
        Value::Text("a,b".to_string()),
        "the quoted comma must stay inside the value"
    );
}

/// A doubled quote is an escape, not a terminator: `'it''s'` is the single
/// value `it's`.
#[test]
fn issue_5056_escaped_quotes_do_not_end_the_value() {
    use sqlrustgo_storage::DataRestorer;

    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("t.sql");
    std::fs::write(&data, "INSERT INTO t VALUES (7, 'it''s, fine');").expect("write");

    let mut storage = sqlrustgo_storage::MemoryStorage::new();
    DataRestorer::restore_from_backup(&mut storage, &data, sqlrustgo_storage::BackupFormat::Sql)
        .expect("restore");

    let rows = storage.scan("t").expect("scan");
    assert_eq!(rows[0].len(), 2, "got {rows:?}");
    assert_eq!(rows[0][0], Value::Integer(7));
    assert_eq!(
        rows[0][1],
        Value::Text("it's, fine".to_string()),
        "the doubled quote must unescape to one quote, and the comma must \
         not split the value"
    );
}
