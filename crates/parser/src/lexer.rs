//! SQL Lexer implementation
//! Tokenizes SQL input strings into tokens

use crate::token::Token;

/// SQL Lexer - Tokenizer
///
/// # What (是什么)
/// Lexer 将原始 SQL 字符串分解为 Token 序列，是编译器的第一阶段
///
/// # Why (为什么)
/// Parser 需要结构化的 Token 而不是原始字符串，Lexer 负责这项转换工作
///
/// # How (如何实现)
/// - 逐字符扫描输入
/// - 识别关键字、标识符、字面量、运算符
/// - 跳过空白字符
/// - 使用有限状态机处理不同 token 类型
pub struct Lexer<'a> {
    input: &'a str,
    position: usize,
}

impl<'a> Lexer<'a> {
    /// Create a new lexer for the given input
    pub fn new(input: &'a str) -> Self {
        Self { input, position: 0 }
    }

    /// Get the current position in the input
    pub fn position(&self) -> usize {
        self.position
    }

    /// Check if we've reached the end of input
    fn is_eof(&self) -> bool {
        self.position >= self.input.len()
    }

    /// Get the current character without advancing
    fn peek_char(&self) -> char {
        self.input[self.position..].chars().next().unwrap_or('\0')
    }

    /// Get the current character and advance
    #[allow(dead_code)]
    fn next_char(&mut self) -> char {
        let ch = self.peek_char();
        self.position += ch.len_utf8();
        ch
    }

    /// Skip whitespace characters and SQL line comments (`-- ...`).
    /// MySQL 5.7 standard line comments start with `--` and run to the
    /// end of the line. The previous lexer didn't handle these, so
    /// `-- === CASE: j_032 ===` inside a subquery was tokenised as
    /// `Minus, Minus, Equal, Equal, ...` and broke the parser.
    fn skip_whitespace(&mut self) {
        while !self.is_eof() {
            let ch = self.peek_char();
            if ch == '-' && self.input[self.position..].starts_with("--") {
                // Line comment: skip to end of line
                while !self.is_eof() && self.peek_char() != '\n' {
                    self.position += 1;
                }
            } else if !ch.is_whitespace() {
                break;
            } else {
                self.position += 1;
            }
        }
    }

    /// Read a sequence of alphanumeric characters (for identifiers)
    fn read_identifier(&mut self) -> String {
        let start = self.position;
        while !self.is_eof() {
            let ch = self.peek_char();
            if !ch.is_alphanumeric() && ch != '_' {
                break;
            }
            self.position += 1;
        }
        self.input[start..self.position].to_string()
    }

    /// Read a double-quoted identifier (e.g., "MyTable" -> MyTable)
    fn read_quoted_identifier(&mut self) -> String {
        self.position += 1; // Skip opening double quote
        let start = self.position;
        while !self.is_eof() {
            let ch = self.peek_char();
            if ch == '"' {
                break;
            }
            self.position += 1;
        }
        let result = self.input[start..self.position].to_string();
        if !self.is_eof() {
            self.position += 1; // Skip closing double quote
        }
        result
    }

    /// Read a number literal
    fn read_number(&mut self) -> String {
        let start = self.position;
        let mut has_decimal = false;
        while !self.is_eof() {
            let ch = self.peek_char();
            if ch == '.' {
                if has_decimal {
                    break;
                }
                has_decimal = true;
            } else if !ch.is_ascii_digit() {
                break;
            }
            self.position += 1;
        }
        self.input[start..self.position].to_string()
    }

