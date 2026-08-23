//! L2: 50 concurrent SELECT + 1 LOAD. Verify LOAD doesn't block SELECT for long.
//!
//! Schema uses BIGINT (not INTEGER) because BINT v3's encode_value_to_bytes
//! emits Value::Integer as i64 (8 bytes) while column_width for INTEGER is
//! 4 bytes — a known pre-existing bug tracked for the final review. BIGINT
//! (width 8) matches the encoding.

use sqlrustgo_storage::binary_storage_v2::BinaryTableStorageV2;
use sqlrustgo_storage::engine::{ColumnDefinition, Record, StorageEngine, Value};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

#[test]
fn test_concurrent_select_during_load() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let mut storage = BinaryTableStorageV2::new(temp_dir.path().to_path_buf()).expect("V2 init");

    let mut id_col = ColumnDefinition::new("id", "BIGINT");
    id_col.primary_key = true;
    let schema = vec![id_col];
    storage.create_table("t1", schema).expect("create_table");

    // Pre-populate 1000 rows
    let initial: Vec<Record> = (0..1000i64).map(|i| vec![Value::Integer(i)]).collect();
    storage
        .insert_streaming("t1", initial)
        .expect("initial insert_streaming");
    storage.flush().expect("initial flush");

    let storage_arc = Arc::new(std::sync::Mutex::new(storage));
    let storage_arc_clone = storage_arc.clone();

    // Spawn 50 SELECT threads — measure that each completes quickly AFTER
    // acquiring the lock (lock-acquire + scan() round-trip < 100ms).
    let mut handles = vec![];
    for tid in 0..50 {
        let s = storage_arc.clone();
        handles.push(thread::spawn(move || {
            let start = Instant::now();
            let storage = s.lock().expect("lock");
            let _rows = storage.scan("t1").expect("scan");
            let elapsed = start.elapsed();
            assert!(
                elapsed < Duration::from_millis(100),
                "thread {} SELECT took {:?}",
                tid,
                elapsed
            );
        }));
    }

    // LOAD 100k more rows (acquires lock for the duration)
    let more: Vec<Record> = (1000..101_000i64)
        .map(|i| vec![Value::Integer(i)])
        .collect();
    {
        let mut storage = storage_arc_clone.lock().expect("lock for LOAD");
        storage
            .insert_streaming("t1", more)
            .expect("LOAD insert_streaming");
        storage.flush().expect("LOAD flush");
    }

    for h in handles {
        h.join().expect("select thread join");
    }
}
