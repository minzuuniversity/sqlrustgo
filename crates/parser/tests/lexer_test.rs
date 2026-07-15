//! Lexer unit tests

use sqlrustgo_parser::lexer;
use sqlrustgo_parser::token::is_keyword;
use sqlrustgo_parser::token::Token;

fn tok(sql: &str) -> Vec<Token> {
    lexer::tokenize(sql)
}

#[test]
fn test_tokenize_select() {
    let t = tok("SELECT id FROM t");
    assert!(!t.is_empty());
    assert!(matches!(t[0], Token::Select));
}
#[test]
fn test_tokenize_integer() {
    assert!(!tok("42").is_empty());
}
#[test]
fn test_tokenize_string() {
    assert!(tok("'hello'")
        .iter()
        .any(|t| matches!(t, Token::StringLiteral(_))));
}
#[test]
fn test_tokenize_float() {
    assert!(!tok("3.14").is_empty());
}
#[test]
fn test_tokenize_operators() {
    assert!(!tok("+-*/=<>").is_empty());
}
#[test]
fn test_tokenize_punctuation() {
    assert!(!tok("();*").is_empty());
}
#[test]
fn test_tokenize_comments_line() {
    let t = tok("-- comment\nSELECT 1");
    assert!(t.iter().any(|t| matches!(t, Token::Select)));
}
#[test]
fn test_tokenize_comments_block() {
    assert!(!tok("/* comment */ SELECT 1").is_empty());
}
#[test]
fn test_tokenize_keywords() {
    assert!(!tok("SELECT INSERT UPDATE DELETE FROM WHERE").is_empty());
}
#[test]
fn test_tokenize_empty() {
    let t = tok("");
    assert!(t.is_empty() || t.len() == 1);
}
#[test]
fn test_tokenize_whitespace_only() {
    let t = tok("   \t\n  ");
    assert!(t.len() <= 1);
}
#[test]
fn test_is_keyword() {
    assert!(is_keyword("SELECT"));
    assert!(is_keyword("FROM"));
    assert!(!is_keyword("foo"));
}
#[test]
fn test_from_keyword() {
    use sqlrustgo_parser::token::from_keyword;
    assert!(from_keyword("SELECT").is_some());
    assert!(from_keyword("foo").is_none());
}
#[test]
fn test_tokenize_param() {
    assert!(!tok("?").is_empty());
}
#[test]
fn test_tokenize_backtick() {
    assert!(!tok("`t`").is_empty());
}
