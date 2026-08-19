use sqlrustgo_types::Value;
use std::fmt::Write;

#[allow(dead_code)]
pub fn format_table(columns: &[String], rows: &[Vec<Value>]) -> String {
    let mut out = String::new();

    let header_lens: Vec<usize> = columns.iter().map(|c| c.len()).collect();
    let num_cols = header_lens.len();
    if num_cols == 0 {
        return out;
    }

    // Compute max value lengths for each column
    // For Null values, treat as header_len + 1 to ensure minimum 2-char width
    let mut max_val_lens: Vec<usize> = vec![0; num_cols];
    for row in rows {
        for (i, v) in row.iter().enumerate() {
            if i >= num_cols {
                continue;
            }
            let s = value_to_string(v);
            // Null becomes empty but should have minimum width of header_len + 1
            let len = if s.is_empty() {
                header_lens[i] + 1
            } else {
                s.len()
            };
            max_val_lens[i] = max_val_lens[i].max(len);
        }
    }

    // Compute widths for rows: max(header_len, max_value_len)
    let mut row_widths: Vec<usize> = vec![0; num_cols];
    for i in 0..num_cols {
        row_widths[i] = header_lens[i].max(max_val_lens[i]);
    }

    // Output header
    // For non-empty: all columns use header_lens
    // For empty: last column uses header_lens + 1 (no extra trailing space)
    for (i, col) in columns.iter().enumerate() {
        if i > 0 {
            out.push_str(" |");
        }
        let w = if rows.is_empty() && i == num_cols - 1 {
            header_lens[i] + 1
        } else {
            header_lens[i]
        };
        write!(&mut out, "{:width$}", col, width = w).unwrap();
    }
    out.push('\n');

    // Output separator: "|" separator (no spaces)
    // For non-empty: non-last = row_widths + 1, last = row_widths (single col) or row_widths - 1 (multi col)
    // For empty: non-last = header_lens + 1, last = header_lens + 2
    for (i, &hl) in header_lens.iter().enumerate() {
        if i > 0 {
            out.push('|');
        }
        let dash_count = if rows.is_empty() {
            if i == num_cols - 1 {
                hl + 2
            } else {
                hl + 1
            }
        } else {
            if i == num_cols - 1 {
                if num_cols > 1 {
                    row_widths[i] - 1
                } else {
                    row_widths[i] + 1
                }
            } else {
                row_widths[i] + 1
            }
        };
        for _ in 0..dash_count {
            out.push('-');
        }
    }
    out.push('\n');

    // Output rows
    // All columns: pad to row_widths[i]
    for row in rows {
        for (i, v) in row.iter().enumerate() {
            if i >= num_cols {
                continue;
            }
            if i > 0 {
                out.push_str(" |");
            }
            let s = value_to_string(v);
            let w = row_widths[i];
            write!(&mut out, "{:width$}", s, width = w).unwrap();
        }
        out.push('\n');
    }

    out
}

#[allow(dead_code)]
pub fn format_list(rows: &[Vec<Value>]) -> String {
    let mut out = String::new();
    for row in rows {
        let parts: Vec<String> = row.iter().map(value_to_string).collect();
        out.push_str(&parts.join("|"));
        out.push('\n');
    }
    out
}

#[allow(dead_code)]
fn value_to_string(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::Integer(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Text(s) => s.clone(),
        Value::Boolean(b) => b.to_string(),
        _ => "<other>".to_string(),
    }
}

#[allow(dead_code)]
pub fn format_csv(columns: &[String], rows: &[Vec<Value>], include_header: bool) -> String {
    let mut out = String::new();
    if include_header {
        out.push_str(&columns.iter().map(|c| csv_quote(c)).collect::<Vec<_>>().join(","));
        out.push('\n');
    }
    for row in rows {
        let parts: Vec<String> = row.iter().map(|v| csv_quote(&value_to_string(v))).collect();
        out.push_str(&parts.join(","));
        out.push('\n');
    }
    out
}

