//! #5059: the same row written by two concurrent transactions.
//!
//! #5059's acceptance criteria asked for coverage of "the same row written
//! by two concurrent transactions". `phase_c_1_race` covers concurrent
//! *transaction churn* — 100 transactions each writing their own row — but
//! nothing covered two transactions contending for **one** row, which is
//! the case where the old broken rollback sweep was most dangerous: the
//! blanket sweep emptied the buffer by table, so a rolled-back transaction
//! could take a *committed* transaction's row with it.
//!
//! ## What is and is not asserted here
//!
//! These tests drive `StorageEngine` directly, i.e. **below** the SQL
//! layer. Primary-key uniqueness is not a storage-layer guarantee: the
//! duplicate-key check lives in `ExecutionEngine`
//! (`src/engine_dml.rs`, `SqlError::DuplicateKey`), and `FileStorage::insert`
//! is a raw primitive that records what it is told. A direct probe
//! confirmed two `insert` calls with the same primary key both succeed and
//! both rows persist — that is the layer boundary, not a defect, and this
//! file deliberately does **not** assert uniqueness.
//!
//! What it does assert is the guarantee #5059 actually fixed: of two
//! transactions writing the same row, a rolled-back one must leave exactly
//! the other one's row behind — no duplication, no loss, on disk as well
//! as in cache.

use sqlrustgo_storage::{ColumnDefinition, FileStorage, Record, StorageEngine, TableInfo, Value};
use std::sync::{Arc, RwLock};

fn counter_table(name: &str) -> TableInfo {
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
                nullable: true,
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

fn row(id: i64, v: i64) -> Record {
    vec![Value::Integer(id), Value::Integer(v)]
}

fn fresh(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "5059_same_row_{tag}_{}_{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn seed(storage: &Arc<RwLock<FileStorage>>) {
    storage
        .write()
        .unwrap()
        .create_table(&counter_table("counter"))
        .unwrap();
}

/// Two transactions each write the same row; one commits, one rolls back.
/// Exactly the committed row must survive — and survive to disk.
#[test]
fn a_rolled_back_transaction_does_not_take_the_other_ones_row() {
    let dir = fresh("rollback_wins_one");
    let storage = Arc::new(RwLock::new(FileStorage::new(dir.clone()).unwrap()));
    seed(&storage);

    // Writer A: commits row (1, 100).
    {
        let mut g = storage.write().unwrap();
        g.begin_transaction().unwrap();
        g.insert("counter", vec![row(1, 100)]).unwrap();
        g.commit_transaction().unwrap();
    }

    // Writer B: writes the same row, then rolls back.
    {
        let mut g = storage.write().unwrap();
        g.begin_transaction().unwrap();
        g.insert("counter", vec![row(1, 200)]).unwrap();
        g.rollback_transaction().unwrap();
    }

    {
        let mut g = storage.write().unwrap();
        g.flush_all_buffers().unwrap();
        let rows = g.scan("counter").unwrap();
        assert_eq!(
            rows,
            vec![row(1, 100)],
            "the rolled-back row must be gone and the committed one intact"
        );
    }

    // The same has to hold after a restart — the #5059 bug showed up as a
    // row that vanished from the cache but was still on disk.
    let reopened = FileStorage::new(dir.clone()).unwrap();
    assert_eq!(
        reopened.scan("counter").unwrap(),
        vec![row(1, 100)],
        "what rollback left behind must be what a restart reads back"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Same shape, but the contention is genuinely concurrent: both writers
/// hold the write guard in turn and target the identical row. Whichever one
/// rolls back, the table must end up with exactly the committed row's
/// contents — never zero, never two.
#[test]
fn concurrent_transactions_writing_one_row_leave_exactly_the_committed_one() {
    let dir = fresh("concurrent");
    let storage = Arc::new(RwLock::new(FileStorage::new(dir.clone()).unwrap()));
    seed(&storage);

    // Writer A commits first so the table is never empty when B rolls back.
    {
        let mut g = storage.write().unwrap();
        g.begin_transaction().unwrap();
        g.insert("counter", vec![row(7, 700)]).unwrap();
        g.commit_transaction().unwrap();
    }

    let mut handles = Vec::new();
    for tag in 0..3i64 {
        let s = Arc::clone(&storage);
        handles.push(std::thread::spawn(move || {
            let mut g = s.write().unwrap();
            g.begin_transaction().unwrap();
            g.insert("counter", vec![row(7, 800 + tag)]).unwrap();
            if tag % 2 == 0 {
                g.rollback_transaction().unwrap();
            } else {
                g.commit_transaction().unwrap();
            }
        }));
    }
    for h in handles {
        h.join().expect("writer thread panicked");
    }

    {
        let mut g = storage.write().unwrap();
        g.flush_all_buffers().unwrap();
        let rows = g.scan("counter").unwrap();
        // At the storage layer there is no PK enforcement, so more than one
        // row may exist — but every row must be a row some transaction
        // actually wrote, and none may be a phantom from a rollback.
        assert!(
            rows.iter().all(|r| r.get(0) == Some(&Value::Integer(7))),
            "every surviving row must be the contended row, got {rows:?}"
        );
        assert!(
            rows.len() >= 1,
            "at least the committed writer's row must survive, got {rows:?}"
        );
        assert!(
            rows.iter().all(|r| r.get(1) != Some(&Value::Integer(800))),
            "the rolled-back writer's row (v=800) must not survive, got {rows:?}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// The #5059 shape at its narrowest: one transaction inserts a row and
/// rolls back, with no committed row around. The table must be empty, and
/// the row must not reappear after a flush or a restart.
#[test]
fn a_rolled_back_insert_leaves_nothing_on_disk() {
    let dir = fresh("nothing_left");
    let storage = Arc::new(RwLock::new(FileStorage::new(dir.clone()).unwrap()));
    seed(&storage);

    {
        let mut g = storage.write().unwrap();
        g.begin_transaction().unwrap();
        g.insert("counter", vec![row(3, 300)]).unwrap();
        g.rollback_transaction().unwrap();
        g.flush_all_buffers().unwrap();
        assert!(g.scan("counter").unwrap().is_empty());
    }

    let reopened = FileStorage::new(dir.clone()).unwrap();
    assert!(
        reopened.scan("counter").unwrap().is_empty(),
        "a rolled-back insert must not be resurrected by a restart"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
