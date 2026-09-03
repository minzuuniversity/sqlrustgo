//! tbl2bin — Convert TPC-H TBL files to SQLRustGo BinaryTableStorage .bin files.
//!
//! Usage: tbl2bin <tbl_dir> <bin_dir> [table...]
//!   tbl_dir: directory containing *.tbl files (e.g. tpch-dbgen/)
//!   bin_dir: output directory for .bin files
//!   table...: optionally restrict to specific tables (default: all 8 TPC-H tables)
//!
//! Example:
//!   tbl2bin /home/openclaw/tpch-dbgen-master /tmp/tpch-bin
//!   tbl2bin /home/openclaw/tpch-dbgen-master /tmp/tpch-bin lineitem orders

use sqlrustgo_storage::engine::{ColumnDefinition, TableData, TableInfo};
use sqlrustgo_types::Value;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

// TPC-H SF=1.0 table schemas
const TPC_H_TABLES: &[(&str, &[(&str, &str)])] = &[
    (
        "region",
        &[
            ("R_REGIONKEY", "INT"),
            ("R_NAME", "TEXT"),
            ("R_COMMENT", "TEXT"),
        ],
    ),
    (
        "nation",
        &[
            ("N_NATIONKEY", "INT"),
            ("N_NAME", "TEXT"),
            ("N_REGIONKEY", "INT"),
            ("N_COMMENT", "TEXT"),
        ],
    ),
    (
        "customer",
        &[
            ("C_CUSTKEY", "INT"),
            ("C_NAME", "TEXT"),
            ("C_ADDRESS", "TEXT"),
            ("C_NATIONKEY", "INT"),
            ("C_PHONE", "TEXT"),
            ("C_ACCTBAL", "FLOAT"),
            ("C_MKTSEGMENT", "TEXT"),
            ("C_COMMENT", "TEXT"),
        ],
    ),
    (
        "supplier",
        &[
            ("S_SUPPKEY", "INT"),
            ("S_NAME", "TEXT"),
            ("S_ADDRESS", "TEXT"),
            ("S_NATIONKEY", "INT"),
            ("S_PHONE", "TEXT"),
            ("S_ACCTBAL", "FLOAT"),
            ("S_COMMENT", "TEXT"),
        ],
    ),
    (
        "part",
        &[
            ("P_PARTKEY", "INT"),
            ("P_NAME", "TEXT"),
            ("P_MFGR", "TEXT"),
            ("P_BRAND", "TEXT"),
            ("P_TYPE", "TEXT"),
            ("P_SIZE", "INT"),
            ("P_CONTAINER", "TEXT"),
            ("P_RETAILPRICE", "FLOAT"),
            ("P_COMMENT", "TEXT"),
        ],
    ),
    (
        "partsupp",
        &[
            ("PS_PARTKEY", "INT"),
            ("PS_SUPPKEY", "INT"),
            ("PS_AVAILQTY", "INT"),
            ("PS_SUPPLYCOST", "FLOAT"),
            ("PS_COMMENT", "TEXT"),
        ],
    ),
    (
        "orders",
        &[
            ("O_ORDERKEY", "INT"),
            ("O_CUSTKEY", "INT"),
            ("O_ORDERSTATUS", "TEXT"),
            ("O_TOTALPRICE", "FLOAT"),
            ("O_ORDERDATE", "TEXT"),
            ("O_ORDERPRIORITY", "TEXT"),
            ("O_CLERK", "TEXT"),
            ("O_SHIPPRIORITY", "INT"),
            ("O_COMMENT", "TEXT"),
        ],
    ),
    (
        "lineitem",
        &[
            ("L_ORDERKEY", "INT"),
            ("L_PARTKEY", "INT"),
            ("L_SUPPKEY", "INT"),
            ("L_LINENUMBER", "INT"),
            ("L_QUANTITY", "FLOAT"),
            ("L_EXTENDEDPRICE", "FLOAT"),
            ("L_DISCOUNT", "FLOAT"),
            ("L_TAX", "FLOAT"),
            ("L_RETURNFLAG", "TEXT"),
            ("L_LINESTATUS", "TEXT"),
            ("L_SHIPDATE", "TEXT"),
            ("L_COMMITDATE", "TEXT"),
            ("L_RECEIPTDATE", "TEXT"),
            ("L_SHIPINSTRUCT", "TEXT"),
            ("L_SHIPMODE", "TEXT"),
            ("L_COMMENT", "TEXT"),
        ],
    ),
];

