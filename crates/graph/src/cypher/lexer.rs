//! Cypher lexer.
//!
//! Hand-written scanner that turns a Cypher source string into a stream of
//! [`Token`]s. Designed for clarity over performance — this is the foundation
//! of the parser, not a hot path.

use crate::types::{GraphError, GraphResult};

/// A lexed token. `Start` marks the (virtual) position before the first real
/// token so the parser can use it for "lookahead-1" decisions.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// What kind of token this is.
    pub kind: TokenKind,
    /// 1-indexed line in the source.
    pub line: usize,
    /// 1-indexed column in the source.
    pub col: usize,
}

/// Token kinds.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Match,
    Optional,
    Where,
    Return,
    With,
    Union,
    All,
    As,
    And,
    Or,
    Not,
    Is,
    Null,
    True,
    False,
    Count,
    Distinct,
    Order,
    By,
    Skip,
    Limit,
    Dash,
    // Punctuation
    LParen,
    RParen,
    LBrack,
    RBrack,
    LBrace,
    RBrace,
    Colon,
    Comma,
    Dot,
    Pipe,
    DashGt, // ->
    LtDash, // <-
    Eq,     // =
    Ne,     // <>
    Lt,
    Le,
    Gt,
    Ge,
    Star,
    // Literals / identifiers
    Ident(String),
    String(String),
    Integer(i64),
    Float(f64),
}

/// End-of-input sentinel.
pub const EOF: TokenKind = TokenKind::Ident(String::new());

