//! Integration test for F-26 Double-Write Buffer main-path integration
//!
//! **V311-04**: Verifies DoubleWriteBuffer works as part of the main storage path.
//!
//! This test demonstrates the complete DWB lifecycle:
//! 1. Pages are staged via `stage()`
//! 2. `fsync()` atomically flushes all staged pages
//! 3. `write_all()` copies pages to their final locations
//! 4. `recover_from_crash()` recovers pages on crash
//!
//! The DoubleWriteBuffer is now in `crates/storage/src/` (not ISOLATED tests/).

use sqlrustgo_storage::{DoubleWriteBuffer, DwbPage};

fn make_page(id: u64, data: &[u8]) -> DwbPage {
    DwbPage { id, data: data.to_vec() }
}

#[test]
fn test_dwb_main_path_write_sequence() {
    let dwb = DoubleWriteBuffer::new();

    // Step 1: Stage pages (simulates page write to DWB)
    dwb.stage(make_page(1, b"page_data_1"));
    dwb.stage(make_page(2, b"page_data_2"));
    dwb.stage(make_page(3, b"page_data_3"));

    assert_eq!(dwb.buffered_count(), 3, "3 pages in staging buffer");
    assert!(!dwb.is_full(), "DWB not at capacity");

    // Step 2: fsync() — flush all staged pages atomically
    let fsynced = dwb.fsync();
    assert_eq!(fsynced.len(), 3, "fsync returns all 3 pages");
    assert_eq!(dwb.fsync_count(), 1, "fsync_count = 1");

    // Step 3: write_all() — copy pages to final locations
    let written = dwb.write_all();
    assert_eq!(written, 3, "3 pages written to final locations");

    // Step 4: Pages are now in final location
    assert!(dwb.get_page(1).is_some(), "Page 1 in final location");
    assert!(dwb.get_page(2).is_some(), "Page 2 in final location");
    assert!(dwb.get_page(3).is_some(), "Page 3 in final location");
    assert_eq!(dwb.get_page(1).unwrap().data, b"page_data_1");
}

#[test]
fn test_dwb_crash_recovery() {
    let dwb = DoubleWriteBuffer::new();

    // Write some pages and simulate crash before write_all
    dwb.stage(make_page(100, b"critical_data"));
    dwb.stage(make_page(101, b"more_critical"));

    // Simulate crash — data is in DWB but not yet at final location
    dwb.simulate_crash();
    // Simulate partial write to final location (torn page)
    dwb.write_page(make_page(100, b"TORN"));

    // Recovery: DWB pages take precedence over partial writes
    let recovered = dwb.recover_from_crash();
    assert_eq!(recovered.len(), 2, "Both pages recovered from DWB");
    assert_eq!(recovered[0].data, b"critical_data", "Correct data recovered");
    assert_eq!(dwb.recovery_count(), 1, "recovery_count incremented");
}

#[test]
fn test_dwb_capacity_and_eviction() {
    let dwb = DoubleWriteBuffer::with_capacity(4);

    // Fill to capacity
    for i in 0..4 {
        dwb.stage(make_page(i, format!("data_{}", i).as_bytes()));
    }

    assert!(dwb.is_full(), "DWB should be full at capacity");

    // Adding 5th page evicts the oldest (page 0)
    dwb.stage(make_page(4, b"data_4"));

    // Buffer still has 4 pages (oldest evicted)
    assert_eq!(dwb.buffered_count(), 4);

    // fsync and write_all — only 4 pages present
    let written = dwb.write_all();
    assert_eq!(written, 4);

    // Page 0 was evicted and not written
    assert!(dwb.get_page(0).is_none(), "Evicted page 0 not in final location");
}

#[test]
fn test_dwb_multiple_write_cycles() {
    let dwb = DoubleWriteBuffer::new();

    // First cycle: stage -> write_all (which calls fsync internally)
    dwb.stage(make_page(1, b"batch1"));
    dwb.stage(make_page(2, b"batch1"));
    dwb.write_all();

    // Second cycle
    dwb.stage(make_page(3, b"batch2"));
    dwb.stage(make_page(4, b"batch2"));
    dwb.write_all();

    // Both batches should be in final location
    assert!(dwb.get_page(1).is_some());
    assert!(dwb.get_page(2).is_some());
    assert!(dwb.get_page(3).is_some());
    assert!(dwb.get_page(4).is_some());

    assert_eq!(dwb.fsync_count(), 2, "Two fsync cycles recorded");
}
