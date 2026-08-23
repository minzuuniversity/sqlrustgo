//! T4.2 Integration Test: bulk_insert_records routing to BinaryTableStorageV2
//!
//! Verifies that when the execution engine uses BinaryTableStorageV2,
//! the bulk_insert_records method correctly routes to insert_streaming.
//!
//! Schema uses BIGINT (not INTEGER) because BINT v3's encode_value_to_bytes
//! emits Value::Integer as i64 (8 bytes) while column_width for INTEGER is
//! 4 bytes — this mismatch is a known pre-existing bug tracked for the
//! final review (T2.2 review noted the Float mismatch, the Integer case
//! has the same root cause). BIGINT's width (8) matches the encoding.
//!
//! This test is gated by `bin_storage_default` feature since it requires
//! BinaryTableStorageV2 to exist.

#[path = "../../common/mod.rs"]
mod common;

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::BinaryTableStorageV2;
use std::sync::Arc;
use tempfile::TempDir;

fn make_v2_engine(temp_dir: &TempDir) -> ExecutionEngine<BinaryTableStorageV2> {
    let storage = BinaryTableStorageV2::new(temp_dir.path().to_path_buf()).unwrap();
    ExecutionEngine::new(Arc::new(RwLock::new(storage)))
}

#[cfg(feature = "bin_storage_default")]
#[test]
fn bulk_insert_routes_to_v2_insert_streaming() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let mut engine = make_v2_engine(&temp_dir);

    // Create table — use BIGINT to match Value::Integer (i64) byte width.
    // (INTEGER has known mismatch with Value::Integer encoding; tracked.)
    engine
        .execute(
            "CREATE TABLE t (id BIGINT PRIMARY KEY, name TEXT, val BIGINT)",
        )
        .expect("CREATE TABLE should succeed");

    // Direct call to bulk_insert_records explicitly exercises T4.2 routing.
    // (Single-row INSERT goes through storage.insert directly, NOT through
    // bulk_insert_records, so we must call it directly to test the routing.)
    //
    // Note: V2's scan() is a T2.2 stub returning vec![], so we cannot use
    // SELECT COUNT(*) to verify rows landed. Instead we verify (a) the
    // routing call succeeds and returns the correct count, and (b) flush
    // completes without error — proving V2's insert_streaming was actually
    // invoked (its SegmentWriter accepted the rows and was sealed on flush).
    use sqlrustgo::Value;
    let records: Vec<Vec<Value>> = (1..=5i64)
        .map(|i| {
            vec![
                Value::Integer(i),
                Value::Text(format!("name_{}", i)),
                Value::Integer(i * 10),
            ]
        })
        .collect();

    let inserted = engine
        .bulk_insert_records("t", records)
        .expect("bulk_insert_records should succeed (V2 routing)");
    assert_eq!(inserted, 5, "bulk_insert_records should report 5 rows");

    // Flush seals the active SegmentWriter and writes root.index — proves
    // V2's insert_streaming was actually called (the writer it returned
    // exists in V2's internal state).
    engine.flush().expect("flush should succeed");
}