    /// Read a string literal (single-quoted) - handles Unicode correctly
    /// and MySQL-style backslash escapes (\\n, \\t, \\, etc.).
    /// Without backslash handling, `ESCAPE '\\\\'` would produce a
    /// 2-char string instead of MySQL's 1-char backslash.
    fn read_string(&mut self) -> String {
        self.position += 1; // Skip opening quote
        let mut result = String::new();

        while !self.is_eof() {
            let ch = self.peek_char();
            if ch == '\'' {
                // Check for escaped quote ''
                let remaining = &self.input[self.position..];
                if remaining.starts_with("''") {
                    self.position += 2;
                    result.push('\'');
                    continue;
                }
                break;
            }
            if ch == '\\' {
                // MySQL backslash escape
                self.position += 1; // consume backslash
                let next = self.peek_char();
                let resolved = match next {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    '\\' => '\\',
                    '\'' => '\'',
                    '"' => '"',
                    '0' => '\0',
                    _ => next, // unknown escape, keep as-is
                };
                result.push(resolved);
                if !self.is_eof() {
                    self.position += 1;
                }
                continue;
            }
            // Move by character, not by byte, to handle Unicode
            result.push(ch);
            self.position += ch.len_utf8();
        }

        if !self.is_eof() {
            self.position += 1; // Skip closing quote
        }
        result
    }

    /// Get the next token from the input
    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();

        if self.is_eof() {
            return Token::Eof;
        }

        let ch = self.peek_char();

