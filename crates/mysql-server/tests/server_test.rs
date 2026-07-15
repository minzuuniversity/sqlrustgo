//! Additional mysql-server tests

use sqlrustgo_mysql_server::parse_tbl_line;

#[test]
fn test_parse_tbl_line_basic() {
    let result = parse_tbl_line("1|foo|3.14||", 4).unwrap();
    assert_eq!(result.len(), 4);
    assert_eq!(result[0], sqlrustgo_types::Value::Integer(1));
    assert_eq!(result[1], sqlrustgo_types::Value::Text("foo".to_string()));
    assert_eq!(result[2], sqlrustgo_types::Value::Float(3.14));
    assert_eq!(result[3], sqlrustgo_types::Value::Null);
}

#[test]
fn test_parse_tbl_line_all_null() {
    let result = parse_tbl_line("||||", 4).unwrap();
    assert_eq!(
        result,
        vec![
            sqlrustgo_types::Value::Null,
            sqlrustgo_types::Value::Null,
            sqlrustgo_types::Value::Null,
            sqlrustgo_types::Value::Null,
        ]
    );
}

#[test]
fn test_parse_tbl_line_integers() {
    let result = parse_tbl_line("0|1|2|3|4", 5).unwrap();
    assert_eq!(
        result,
        vec![
            sqlrustgo_types::Value::Integer(0),
            sqlrustgo_types::Value::Integer(1),
            sqlrustgo_types::Value::Integer(2),
            sqlrustgo_types::Value::Integer(3),
            sqlrustgo_types::Value::Integer(4),
        ]
    );
}

#[test]
fn test_parse_tbl_line_floats() {
    let result = parse_tbl_line("1.5|2.718|99.99||", 4).unwrap();
    assert_eq!(result[0], sqlrustgo_types::Value::Float(1.5));
    assert_eq!(result[1], sqlrustgo_types::Value::Float(2.718));
    assert_eq!(result[2], sqlrustgo_types::Value::Float(99.99));
    assert_eq!(result[3], sqlrustgo_types::Value::Null);
}

#[test]
fn test_parse_tbl_line_mixed() {
    let result = parse_tbl_line("42|text|3.14||", 4).unwrap();
    assert_eq!(result[0], sqlrustgo_types::Value::Integer(42));
    assert_eq!(result[1], sqlrustgo_types::Value::Text("text".to_string()));
    assert_eq!(result[2], sqlrustgo_types::Value::Float(3.14));
    assert_eq!(result[3], sqlrustgo_types::Value::Null);
}

#[test]
fn test_parse_tbl_line_too_few_fields() {
    let result = parse_tbl_line("1|2||", 5);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("3 fields"));
    assert!(err.contains("expected at least 5"));
}

#[test]
fn test_parse_tbl_line_trailing_whitespace() {
    let result = parse_tbl_line("1|foo|3.14||  \n", 4).unwrap();
    assert_eq!(result[0], sqlrustgo_types::Value::Integer(1));
}

#[test]
fn test_parse_tbl_line_carriage_return() {
    let result = parse_tbl_line("1|foo|3.14||\r\n", 4).unwrap();
    assert_eq!(result[0], sqlrustgo_types::Value::Integer(1));
}

#[test]
fn test_parse_tbl_line_empty_string_is_null() {
    // "||foo||" → split ['', '', 'foo', '', ''] → strip trailing '' → ['', '', 'foo', '']
    let result = parse_tbl_line("||foo||", 4).unwrap();
    assert_eq!(result.len(), 4);
    assert_eq!(result[0], sqlrustgo_types::Value::Null); // empty field
    assert_eq!(result[1], sqlrustgo_types::Value::Null); // empty field
    assert_eq!(result[2], sqlrustgo_types::Value::Text("foo".to_string()));
    assert_eq!(result[3], sqlrustgo_types::Value::Null); // trailing empty after strip
}

#[test]
fn test_parse_tbl_line_text_with_special_chars() {
    let result = parse_tbl_line("1|hello world|3.14||", 4).unwrap();
    assert_eq!(
        result[1],
        sqlrustgo_types::Value::Text("hello world".to_string())
    );
}

#[test]
fn test_parse_tbl_line_negative_numbers() {
    let result = parse_tbl_line("-1|-2.5||", 3).unwrap();
    assert_eq!(result[0], sqlrustgo_types::Value::Integer(-1));
    assert_eq!(result[1], sqlrustgo_types::Value::Float(-2.5));
}

#[test]
fn test_parse_tbl_line_exact_columns_no_trailing_pipe() {
    let result = parse_tbl_line("1|2|3", 3).unwrap();
    assert_eq!(result.len(), 3);
}

#[test]
fn test_parse_tbl_line_extra_fields_ignored() {
    let result = parse_tbl_line("1|2|3|4|5|extra", 3).unwrap();
    assert_eq!(result.len(), 3);
    assert_eq!(result[0], sqlrustgo_types::Value::Integer(1));
}
