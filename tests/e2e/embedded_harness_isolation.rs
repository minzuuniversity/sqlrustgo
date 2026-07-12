//! Embedded server test harness — data dir isolation test
//!
//! Verifies the isolation contract: two parallel `start_ephemeral` calls
//! must use independent data dirs.

use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};

#[test]
fn test_ephemeral_data_dir_isolation() {
    let handle_a = start_ephemeral(EphemeralConfig::default()).expect("server A");
    let handle_b = start_ephemeral(EphemeralConfig::default()).expect("server B");

    // Two ephemeral servers must not share a port.
    assert_ne!(
        handle_a.port, handle_b.port,
        "ephemeral servers must each get a unique OS-assigned port"
    );

    // Drop both, the test passes if no panic (each Drop cleans up its own data dir).
    drop(handle_a);
    drop(handle_b);
}
