//! SLT File Parser
//!
//! Parses SQLite SQLLogicTest .test files into structured statements.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("line {line}: {msg}")]
    Invalid { line: usize, msg: String },
    #[error("unterminated block at line {line}")]
    UnterminatedBlock { line: usize },
}

#[derive(Debug, Clone, PartialEq)]
pub enum StatementKind {
    StatementOk,
    StatementError,
    StatementOnly,
    Query,
    HashQuery,
    QueryParallel,
    Skip,
    Halt,
    Connect,
    Rebuild,
    Mode,
}

impl StatementKind {
    pub fn from_line(line: &str) -> Option<Self> {
        let line = line.trim();
        if line == "statement ok" {
            Some(Self::StatementOk)
        } else if line == "statement error" {
            Some(Self::StatementError)
        } else if line == "statement only" {
            Some(Self::StatementOnly)
        } else if line.starts_with("query ") {
            Some(Self::Query)
        } else if line.starts_with("hash query ") {
            Some(Self::HashQuery)
        } else if line.starts_with("query parallel ") {
            Some(Self::QueryParallel)
        } else if line == "skip" {
            Some(Self::Skip)
        } else if line == "halt" {
            Some(Self::Halt)
        } else if line == "connect" {
            Some(Self::Connect)
        } else if line == "rebuild" {
            Some(Self::Rebuild)
        } else if line == "mode" {
            Some(Self::Mode)
        } else {
            None
        }
    }
}

/// A parsed SLT statement
#[derive(Debug, Clone)]
pub struct Statement {
    pub kind: StatementKind,
    /// The SQL text (for queries, this is the query itself)
    pub sql: String,
    /// Expected error code (for statement error)
    pub error_code: Option<String>,
    /// Expected column types (for query) e.g. "I" or "II"
    pub col_types: Option<String>,
    /// Expected output rows (for query), one line per row
    pub expected: Vec<String>,
    /// Sort mode for query results
    pub sort_mode: Option<String>,
}

impl Statement {
    /// Extract column types from a query line like "query I II T"
    pub fn parse_col_types(line: &str) -> Option<String> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        // Format: "query <types>..." or "query <sortmode> <types>..."
        if parts.len() >= 2 && parts[0] == "query" {
            let rest = parts[1..].join(" ");
            // If it looks like types (I, T, R, N), return it
            if rest.chars().all(|c| c.is_ascii_uppercase() || c.is_whitespace()) {
                return Some(rest.trim().to_string());
            }
            // Otherwise it's a sort mode
        }
        None
    }
}

/// A parsed .test file
#[derive(Debug, Clone)]
pub struct SltFile {
    pub path: String,
    pub stmts: Vec<Statement>,
}

impl SltFile {
    /// Parse a .test file from string content
    pub fn parse(content: &str, path: &str) -> Result<Self, ParseError> {
        let mut stmts = Vec::new();
        let mut sql_buf = String::new();
        let mut expected_buf = Vec::new();
        let mut in_expected = false;
        let mut current_kind: Option<StatementKind> = None;
        let mut current_col_types: Option<String> = None;
        let mut current_sort_mode: Option<String> = None;

        for (_line_no, raw_line) in content.lines().enumerate() {
            let line = raw_line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                if in_expected {
                    // Empty line in expected block ends it
                    in_expected = false;
                }
                continue;
            }

            // Start of a new statement?
            if let Some(kind) = StatementKind::from_line(line) {
                // Flush previous query if any
                if let Some(k) = current_kind.take() {
                    // Flush any non-empty statement (query or statement ok/only/error)
                    if !sql_buf.is_empty()
                        || matches!(k, StatementKind::StatementOk | StatementKind::StatementOnly | StatementKind::StatementError)
                    {
                        stmts.push(Statement {
                            kind: k.clone(),
                            sql: sql_buf.trim().to_string(),
                            error_code: None,
                            col_types: current_col_types.take(),
                            expected: std::mem::take(&mut expected_buf),
                            sort_mode: current_sort_mode.take(),
                        });
                        sql_buf.clear();
                    }
                }

                current_kind = Some(kind.clone());

                // Handle query with optional sort mode and types
                if matches!(kind, StatementKind::Query | StatementKind::HashQuery | StatementKind::QueryParallel) {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    // Try to extract sort mode and col types
                    // Format: "query [sortmode] <types>..."
                    for (i, p) in parts.iter().enumerate().skip(1) {
                        if *p == "nosort" || *p == "sort" || *p == "hash" {
                            current_sort_mode = Some((*p).to_string());
                        } else {
                            current_col_types = Some(parts[i..].join(" "));
                            break;
                        }
                    }
                }
                in_expected = false;
                continue;
            }

            // "----" marks the start of expected output
            if line == "----" {
                in_expected = true;
                continue;
            }

            // Inside expected output block
            if in_expected {
                expected_buf.push(line.to_string());
                continue;
            }

            // Regular SQL line
            if current_kind.is_some() {
                if !sql_buf.is_empty() {
                    sql_buf.push(' ');
                }
                sql_buf.push_str(line);
            }
        }

        // Flush final statement
        if let Some(k) = current_kind.clone() {
            if matches!(k, StatementKind::Query | StatementKind::HashQuery) && !sql_buf.is_empty() {
                stmts.push(Statement {
                    kind: k,
                    sql: sql_buf.trim().to_string(),
                    error_code: None,
                    col_types: current_col_types,
                    expected: expected_buf,
                    sort_mode: current_sort_mode,
                });
            }
        }

        Ok(SltFile { path: path.to_string(), stmts })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
statement ok
CREATE TABLE t1 (a INTEGER, b TEXT)

query I T
SELECT a, b FROM t1 ORDER BY a
----
1    hello
2    world

statement ok
INSERT INTO t1 VALUES (3, 'three')

query I
SELECT COUNT(*) FROM t1
----
3
"#;

    #[test]
    fn test_parse_slt_file() {
        let file = SltFile::parse(SAMPLE, "test.test").unwrap();
        assert_eq!(file.stmts.len(), 3);
        assert!(matches!(file.stmts[0].kind, StatementKind::StatementOk));
        assert_eq!(file.stmts[1].sql.trim(), "SELECT a, b FROM t1 ORDER BY a");
        assert_eq!(file.stmts[1].expected.len(), 2);
        assert_eq!(file.stmts[1].expected[0], "1    hello");
    }
}
