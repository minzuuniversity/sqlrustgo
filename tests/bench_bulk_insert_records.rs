mod common;
use common::MySqlTestClient;
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};
use std::time::Instant;

#[test]
fn bench_bulk_insert_records_wire() {
    let mut client: MySqlTestClient = match start_ephemeral(EphemeralConfig::default())
        .ok()
        .and_then(|h| MySqlTestClient::connect_handle(h).ok())
    {
        Some(c) => c,
        None => panic!("start_ephemeral failed"),
    };
    let _ = client.exec("DROP TABLE IF EXISTS bulk");
    let r = client.exec("CREATE TABLE bulk (id INTEGER PRIMARY KEY, data TEXT)");
    assert!(r.is_ok());

    let n = 1000;
    let start = Instant::now();
    for i in 0..n {
        let _ = client.exec(&format!("INSERT INTO bulk VALUES ({i}, 'row_{i}')"));
    }
    let elapsed = start.elapsed();
    let rows = client
        .query_rows("SELECT COUNT(*) FROM bulk")
        .expect("count");
    assert!(!rows.is_empty());
    let _ = client.exec("DROP TABLE bulk");
    println!(
        "Bulk insert {n} rows in {elapsed:?} ({:.0} rows/s)",
        n as f64 / elapsed.as_secs_f64()
    );
}
