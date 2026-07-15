// Additional parser coverage tests for split_sql_statements and Statement types

use sqlrustgo_parser::parse_statements;
use sqlrustgo_parser::split_sql_statements;

// ============ split_sql_statements edge cases ============

#[test]
fn test_split_empty_input() {
    assert!(split_sql_statements("").is_empty());
}

#[test]
fn test_split_whitespace_only() {
    assert!(split_sql_statements("  ").is_empty());
    assert!(split_sql_statements("\t").is_empty());
    assert!(split_sql_statements("  \n\t  ").is_empty());
}

#[test]
fn test_split_single_statement_no_semi() {
    assert_eq!(split_sql_statements("SELECT 1"), vec!["SELECT 1"]);
}

#[test]
fn test_split_two_statements() {
    assert_eq!(split_sql_statements("SELECT 1; SELECT 2"), vec!["SELECT 1", "SELECT 2"]);
}

#[test]
fn test_split_trailing_semicolon() {
    assert_eq!(split_sql_statements("SELECT 1; SELECT 2;"), vec!["SELECT 1", "SELECT 2"]);
}

#[test]
fn test_split_semi_inside_parens_preserved() {
    let frags = split_sql_statements("INSERT INTO t VALUES (1, ';x'); SELECT 1");
    assert_eq!(frags.len(), 2);
    assert_eq!(frags[0], "INSERT INTO t VALUES (1, ';x')");
    assert_eq!(frags[1], "SELECT 1");
}

#[test]
fn test_split_semi_inside_string_literal_preserved() {
    let frags = split_sql_statements("SELECT 'a;b'; SELECT 1");
    assert_eq!(frags.len(), 2);
    assert_eq!(frags[0], "SELECT 'a;b'");
    assert_eq!(frags[1], "SELECT 1");
}

#[test]
fn test_split_escaped_single_quote() {
    let frags = split_sql_statements("SELECT 'it''s ok'; SELECT 1");
    assert_eq!(frags.len(), 2);
}

#[test]
fn test_split_line_comment_not_stripped() {
    let frags = split_sql_statements("-- comment\nSELECT 1");
    assert_eq!(frags.len(), 1);
}

#[test]
fn test_split_with_newlines() {
    let frags = split_sql_statements("SELECT 1\n;SELECT 2\n\n;SELECT 3");
    assert_eq!(frags.len(), 3);
    assert_eq!(frags[0], "SELECT 1");
    assert_eq!(frags[1], "SELECT 2");
    assert_eq!(frags[2], "SELECT 3");
}

#[test]
fn test_split_trims_result() {
    let frags = split_sql_statements("  SELECT 1  ;  SELECT 2  ;  ");
    assert_eq!(frags.len(), 2);
    assert_eq!(frags[0], "SELECT 1");
    assert_eq!(frags[1], "SELECT 2");
}

#[test]
fn test_split_multiple_semicolons() {
    assert_eq!(split_sql_statements("SELECT 1;;SELECT 2"), vec!["SELECT 1", "SELECT 2"]);
}

#[test]
fn test_split_many_statements() {
    let sql = "SELECT 1; SELECT 2; SELECT 3; SELECT 4; SELECT 5";
    let frags = split_sql_statements(sql);
    assert_eq!(frags.len(), 5);
}

#[test]
fn test_split_nested_parens() {
    let frags = split_sql_statements("SELECT COALESCE('a;', 'b'); SELECT 1");
    assert_eq!(frags.len(), 2);
    assert_eq!(frags[0], "SELECT COALESCE('a;', 'b')");
}

fn test_split_multiple_string_literals() {
    let frags = split_sql_statements("SELECT 'a;b' FROM t WHERE x = 'c;d'; SELECT 1");
    assert_eq!(frags.len(), 2);
    assert_eq!(frags[0], "SELECT 'a;b' FROM t WHERE x = 'c;d'");
}

