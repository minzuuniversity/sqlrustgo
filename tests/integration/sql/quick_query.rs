/// Quick query test against the existing WAL data in /tmp/tpch-sf1
#[path = "../../common/mod.rs"]
mod common;

#[test]
fn quick_query_sfid1() {
    use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};
    use std::path::Path;

    let data_dir = Path::new("/tmp/tpch-sf1");

    // The pre-built WAL directory is **not** repository content and no CI
    // step produces it: it only exists on a machine where somebody ran the
    // TPC-H SF=1 workload once and left the WAL behind. Asserting on it
    // made this target fail on every clean environment (CI run 8117:
    // "assertion failed: data_dir.exists()").
    //
    // The honest state of such a check here is **not run** — counted
    // separately from the passed tests and disclosed in the CI summary,
    // never counted as passed. See issue #5428 for the real fix (the test
    // has to build its own precondition).
    let missing = if !data_dir.exists() {
        Some(format!(
            "the WAL directory {} does not exist",
            data_dir.display()
        ))
    } else if !data_dir.join("sqlrustgo.wal").exists() {
        Some(format!(
            "the WAL file {} does not exist",
            data_dir.join("sqlrustgo.wal").display()
        ))
    } else {
        None
    };
    if let Some(detail) = missing {
        common::tpch_cli_harness::tpch_fixture::report_not_run(
            "quick_query::quick_query_sfid1",
            common::tpch_cli_harness::tpch_fixture::reason::PREBUILT_DATA_UNAVAILABLE,
            &detail,
        );
        return;
    }

    let config = EphemeralConfig {
        data_dir: Some(data_dir.to_path_buf()),
        bootstrap_tables: false,
        bootstrap_users: true,
        metrics_port: None,

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
