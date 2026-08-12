/// Quick query test against the existing WAL data in /tmp/tpch-sf1
#[path = "../../common/mod.rs"]
mod common;

#[test]
fn quick_query_sfid1() {
    use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};
    use std::path::Path;

    let data_dir = Path::new("/tmp/tpch-sf1");
    assert!(data_dir.exists());

    // Truncate WAL before starting — we want to recover from the existing WAL,
    // not create a new empty one.
    let wal_path = data_dir.join("sqlrustgo.wal");
    assert!(wal_path.exists(), "WAL must exist");

    let config = EphemeralConfig {
        data_dir: Some(data_dir.to_path_buf()),
        bootstrap_tables: false,
        bootstrap_users: true,        metrics_port: None,

        ..Default::default()
    };

    // Start the ephemeral server
    let handle = start_ephemeral(config).expect("start_ephemeral");

    // Connect via the test harness
    use common::MySqlTestClient;
    let mut client = MySqlTestClient::connect_handle(handle).expect("connect");

    // Check what tables exist
    match client.query_rows("SHOW TABLES") {
        Ok(rows) => {
            eprintln!("Tables: {}", rows.len());
            for r in &rows {
                eprintln!("  {:?}", r);
            }

            // Check row counts
            let tables = [
                "region", "nation", "supplier", "customer", "part", "partsupp", "orders",
                "lineitem",
            ];
            for tbl in &tables {
                let sql = format!("SELECT COUNT(*) AS cnt FROM {}", tbl);
                match client.query_rows(&sql) {
                    Ok(rows) => {
                        eprintln!("  {} : {} rows", tbl, rows[0][0]);
                    }
                    Err(e) => {
                        eprintln!("  {} : ERROR ({})", tbl, e);
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("SHOW TABLES error: {}", e);
        }
    }
}