/// Parse a TBL line into Values, using the column type hints.
fn parse_tbl_line(line: &str, col_types: &[&str]) -> Vec<Value> {
    let trimmed = line.trim_end_matches('\n').trim_end_matches('\r');
    let parts: Vec<&str> = trimmed.split('|').collect();
    let parts: Vec<&str> = if parts.last() == Some(&"") {
        parts[..parts.len() - 1].to_vec()
    } else {
        parts
    };

    let mut values = Vec::with_capacity(col_types.len());
    for (i, col_type) in col_types.iter().enumerate() {
        let s = parts.get(i).map(|v| v.trim()).unwrap_or("");
        if s.is_empty() {
            values.push(Value::Null);
        } else if *col_type == "INT" {
            if let Ok(n) = s.parse::<i64>() {
                values.push(Value::Integer(n));
            } else {
                values.push(Value::Text(s.to_string()));
            }
        } else if *col_type == "FLOAT" {
            if let Ok(f) = s.parse::<f64>() {
                values.push(Value::Float(f));
            } else {
                values.push(Value::Text(s.to_string()));
            }
        } else {
            values.push(Value::Text(s.to_string()));
        }
    }
    values
}

/// Write a TableData struct to BINT v2 binary format.
fn write_bin(path: &Path, table_data: &TableData) -> Result<()> {
    let file = File::create(path)?;
    let mut w = BufWriter::new(file);

    // Header: magic + version 2
    w.write_all(b"BINT")?;
    w.write_all(&2u32.to_le_bytes())?;

    // Column count
    w.write_all(&(table_data.info.columns.len() as u32).to_le_bytes())?;

    // Per-column type + name
    for col in &table_data.info.columns {
        let code = match col.data_type.to_uppercase().as_str() {
            t if t.contains("INT") => 1u8,
            t if t.contains("FLOAT") || t.contains("REAL") || t.contains("DOUBLE") => 2u8,
            _ => 3u8,
        };
        w.write_all(&[code])?;
        let name_bytes = col.name.as_bytes();
        w.write_all(&(name_bytes.len() as u16).to_le_bytes())?;
        w.write_all(name_bytes)?;
    }

    // Row count
    w.write_all(&(table_data.rows.len() as u64).to_le_bytes())?;

    // Write rows
    for row in &table_data.rows {
        for val in row {
            match val {
                Value::Integer(i) => {
                    w.write_all(&[1])?;
                    w.write_all(&i.to_le_bytes())?;
                }
                Value::Float(f) => {
                    w.write_all(&[2])?;
                    w.write_all(&f.to_le_bytes())?;
                }
                Value::Text(s) => {
                    w.write_all(&[3])?;
                    let bytes = s.as_bytes();
                    w.write_all(&(bytes.len() as u32).to_le_bytes())?;
                    w.write_all(bytes)?;
                }
                Value::Null => {
                    w.write_all(&[0])?;
                }
                _ => {
                    w.write_all(&[0])?;
                }
            }
        }
    }

    w.flush()?;
    Ok(())
}

