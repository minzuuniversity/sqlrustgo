//! V400-02 / Issue #3730 (V1): WAL entry type extension for vector ops.
//!
//! Verifies the `WalEntryType` enum now carries the 6 vector variants
//! (VectorInsert, VectorUpdate, VectorDelete, CreateVectorIndex,
//! DropVectorIndex, RebuildVectorIndex) and that they round-trip
//! through `WalEntryType::from_u8(...)` without conflict with the
//! pre-existing SQL DML variants (Insert/Update/Delete).
//!
//! The full `to_bytes()` / `from_bytes()` round-trip with the vector
//! variants is exercised in `v400_vector_wal.rs` (already 18/18 PASS).

use sqlrustgo_storage::wal_legacy::WalEntryType;

#[test]
fn vector_variants_are_added_and_distinct() {
    // SQL DML (1-8) and vector ops (9-14) must all be distinct.
    let variants = [
        WalEntryType::VectorInsert as u8,
        WalEntryType::VectorUpdate as u8,
        WalEntryType::VectorDelete as u8,
        WalEntryType::CreateVectorIndex as u8,
        WalEntryType::DropVectorIndex as u8,
        WalEntryType::RebuildVectorIndex as u8,
    ];
    let set: std::collections::HashSet<u8> = variants.iter().copied().collect();
    assert_eq!(
        set.len(),
        6,
        "all 6 vector variants must have distinct discriminant values, got {:?}",
        variants
    );
}

#[test]
fn vector_variants_dont_collide_with_sql_variants() {
    // The vector enum values (9-14) must not collide with the SQL DML
    // variants (1-8). This guards against accidental re-numbering that
    // would silently corrupt existing WAL files on disk.
    let sql = [
        WalEntryType::Begin as u8,
        WalEntryType::Insert as u8,
        WalEntryType::Update as u8,
        WalEntryType::Delete as u8,
        WalEntryType::Commit as u8,
        WalEntryType::Rollback as u8,
        WalEntryType::Checkpoint as u8,
        WalEntryType::Prepare as u8,
    ];
    let vector = [
        WalEntryType::VectorInsert as u8,
        WalEntryType::VectorUpdate as u8,
        WalEntryType::VectorDelete as u8,
        WalEntryType::CreateVectorIndex as u8,
        WalEntryType::DropVectorIndex as u8,
        WalEntryType::RebuildVectorIndex as u8,
    ];
    for v in vector {
        assert!(
            !sql.contains(&v),
            "vector variant discriminant {} collides with an SQL variant",
            v
        );
    }
}

#[test]
fn from_u8_round_trips_all_variants() {
    let variants = [
        (9u8, WalEntryType::VectorInsert),
        (10u8, WalEntryType::VectorUpdate),
        (11u8, WalEntryType::VectorDelete),
        (12u8, WalEntryType::CreateVectorIndex),
        (13u8, WalEntryType::DropVectorIndex),
        (14u8, WalEntryType::RebuildVectorIndex),
    ];
    for (raw, expected) in variants {
        let parsed = WalEntryType::from_u8(raw)
            .unwrap_or_else(|| panic!("from_u8({}) should yield a variant", raw));
        assert_eq!(
            parsed, expected,
            "from_u8({}) yielded {:?}, expected {:?}",
            raw, parsed, expected
        );
    }
}

#[test]
fn from_u8_legacy_values_still_work() {
    // V1 must not break the existing 8-variant encoding.
    let legacy = [
        (1u8, WalEntryType::Begin),
        (2u8, WalEntryType::Insert),
        (3u8, WalEntryType::Update),
        (4u8, WalEntryType::Delete),
        (5u8, WalEntryType::Commit),
        (6u8, WalEntryType::Rollback),
        (7u8, WalEntryType::Checkpoint),
        (8u8, WalEntryType::Prepare),
    ];
    for (raw, expected) in legacy {
        let parsed = WalEntryType::from_u8(raw)
            .unwrap_or_else(|| panic!("legacy from_u8({}) should still work", raw));
        assert_eq!(parsed, expected);
    }
}

#[test]
fn from_u8_out_of_range_returns_none() {
    // 15+ and 0 must remain `None` so old code paths that hit an
    // unknown variant gracefully skip rather than panic.
    assert_eq!(WalEntryType::from_u8(0), None);
    assert_eq!(WalEntryType::from_u8(15), None);
    assert_eq!(WalEntryType::from_u8(255), None);
}