        match ch {
            '(' => {
                self.position += 1;
                Token::LParen
            }
            ')' => {
                self.position += 1;
                Token::RParen
            }
            ',' => {
                self.position += 1;
                Token::Comma
            }
            ';' => {
                self.position += 1;
                Token::Semicolon
            }
            '*' => {
                self.position += 1;
                Token::Star
            }
            '+' => {
                self.position += 1;
                Token::Plus
            }
            '-' => {
                if self.input[self.position..].starts_with("->>") {
                    self.position += 3;
                    Token::JsonArrowText
                } else if self.input[self.position..].starts_with("->") {
                    self.position += 2;
                    Token::JsonArrow
                } else {
                    self.position += 1;
                    Token::Minus
                }
            }
            '/' => {
                self.position += 1;
                Token::Slash
            }
            '%' => {
                self.position += 1;
                Token::Percent
            }
            '.' => {
                self.position += 1;
                Token::Dot
            }
            ':' => {
                self.position += 1;
                Token::Colon
            }
            // MySQL system variable: `@@version_comment`, `@@autocommit`, etc.
            // The double-`@` is a single token pulled out of the identifier
            // stream so the parser/executor can resolve it as a scalar value
            // rather than mistaking each `@` for a column reference.
            '@' => {
                if self.input[self.position..].starts_with("@@")
                    && self.position + 2 < self.input.len()
                {
                    let next = self
                        .input
                        .chars()
                        .nth(self.position + 2)
                        .unwrap_or('\0');
                    if next.is_alphabetic() || next == '_' {
                        self.position += 2;
                        let name = self.read_identifier();
                        return Token::SystemVariable(name);
                    }
                }
                // Fallback: single `@` is not legal in this dialect; treat as
                // an identifier so the parser can produce a clearer error
                // than the catch-all fallback below.
                self.position += 1;
                Token::Identifier("@".to_string())
            }
            '"' => Token::Identifier(self.read_quoted_identifier()),
            '\'' => Token::StringLiteral(self.read_string()),
            '=' => {
                self.position += 1;
                Token::Equal
            }
            '!' => {
                if self.input[self.position..].starts_with("!=") {
                    self.position += 2;
                    Token::NotEqual
                } else {
                    self.position += 1;
                    Token::Not
                }
            }
            '>' => {
                if self.input[self.position..].starts_with(">=") {
                    self.position += 2;
                    Token::GreaterEqual
                } else {
                    self.position += 1;
                    Token::Greater
                }
            }
            '<' => {
                if self.input[self.position..].starts_with("<=") {
                    self.position += 2;
                    Token::LessEqual
                } else if self.input[self.position..].starts_with("<>") {
                    self.position += 2;
                    Token::NotEqual
                } else {
                    self.position += 1;
                    Token::Less
                }
            }
            '|' => {
                // `||` is string concatenation in MySQL/PostgreSQL/Oracle SQL.
                // Token is `Or` (also used for boolean OR) — the parser and
                // evaluator distinguish based on operand type at runtime.
                if self.input[self.position..].starts_with("||") {
                    self.position += 2;
                    Token::Or
                } else {
                    // Single `|` is not a valid SQL token in our grammar;
                    // fall through to identifier path which will treat it
                    // as a stray identifier (and the parser will reject it).
                    self.position += 1;
                    Token::Identifier("|".to_string())
                }
            }
            _ if ch.is_alphabetic() || ch == '_' => {
                let ident = self.read_identifier();
                match ident.to_uppercase().as_str() {
                    "SELECT" => Token::Select,
                    "FROM" => Token::From,
                    "WHERE" => Token::Where,
                    "INSERT" => Token::Insert,
                    "IGNORE" => Token::Ignore,
                    "INTO" => Token::Into,
                    "VALUES" => Token::Values,
                    // "VALUE" is intentionally NOT a reserved keyword. It is only
                    // meaningful in the SQL:2003 `NEXT VALUE FOR sequence_name` form,
                    // where the parser matches `Identifier("value")` between NEXT and FOR.
                    // Treating it as a keyword broke column/alias use cases
                    // (e.g. `UPDATE t SET value = 1`, `SELECT 1 AS value`).
                    // Token::Value is now unused; retained for backwards-compat.
                    "VALUE" => Token::Identifier(ident.to_string()),
                    "UPDATE" => Token::Update,
                    "SET" => Token::Set,
                    "DELETE" => Token::Delete,
                    "MERGE" => Token::Merge,
                    "USING" => Token::Using,
                    "USE" => Token::Use,
                    "USER" => Token::User,
                    "MATCHED" => Token::Matched,
                    "CREATE" => Token::Create,
                    "TABLE" => Token::Table,
                    "DROP" => Token::Drop,
                    "ALTER" => Token::Alter,
                    "TRUNCATE" => Token::Truncate,
                    "DUPLICATE" => Token::Duplicate,
                    "INDEX" => Token::Index,
                    "IDENTIFIED" => Token::Identified,
                    "ON" => Token::On,
                    "PRIMARY" => Token::Primary,
                    "KEY" => Token::Key,
                    "ADD" => Token::Add,
                    "COLUMN" => Token::Column,
                    "MODIFY" => Token::Modify,
                    "RENAME" => Token::Rename,
                    "TO" => Token::To,
                    "BEGIN" => Token::Begin,
                    "COMMIT" => Token::Commit,
                    "ROLLBACK" => Token::Rollback,
                    "GRANT" => Token::Grant,
                    "REVOKE" => Token::Revoke,
                    "ANALYZE" => Token::Analyze,
                    "FOREIGN" => Token::Foreign,
                    "REFERENCES" => Token::References,
                    "UNIQUE" => Token::Unique,
                    "CHECK" => Token::Check,
                    "CONSTRAINT" => Token::Constraint,
                    "CASCADE" => Token::Cascade,
                    "RESTRICT" => Token::Restrict,
                    "NO" => Token::No,
                    "ACTION" => Token::Action,
                    "DEFAULT" => Token::Default,
                    "AUTO_INCREMENT" => Token::AutoIncrement,
                    "LOCK" => Token::Lock,
                    "LOCKED" => Token::Locked,
                    "PASSWORD" => Token::Password,
                    "SHARE" => Token::Share,
                    "MODE" => Token::Mode,
                    "SKIP" => Token::Skip,
                    "NOWAIT" => Token::Nowait,
                    "BOOLEAN" | "BOOL" => Token::Boolean,
                    "BY" => Token::By,
                    "BLOB" => Token::Blob,
                    "NULL" => Token::Null,
                    "TRUE" => Token::BooleanLiteral(true),
                    "FALSE" => Token::BooleanLiteral(false),
                    "AND" => Token::And,
                    "OR" => Token::Or,
                    "NOT" => Token::Not,
                    "IF" => Token::If,
                    "COLLATE" => Token::Collate,
                    "CASE" => Token::Case,
                    "WHEN" => Token::When,
                    "THEN" => Token::Then,
                    "ELSE" => Token::Else,
                    "EXPIRE" => Token::Expire,
                    "EXISTS" => Token::Exists,
                    "IN" => Token::In,
                    "IS" => Token::Is,
                    "ALL" => Token::All,
                    "ANY" => Token::Any,
                    "SOME" => Token::Some,
                    "AS" => Token::As,
                    "WITH" => Token::With,
                    "RECURSIVE" => Token::Recursive,
                    "COUNT" => Token::Count,
                    "SUM" => Token::Sum,
                    "AVG" => Token::Avg,
                    "MIN" => Token::Min,
                    "MAX" => Token::Max,
                    "GROUP" => Token::Group,
                    "HAVING" => Token::Having,
                    "ORDER" => Token::Order,
                    "LIMIT" => Token::Limit,
                    "OFFSET" => Token::Offset,
                    "DISTINCT" => Token::Distinct,
                    "JOIN" => Token::Join,
                    "INNER" => Token::Inner,
                    "LEFT" => Token::Left,
                    "RIGHT" => Token::Right,
                    "FULL" => Token::Full,
                    "CROSS" => Token::Cross,
                    "OUTER" => Token::Outer,
                    "NATURAL" => Token::Natural,
                    "UNION" => Token::Union,
                    "INTERSECT" => Token::Intersect,
                    "EXCEPT" => Token::Except,
                    "TRANSACTION" => Token::Transaction,
                    "WORK" => Token::Work,
                    "SAVEPOINT" => Token::Savepoint,
                    "START" => Token::Start,
                    "PREPARE" => Token::Prepare,
                    "EXECUTE" => Token::Execute,
                    "DEALLOCATE" => Token::Deallocate,
                    "RELEASE" => Token::Release,
                    "ISOLATION" => Token::Isolation,
                    "LEVEL" => Token::Level,
                    "SERIALIZABLE" => Token::Serializable,
                    "REPEATABLE" => Token::Repeatable,
                    "READ" => Token::Read,
                    "WRITE" => Token::Write,
                    "ONLY" => Token::Only,
                    "CALL" => Token::Call,
                    "PROCEDURE" => Token::Procedure,
                    "END" => Token::End,
                    "SHOW" => Token::Show,
                    "DESCRIBE" => Token::Describe,
                    "DESC" => Token::Desc,
                    "TRIGGER" => Token::Trigger,
                    "DATABASE" => Token::Database,
                    "VIEW" => Token::View,
                    "BEFORE" => Token::Before,
                    "AFTER" => Token::After,
                    "FOR" => Token::For,
                    "EACH" => Token::Each,
                    "UNBOUNDED" => Token::Unbounded,
                    "PRECEDING" => Token::Preceding,
                    "FOLLOWING" => Token::Following,
                    "CURRENT" => Token::Current,
                    "ROW" => Token::Row,
                    "ROWS" => Token::Rows,
                    "GROUPING" => Token::Grouping,
                    "POSITION" => Token::Position,
                    "INTERVAL" => Token::Interval,
                    "HIGH_PRIORITY" => Token::HighPriority,
                    "SQL_CACHE" => Token::SqlCache,
                    "SQL_NO_CACHE" => Token::SqlNoCache,
                    "SQL_CALC_FOUND_ROWS" => Token::SqlCalcFoundRows,
                    "CONVERT" => Token::Convert,
                    "DATE" => Token::Date,
                    "DATE_ADD" => Token::DateAdd,
                    "DATE_SUB" => Token::DateSub,
                    "SUBSTRING" => Token::Substring,
                    "SUBSTR" => Token::Substring,
                    "ROLLUP" => Token::Rollup,
                    "CUBE" => Token::Cube,
                    "ASOF" => Token::AsOf,
                    "REPLACE" => Token::Replace,
                    "WINDOW" => Token::Window,
                    "PARTITION" => Token::Partition,
                    "RANGE" => Token::Range,
                    "LIST" => Token::List,
                    "FULLTEXT" => Token::Fulltext,
                    "OVER" => Token::Over,
                    "BETWEEN" => Token::Between,
                    "ESCAPE" => Token::Escape,
                    // F-30 CREATE SEQUENCE
                    "SEQUENCE" => Token::Sequence,
                    "CYCLE" => Token::Cycle,
                    "NOCYCLE" => Token::NoCycle,
                    "CACHE" => Token::Cache,
                    "INCREMENT" => Token::Increment,
                    "WHILE" => Token::While,
                    "DO" => Token::Do,
                    "LOOP" => Token::Loop,
                    "LEAVE" => Token::Leave,
                    "ITERATE" => Token::Iterate,
                    "DECLARE" => Token::Declare,
                    "RETURN" => Token::Return,
                    "REPEAT" => Token::Repeat,
                    "UNTIL" => Token::Until,
                    "CONDITION" => Token::Condition,
                    "SIGNAL" => Token::Signal,
                    "RESIGNAL" => Token::Resignal,
                    "OUT" => Token::Out,
                    "INOUT" => Token::InOut,
                    "CURSOR" => Token::Cursor,
                    "HANDLER" => Token::Handler,
                    "SQL" => Token::SQL,
                    "LANGUAGE" => Token::Language,
                    "DETERMINISTIC" => Token::Deterministic,
                    "CONTAINS" => Token::Contains,
                    // NO and DATA handled separately to avoid conflicts
                    "OWNED" => Token::Owned,
                    "NEXT" => Token::NextValue,
                    "CURRVAL" => Token::Currval,
                    "RESTART" => Token::Restart,
                    "MINVALUE" => Token::Minvalue,
                    "MAXVALUE" => Token::Maxvalue,
                    "NOMINVALUE" => Token::NoMinValue,
                    "NOMAXVALUE" => Token::NoMaxValue,
                    _ => Token::Identifier(ident),
                }
            }
            _ if ch.is_ascii_digit() => Token::NumberLiteral(self.read_number()),
            _ => {
                self.position += 1;
                Token::Identifier(ch.to_string())
            }
        }
    }

    /// Tokenize the entire input and return a vector of tokens
    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token();
            tokens.push(token.clone());
            if matches!(token, Token::Eof) {
                break;
            }
        }
        tokens
    }
}