#[test]
fn test_split_single_quoted_escape() {
    let frags = split_sql_statements("SELECT 'it''s''fine'; SELECT 1");
    assert_eq!(frags.len(), 2);
    assert_eq!(frags[0], "SELECT 'it''s''fine'");
}

#[test]
fn test_split_double_quoted_identifier() {
    let frags = split_sql_statements("SELECT \"col;name\" FROM \"t;able\"; SELECT 1");
    assert_eq!(frags.len(), 2);
}

#[test]
fn test_split_paren_with_semi_inside() {
    let frags = split_sql_statements("INSERT INTO t VALUES (1, 2); INSERT INTO t VALUES (3, 4);");
    assert_eq!(frags.len(), 2);
}

// ============ parse_statements tests ============

#[test]
fn test_parse_statements_single() {
    let result = parse_statements("SELECT 1");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 1);
}

#[test]
fn test_parse_statements_mixed_dml() {
    let result = parse_statements("CREATE TABLE t (id INT); INSERT INTO t VALUES (1); SELECT * FROM t");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 3);
}

// ============ Statement Debug and Clone ============

#[test]
fn test_statement_clone_equality() {
    let select_stmt = sqlrustgo_parser::parse("SELECT 1").unwrap();
    let cloned = select_stmt.clone();
    assert_eq!(select_stmt, cloned);
}

#[test]
fn test_statement_debug_format() {
    let result = sqlrustgo_parser::parse("SELECT 1");
    assert!(result.is_ok());
    let debug = format!("{:?}", result.unwrap());
    assert!(!debug.is_empty());
}

#[test]
fn test_transaction_statement_debug() {
    let result = sqlrustgo_parser::parse("BEGIN");
    assert!(result.is_ok());
    let debug = format!("{:?}", result.unwrap());
    assert!(debug.contains("Transaction"));
}

#[test]
fn test_show_statement_debug() {
    let result = sqlrustgo_parser::parse("SHOW TABLES");
    assert!(result.is_ok());
    let debug = format!("{:?}", result.unwrap());
    assert!(debug.contains("Show"));
}

// ============ Supported Statement type coverage ============

#[test]
fn test_parse_create_database() {
    assert!(sqlrustgo_parser::parse("CREATE DATABASE mydb").is_ok());
}

#[test]
fn test_parse_drop_database() {
    assert!(sqlrustgo_parser::parse("DROP DATABASE mydb").is_ok());
}

#[test]
fn test_parse_truncate() {
    assert!(sqlrustgo_parser::parse("TRUNCATE TABLE t").is_ok());
}

#[test]
fn test_parse_use_database() {
    assert!(sqlrustgo_parser::parse("USE mydb").is_ok());
}

#[test]
fn test_parse_describe() {
    assert!(sqlrustgo_parser::parse("DESCRIBE t").is_ok());
}

#[test]
fn test_parse_desc_alias() {
    assert!(sqlrustgo_parser::parse("DESC t").is_ok());
}

#[test]
fn test_parse_grant_role() {
    assert!(sqlrustgo_parser::parse("GRANT admin TO user1").is_ok());
}

#[test]
fn test_parse_revoke_role() {
    assert!(sqlrustgo_parser::parse("REVOKE admin FROM user1").is_ok());
}

#[test]
fn test_parse_show_grants() {
    assert!(sqlrustgo_parser::parse("SHOW GRANTS FOR user1").is_ok());
}

#[test]
fn test_parse_show_tables() {
    assert!(sqlrustgo_parser::parse("SHOW TABLES").is_ok());
}

#[test]
fn test_parse_begin() {
    assert!(sqlrustgo_parser::parse("BEGIN").is_ok());
}

#[test]
fn test_parse_commit() {
    assert!(sqlrustgo_parser::parse("COMMIT").is_ok());
}

#[test]
fn test_parse_rollback() {
    assert!(sqlrustgo_parser::parse("ROLLBACK").is_ok());
}

#[test]
fn test_parse_show_columns() {
    assert!(sqlrustgo_parser::parse("SHOW COLUMNS FROM t").is_ok());
}
