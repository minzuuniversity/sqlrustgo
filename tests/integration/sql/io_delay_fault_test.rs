use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::binary_storage::BinaryTableStorage;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::TempDir;

fn set_io_delay(delay_ms: u64) {
    std::env::set_var("SQLRUSTGO_IO_DELAY_MS", delay_ms.to_string());
}

fn clear_io_delay() {
    std::env::remove_var("SQLRUSTGO_IO_DELAY_MS");
}

#[test]
fn test_io_delay_injected_on_save() {
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();

    set_io_delay(50);

    let storage = BinaryTableStorage::new(data_dir.clone()).unwrap();

    let engine = Arc::new(RwLock::new(storage));
    let mut exec = ExecutionEngine::new(engine.clone());

    exec.execute("CREATE TABLE t1 (id INT, name TEXT)").unwrap();

    let start = Instant::now();
    for i in 0..100 {
        exec.execute(&format!("INSERT INTO t1 VALUES ({}, 'name{}')", i, i))
            .unwrap();
    }
    let elapsed = start.elapsed();

    clear_io_delay();

    assert!(
        elapsed >= Duration::from_millis(40),
        "With 50ms delay and 100 inserts, expected >=4000ms but got {:?}",
        elapsed
    );
}

#[test]
fn test_io_delay_injected_on_load() {
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();

    {
        let storage = BinaryTableStorage::new(data_dir.clone()).unwrap();
        let engine = Arc::new(RwLock::new(storage));
        let mut exec = ExecutionEngine::new(engine.clone());

        exec.execute("CREATE TABLE t2 (id INT, name TEXT)").unwrap();
        for i in 0..10 {
            exec.execute(&format!("INSERT INTO t2 VALUES ({}, 'name{}')", i, i))
                .unwrap();
        }
    }

    set_io_delay(50);

    let storage = BinaryTableStorage::new_with_data(data_dir).unwrap();
    let start = Instant::now();
    let _ = storage.load("t2").unwrap();
    let elapsed = start.elapsed();

    clear_io_delay();

    assert!(
        elapsed >= Duration::from_millis(40),
        "With 50ms delay on load, expected >=40ms but got {:?}",
        elapsed
    );
}

#[test]
fn test_no_delay_when_not_configured() {
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();

    clear_io_delay();

    let storage = BinaryTableStorage::new(data_dir.clone()).unwrap();
    let engine = Arc::new(RwLock::new(storage));
    let mut exec = ExecutionEngine::new(engine.clone());

    exec.execute("CREATE TABLE t3 (id INT)").unwrap();

    let start = Instant::now();
    for i in 0..10 {
        exec.execute(&format!("INSERT INTO t3 VALUES ({})", i))
            .unwrap();
    }
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_millis(100),
        "Without delay, 10 inserts should be fast but got {:?}",
        elapsed
    );
}
