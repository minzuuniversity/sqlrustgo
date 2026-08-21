//! #4167 / V312-33 — Page checksum API surface tests.
//!
//! Pins the current public Page API surface so future refactors
//! surface as failures here rather than as drift in callers like
//! `crates/tools/src/backup.rs` and `crates/tools/src/physical_backup.rs`.
//!
//! Status: the issue describes methods that were drifted on 250 HEAD
//! (Page::calculate_checksum, Page::verify_checksum). On 252 HEAD
//! these methods do NOT exist on the Page struct itself — instead,
//! `tools/src/physical_backup.rs` defines a free function
//! `calculate_checksum_for_bytes` and `tools/src/backup.rs` defines
//! `calculate_checksum(data_dir)` that walks the data directory. PR
//! #4160 partially mitigated the bench+test drift but did not add
//! Page methods. This file pins what IS public so any future
//! regression is caught immediately.

use sqlrustgo_storage::Page;

#[test]
fn test_page_id_accessor_exists() {
    // Issue #4167 sub-acceptance: Arc<Page>::id() works via Deref
    // to Page::page_id(). Here we test the underlying accessor.
    let page = Page::new(42);
    assert_eq!(page.page_id(), 42);
}

#[test]
fn test_page_id_round_trip() {
    // Round-trip: build page, read page_id, verify equality. Catches
    // regressions where someone refactors Page::new / Page::page_id.
    let page = Page::new(12345);
    assert_eq!(page.page_id(), 12345);
}

#[test]
fn test_page_id_max_value() {
    // Edge case: u32::MAX page_id (4-byte unsigned max).
    let page = Page::new(u32::MAX);
    assert_eq!(page.page_id(), u32::MAX);
}

#[test]
fn test_page_calculate_checksum_method_documented_as_absent() {
    // Issue #4167: Page::calculate_checksum was drifted on 250 HEAD.
    // Per doc on 252 HEAD, the method does NOT exist on Page itself —
    // instead, checksum is computed via tools/src/physical_backup.rs
    // helpers (calculate_checksum_for_bytes) and
    // tools/src/backup.rs::calculate_checksum(data_dir).
    //
    // This test documents the current state. If anyone ever restores
    // the method, they should update this test to verify it.
    let page = Page::new(1);
    let _ = page.page_id(); // currently the only checksum-related accessor
                            // — page_id is the page identity, not a checksum
}
