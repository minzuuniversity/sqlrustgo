//! #5179 regression: a text-protocol row whose first value is the empty
//! string must not be decoded as a binary-protocol row.
//!
//! # The defect
//!
//! `parse_result_set_with_infile` decided a row's encoding by sniffing its
//! first byte: `0x00` meant "binary row" (the `COM_STMT_EXECUTE` header),
//! anything else meant "text row".
//!
//! That byte is ambiguous. In the text protocol every value is a
//! length-encoded string, and a zero-length string is encoded as the
//! single byte `0x00`. So a perfectly ordinary row — one whose first
//! column happens to be `''` — also starts with `0x00` and was routed into
//! `parse_binary_row`, which then read the remaining bytes as a NULL
//! bitmap and failed:
//!
//! ```text
//! Protocol error: Binary row: data too short for null bitmap
//! ```
//!
//! The whole query failed, not just the row. The B2 gate had been
//! carrying `mysqladmin_e2e_test` as a disabled binary with the note
//! "pre-existing failure / Investigate mysqladmin e2e flow" since v3.12;
//! this is that failure.
//!
//! `SELECT @@global_status` (what `WireAdmin::status` sends) is the
//! reachable trigger, because `resolve_system_variable` answers unknown
//! variables with an empty `Text` rather than NULL.
//!
//! # Why the fix is structural
//!
//! The caller always knows the protocol — it chose the command it just
//! sent. `parse_result_set_with_infile` now takes `binary_rows: bool` and
//! is told, instead of inferring from a byte that cannot carry the
//! distinction.
//!
//! These tests go over the real wire to a real server, so they cover the
//! server's row encoding too, not just the parser in isolation.

use sqlrustgo_mysql_client::{MySqlConnection, ResultSet};
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};

fn start() -> sqlrustgo_mysql_server::testing::EphemeralHandle {
    start_ephemeral(EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        bulk_insert_rows_per_flush: 10_000,
        load_infile_dir: None,
        server_threads: 8,
        storage: None,
        slow_query_log: None,
        metrics_port: None,
        wal_sync_mode_override: None,
    })
    .expect("ephemeral server starts")
}

fn connect(h: &sqlrustgo_mysql_server::testing::EphemeralHandle) -> MySqlConnection {
    let addr = format!("127.0.0.1:{}", h.port).parse().expect("addr");
    MySqlConnection::connect(&addr, "root", "", "test").expect("connect")
}

/// Run one query and return its single row, asserting it parsed at all.
fn one_row(sql: &str) -> Vec<String> {
    let h = start();
    let mut c = connect(&h);
    match c.execute(sql) {
        Ok(ResultSet::Select { rows, .. }) => {
            assert_eq!(rows.len(), 1, "{sql}: expected exactly 1 row, got {rows:?}");
            rows.into_iter().next().expect("row 0")
        }
        Ok(other) => panic!("{sql}: expected a result set, got {other:?}"),
        Err(e) => panic!("{sql}: query failed: {e}"),
    }
}

/// The literal case: a one-column result whose only value is `''`.
#[test]
fn empty_string_as_the_only_value_parses() {
    assert_eq!(
        one_row("SELECT ''"),
        vec![String::new()],
        "#5179: a text row starting with an empty string must decode as \
         text, not as a binary row (null-bitmap length error)"
    );
}

/// The case that made `mysqladmin_e2e_test` fail.
#[test]
fn unknown_system_variable_returning_empty_text_parses() {
    assert_eq!(
        one_row("SELECT @@global_status"),
        vec![String::new()],
        "#5179: resolve_system_variable answers unknown variables with \
         empty Text; that row must not be mistaken for a binary row"
    );
}

/// `@@sql_mode` is modelled as empty text, so it hits the same path as
/// `@@global_status` — the two differ only in that one is explicitly
/// modelled and the other falls through to the default arm.
#[test]
fn modelled_empty_text_variable_parses() {
    assert_eq!(one_row("SELECT @@sql_mode"), vec![String::new()]);
}

/// The empty value is not only legal in the first column; this pins that
/// the whole row is decoded, not short-circuited on column 0.
#[test]
fn empty_value_in_a_later_column_parses() {
    assert_eq!(
        one_row("SELECT 'a', ''"),
        vec!["a".to_string(), String::new()],
        "#5179: row must be decoded in full; an empty trailing value must \
         not truncate or corrupt the preceding columns"
    );
}

/// Guards the other side of the same change: a non-empty first value must
/// still decode, and must still come back as the empty-``Text`` fallback
/// rather than as NULL. Before the fix these passed by accident — a text
/// row's first byte happened to be a non-zero length.
#[test]
fn non_empty_first_value_still_parses() {
    assert_eq!(one_row("SELECT 'x'"), vec!["x".to_string()]);
}