/// Lex `source` into a `Vec<Token>`, terminating with an EOF marker token.
///
/// Returns `GraphError::CypherParse { line, col, message }` on the first
/// unrecognised character.
pub fn lex(source: &str) -> GraphResult<Vec<Token>> {
    let mut tokens = Vec::new();
    let bytes = source.as_bytes();
    let mut i = 0usize;
    let mut line = 1usize;
    let mut col = 1usize;

    while i < bytes.len() {
        let c = bytes[i] as char;

        // Skip whitespace.
        if c.is_whitespace() {
            if c == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
            i += 1;
            continue;
        }

        // Skip line comments: // ...
        if c == '/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }

        // Skip block comments: /* ... */
        if c == '/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            i += 2;
            col += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                if bytes[i] == b'\n' {
                    line += 1;
                    col = 1;
                } else {
                    col += 1;
                }
                i += 1;
            }
            if i + 1 >= bytes.len() {
                return Err(GraphError::CypherParse {
                    line,
                    col,
                    message: "unterminated block comment".into(),
                });
            }
            i += 2;
            col += 2;
            continue;
        }

        let start_line = line;
        let start_col = col;

        // Identifiers and keywords.
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
                col += 1;
            }
            let word = std::str::from_utf8(&bytes[start..i]).unwrap().to_string();
            let kind = keyword_or_ident(&word);
            tokens.push(Token {
                kind,
                line: start_line,
                col: start_col,
            });
            continue;
        }

        // Numbers (integer or float).
        if c.is_ascii_digit() {
            let start = i;
            let mut is_float = false;
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                if bytes[i] == b'.' {
                    if is_float {
                        break;
                    }
                    is_float = true;
                }
                i += 1;
                col += 1;
            }
            let s = std::str::from_utf8(&bytes[start..i]).unwrap();
            let kind = if is_float {
                match s.parse::<f64>() {
                    Ok(f) => TokenKind::Float(f),
                    Err(_) => {
                        return Err(GraphError::CypherParse {
                            line: start_line,
                            col: start_col,
                            message: format!("invalid float: {s}"),
                        })
                    }
                }
            } else {
                match s.parse::<i64>() {
                    Ok(n) => TokenKind::Integer(n),
                    Err(_) => {
                        return Err(GraphError::CypherParse {
                            line: start_line,
                            col: start_col,
                            message: format!("invalid integer: {s}"),
                        })
                    }
                }
            };
            tokens.push(Token {
                kind,
                line: start_line,
                col: start_col,
            });
            continue;
        }

        // String literal: '...' (single quote; doubled '' is escape for one quote)
        if c == '\'' {
            i += 1;
            col += 1;
            let mut s = String::new();
            while i < bytes.len() {
                // Doubled-quote escape: '' -> '
                if bytes[i] == b'\'' && i + 1 < bytes.len() && bytes[i + 1] == b'\'' {
                    s.push('\'');
                    i += 2;
                    col += 2;
                    continue;
                }
                // Backslash escape: \n \t \r \\ \' \" \X
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    match bytes[i + 1] {
                        b'n' => s.push('\n'),
                        b't' => s.push('\t'),
                        b'r' => s.push('\r'),
                        b'\\' => s.push('\\'),
                        b'\'' => s.push('\''),
                        b'"' => s.push('"'),
                        other => {
                            s.push('\\');
                            s.push(other as char);
                        }
                    }
                    i += 2;
                    col += 2;
                    continue;
                }
                // Closing quote
                if bytes[i] == b'\'' {
                    i += 1;
                    col += 1;
                    break;
                }
                // Regular char
                s.push(bytes[i] as char);
                i += 1;
                col += 1;
            }
            if i >= bytes.len() && (bytes.is_empty() || bytes[bytes.len() - 1] != b'\'') {
                return Err(GraphError::CypherParse {
                    line: start_line,
                    col: start_col,
                    message: "unterminated string literal".into(),
                });
            }
            tokens.push(Token {
                kind: TokenKind::String(s),
                line: start_line,
                col: start_col,
            });
            continue;
        }

        // Punctuation.
        match c {
            '(' => {
                i += 1;
                col += 1;
                tokens.push(Token {
                    kind: TokenKind::LParen,
                    line: start_line,
                    col: start_col,
                });
            }
            ')' => {
                i += 1;
                col += 1;
                tokens.push(Token {
                    kind: TokenKind::RParen,
                    line: start_line,
                    col: start_col,
                });
            }
            '[' => {
                i += 1;
                col += 1;
                tokens.push(Token {
                    kind: TokenKind::LBrack,
                    line: start_line,
                    col: start_col,
                });
            }
            ']' => {
                i += 1;
                col += 1;
                tokens.push(Token {
                    kind: TokenKind::RBrack,
                    line: start_line,
                    col: start_col,
                });
            }
            '{' => {
                i += 1;
                col += 1;
                tokens.push(Token {
                    kind: TokenKind::LBrace,
                    line: start_line,
                    col: start_col,
                });
            }
            '}' => {
                i += 1;
                col += 1;
                tokens.push(Token {
                    kind: TokenKind::RBrace,
                    line: start_line,
                    col: start_col,
                });
            }
            ':' => {
                i += 1;
                col += 1;
                tokens.push(Token {
                    kind: TokenKind::Colon,
                    line: start_line,
                    col: start_col,
                });
            }
            ',' => {
                i += 1;
                col += 1;
                tokens.push(Token {
                    kind: TokenKind::Comma,
                    line: start_line,
                    col: start_col,
                });
            }
            '.' => {
                i += 1;
                col += 1;
                tokens.push(Token {
                    kind: TokenKind::Dot,
                    line: start_line,
                    col: start_col,
                });
            }
            '|' => {
                i += 1;
                col += 1;
                tokens.push(Token {
                    kind: TokenKind::Pipe,
                    line: start_line,
                    col: start_col,
                });
            }
            '*' => {
                i += 1;
                col += 1;
                tokens.push(Token {
                    kind: TokenKind::Star,
                    line: start_line,
                    col: start_col,
                });
            }
            '-' if i + 1 < bytes.len() && bytes[i + 1] == b'>' => {
                i += 2;
                col += 2;
                tokens.push(Token {
                    kind: TokenKind::DashGt,
                    line: start_line,
                    col: start_col,
                });
            }
            '-' => {
                i += 1;
                col += 1;
                tokens.push(Token {
                    kind: TokenKind::Dash,
                    line: start_line,
                    col: start_col,
                });
            }
            '<' if i + 1 < bytes.len() && bytes[i + 1] == b'-' => {
                i += 2;
                col += 2;
                tokens.push(Token {
                    kind: TokenKind::LtDash,
                    line: start_line,
                    col: start_col,
                });
            }
            '<' if i + 1 < bytes.len() && bytes[i + 1] == b'>' => {
                i += 2;
                col += 2;
                tokens.push(Token {
                    kind: TokenKind::Ne,
                    line: start_line,
                    col: start_col,
                });
            }
            '<' if i + 1 < bytes.len() && bytes[i + 1] == b'=' => {
                i += 2;
                col += 2;
                tokens.push(Token {
                    kind: TokenKind::Le,
                    line: start_line,
                    col: start_col,
                });
            }
            '>' if i + 1 < bytes.len() && bytes[i + 1] == b'=' => {
                i += 2;
                col += 2;
                tokens.push(Token {
                    kind: TokenKind::Ge,
                    line: start_line,
                    col: start_col,
                });
            }
            '<' => {
                i += 1;
                col += 1;
                tokens.push(Token {
                    kind: TokenKind::Lt,
                    line: start_line,
                    col: start_col,
                });
            }
            '>' => {
                i += 1;
                col += 1;
                tokens.push(Token {
                    kind: TokenKind::Gt,
                    line: start_line,
                    col: start_col,
                });
            }
            '=' => {
                i += 1;
                col += 1;
                tokens.push(Token {
                    kind: TokenKind::Eq,
                    line: start_line,
                    col: start_col,
                });
            }
            _ => {
                return Err(GraphError::CypherParse {
                    line: start_line,
                    col: start_col,
                    message: format!("unexpected character: {c:?}"),
                });
            }
        }
    }

    tokens.push(Token {
        kind: TokenKind::Eof(),
        line,
        col,
    });
    Ok(tokens)
}

