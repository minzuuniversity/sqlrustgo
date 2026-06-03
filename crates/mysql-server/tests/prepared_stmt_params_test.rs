//! Tests for the COM_STMT_EXECUTE binary-protocol parameter parser.
//!
//! Regression tests for Issue #2813: `replace_placeholders` was previously
//! called with an empty `Vec<Vec<u8>>` so `?` placeholders in prepared
//! statements were never substituted. These tests exercise the
//! `parse_stmt_execute_params` helper directly, plus a higher-level
//! `replace_placeholders` round-trip with realistic MySQL wire payloads.

use sqlrustgo_mysql_server::test_helpers::parse_stmt_execute_params;
use sqlrustgo_mysql_server::test_helpers::replace_placeholders;
use sqlrustgo_mysql_server::test_helpers::StmtParam;

/// Build a binary-protocol COM_STMT_EXECUTE payload for use in tests.
///
/// Layout (MySQL COM_STMT_EXECUTE binary protocol):
///   0..4   stmt_id (u32 LE)
///   4      flags
///   5..9   iteration_count (u32 LE, 0x01)
///   9..    null-bitmap ((param_count + 7) / 8 bytes)
///   next   new_params_bound_flag
///   next   type codes (2 bytes each, if new_params_bound=1)
///   next   values (omitted for NULL params)
///
/// A param is considered NULL iff its entry in the `params` slice is
/// `(0x06, &[])` (i.e. the explicit NULL type code).
fn stmt_payload(
    stmt_id: u32,
    params: &[(u8, &[u8])],
    new_params_bound: bool,
) -> Vec<u8> {
    let mut null_bitmap: u8 = 0;
    for (i, (tc, _)) in params.iter().enumerate() {
        if *tc == 0x06 {
            // MYSQL_TYPE_NULL
            null_bitmap |= 1 << (i % 8);
        }
    }

    let mut p = Vec::new();
    p.extend_from_slice(&stmt_id.to_le_bytes());
    p.push(0x00); // flags
    p.extend_from_slice(&1u32.to_le_bytes()); // iteration_count
    let null_bytes = (params.len() + 7) / 8;
    p.extend(std::iter::repeat(0u8).take(null_bytes));
    if null_bytes >= 1 {
        let last = p.len() - null_bytes;
        p[last] = null_bitmap;
    }
    p.push(if new_params_bound { 0x01 } else { 0x00 });
    if new_params_bound {
        for (tc, _) in params {
            p.push(*tc);
            p.push(0x00); // signed/unsigned flag
        }
    }
    for (tc, v) in params {
        if *tc == 0x06 {
            // NULL: no value bytes.
            continue;
        }
        match *tc {
            0x01 => p.push(v[0]),                 // MYSQL_TYPE_TINY
            0x02 => p.extend_from_slice(&v[..2]), // MYSQL_TYPE_SHORT
            0x03 => p.extend_from_slice(&v[..4]), // MYSQL_TYPE_LONG
            0x08 => p.extend_from_slice(&v[..8]), // MYSQL_TYPE_LONGLONG
            0x0f | 0xfd | 0xfe | 0xfc => {
                // Length-encoded string/blob.
                p.push(v.len() as u8);
                p.extend_from_slice(v);
            }
            other => panic!("unsupported test type code 0x{:02x}", other),
        }
    }
    p
}

#[test]
fn parses_no_params_correctly() {
    let p = vec![0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00];
    let out = parse_stmt_execute_params(&p, 0);
    assert_eq!(out, Vec::<StmtParam>::new());
}

#[test]
fn parses_single_long_param() {
    let p = stmt_payload(1, &[(0x03, &12345i32.to_le_bytes())], true);
    let out = parse_stmt_execute_params(&p, 1);
    assert_eq!(out, vec![(b"12345".to_vec(), true)]);
}

#[test]
fn parses_single_string_param() {
    let p = stmt_payload(1, &[(0xfd, b"hello")], true);
    let out = parse_stmt_execute_params(&p, 1);
    assert_eq!(out, vec![(b"hello".to_vec(), false)]);
}

#[test]
fn parses_three_mixed_params() {
    let p = stmt_payload(
        1,
        &[
            (0x03, &42i32.to_le_bytes()),
            (0xfd, b"hi"),
            (0x08, &9_000_000_000i64.to_le_bytes()),
        ],
        true,
    );
    let out = parse_stmt_execute_params(&p, 3);
    assert_eq!(out.len(), 3);
    assert_eq!(out[0], (b"42".to_vec(), true));
    assert_eq!(out[1], (b"hi".to_vec(), false));
    assert_eq!(out[2], (b"9000000000".to_vec(), true));
}

#[test]
fn parses_null_via_bitmap() {
    let p = stmt_payload(
        1,
        &[
            (0x03, &1i32.to_le_bytes()),
            (0x06, &[]), // NULL
            (0x03, &3i32.to_le_bytes()),
        ],
        true,
    );
    let out = parse_stmt_execute_params(&p, 3);
    assert_eq!(out.len(), 3);
    assert_eq!(out[0], (b"1".to_vec(), true));
    assert_eq!(out[1], (b"".to_vec(), false)); // empty = NULL
    assert_eq!(out[2], (b"3".to_vec(), true));
}

#[test]
fn replace_placeholders_with_parsed_params_round_trips() {
    let sql = "SELECT * FROM users WHERE id = ? AND name = ?";
    let p = stmt_payload(1, &[(0x08, &42i64.to_le_bytes()), (0xfd, b"alice")], true);
    let params = parse_stmt_execute_params(&p, 2);
    let final_sql = replace_placeholders(sql, &params);
    assert_eq!(
        final_sql,
        "SELECT * FROM users WHERE id = 42 AND name = 'alice'"
    );
}

#[test]
fn replace_placeholders_with_null_inserts_null_keyword() {
    let sql = "INSERT INTO t VALUES (?, ?, ?)";
    let p = stmt_payload(
        1,
        &[
            (0x03, &1i32.to_le_bytes()),
            (0x06, &[]), // NULL
            (0x03, &3i32.to_le_bytes()),
        ],
        true,
    );
    let params = parse_stmt_execute_params(&p, 3);
    let final_sql = replace_placeholders(sql, &params);
    assert_eq!(final_sql, "INSERT INTO t VALUES (1, NULL, 3)");
}

#[test]
fn string_with_quote_is_escaped() {
    let sql = "SELECT 'x' WHERE name = ?";
    let p = stmt_payload(1, &[(0xfd, b"O'Brien")], true);
    let params = parse_stmt_execute_params(&p, 1);
    let final_sql = replace_placeholders(sql, &params);
    assert_eq!(final_sql, "SELECT 'x' WHERE name = 'O''Brien'");
}
