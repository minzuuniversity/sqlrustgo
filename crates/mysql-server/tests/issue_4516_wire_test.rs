//! Regression tests for Issue #4516 — wire-layer MySQL-style column
//! headers for `SHOW DATABASES` / `SHOW TABLES` / `SHOW COLUMNS` /
//! `DESCRIBE` / `SHOW FULL TABLES` / `SHOW TABLE STATUS` / `SHOW
//! CREATE TABLE` / `SHOW GRANTS` / `SHOW INDEX` / `SHOW VARIABLES`
//! / `SHOW STATUS` / `SHOW WARNINGS` / `SHOW ERRORS` / `SHOW
//! SEQUENCES` / `SHOW PROCEDURE STATUS` / `SHOW PROCESSLIST`.
//!
//! Background
//! ----------
//! Before #4516, the COM_QUERY and COM_STMT_EXECUTE read-only
//! dispatch paths fell back to `col_1, col_2, ...` for any result
//! that did not come from a SELECT projection. The header naming
//! for SHOW/DESCRIBE statements is centralized in the
//! `show_column_headers` helper in `crates/mysql-server/src/lib.rs`,
//! and this file pins each mapping.

use sqlrustgo_mysql_server::show_column_headers;
use sqlrustgo_parser::parser::{DescribeStatement, ShowStatement, Statement};

// ---------- SHOW DATABASES ----------

#[test]
fn show_databases_header_is_database() {
    let stmt = Statement::Show(ShowStatement::Databases);
    let h = show_column_headers(&stmt).expect("should produce headers");
    assert_eq!(h, vec!["Database".to_string()]);
}

// ---------- SHOW TABLES ----------

#[test]
fn show_tables_header_uses_default_schema_label_when_no_from() {
    let stmt = Statement::Show(ShowStatement::Tables {
        db: None,
        like: None,
        where_clause: None,
    });
    let h = show_column_headers(&stmt).expect("should produce headers");
    assert_eq!(h, vec!["Tables_in_default".to_string()]);
}

#[test]
fn show_tables_header_uses_explicit_schema_label() {
    let stmt = Statement::Show(ShowStatement::Tables {
        db: Some("mydb".to_string()),
        like: None,
        where_clause: None,
    });
    let h = show_column_headers(&stmt).unwrap();
    assert_eq!(h, vec!["Tables_in_mydb".to_string()]);
}

#[test]
fn show_tables_header_ignores_like_filter_in_naming() {
    // LIKE narrows rows, not column names. The header is still
    // `Tables_in_<db>` regardless of the filter.
    let stmt = Statement::Show(ShowStatement::Tables {
        db: None,
        like: Some("foo%".to_string()),
        where_clause: None,
    });
    let h = show_column_headers(&stmt).unwrap();
    assert_eq!(h, vec!["Tables_in_default".to_string()]);
}

// ---------- DESCRIBE / SHOW COLUMNS ----------

#[test]
fn describe_returns_six_column_mysql_header() {
    let stmt = Statement::Describe(DescribeStatement {
        table: "t".to_string(),
    });
    let h = show_column_headers(&stmt).expect("should produce headers");
    assert_eq!(
        h,
        vec![
            "Field".to_string(),
            "Type".to_string(),
            "Null".to_string(),
            "Key".to_string(),
            "Default".to_string(),
            "Extra".to_string(),
        ]
    );
}

#[test]
fn show_columns_returns_six_column_mysql_header() {
    let stmt = Statement::Show(ShowStatement::Columns {
        table: "t".to_string(),
        pattern: None,
    });
    let h = show_column_headers(&stmt).expect("should produce headers");
    assert_eq!(h.len(), 6);
    assert_eq!(h[0], "Field");
    assert_eq!(h[1], "Type");
    assert_eq!(h[2], "Null");
    assert_eq!(h[3], "Key");
    assert_eq!(h[4], "Default");
    assert_eq!(h[5], "Extra");
}

#[test]
fn show_columns_with_pattern_still_uses_six_column_header() {
    // LIKE pattern narrows rows but not column names.
    let stmt = Statement::Show(ShowStatement::Columns {
        table: "t".to_string(),
        pattern: Some("foo%".to_string()),
    });
    let h = show_column_headers(&stmt).unwrap();
    assert_eq!(h[0], "Field");
    assert_eq!(h[5], "Extra");
}

// ---------- SHOW FULL TABLES ----------

#[test]
fn show_full_tables_returns_name_and_type_columns() {
    // V312-59-A / #4384: SHOW FULL TABLES returns (Name, Table_type)
    let stmt = Statement::Show(ShowStatement::FullTables {
        full: true,
        db: None,
        like: None,
        where_clause: None,
    });
    let h = show_column_headers(&stmt).expect("should produce headers");
    assert_eq!(h, vec!["Name".to_string(), "Table_type".to_string()]);
}

// ---------- SHOW TABLE STATUS ----------

