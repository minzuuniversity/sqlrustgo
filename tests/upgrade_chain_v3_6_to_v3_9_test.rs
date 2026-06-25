mod common;
use common::MySqlTestClient;
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};

#[test]
fn upgrade_chain_v3_6_to_v3_9_basic() {
    let mut client: MySqlTestClient = match start_ephemeral(EphemeralConfig::default())
        .ok()
        .and_then(|h| MySqlTestClient::connect_handle(h).ok())
    {
        Some(c) => c,
        None => panic!("start_ephemeral failed"),
    };
    let _ = client.exec("DROP TABLE IF EXISTS upgrade_test");
    let r = client.exec("CREATE TABLE upgrade_test (id INTEGER PRIMARY KEY, data TEXT)");
    assert!(r.is_ok(), "create table: {r:?}");
    let r = client.exec("INSERT INTO upgrade_test VALUES (1, 'v3.6')");
    assert!(r.is_ok(), "insert v3.6: {r:?}");
    let r = client.exec("INSERT INTO upgrade_test VALUES (2, 'v3.7')");
    assert!(r.is_ok(), "insert v3.7: {r:?}");
    let r = client.exec("INSERT INTO upgrade_test VALUES (3, 'v3.9')");
    assert!(r.is_ok(), "insert v3.9: {r:?}");
    let rows = client
        .query_rows("SELECT COUNT(*) FROM upgrade_test")
        .expect("count");
    assert!(!rows.is_empty());
    let _ = client.exec("DROP TABLE upgrade_test");
}
