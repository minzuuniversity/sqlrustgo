//! Integration test for F-25 Change Buffer main-path integration
//!
//! **V311-03**: Verifies ChangeBuffer works as part of the main storage path.
//!
//! This test demonstrates the complete change buffer lifecycle:
//! 1. Secondary index updates are deferred via `defer_update()`
//! 2. Pages loaded from storage have deferred changes merged via `merge_on_read()`
//! 3. Flushing returns all pending entries
//!
//! The ChangeBuffer is now in `crates/storage/src/` (not ISOLATED tests/).

use sqlrustgo_storage::{ChangeBuffer, ChangeOp};

#[test]
fn test_change_buffer_main_path_integration() {
    let cb = ChangeBuffer::new();

    // Step 1: Defer several secondary index updates
    cb.defer_update(
        1,
        ChangeOp::Insert {
            key: b"idx_a".to_vec(),
            value: b"1".to_vec(),
        },
    );
    cb.defer_update(
        1,
        ChangeOp::Update {
            key: b"idx_a".to_vec(),
            new_value: b"2".to_vec(),
        },
    );
    cb.defer_update(
        2,
        ChangeOp::Delete {
            key: b"idx_b".to_vec(),
        },
    );
    cb.defer_update(
        3,
        ChangeOp::Insert {
            key: b"idx_c".to_vec(),
            value: b"3".to_vec(),
        },
    );

    assert_eq!(cb.pending_count(), 4, "Should have 4 deferred updates");

    // Step 2: Simulate reading page 1 — merge_on_read applies all changes for that page
    let merged = cb.merge_on_read(1);
    assert_eq!(merged.len(), 2, "Page 1 should have 2 deferred changes");
    assert_eq!(cb.pending_count(), 2, "2 entries remain for pages 2 and 3");
    assert_eq!(cb.merge_count(), 1, "merge_count should be 1");

    // Step 3: Read page 2
    let merged = cb.merge_on_read(2);
    assert_eq!(merged.len(), 1, "Page 2 should have 1 deferred change");
    assert_eq!(cb.pending_count(), 1, "Only page 3 remains");

    // Step 4: Read page 3
    let merged = cb.merge_on_read(3);
    assert_eq!(merged.len(), 1, "Page 3 should have 1 deferred change");
    assert_eq!(cb.pending_count(), 0, "All deferred updates consumed");

    // Step 5: Verify merge_count accumulated correctly
    assert_eq!(cb.merge_count(), 3, "All 3 pages were merged");
}

#[test]
fn test_change_buffer_flush_integrates() {
    let cb = ChangeBuffer::with_capacity(5);

    // Fill up to just under threshold
    for i in 0..4 {
        cb.defer_update(
            i,
            ChangeOp::Insert {
                key: format!("key_{}", i).into_bytes(),
                value: format!("value_{}", i).into_bytes(),
            },
        );
    }

    assert!(
        !cb.should_flush(),
        "Below threshold (4 < 5), should not flush"
    );

    // At threshold
    cb.defer_update(
        4,
        ChangeOp::Insert {
            key: b"d".to_vec(),
            value: b"4".to_vec(),
        },
    );
    assert!(cb.should_flush(), "At threshold (5 >= 5), should flush");

    // Flush returns all entries
    let flushed = cb.flush();
    assert_eq!(flushed.len(), 5, "All 5 entries should be flushed");
    assert_eq!(cb.pending_count(), 0, "Pending queue should be empty");
    assert_eq!(cb.flush_count(), 1, "Should record 1 flush");

    // After flush, merge_on_read returns nothing
    assert!(cb.merge_on_read(0).is_empty());
    assert!(cb.merge_on_read(3).is_empty());
}

#[test]
fn test_change_buffer_capacity_threshold() {
    let cb = ChangeBuffer::with_capacity(3);

    // Just under threshold
    cb.defer_update(
        1,
        ChangeOp::Insert {
            key: b"a".to_vec(),
            value: b"1".to_vec(),
        },
    );
    cb.defer_update(
        2,
        ChangeOp::Insert {
            key: b"b".to_vec(),
            value: b"2".to_vec(),
        },
    );
    assert!(!cb.should_flush(), "Below threshold, should not flush");

    // At threshold
    cb.defer_update(
        3,
        ChangeOp::Insert {
            key: b"c".to_vec(),
            value: b"3".to_vec(),
        },
    );
    assert!(cb.should_flush(), "At threshold, should flush");

    // Flush and verify threshold resets
    let _ = cb.flush();
    assert!(!cb.should_flush(), "After flush, threshold should reset");
}

#[test]
fn test_change_buffer_multiple_pages() {
    let cb = ChangeBuffer::new();

    // Many pages, each with one update
    for page_id in 1..=10 {
        cb.defer_update(
            page_id,
            ChangeOp::Insert {
                key: vec![page_id as u8],
                value: vec![(page_id * 2) as u8],
            },
        );
    }

    assert_eq!(cb.pending_count(), 10);

    // Read pages 5, 3, 7 — only those should be merged
    let merged_5 = cb.merge_on_read(5);
    let merged_3 = cb.merge_on_read(3);
    let merged_7 = cb.merge_on_read(7);

    assert_eq!(merged_5.len(), 1);
    assert_eq!(merged_3.len(), 1);
    assert_eq!(merged_7.len(), 1);

    assert_eq!(cb.pending_count(), 7, "7 pages remain");
    assert_eq!(cb.merge_count(), 3, "3 pages merged");
}