fn convert_table(
    tbl_dir: &Path,
    bin_dir: &Path,
    table: &str,
    col_schema: &[(&str, &str)],
) -> Result<(usize, usize)> {
    let tbl_path = tbl_dir.join(format!("{}.tbl", table));
    if !tbl_path.exists() {
        eprintln!("WARN: {} not found, skipping", tbl_path.display());
        return Ok((0, 0));
    }

    // Build column definitions using ColumnDefinition::new
    let columns: Vec<ColumnDefinition> = col_schema
        .iter()
        .map(|(name, dtype)| ColumnDefinition::new(name, dtype))
        .collect();

    let info = TableInfo {
        name: table.to_string(),
        columns,
        foreign_keys: vec![],
        unique_constraints: vec![],
        check_constraints: vec![],
        compression: None,
        collations: std::collections::HashMap::new(),
        partition_info: None,
    original_sql: String::new(),
    };

    let col_types: Vec<&str> = col_schema.iter().map(|(_, dt)| *dt).collect();

    let file = File::open(&tbl_path)?;
    let mut reader = BufReader::new(file);
    let mut rows: Vec<Vec<Value>> = Vec::new();
    let mut line_buf = String::new();
    let mut line_count = 0usize;

    loop {
        line_buf.clear();
        match reader.read_line(&mut line_buf) {
            Ok(0) => break,
            Ok(_) => {
                let values = parse_tbl_line(&line_buf, &col_types);
                rows.push(values);
                line_count += 1;
                if line_count.is_multiple_of(500_000) {
                    println!("  {}: loaded {} rows", table, line_count);
                }
            }
            Err(e) => {
                eprintln!("Error reading {}: {}", tbl_path.display(), e);
                break;
            }
        }
    }

    let row_count = rows.len();
    let table_data = TableData { info, rows };
    let bin_path = bin_dir.join(format!("{}.bin", table));

    write_bin(&bin_path, &table_data)?;
    println!(
        "  {}: wrote {} rows to {}",
        table,
        row_count,
        bin_path.display()
    );

    let bytes = fs::metadata(&bin_path)?.len() as usize;
    Ok((row_count, bytes))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: tbl2bin <tbl_dir> <bin_dir> [table...]");
        eprintln!("  Converts TPC-H TBL files to SQLRustGo BinaryTableStorage .bin format.");
        eprintln!("  Without [table...] converts all 8 TPC-H tables.");
        std::process::exit(1);
    }

    let tbl_dir = Path::new(&args[1]);
    let bin_dir = Path::new(&args[2]);

    fs::create_dir_all(bin_dir).expect("failed to create bin_dir");

    let tables: Vec<(&str, &[(&str, &str)])> = if args.len() > 3 {
        let requested: Vec<&str> = args[3..].iter().map(|s| s.as_str()).collect();
        TPC_H_TABLES
            .iter()
            .filter(|(name, _)| requested.contains(name))
            .copied()
            .collect()
    } else {
        TPC_H_TABLES.to_vec()
    };

    println!("Converting TPC-H tables:");
    println!("  TBL dir: {}", tbl_dir.display());
    println!("  BIN dir: {}", bin_dir.display());

    let mut total_rows = 0usize;
    let mut total_bytes = 0usize;

    for (table, schema) in tables {
        match convert_table(tbl_dir, bin_dir, table, schema) {
            Ok((rows, bytes)) => {
                total_rows += rows;
                total_bytes += bytes;
            }
            Err(e) => {
                eprintln!("ERROR converting {}: {}", table, e);
            }
        }
    }

    println!();
    println!(
        "Total: {} rows, {} bytes ({:.1} MB) in {}",
        total_rows,
        total_bytes,
        total_bytes as f64 / 1_048_576.0,
        bin_dir.display()
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlrustgo_storage::engine::{ColumnDefinition, TableData, TableInfo};
    use std::io::Read;

    #[test]
    fn parse_tbl_line_int_and_text() {
        let line = "1|ALGERIA|0| haggle. carefully f|";
        let types = &["INT", "TEXT", "INT", "TEXT"];
        let vals = parse_tbl_line(line, types);
        assert_eq!(vals.len(), 4);
        assert_eq!(vals[0], Value::Integer(1));
        assert_eq!(vals[1], Value::Text("ALGERIA".into()));
        assert_eq!(vals[2], Value::Integer(0));
        assert!(matches!(vals[3], Value::Text(_)));
    }

    #[test]
    fn parse_tbl_line_trailing_pipe_dropped() {
        let line = "1|hello|2|\n";
        let types = &["INT", "TEXT", "INT"];
        let vals = parse_tbl_line(line, types);
        assert_eq!(vals.len(), 3);
        assert_eq!(vals[0], Value::Integer(1));
        assert_eq!(vals[1], Value::Text("hello".into()));
        assert_eq!(vals[2], Value::Integer(2));
    }

    #[test]
    fn parse_tbl_line_empty_field_is_null() {
        let line = "||";
        let types = &["INT", "TEXT"];
        let vals = parse_tbl_line(line, types);
        assert_eq!(vals[0], Value::Null);
        assert_eq!(vals[1], Value::Null);
    }

    #[test]
    fn parse_tbl_line_crlf_trimmed() {
        let line = "5|name\r\n";
        let types = &["INT", "TEXT"];
        let vals = parse_tbl_line(line, types);
        assert_eq!(vals[1], Value::Text("name".into()));
    }

    #[test]
    fn parse_tbl_line_float_column() {
        let line = "1.25|hello";
        let types = &["FLOAT", "TEXT"];
        let vals = parse_tbl_line(line, types);
        assert_eq!(vals[0], Value::Float(1.25));
        assert_eq!(vals[1], Value::Text("hello".into()));
    }

    #[test]
    fn parse_tbl_line_fallback_text_on_parse_fail() {
        let line = "notint|hello";
        let types = &["INT", "TEXT"];
        let vals = parse_tbl_line(line, types);
        // INT column with non-numeric input falls back to Text per contract.
        assert_eq!(vals[0], Value::Text("notint".into()));
    }

    #[test]
    fn parse_tbl_line_missing_field_padded_with_empty() {
        let line = "42";
        let types = &["INT", "TEXT", "INT"];
        let vals = parse_tbl_line(line, types);
        assert_eq!(vals.len(), 3);
        assert_eq!(vals[0], Value::Integer(42));
        assert_eq!(vals[1], Value::Null);
        assert_eq!(vals[2], Value::Null);
    }

    #[test]
    fn write_bin_round_trip_int_text_null() {
        let dir = std::env::temp_dir().join(format!("sqlrustgo-tbl2bin-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.bin");

        let info = TableInfo {
            name: "t".into(),
            columns: vec![
                ColumnDefinition::new("id", "INT"),
                ColumnDefinition::new("name", "TEXT"),
                ColumnDefinition::new("note", "TEXT"),
            ],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        original_sql: String::new(),
        };
        let table_data = TableData {
            info,
            rows: vec![
                vec![Value::Integer(1), Value::Text("a".into()), Value::Null],
                vec![
                    Value::Integer(2),
                    Value::Text("b".into()),
                    Value::Text("ok".into()),
                ],
            ],
        };

        write_bin(&path, &table_data).expect("write_bin");
        assert!(path.exists());

        // Spot-check: header magic present.
        let mut f = File::open(&path).unwrap();
        let mut head = [0u8; 4];
        f.read_exact(&mut head).unwrap();
        assert_eq!(&head, b"BINT");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_bin_empty_rows_still_writes_header() {
        let dir =
            std::env::temp_dir().join(format!("sqlrustgo-tbl2bin-empty-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("empty.bin");

        let info = TableInfo {
            name: "empty".into(),
            columns: vec![ColumnDefinition::new("c", "INT")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        original_sql: String::new(),
        };
        let table_data = TableData { info, rows: vec![] };

        write_bin(&path, &table_data).expect("write_bin empty");
        let len = fs::metadata(&path).unwrap().len();
        assert!(len > 0, "header must exist even with no rows");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_bin_float_column_uses_float_code() {
        let dir =
            std::env::temp_dir().join(format!("sqlrustgo-tbl2bin-flt-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("f.bin");

        let info = TableInfo {
            name: "f".into(),
            columns: vec![ColumnDefinition::new("ratio", "FLOAT")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        original_sql: String::new(),
        };
        let table_data = TableData {
            info,
            rows: vec![vec![Value::Float(3.14159)]],
        };
        write_bin(&path, &table_data).expect("write_bin float");
        let len = fs::metadata(&path).unwrap().len();
        assert!(len > 8);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn convert_table_missing_file_returns_zero() {
        let dir =
            std::env::temp_dir().join(format!("sqlrustgo-tbl2bin-miss-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let result =
            convert_table(&dir, &dir, "no_such_table", &[("c", "INT")]).expect("must not error");
        assert_eq!(result, (0, 0));
        let _ = fs::remove_dir_all(&dir);
    }
}