#[test]
fn show_table_status_returns_eighteen_columns() {
    // MySQL 18-column SHOW TABLE STATUS: Name, Engine, Version, Row_format,
    // Rows, Avg_row_length, Data_length, Max_data_length, Index_length,
    // Data_free, Auto_increment, Create_time, Update_time, Check_time,
    // Collation, Checksum, Create_options, Comment
    let stmt = Statement::Show(ShowStatement::TableStatus {
        db: None,
        like: None,
        where_clause: None,
    });
    let h = show_column_headers(&stmt).expect("should produce headers");
    assert_eq!(h.len(), 18, "expected 18-column header, got {h:?}");
    assert_eq!(h[0], "Name");
    assert_eq!(h[17], "Comment");
}

// ---------- Other SHOW variants ----------

#[test]
fn show_index_header_is_twelve_columns() {
    // The wire helper maps SHOW INDEX to a 12-column header.
    let stmt = Statement::Show(ShowStatement::Index {
        table: "t".to_string(),
    });
    let h = show_column_headers(&stmt).expect("should produce headers");
    assert_eq!(h.len(), 12, "expected 12-column header, got {h:?}");
    assert_eq!(h[0], "Table");
    assert_eq!(h[4], "Column_name");
}

#[test]
fn show_create_table_returns_table_and_create_table_columns() {
    let stmt = Statement::Show(ShowStatement::CreateTable {
        table: "t".to_string(),
    });
    let h = show_column_headers(&stmt).expect("should produce headers");
    assert_eq!(h, vec!["Table".to_string(), "Create Table".to_string()]);
}

#[test]
fn show_grants_returns_user_column() {
    let stmt = Statement::Show(ShowStatement::Grants { user: None });
    let h = show_column_headers(&stmt).expect("should produce headers");
    assert!(h.iter().any(|c| c == "Grants for User@Host"));
}

#[test]
fn show_sequences_returns_header_array() {
    let stmt = Statement::Show(ShowStatement::Sequences);
    let h = show_column_headers(&stmt).expect("should produce headers");
    assert!(!h.is_empty(), "SHOW SEQUENCES should map to headers");
    assert_eq!(h[0], "Sequence");
}

#[test]
fn show_procedure_status_returns_eleven_columns() {
    // MySQL 11-column SHOW PROCEDURE STATUS
    let stmt = Statement::Show(ShowStatement::ProcedureStatus { pattern: None });
    let h = show_column_headers(&stmt).expect("should produce headers");
    assert_eq!(h.len(), 11, "expected 11-column header, got {h:?}");
    assert_eq!(h[0], "Db");
    assert_eq!(h[1], "Name");
}

#[test]
fn show_warnings_returns_three_columns() {
    // MySQL: Level, Code, Message
    let stmt = Statement::Show(ShowStatement::Warnings);
    let h = show_column_headers(&stmt).expect("should produce headers");
    assert_eq!(
        h,
        vec![
            "Level".to_string(),
            "Code".to_string(),
            "Message".to_string()
        ]
    );
}

#[test]
fn show_errors_returns_three_columns() {
    let stmt = Statement::Show(ShowStatement::Errors);
    let h = show_column_headers(&stmt).expect("should produce headers");
    assert_eq!(
        h,
        vec![
            "Level".to_string(),
            "Code".to_string(),
            "Message".to_string()
        ]
    );
}

#[test]
fn show_status_returns_name_value_columns() {
    // MySQL: Variable_name, Value
    let stmt = Statement::Show(ShowStatement::Status);
    let h = show_column_headers(&stmt).expect("should produce headers");
    assert_eq!(h, vec!["Variable_name".to_string(), "Value".to_string()]);
}

#[test]
fn show_variables_returns_name_value_columns() {
    let stmt = Statement::Show(ShowStatement::Variables);
    let h = show_column_headers(&stmt).expect("should produce headers");
    assert_eq!(h, vec!["Variable_name".to_string(), "Value".to_string()]);
}

#[test]
fn show_processlist_returns_eight_columns() {
    // MySQL 8-column SHOW PROCESSLIST
    let stmt = Statement::Show(ShowStatement::Processlist { full: false });
    let h = show_column_headers(&stmt).expect("should produce headers");
    assert_eq!(h.len(), 8, "expected 8-column header, got {h:?}");
    assert_eq!(h[0], "Id");
}

#[test]
fn show_full_processlist_still_returns_eight_columns() {
    let stmt = Statement::Show(ShowStatement::Processlist { full: true });
    let h = show_column_headers(&stmt).unwrap();
    assert_eq!(h.len(), 8);
    assert_eq!(h[0], "Id");
}

// ---------- Non-show statement returns None ----------

#[test]
fn select_statement_returns_none() {
    // The wire helper is for SHOW/DESCRIBE; SELECT projection
    // names come from the parser.
    let sql = "SELECT 1";
    let parsed = sqlrustgo_parser::parser::parse(sql).unwrap();
    assert!(show_column_headers(&parsed).is_none());
}

#[test]
fn insert_statement_returns_none() {
    let sql = "INSERT INTO t VALUES (1)";
    let parsed = sqlrustgo_parser::parser::parse(sql).unwrap();
    assert!(show_column_headers(&parsed).is_none());
}
