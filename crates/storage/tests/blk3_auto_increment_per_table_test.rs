// BLK-3 / Issue #4945 — AUTO_INCREMENT sequences must be monotonic across
// concurrent autocommit connections.
//
// docs/releases/v4.1.0/ISSUES_PLAN.md §4.8 (BLK-3).
//
// Before the fix, the `MemoryStorage::insert` path computed `next_auto` by
// scanning the existing rows in the table. With 8 concurrent connections
// each inserting into an empty table, every connection saw an empty row
// set and assigned id=1 to its first insert. The result was 480 successful
// inserts but only ~99 distinct ids (duplicates were tolerated because
// the first connection committed first and the rest overwrote).
//
// The fix introduces a shared per-table `AtomicU64` counter that is
// advanced monotonically. The counter is also lifted by the max id
// (explicit or auto-assigned) seen in any committed batch, so the
// `INSERT id=500; INSERT;` sequence continues from 501 as MySQL semantics
// require.
//
// Each test below pins a different aspect of the contract.

use parking_lot::RwLock;
use sqlrustgo_storage::engine::{ColumnDefinition, MemoryStorage, StorageEngine, TableInfo, Value};
use std::sync::Arc;

fn table_with_auto_inc(name: &str) -> TableInfo {
    TableInfo {
        name: name.to_string(),
        columns: vec![ColumnDefinition {
            name: "id".to_string(),
            data_type: "INTEGER".to_string(),
            auto_increment: true,
            primary_key: true,
            ..Default::default()
        }],
        ..Default::default()
    }
}

#[test]
fn single_connection_monotonic_ids() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    storage
        .write()
        .create_table(&table_with_auto_inc("t"))
        .unwrap();

    let mut s = storage.write();
    for _ in 0..5 {
        s.insert("t", vec![vec![Value::Null]]).unwrap();
    }
    drop(s);

    let rows = storage.read().scan("t").unwrap();
    let ids: Vec<i64> = rows
        .iter()
        .map(|r| match r.first().unwrap() {
            Value::Integer(n) => *n,
            other => panic!("expected Integer, got {:?}", other),
        })
        .collect();
    assert_eq!(
        ids,
        vec![1, 2, 3, 4, 5],
        "single-thread ids must be contiguous"
    );
}

#[test]
fn explicit_id_advances_counter() {
    // Insert id=500 explicitly, then insert with null — must be 501, not 1.
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    storage
        .write()
        .create_table(&table_with_auto_inc("t"))
        .unwrap();

    let mut s = storage.write();
    s.insert("t", vec![vec![Value::Integer(500)]]).unwrap();
    s.insert("t", vec![vec![Value::Null]]).unwrap();
    drop(s);

    let rows = storage.read().scan("t").unwrap();
    let mut ids: Vec<i64> = rows
        .iter()
        .map(|r| match r.first().unwrap() {
            Value::Integer(n) => *n,
            _ => panic!("expected Integer"),
        })
        .collect();
    ids.sort();
    assert_eq!(
        ids,
        vec![500, 501],
        "next auto id after explicit 500 must be 501"
    );
}

#[test]
fn concurrent_autocommit_yields_unique_monotonic_ids() {
    // The exact reproduction from #4945: 8 threads x 60 inserts each.
    // Before fix: id range ≈ 1..99 with 480 successful inserts. After fix:
    // exactly 480 distinct ids.
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    storage
        .write()
        .create_table(&table_with_auto_inc("t"))
        .unwrap();
    let storage = Arc::new(storage);

    const THREADS: usize = 8;
    const PER_THREAD: usize = 60;
    let mut handles = Vec::new();
    for _ in 0..THREADS {
        let s = Arc::clone(&storage);
        handles.push(std::thread::spawn(move || {
            for _ in 0..PER_THREAD {
                let mut g = s.write();
                g.insert("t", vec![vec![Value::Null]]).unwrap();
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }

    let rows = storage.read().scan("t").unwrap();
    let total = THREADS * PER_THREAD;
    assert_eq!(rows.len(), total, "all inserts must commit");

    let mut ids: Vec<i64> = rows
        .iter()
        .map(|r| match r.first().unwrap() {
            Value::Integer(n) => *n,
            _ => panic!("expected Integer"),
        })
        .collect();
    ids.sort();
    let unique: std::collections::HashSet<i64> = ids.iter().copied().collect();
    assert_eq!(
        unique.len(),
        total,
        "all ids must be unique across concurrent autocommit connections"
    );
    // The smallest id must be 1 and the largest must equal total (monotonic).
    assert_eq!(ids[0], 1, "smallest auto id must be 1");
    assert_eq!(
        ids[total - 1],
        total as i64,
        "largest auto id must equal insert count (monotonic, no duplicates)"
    );
}