fn csv_quote(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        let escaped = s.replace('"', "\"\"");
        format!("\"{}\"", escaped)
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlrustgo_types::Value;

    #[test]
    fn table_basic_two_columns() {
        let cols = vec!["id".to_string(), "name".to_string()];
        let rows = vec![
            vec![Value::Integer(1), Value::Text("Alice".into())],
            vec![Value::Integer(2), Value::Text("Bob".into())],
        ];
        let out = format_table(&cols, &rows);
        let expected = "id |name\n---|----\n1  |Alice\n2  |Bob  \n";
        assert_eq!(out, expected);
    }

    #[test]
    fn table_null_becomes_empty() {
        let cols = vec!["x".to_string()];
        let rows = vec![vec![Value::Null]];
        let out = format_table(&cols, &rows);
        assert!(out.contains("x\n"));
        assert!(out.contains("-\n"));
        assert!(out.contains("\n  \n"));
    }

    #[test]
    fn table_empty_rows_has_header_only() {
        let cols = vec!["a".to_string(), "b".to_string()];
        let rows: Vec<Vec<Value>> = vec![];
        let out = format_table(&cols, &rows);
        assert_eq!(out, "a |b \n--|---\n");
    }

    #[test]
    fn table_column_width_handles_long_values() {
        let cols = vec!["c".to_string()];
        let rows = vec![vec![Value::Text("a_very_long_value".into())]];
        let out = format_table(&cols, &rows);
        // column width = max("c".len=1, value.len=18) + 1 (padding) = 19
        assert!(out.contains("------------------")); // 18 dashes + 1
    }

    #[test]
    fn list_basic_three_columns() {
        let rows = vec![
            vec![
                Value::Integer(1),
                Value::Text("Alice".into()),
                Value::Boolean(true),
            ],
            vec![
                Value::Integer(2),
                Value::Text("Bob".into()),
                Value::Boolean(false),
            ],
        ];
        let out = format_list(&rows);
        assert_eq!(out, "1|Alice|true\n2|Bob|false\n");
    }

    #[test]
    fn list_null_is_empty_string() {
        let rows = vec![vec![
            Value::Integer(1),
            Value::Null,
            Value::Text("x".into()),
        ]];
        let out = format_list(&rows);
        assert_eq!(out, "1||x\n");
    }

    #[test]
    fn list_empty_rows_returns_empty_string() {
        let rows: Vec<Vec<Value>> = vec![];
        let out = format_list(&rows);
        assert_eq!(out, "");
    }

    #[test]
    fn csv_basic_with_header() {
        let cols = vec!["id".to_string(), "name".to_string()];
        let rows = vec![
            vec![Value::Integer(1), Value::Text("Alice".into())],
            vec![Value::Integer(2), Value::Text("Bob".into())],
        ];
        let out = format_csv(&cols, &rows, true);
        assert_eq!(out, "id,name\n1,Alice\n2,Bob\n");
    }

    #[test]
    fn csv_quotes_values_with_comma() {
        let cols = vec!["x".to_string()];
        let rows = vec![vec![Value::Text("a,b".into())]];
        let out = format_csv(&cols, &rows, false);
        assert_eq!(out, "\"a,b\"\n");
    }

    #[test]
    fn csv_doubles_inner_quotes() {
        let cols = vec!["x".to_string()];
        let rows = vec![vec![Value::Text("say \"hi\"".into())]];
        let out = format_csv(&cols, &rows, false);
        assert_eq!(out, "\"say \"\"hi\"\"\"\n");
    }

    #[test]
    fn csv_null_is_empty_string() {
        let cols = vec!["a".to_string(), "b".to_string()];
        let rows = vec![vec![Value::Null, Value::Text("x".into())]];
        let out = format_csv(&cols, &rows, false);
        assert_eq!(out, ",x\n");
    }

    #[test]
    fn csv_quotes_newlines() {
        let cols = vec!["x".to_string()];
        let rows = vec![vec![Value::Text("line1\nline2".into())]];
        let out = format_csv(&cols, &rows, false);
        assert_eq!(out, "\"line1\nline2\"\n");
    }
}