/// Convenience function to tokenize a SQL string
pub fn tokenize(sql: &str) -> Vec<Token> {
    let mut lexer = Lexer::new(sql);
    lexer.tokenize()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_select() {
        let mut lexer = Lexer::new("SELECT id FROM users WHERE id = 1");
        let tokens = lexer.tokenize();
        assert_eq!(tokens[0], Token::Select);
        assert_eq!(tokens[1], Token::Identifier("id".to_string()));
        assert_eq!(tokens[2], Token::From);
        assert_eq!(tokens[3], Token::Identifier("users".to_string()));
        assert_eq!(tokens[4], Token::Where);
        assert_eq!(tokens[5], Token::Identifier("id".to_string()));
        assert_eq!(tokens[6], Token::Equal);
        assert_eq!(tokens[7], Token::NumberLiteral("1".to_string()));
        assert_eq!(tokens.last().unwrap(), &Token::Eof);
    }

    #[test]
    fn test_operators() {
        // Basic lexer test
        let tokens = tokenize("SELECT");
        assert_eq!(tokens.len(), 2); // SELECT + EOF
        assert_eq!(tokens[0], Token::Select);
    }

    #[test]
    fn test_keywords_case_insensitive() {
        let tokens = tokenize("select * from users");
        assert_eq!(tokens[0], Token::Select);
        assert_eq!(tokens[2], Token::From);
    }

    #[test]
    fn test_all_keywords() {
        let sql = "SELECT INSERT UPDATE DELETE CREATE DROP TABLE";
        let tokens = tokenize(sql);
        assert_eq!(tokens[0], Token::Select);
        assert_eq!(tokens[1], Token::Insert);
        assert_eq!(tokens[2], Token::Update);
        assert_eq!(tokens[3], Token::Delete);
        assert_eq!(tokens[4], Token::Create);
        assert_eq!(tokens[5], Token::Drop);
        assert_eq!(tokens[6], Token::Table);
    }

    #[test]
    fn test_merge_keyword_uppercase() {
        let tokens = Lexer::new("MERGE").tokenize();
        assert_eq!(tokens[0], Token::Merge);
    }

    #[test]
    fn test_merge_keyword_lowercase() {
        let tokens = Lexer::new("merge").tokenize();
        assert_eq!(tokens[0], Token::Merge);
    }

    #[test]
    fn test_merge_keyword_mixedcase() {
        let tokens = Lexer::new("Merge").tokenize();
        assert_eq!(tokens[0], Token::Merge);
    }

    #[test]
    fn test_merge_using_when_matched_keywords() {
        let tokens =
            Lexer::new("MERGE INTO t USING s ON t.id = s.id WHEN MATCHED THEN UPDATE").tokenize();
        assert_eq!(tokens[0], Token::Merge);
        assert_eq!(tokens[1], Token::Into);
        // tokens[2] = Identifier("t"), tokens[3] = Using, ...
        assert_eq!(tokens[3], Token::Using);
        assert_eq!(tokens[5], Token::On);
        // After "t.id = s.id" comes WHEN
        let when_pos = tokens
            .iter()
            .position(|t| matches!(t, Token::When))
            .expect("expected WHEN token");
        assert_eq!(tokens[when_pos], Token::When);
        assert_eq!(tokens[when_pos + 1], Token::Matched);
        assert_eq!(tokens[when_pos + 2], Token::Then);
        assert_eq!(tokens[when_pos + 3], Token::Update);
    }

    #[test]
    fn test_lexer_string_literal() {
        let tokens = Lexer::new("'hello world'").tokenize();
        assert!(matches!(&tokens[0], Token::StringLiteral(s) if s == "hello world"));
    }

    #[test]
    fn test_lexer_operators() {
        let tokens = Lexer::new("<> <= >=").tokenize();
        assert!(matches!(tokens[0], Token::NotEqual));
        assert!(matches!(tokens[1], Token::LessEqual));
        assert!(matches!(tokens[2], Token::GreaterEqual));
    }

    #[test]
    fn test_lexer_multiple_statements() {
        let tokens = Lexer::new("SELECT 1; SELECT 2").tokenize();
        // tokens structure: [Select, NumberLiteral(1), Semicolon, Select, NumberLiteral(2), Eof]
        // Semicolon is at index 2
        assert!(matches!(&tokens[2], Token::Semicolon));
    }

    #[test]
    fn test_lexer_float_number() {
        let tokens = Lexer::new("3.14").tokenize();
        assert!(matches!(&tokens[0], Token::NumberLiteral(s) if s == "3.14"));
    }

    #[test]
    fn test_lexer_negative_number() {
        let tokens = Lexer::new("-42").tokenize();
        assert!(matches!(&tokens[0], Token::Minus));
        assert!(matches!(&tokens[1], Token::NumberLiteral(s) if s == "42"));
    }

    #[test]
    fn test_lexer_boolean_literal() {
        let tokens = Lexer::new("TRUE FALSE").tokenize();
        assert!(matches!(&tokens[0], Token::BooleanLiteral(true)));
        assert!(matches!(&tokens[1], Token::BooleanLiteral(false)));
    }

    #[test]
    fn test_lexer_null() {
        let tokens = Lexer::new("NULL").tokenize();
        assert_eq!(tokens[0], Token::Null);
    }
}

#[cfg(test)]
mod double_quote_tests {
    use super::*;

    #[test]
    fn test_double_quoted_identifier() {
        let sql = r#"CREATE TABLE "MyTable"(i integer)"#;
        let tokens = tokenize(sql);
        println!("Tokens: {:?}", tokens);
        // CREATE TABLE "MyTable" ( i integer )
        // = 7 tokens: Create, Table, Identifier("MyTable"), LParen, Identifier("i"), Identifier("INTEGER"), RParen
        assert!(
            tokens.len() >= 3,
            "Expected at least 3 tokens, got {:?}",
            tokens
        );
        assert_eq!(tokens[0], Token::Create);
        assert_eq!(tokens[1], Token::Table);
        assert_eq!(tokens[2], Token::Identifier("MyTable".to_string()));
    }

    #[test]
    fn test_values_keyword() {
        let sql = "VALUES (1, 2)";
        let tokens = tokenize(sql);
        assert_eq!(tokens[0], Token::Values);
        assert_eq!(tokens[1], Token::LParen);
    }
}
