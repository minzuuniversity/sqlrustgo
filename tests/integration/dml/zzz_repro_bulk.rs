#[path = "../../common/mod.rs"]
mod common;
use common::MySqlTestClient;
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};

#[test]
fn repro_drop_only() {
    let mut client: MySqlTestClient = match start_ephemeral(EphemeralConfig::default())
        .ok()
        .and_then(|h| MySqlTestClient::connect_handle(h).ok())
    {
        Some(c) => c,
        None => panic!("start_ephemeral failed"),
    };
    eprintln!("STEP: create");
    let _ = client.exec("CREATE TABLE bulk (id INTEGER PRIMARY KEY, data TEXT)");
    eprintln!("STEP: drop");
    let _ = client.exec("DROP TABLE bulk");
    eprintln!("STEP: done");
}

#[test]
fn repro_drop_if_exists_only() {
    let mut client: MySqlTestClient = match start_ephemeral(EphemeralConfig::default())
        .ok()
        .and_then(|h| MySqlTestClient::connect_handle(h).ok())
    {
        Some(c) => c,
        None => panic!("start_ephemeral failed"),
    };
    eprintln!("STEP: drop-if-exists (no table)");
    let _ = client.exec("DROP TABLE IF EXISTS bulk");
    eprintln!("STEP: done");
}