/// Classify a word: keyword (case-insensitive) or identifier.
fn keyword_or_ident(word: &str) -> TokenKind {
    // Uppercase the input for case-insensitive matching.
    match word.to_ascii_uppercase().as_str() {
        "MATCH" => TokenKind::Match,
        "OPTIONAL" => TokenKind::Optional,
        "WHERE" => TokenKind::Where,
        "RETURN" => TokenKind::Return,
        "WITH" => TokenKind::With,
        "UNION" => TokenKind::Union,
        "ALL" => TokenKind::All,
        "AS" => TokenKind::As,
        "AND" => TokenKind::And,
        "OR" => TokenKind::Or,
        "NOT" => TokenKind::Not,
        "IS" => TokenKind::Is,
        "NULL" => TokenKind::Null,
        "TRUE" => TokenKind::True,
        "FALSE" => TokenKind::False,
        "COUNT" => TokenKind::Count,
        "DISTINCT" => TokenKind::Distinct,
        "ORDER" => TokenKind::Order,
        "BY" => TokenKind::By,
        "SKIP" => TokenKind::Skip,
        "LIMIT" => TokenKind::Limit,
        _ => TokenKind::Ident(word.to_string()),
    }
}

impl TokenKind {
    /// Sentinel for end of input.
    pub fn Eof() -> Self {
        TokenKind::Ident(String::new()) // unused; lex() pushes EOF manually
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<TokenKind> {
        lex(src).unwrap().into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn lex_keywords_case_insensitive() {
        assert_eq!(
            kinds("MATCH return Where Optional"),
            vec![
                TokenKind::Match,
                TokenKind::Return,
                TokenKind::Where,
                TokenKind::Optional,
                TokenKind::Ident(String::new())
            ]
        );
    }

    #[test]
    fn lex_identifiers_and_arrows() {
        let k = kinds("(n)-[r:KNOWS]->(m)");
        assert!(matches!(k[0], TokenKind::LParen));
        assert!(matches!(k[1], TokenKind::Ident(ref s) if s == "n"));
        assert!(matches!(k[2], TokenKind::RParen));
        // "-[" is two tokens: Dash then LBrack; "->" is a single DashGt token.
        assert!(matches!(k[3], TokenKind::Dash));
        assert!(matches!(k[4], TokenKind::LBrack));
        assert!(matches!(k[5], TokenKind::Ident(ref s) if s == "r"));
        assert!(matches!(k[6], TokenKind::Colon));
        assert!(matches!(k[7], TokenKind::Ident(ref s) if s == "KNOWS"));
        assert!(matches!(k[8], TokenKind::RBrack));
        assert!(matches!(k[9], TokenKind::DashGt));
    }

    #[test]
    fn lex_integers_and_floats() {
        let k = kinds("42 3.14 0.5 .5");
        assert!(matches!(k[0], TokenKind::Integer(42)));
        assert!(matches!(k[1], TokenKind::Float(f) if (f - 3.14).abs() < 1e-9));
        assert!(matches!(k[2], TokenKind::Float(f) if (f - 0.5).abs() < 1e-9));
        // ".5" is Dot then Integer(5) — lexer can't peek; downstream parser will handle.
        assert!(matches!(k[3], TokenKind::Dot));
        assert!(matches!(k[4], TokenKind::Integer(5)));
    }

    #[test]
    fn lex_strings_with_doubled_quote_escape() {
        let k = kinds("'hello' 'it''s'");
        assert!(matches!(&k[0], TokenKind::String(s) if s == "hello"));
        assert!(matches!(&k[1], TokenKind::String(s) if s == "it's"));
    }

    #[test]
    fn lex_unterminated_string_errors() {
        let err = lex("'oops").unwrap_err();
        match err {
            GraphError::CypherParse { message, .. } => {
                assert!(message.contains("unterminated"), "got: {message}");
            }
            _ => panic!("wrong error variant"),
        }
    }

    #[test]
    fn lex_line_comment_skipped() {
        let k = kinds("MATCH // ignore this\nRETURN n");
        assert!(matches!(k[0], TokenKind::Match));
        assert!(matches!(k[1], TokenKind::Return));
        assert!(matches!(k[2], TokenKind::Ident(ref s) if s == "n"));
    }

    #[test]
    fn lex_block_comment_skipped() {
        let k = kinds("MATCH /* a\nb\nc */ RETURN n");
        assert!(matches!(k[0], TokenKind::Match));
        assert!(matches!(k[1], TokenKind::Return));
        assert!(matches!(k[2], TokenKind::Ident(ref s) if s == "n"));
    }
}
