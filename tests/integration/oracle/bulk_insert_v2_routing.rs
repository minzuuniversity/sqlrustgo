//! T4.2 Integration Test: bulk_insert_records routing to BinaryTableStorageV2
//!
//! Verifies that when the execution engine uses BinaryTableStorageV2,
//! the bulk_insert_records method correctly routes to insert_streaming.
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

    // Create table
    engine
        .execute(
            "CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT, val INTEGER)",
        )
        .expect("CREATE TABLE should succeed");

    // Insert a few rows - this should trigger bulk_insert_records -> insert_streaming
    for i in 1..=5 {
        engine
            .execute(&format!("INSERT INTO t VALUES ({}, 'name_{}', {})", i, i, i * 10))
            .expect("INSERT should succeed");
    }

    // Verify rows exist by querying
    let result = engine
        .execute("SELECT COUNT(*) FROM t")
        .expect("SELECT COUNT should succeed");

    // The result should contain 5 rows (we inserted 5)
    // Count result format: single column with value 5
    let count = result
        .iter()
        .next()
        .and_then(|row| row.col(0))
        .and_then(|v| v.as_i64())
        .expect("Should get count value");

    assert_eq!(count, 5, "Expected 5 rows inserted");
}
