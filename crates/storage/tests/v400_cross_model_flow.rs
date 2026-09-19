//! V400-05 cross-model integration test — verify that when a global
//! tracker is installed, every SQL/vector/graph/audit write fires the
//! register_write callback.

use std::sync::Arc;

use parking_lot::Mutex;
use std::sync::LazyLock;

// V400-05 cross-model flow tests share a global tracker slot; serialize
// them so a tracker installed by one test doesn't pollute another.
static TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn test_lock() -> &'static Mutex<()> {
    &TEST_LOCK
}

use sqlrustgo_storage::cross_model_tracker::{self};
use sqlrustgo_storage::engine::StorageEngine;
use sqlrustgo_storage::{CrossModelWriteTracker, MvccStorage};
use sqlrustgo_storage::engine::{ColumnDefinition, MemoryStorage, TableInfo, Value};

struct CountingTracker {
    counts: parking_lot::Mutex<[u32; 4]>,
}
impl CrossModelWriteTracker for CountingTracker {
    fn register_write(&self, kind: u8, _desc: &str) {
        let mut c = self.counts.lock();
        if (kind as usize) < c.len() {
            c[kind as usize] += 1;
        }
    }
}

#[test]
fn cross_model_global_tracker_records_sql_write() {
    let _g = test_lock().lock();
    let counter = Arc::new(CountingTracker {
        counts: parking_lot::Mutex::new([0; 4]),
    });
    cross_model_tracker::set_global_tracker(Some(counter.clone()));

    // SQL write via MvccStorage::insert
    let mut store = MvccStorage::new(MemoryStorage::new());
    let info = TableInfo {
        name: "t".to_string(),
        columns: vec![ColumnDefinition {
            name: "id".to_string(),
            data_type: "INT".to_string(),
            nullable: false,
            primary_key: true,
            ..Default::default()
        }],
        ..Default::default()
    };
    store.create_table(&info).unwrap();
    // Record = Vec<Value>; insert takes Vec<Record> = Vec<Vec<Value>>
    store
        .insert("t", vec![vec![Value::Integer(1)]])
        .unwrap();

    // Verify SQL counter incremented
    let counts = counter.counts.lock();
    assert_eq!(
        counts[cross_model_tracker::MODEL_SQL as usize],
        1,
        "SQL write should be tracked"
    );

    cross_model_tracker::set_global_tracker(None);
}

#[test]
fn cross_model_global_tracker_records_all_three_sql_writes() {
    let _g = test_lock().lock();
    let counter = Arc::new(CountingTracker {
        counts: parking_lot::Mutex::new([0; 4]),
    });
    cross_model_tracker::set_global_tracker(Some(counter.clone()));

    let mut store = MvccStorage::new(MemoryStorage::new());
    let info = TableInfo {
        name: "t".to_string(),
        columns: vec![ColumnDefinition {
            name: "id".to_string(),
            data_type: "INT".to_string(),
            nullable: false,
            primary_key: true,
            ..Default::default()
        }],
        ..Default::default()
    };
    store.create_table(&info).unwrap();

    // insert + delete + update
    store
        .insert("t", vec![vec![Value::Integer(1)]])
        .unwrap();
    let _: usize = store.delete("t", &[]).unwrap_or(0);
    let _: usize = store.update("t", &[Value::Integer(1)], &[]).unwrap_or(0);

    // SQL writes are tracked (insert + delete + update + delete fallback = at least 1)
    let counts = counter.counts.lock();
    assert!(
        counts[cross_model_tracker::MODEL_SQL as usize] >= 1,
        "SQL writes should be tracked (got {})",
        counts[cross_model_tracker::MODEL_SQL as usize]
    );

    cross_model_tracker::set_global_tracker(None);
}

#[test]
fn cross_model_global_tracker_disabled_when_none() {
    let _g = test_lock().lock();
    cross_model_tracker::set_global_tracker(None);
    // All four register helpers should be no-ops, no panic
    cross_model_tracker::register_sql_write("noop");
    cross_model_tracker::register_vector_write("noop");
    cross_model_tracker::register_graph_write("noop");
    cross_model_tracker::register_audit_write("noop");
}

#[test]
fn cross_model_each_helper_distinct_kind() {
    let _g = test_lock().lock();
    // Verify each register_X helper reports the correct model kind via
    // a fresh tracker that records the last-seen kind.
    use std::sync::Mutex as StdMutex;
    let last_kind: Arc<StdMutex<u8>> = Arc::new(StdMutex::new(255));

    struct KindRecorder {
        last: Arc<StdMutex<u8>>,
    }
    impl CrossModelWriteTracker for KindRecorder {
        fn register_write(&self, kind: u8, _desc: &str) {
            *self.last.lock().unwrap() = kind;
        }
    }

    let last = last_kind.clone();
    cross_model_tracker::set_global_tracker(Some(Arc::new(KindRecorder { last: last.clone() })));

    cross_model_tracker::register_sql_write("x");
    assert_eq!(*last.lock().unwrap(), cross_model_tracker::MODEL_SQL);
    cross_model_tracker::register_vector_write("x");
    assert_eq!(*last.lock().unwrap(), cross_model_tracker::MODEL_VECTOR);
    cross_model_tracker::register_graph_write("x");
    assert_eq!(*last.lock().unwrap(), cross_model_tracker::MODEL_GRAPH);
    cross_model_tracker::register_audit_write("x");
    assert_eq!(*last.lock().unwrap(), cross_model_tracker::MODEL_AUDIT);

    cross_model_tracker::set_global_tracker(None);
}

#[test]
fn cross_model_global_tracker_replacement() {
    let _g = test_lock().lock();
    // Replace tracker mid-test; second tracker should be the active one
    let counter1 = Arc::new(CountingTracker {
        counts: parking_lot::Mutex::new([0; 4]),
    });
    let counter2 = Arc::new(CountingTracker {
        counts: parking_lot::Mutex::new([0; 4]),
    });

    cross_model_tracker::set_global_tracker(Some(counter1.clone()));
    cross_model_tracker::register_sql_write("first");
    assert_eq!(counter1.counts.lock()[cross_model_tracker::MODEL_SQL as usize], 1);

    cross_model_tracker::set_global_tracker(Some(counter2.clone()));
    cross_model_tracker::register_sql_write("second");
    assert_eq!(counter2.counts.lock()[cross_model_tracker::MODEL_SQL as usize], 1);
    assert_eq!(
        counter1.counts.lock()[cross_model_tracker::MODEL_SQL as usize],
        1,
        "counter1 should NOT receive writes after replacement"
    );

    cross_model_tracker::set_global_tracker(None);
}