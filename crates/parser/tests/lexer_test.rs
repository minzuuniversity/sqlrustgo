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

// --- UTF-8 regression tests for #4489 ---
//
// `skip_whitespace` (line-comment skip) and `read_identifier` used to
// advance `self.position` by 1 byte per iteration. When the byte stream
// contained a multi-byte UTF-8 character (e.g. a CJK comment or
// identifier), the position was left in the middle of a character and
// the next `peek_char()` panic'd at
// `byte index N is not a char boundary; it is inside '中'`.

#[test]
fn test_tokenize_cjk_line_comment_no_panic() {
    // Regression for #4489: line comment containing CJK characters.
    let t = tok("-- 中文注释\nSELECT 1");
    assert!(
        t.iter().any(|t| matches!(t, Token::Select)),
        "SELECT must be tokenized after a CJK line comment, got: {t:?}"
    );
}

#[test]
fn test_tokenize_cjk_identifier_no_panic() {
    // Regression for #4489: CJK identifier characters in DDL.
    let t = tok("create table 学生(id int)");
    assert!(!t.is_empty(), "CJK identifier must not panic");
    // `学生` should appear as a single Identifier token (chars are kept
    // by `read_identifier` since `is_alphanumeric` is true for CJK).
    let has_cjk_ident = t.iter().any(|t| match t {
        Token::Identifier(s) => s.contains('学'),
        _ => false,
    });
    assert!(has_cjk_ident, "CJK identifier token not found in {t:?}");
}

#[test]
fn test_tokenize_mixed_ascii_cjk_no_panic() {
    // Regression for #4489: identifier with mixed ASCII + CJK + digits.
    let t = tok("select 姓名_2 from t");
    assert!(t.iter().any(|t| matches!(t, Token::Select)));
    assert!(t.iter().any(|t| matches!(t, Token::From)));
}

#[test]
fn test_tokenize_whitespace_cjk_no_panic() {
    // Regression for #4489: CJK whitespace-equivalent characters in
    // skip_whitespace's "non-whitespace => break" branch must not panic
    // when subsequently stepped over via `position += ch.len_utf8()`.
    let t = tok("SELECT 1; -- 中文");
    assert!(t.iter().any(|t| matches!(t, Token::Select)));
    assert!(t.iter().any(|t| matches!(t, Token::NumberLiteral(_))));
}
