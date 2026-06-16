mod common;
use common::tpch_wire_harness::start_sf01;
use std::time::{Duration, Instant};

#[test]
fn test_sustained_write_load() {
    let mut client = start_sf01();
    let _ = client.set_timeouts(Duration::from_secs(60), Duration::from_secs(60));
    let _ = client.query_rows("DROP TABLE IF EXISTS stability_w");
    client
        .query_rows("CREATE TABLE stability_w (id INTEGER PRIMARY KEY, value TEXT)")
        .expect("create");

    let n = 20;
    let start = Instant::now();
    for i in 0..n {
        let sql = format!("INSERT INTO stability_w VALUES ({i}, 'value_{i}')");
        let _ = client.query_rows(&sql);
    }
    let elapsed = start.elapsed();
    println!("{n} inserts in {elapsed:?} ({:.1} ops/s)", n as f64 / elapsed.as_secs_f64());
    let count = client
        .query_rows("SELECT COUNT(*) FROM stability_w")
        .expect("count");
    let row_count: i64 = count
        .first()
        .and_then(|r| r.first())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    assert!(row_count > 0, "no rows committed");
    let _ = client.query_rows("DROP TABLE stability_w");
}

#[test]
fn test_sustained_read_load() {
    let mut client = start_sf01();
    let _ = client.set_timeouts(Duration::from_secs(60), Duration::from_secs(60));
    let _ = client.query_rows("DROP TABLE IF EXISTS stability_r");
    client
        .query_rows("CREATE TABLE stability_r (id INTEGER PRIMARY KEY, value TEXT)")
        .expect("create");
    for i in 0..10 {
        let sql = format!("INSERT INTO stability_r VALUES ({i}, 'v_{i}')");
        let _ = client.query_rows(&sql);
    }
    let n = 20;
    let start = Instant::now();
    for _ in 0..n {
        let _ = client.query_rows("SELECT * FROM stability_r");
    }
    let elapsed = start.elapsed();
    println!("{n} scans in {elapsed:?}");
    let _ = client.query_rows("DROP TABLE stability_r");
}
