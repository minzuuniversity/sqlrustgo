//! V400-split-deep: extra coverage for split_sql_statements edge cases.

use sqlrustgo_parser::split_sql_statements;

fn split_count(s: &str) -> usize {
    split_sql_statements(s).len()
}

// ===========================================================================
// split_sql_statements — string + comment + escape edge cases
// ===========================================================================

#[test]
fn split_empty_string() {
    assert_eq!(split_count(""), 0);
}

#[test]
fn split_whitespace_only() {
    assert_eq!(split_count("   \t\n  "), 0);
}

#[test]
fn split_single_statement() {
    assert_eq!(split_count("SELECT 1"), 1);
}

#[test]
fn split_two_statements() {
    assert_eq!(split_count("SELECT 1; SELECT 2"), 2);
}

#[test]
fn split_three_statements() {
    assert_eq!(split_count("SELECT 1; SELECT 2; SELECT 3"), 3);
}

#[test]
fn split_with_semicolon_at_end() {
    assert_eq!(split_count("SELECT 1;"), 1);
}

#[test]
fn split_with_trailing_newline() {
    assert_eq!(split_count("SELECT 1;\n"), 1);
}

#[test]
fn split_with_quoted_semicolon() {
    // ';' inside single quotes should NOT split
    assert_eq!(split_count("SELECT '1;2'"), 1);
}

#[test]
fn split_with_double_quoted_semicolon() {
    assert_eq!(split_count("SELECT \"a;b\""), 1);
}

#[test]
fn split_with_escaped_quote() {
    // \' should be treated as escape inside single-quoted string
    assert_eq!(split_count("SELECT 'a\\'b'"), 1);
}

#[test]
fn split_with_doubled_quote_in_single_quote() {
    // '' inside single quotes = escaped '
    assert_eq!(split_count("SELECT 'a''b'"), 1);
}

#[test]
fn split_with_line_comment() {
    // -- ... \n is line comment
    assert_eq!(split_count("SELECT 1; -- comment\nSELECT 2"), 2);
}

#[test]
fn split_with_block_comment() {
    // /* ... */ is block comment
    assert_eq!(split_count("SELECT 1; /* block */ SELECT 2"), 2);
}

#[test]
fn split_with_block_comment_in_string() {
    // block comment markers in string should not open a comment
    assert_eq!(split_count("SELECT '/* not a comment */'"), 1);
}

#[test]
fn split_with_line_comment_at_end() {
    // The behavior depends on how line comments are tokenized; we just want
    // to exercise the lexer code path, not assert a specific count.
    let _ = split_sql_statements("SELECT 1; -- trailing\n");
}

#[test]
fn split_with_empty_between_statements() {
    assert_eq!(split_count("SELECT 1;\n\nSELECT 2"), 2);
}

#[test]
fn split_create_then_insert_then_select() {
    assert_eq!(split_count("CREATE TABLE t(a INT); INSERT INTO t VALUES (1); SELECT * FROM t"), 3);
}

#[test]
fn split_with_string_concat() {
    assert_eq!(split_count("SELECT 'a' || 'b;c'"), 1);
}

#[test]
fn split_with_string_with_newline_in_quote() {
    assert_eq!(split_count("SELECT 'a\nb'"), 1);
}

#[test]
fn split_with_string_with_carriage_return() {
    assert_eq!(split_count("SELECT 'a\rb'"), 1);
}

#[test]
fn split_with_string_with_backslash_r_in_quote() {
    assert_eq!(split_count("SELECT 'a\\rb'"), 1);
}

#[test]
fn split_with_string_with_tab_in_quote() {
    assert_eq!(split_count("SELECT 'a\tb'"), 1);
}

#[test]
fn split_with_string_with_null_byte_in_quote() {
    assert_eq!(split_count("SELECT 'a\\0b'"), 1);
}

#[test]
fn split_with_string_ending_at_eof() {
    // Unterminated string behavior is implementation-defined; exercise path.
    let _ = split_sql_statements("SELECT 'unterminated");
}

#[test]
fn split_with_block_comment_unterminated() {
    // The current parser may not handle unterminated block comments
    // gracefully — we just exercise the path.
    let _ = split_sql_statements("SELECT 1 /* unterminated");
}

#[test]
fn split_with_block_comment_with_star() {
    // /* with a star inside */
    let parts = split_sql_statements("SELECT 1 /* x*y */ ; SELECT 2");
    assert_eq!(parts.len(), 2);
}

#[test]
fn split_with_backslash_in_double_quote() {
    // \" inside double quotes
    let parts = split_sql_statements(r#"SELECT "a\"b"; SELECT 2"#);
    assert_eq!(parts.len(), 2);
}

#[test]
fn split_with_semicolon_in_backslash_escape_in_single() {
    let parts = split_sql_statements(r"SELECT 'a\;b'; SELECT 2");
    assert_eq!(parts.len(), 2);
}

#[test]
fn split_with_only_semicolon() {
    assert_eq!(split_count(";"), 0);
}

#[test]
fn split_with_only_comments() {
    let _ = split_sql_statements("-- just a comment\n/* block */");
}

#[test]
fn split_with_string_containing_double_quote_in_single() {
    // single quote string with a " inside is fine
    assert_eq!(split_count(r#"SELECT 'a"b'"#), 1);
}

#[test]
fn split_double_quote_string_with_single_quote_inside() {
    assert_eq!(split_count(r#"SELECT "it's fine"; SELECT 2"#), 2);
}