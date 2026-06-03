//! LOAD DATA LOCAL INFILE — wire-protocol integration tests.
//!
//! See docs/superpowers/specs/2026-06-04-load-data-local-infile-design.md.

mod common;

use common::MySqlTestClient;
use sqlrustgo_mysql_server::EphemeralConfig;
use std::io::Write;

#[test]
fn test_load_local_infile_basic() {
    // Will be implemented in Task 8. This stub ensures the test
    // binary compiles from this point on.
    let _ = EphemeralConfig::default();
    let _ = std::any::type_name::<MySqlTestClient>();
}
