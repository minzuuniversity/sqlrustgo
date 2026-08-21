//! PR-V312-59-A — 56A-R3 anti-deferral: `SHOW TABLE STATUS`.
//!
//! **Issue**: #4384 (V312-59-A) — 56A-R3 (SHOW FULL TABLES /
//! SHOW TABLE STATUS) was deferred to v3.13+ in V312-56-VERIFICATION.md;
//! in-v3.12 implementation is required before RC/GA.
//!
//! **Fix**: parser `parse_show` + `ShowStatement::TableStatus` +
//! `execute_show_table_status` (MySQL 18-column rows; LIKE + WHERE
//! filters evaluated against generated rows).

#[path = "../../common/mod.rs"]
mod common;

use common::MySqlTestClient;
use sqlrustgo_mysql_server::testing::EphemeralConfig;

fn clean_client() -> MySqlTestClient {
    MySqlTestClient::connect_with_config(EphemeralConfig {
        bootstrap_tables: false,
        slow_query_log: None,
        metrics_port: None,
        ..EphemeralConfig::default()
    })
    .expect("ephemeral server (clean catalog) + raw client should come up")
}

#[test]
fn show_table_status_on_empty_db_returns_empty() {
    let mut client = clean_client();
    let rows = client
        .query_rows("SHOW TABLE STATUS")
        .expect("SHOW TABLE STATUS should succeed");
    assert_eq!(
        rows.len(),
        0,
        "SHOW TABLE STATUS on empty DB should return 0 rows, got {}",
        rows.len()
    );
}

#[test]
fn show_table_status_returns_18_columns() {
    let mut client = clean_client();
    client
        .exec("CREATE TABLE t1 (id INTEGER)")
        .expect("CREATE t1");

    let rows = client
        .query_rows("SHOW TABLE STATUS")
        .expect("SHOW TABLE STATUS should succeed");
    assert_eq!(rows.len(), 1, "expected 1 row, got {:?}", rows);
    assert_eq!(
        rows[0].len(),
        18,
        "SHOW TABLE STATUS must return 18 MySQL columns, got {}: {:?}",
        rows[0].len(),
        rows[0]
    );
}

#[test]
fn show_table_status_name_and_engine_fields() {
    let mut client = clean_client();
    client
        .exec("CREATE TABLE alpha (id INTEGER)")
        .expect("CREATE alpha");

    let rows = client
        .query_rows("SHOW TABLE STATUS")
        .expect("SHOW TABLE STATUS should succeed");
    assert_eq!(rows[0][0], "alpha", "Name column (col 0)");
    assert_eq!(rows[0][1], "InnoDB", "Engine column (col 1)");
    assert_eq!(rows[0][2], "10", "Version column (col 2)");
    assert_eq!(rows[0][3], "Dynamic", "Row_format column (col 3)");
}

#[test]
fn show_table_status_rows_field_is_real_count() {
    let mut client = clean_client();
    client
        .exec("CREATE TABLE t1 (id INTEGER)")
        .expect("CREATE t1");
    client.exec("INSERT INTO t1 VALUES (1)").expect("INSERT 1");
    client.exec("INSERT INTO t1 VALUES (2)").expect("INSERT 2");

    let rows = client
        .query_rows("SHOW TABLE STATUS")
        .expect("SHOW TABLE STATUS should succeed");
    assert_eq!(
        rows[0][4], "2",
        "Rows column (col 4) should be the real row count"
    );
}

#[test]
fn show_table_status_collation_and_numeric_fields() {
    let mut client = clean_client();
    client
        .exec("CREATE TABLE t1 (id INTEGER, name TEXT)")
        .expect("CREATE t1");

    let rows = client
        .query_rows("SHOW TABLE STATUS")
        .expect("SHOW TABLE STATUS should succeed");
    assert_eq!(rows[0][5], "0", "Avg_row_length (col 5) is a numeric field");
    assert_eq!(rows[0][14], "utf8mb4_general_ci", "Collation (col 14)");
    assert_eq!(rows[0][17], "", "Comment (col 17) defaults to empty");
}

#[test]
fn show_table_status_lists_all_tables() {
    let mut client = clean_client();
    for n in 1..=3 {
        let ddl = format!("CREATE TABLE t{} (id INTEGER)", n);
        client.exec(&ddl).expect(&ddl);
    }

    let rows = client
        .query_rows("SHOW TABLE STATUS")
        .expect("SHOW TABLE STATUS should succeed");
    assert_eq!(rows.len(), 3, "expected 3 rows, got {:?}", rows);
    let names: Vec<&str> = rows.iter().map(|r| r[0].as_str()).collect();
    for n in 1..=3 {
        let want = format!("t{}", n);
        assert!(names.contains(&want.as_str()), "missing {}", want);
    }
}

#[test]
fn show_table_status_like_filters() {
    let mut client = clean_client();
    client
        .exec("CREATE TABLE alpha (id INTEGER)")
        .expect("CREATE alpha");
    client
        .exec("CREATE TABLE beta (id INTEGER)")
        .expect("CREATE beta");

    let rows = client
        .query_rows("SHOW TABLE STATUS LIKE 'a%'")
        .expect("SHOW TABLE STATUS LIKE should succeed");
    assert_eq!(rows.len(), 1, "expected 1 row, got {:?}", rows);
    assert_eq!(rows[0][0], "alpha");
}
