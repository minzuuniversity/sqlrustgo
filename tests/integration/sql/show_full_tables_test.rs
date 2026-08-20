//! PR-V312-59-A — 56A-R3 anti-deferral: `SHOW FULL TABLES`.
//!
//! **Issue**: #4384 (V312-59-A) — 56A-R3 (SHOW FULL TABLES /
//! SHOW TABLE STATUS) was deferred to v3.13+ in V312-56-VERIFICATION.md;
//! in-v3.12 implementation is required before RC/GA.
//!
//! **Fix**: parser `parse_show` + `ShowStatement::FullTables` +
//! `execute_show_full_tables` (Name / Type columns, LIKE + WHERE
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
fn show_full_tables_on_empty_db_returns_empty() {
    let mut client = clean_client();
    let rows = client
        .query_rows("SHOW FULL TABLES")
        .expect("SHOW FULL TABLES should succeed");
    assert_eq!(
        rows.len(),
        0,
        "SHOW FULL TABLES on empty DB should return 0 rows, got {}",
        rows.len()
    );
}

#[test]
fn show_full_tables_returns_name_and_type_columns() {
    let mut client = clean_client();
    client
        .exec("CREATE TABLE t1 (id INTEGER)")
        .expect("CREATE t1");
    client
        .exec("CREATE TABLE t2 (id INTEGER)")
        .expect("CREATE t2");

    let rows = client
        .query_rows("SHOW FULL TABLES")
        .expect("SHOW FULL TABLES should succeed");
    assert_eq!(rows.len(), 2, "expected 2 rows, got {:?}", rows);
    for row in &rows {
        assert_eq!(row.len(), 2, "FULL TABLES must return 2 columns (Name, Type), got {:?}", row);
        assert_eq!(row[1], "BASE TABLE", "all created tables are BASE TABLE, got {:?}", row);
    }
    let names: Vec<&str> = rows.iter().map(|r| r[0].as_str()).collect();
    assert!(names.contains(&"t1"));
    assert!(names.contains(&"t2"));
}

#[test]
fn show_full_tables_distinguishes_views() {
    let mut client = clean_client();
    client
        .exec("CREATE TABLE base_t (id INTEGER)")
        .expect("CREATE base_t");
    client
        .exec("CREATE VIEW v1 AS SELECT * FROM base_t")
        .expect("CREATE VIEW v1");

    let rows = client
        .query_rows("SHOW FULL TABLES")
        .expect("SHOW FULL TABLES should succeed");
    assert_eq!(rows.len(), 2, "expected 2 rows, got {:?}", rows);
    let mut base_row: Option<&Vec<String>> = None;
    let mut view_row: Option<&Vec<String>> = None;
    for row in &rows {
        match row[0].as_str() {
            "base_t" => base_row = Some(row),
            "v1" => view_row = Some(row),
            other => panic!("unexpected table name {:?}", other),
        }
    }
    let base = base_row.expect("base_t row missing");
    let view = view_row.expect("v1 row missing");
    assert_eq!(base[1], "BASE TABLE");
    assert_eq!(view[1], "VIEW");
}

#[test]
fn show_full_tables_from_db_works() {
    let mut client = clean_client();
    client
        .exec("CREATE TABLE t1 (id INTEGER)")
        .expect("CREATE t1");

    let rows = client
        .query_rows("SHOW FULL TABLES FROM public")
        .expect("SHOW FULL TABLES FROM public should succeed");
    assert_eq!(rows.len(), 1, "expected 1 row, got {:?}", rows);
    assert_eq!(rows[0][0], "t1");
}

#[test]
fn show_full_tables_like_filters() {
    let mut client = clean_client();
    client
        .exec("CREATE TABLE alpha (id INTEGER)")
        .expect("CREATE alpha");
    client
        .exec("CREATE TABLE beta (id INTEGER)")
        .expect("CREATE beta");

    let rows = client
        .query_rows("SHOW FULL TABLES LIKE 'a%'")
        .expect("SHOW FULL TABLES LIKE should succeed");
    assert_eq!(rows.len(), 1, "expected 1 row, got {:?}", rows);
    assert_eq!(rows[0][0], "alpha");
}

#[test]
fn show_full_tables_where_filters_on_table_type() {
    let mut client = clean_client();
    client
        .exec("CREATE TABLE base_t (id INTEGER)")
        .expect("CREATE base_t");
    client
        .exec("CREATE VIEW v1 AS SELECT * FROM base_t")
        .expect("CREATE VIEW v1");

    let rows = client
        .query_rows("SHOW FULL TABLES WHERE Table_type != 'VIEW'")
        .expect("SHOW FULL TABLES WHERE should succeed");
    assert_eq!(rows.len(), 1, "expected only base_t, got {:?}", rows);
    assert_eq!(rows[0][0], "base_t");
    assert_eq!(rows[0][1], "BASE TABLE");
}

#[test]
fn show_full_tables_after_drop_reflects_drop() {
    let mut client = clean_client();
    client
        .exec("CREATE TABLE keep_me (id INTEGER)")
        .expect("CREATE keep_me");
    client
        .exec("CREATE TABLE drop_me (id INTEGER)")
        .expect("CREATE drop_me");
    client
        .exec("DROP TABLE drop_me")
        .expect("DROP drop_me");

    let rows = client
        .query_rows("SHOW FULL TABLES")
        .expect("SHOW FULL TABLES should succeed");
    let names: Vec<&str> = rows.iter().map(|r| r[0].as_str()).collect();
    assert!(names.contains(&"keep_me"));
    assert!(
        !names.contains(&"drop_me"),
        "drop_me should not appear after DROP"
    );
}

#[test]
fn show_full_tables_lists_multiple_schemas_of_tables() {
    let mut client = clean_client();
    for n in 1..=4 {
        let ddl = format!("CREATE TABLE t{} (id INTEGER)", n);
        client.exec(&ddl).expect(&ddl);
    }

    let rows = client
        .query_rows("SHOW FULL TABLES")
        .expect("SHOW FULL TABLES should succeed");
    assert_eq!(rows.len(), 4, "expected 4 tables, got {:?}", rows);
    let names: Vec<&str> = rows.iter().map(|r| r[0].as_str()).collect();
    for n in 1..=4 {
        let want = format!("t{}", n);
        assert!(names.contains(&want.as_str()), "missing {}", want);
    }
}
