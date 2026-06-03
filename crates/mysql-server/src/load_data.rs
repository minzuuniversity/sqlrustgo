//! LOAD DATA LOCAL INFILE — TBL parsing and batched insert.
//!
//! TBL format (TPC-H standard):
//!   - One row per line, fields separated by `|`
//!   - Lines end with `|\n` (trailing pipe), but we tolerate `|\n` or `\n`
//!   - Empty field → NULL
//!   - Integer-parseable → i64
//!   - Float-parseable → f64
//!   - Otherwise → Text

use sqlrustgo_types::Value as SqlValue;

pub fn parse_tbl_line(line: &str, expected_columns: usize) -> Result<Vec<SqlValue>, String> {
    let trimmed = line.trim_end_matches('\n').trim_end_matches('\r');
    // TPC-H .tbl: trailing `|` means last field is empty (counted)
    // We split on `|` and drop the trailing empty if present.
    let parts: Vec<&str> = trimmed.split('|').collect();
    let parts: Vec<&str> = if parts.last() == Some(&"") {
        parts[..parts.len() - 1].to_vec()
    } else {
        parts
    };

    if parts.len() < expected_columns {
        return Err(format!(
            "line has {} fields, expected at least {}: {:?}",
            parts.len(),
            expected_columns,
            line
        ));
    }

    let record: Vec<SqlValue> = parts[..expected_columns]
        .iter()
        .map(|v| {
            let s = v.trim();
            if s.is_empty() {
                SqlValue::Null
            } else if let Ok(i) = s.parse::<i64>() {
                SqlValue::Integer(i)
            } else if let Ok(f) = s.parse::<f64>() {
                SqlValue::Float(f)
            } else {
                SqlValue::Text(s.to_string())
            }
        })
        .collect();

    Ok(record)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tbl_line_basic_text() {
        // TPC-H region.tbl: r_regionkey|i_name|r_comment|
        let line = "0|AFRICA|lar deposits. blithely final packages cajole|\n";
        let cols = vec![
            SqlValue::Integer(0),
            SqlValue::Text("AFRICA".to_string()),
            SqlValue::Text("lar deposits. blithely final packages cajole".to_string()),
        ];
        assert_eq!(parse_tbl_line(line, 3).unwrap(), cols);
    }

    #[test]
    fn test_parse_tbl_line_with_ints() {
        let line = "1|2|3|\n";
        assert_eq!(
            parse_tbl_line(line, 3).unwrap(),
            vec![
                SqlValue::Integer(1),
                SqlValue::Integer(2),
                SqlValue::Integer(3),
            ]
        );
    }

    #[test]
    fn test_parse_tbl_line_with_null() {
        // Empty field in middle → NULL
        let line = "1||3|\n";
        assert_eq!(
            parse_tbl_line(line, 3).unwrap(),
            vec![SqlValue::Integer(1), SqlValue::Null, SqlValue::Integer(3)],
        );
    }

    #[test]
    fn test_parse_tbl_line_too_few_fields_errors() {
        let line = "1|2|\n";
        let result = parse_tbl_line(line, 3);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expected at least 3"));
    }

    #[test]
    fn test_parse_tbl_line_no_trailing_pipe() {
        // Tolerate missing trailing pipe
        let line = "0|AFRICA|comment\n";
        assert_eq!(
            parse_tbl_line(line, 3).unwrap(),
            vec![
                SqlValue::Integer(0),
                SqlValue::Text("AFRICA".to_string()),
                SqlValue::Text("comment".to_string()),
            ]
        );
    }
}
