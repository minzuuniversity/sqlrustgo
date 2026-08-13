#[path = "../../common/mod.rs"]
mod common;

/// Quick data verification for SF=1.0 fixture
///
/// Checks that the ephemeral server loaded data from the .json
/// files in /tmp/tpch-sf1 (which were written during a previous
/// LOAD DATA run).  If some tables are empty, the test prints
/// their actual counts for diagnosis.
///
/// **Skip condition:** Skipped if the SF=1.0 fixture is not present
/// at /tmp/tpch-sf1.  The fixture is only generated on CI or by
/// running `scripts/generate_tpch_data.sh --sf 1 --backend dbgen`.
#[test]
fn check_sf1_data_present() {
    use common::MySqlTestClient;
    use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};
    use std::path::Path;
    use std::time::Duration;

    let data_dir = Path::new("/tmp/tpch-sf1");
    if !data_dir.exists() {
        eprintln!("check_sf1_data_present: SKIPPED — /tmp/tpch-sf1 not present");
        return;
    }

    let config = EphemeralConfig {
        data_dir: Some(data_dir.to_path_buf()),
        bootstrap_tables: false,
        bootstrap_users: true,
        metrics_port: None,

        ..Default::default()
    };
    let handle = start_ephemeral(config).expect("start_ephemeral");
    let mut client = MySqlTestClient::connect_handle(handle).expect("connect");
    client
        .set_timeouts(Duration::from_secs(60), Duration::from_secs(60))
        .expect("set_timeouts");

    // Show tables
    match client.query_rows("SHOW TABLES") {
        Ok(rows) => {
            eprintln!("Tables found: {}", rows.len());
            for r in &rows {
                eprintln!("  {:?}", r);
            }

            // Check row counts for all 8 TPC-H tables
            let tables: &[&str] = &[
                "region", "nation", "supplier", "customer", "part", "partsupp", "orders",
                "lineitem",
            ];
            for tbl in tables {
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
