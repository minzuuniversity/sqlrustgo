//! V312-12 coverage tests for `sqlrustgo_storage::bin_compactor`.

use sqlrustgo_storage::bin_compactor::BinCompactor;
use sqlrustgo_storage::bin_compactor::CompactorConfig;
use tempfile::TempDir;

#[test]
fn cov_bin_compactor_new() {
    let _ = BinCompactor::new(CompactorConfig {
        max_segment_count: 1,
    });
}

#[test]
fn cov_bin_compactor_new_max_2() {
    let _ = BinCompactor::new(CompactorConfig {
        max_segment_count: 2,
    });
}

#[test]
fn cov_bin_compactor_new_max_10() {
    let _ = BinCompactor::new(CompactorConfig {
        max_segment_count: 10,
    });
}
