//! V312-56A / 56A-R4 — SHOW WARNINGS / ERRORS / STATUS / VARIABLES integration tests.
//!
//! Issue #4251 acceptance criteria for V312-56A residual sub-task 56A-R4:
//! each statement must parse + execute + return a controlled-subset result
//! without panicking or returning a parse error.
//!
//! Controlled-subset v3.12 returns:
//! - SHOW WARNINGS / ERRORS: empty result (no session accumulator yet)
//! - SHOW STATUS: hard-coded catalog (Uptime, Threads, Questions, Slow_queries)
//! - SHOW VARIABLES: hard-coded catalog (version, sql_mode, autocommit, charset)
//!
//! Tests run via the ephemeral MySQL wire harness to cover the full
//! parser → AST → executor → wire response path.

#[path = "../../common/mod.rs"]
mod common;

use common::MySqlTestClient;
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};

fn client() -> MySqlTestClient {
    let cfg = EphemeralConfig {
        bootstrap_tables: false,
        ..Default::default()
    };
    let handle = start_ephemeral(cfg).expect("start_ephemeral");
    MySqlTestClient::connect_handle(handle).expect("connect_handle")
}

#[test]
fn show_warnings_parses_and_executes_empty() {
    let mut c = client();
    let rows = c.query_rows("SHOW WARNINGS").expect("SHOW WARNINGS");
    assert_eq!(rows.len(), 0, "WARNINGS should be empty in v3.12");
}

#[test]
fn show_errors_parses_and_executes_empty() {
    let mut c = client();
    let rows = c.query_rows("SHOW ERRORS").expect("SHOW ERRORS");
    assert_eq!(rows.len(), 0, "ERRORS should be empty in v3.12");
}

#[test]
fn show_status_returns_hardcoded_catalog() {
    let mut c = client();
    let rows = c
        .query_rows("SHOW STATUS")
        .expect("SHOW STATUS");
    assert!(
        rows.len() >= 4,
        "STATUS should return >=4 hard-coded rows, got {}",
        rows.len()
    );
    let names: Vec<&str> = rows.iter().map(|r| r[0].as_str()).collect();
    assert!(names.contains(&"Uptime"), "STATUS should include Uptime");
    assert!(names.contains(&"Threads"), "STATUS should include Threads");
    assert!(names.contains(&"Questions"), "STATUS should include Questions");
    assert!(names.contains(&"Slow_queries"), "STATUS should include Slow_queries");
}

#[test]
fn show_variables_returns_hardcoded_catalog() {
    let mut c = client();
    let rows = c
        .query_rows("SHOW VARIABLES")
        .expect("SHOW VARIABLES");
    assert!(
        rows.len() >= 4,
        "VARIABLES should return >=4 hard-coded rows, got {}",
        rows.len()
    );
    let names: Vec<&str> = rows.iter().map(|r| r[0].as_str()).collect();
    assert!(names.contains(&"version"), "VARIABLES should include version");
    assert!(names.contains(&"sql_mode"), "VARIABLES should include sql_mode");
    assert!(names.contains(&"autocommit"), "VARIABLES should include autocommit");
    assert!(
        names.contains(&"character_set_server"),
        "VARIABLES should include character_set_server"
    );
}

#[test]
fn show_warnings_lowercase_parses() {
    let mut c = client();
    // Case-insensitive parse: SHOW warnings (mixed case).
    let rows = c
        .query_rows("SHOW warnings")
        .expect("SHOW warnings (lowercase)");
    assert_eq!(rows.len(), 0);
}

#[test]
fn show_status_uppercase_parses() {
    let mut c = client();
    let rows = c
        .query_rows("SHOW STATUS")
        .expect("SHOW STATUS (uppercase)");
    assert!(rows.len() >= 4);
}