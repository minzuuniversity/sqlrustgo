//! BINT to TBL converter
//! Reads BINT files and writes TBL files for SF=1 fixture detection
use sqlrustgo_storage::binary_storage::BinaryTableStorage;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

fn format_value(v: &sqlrustgo_storage::Value) -> String {
    match v {
        sqlrustgo_storage::Value::Integer(i) => i.to_string(),
        sqlrustgo_storage::Value::Float(f) => f.to_string(),
        sqlrustgo_storage::Value::Text(s) => s.clone(),
        sqlrustgo_storage::Value::Null => String::new(),
        sqlrustgo_storage::Value::Boolean(b) => b.to_string(),
        sqlrustgo_storage::Value::Blob(b) => String::from_utf8_lossy(b).to_string(),
        sqlrustgo_storage::Value::Point(x, y) => format!("POINT({}, {})", x, y),
        sqlrustgo_storage::Value::Json(j) => j.to_string(),
    }
}

fn main() {
    let bint_dir = PathBuf::from("/tmp/tpch-sf1-bin");
    let tbl_dir = PathBuf::from("/tmp/sf1_tbl_real");
    let _ = std::fs::remove_dir_all(&tbl_dir);
    std::fs::create_dir_all(&tbl_dir).unwrap();

    let bint = BinaryTableStorage::new(bint_dir).unwrap();
    let tables = [
        "region", "nation", "supplier", "customer", "part", "partsupp", "orders", "lineitem",
    ];

    for table in tables.iter() {
        let data = bint.load(table).expect("load");
        let path = tbl_dir.join(format!("{}.tbl", table));
        let file = File::create(&path).unwrap();
        let mut w = BufWriter::new(file);
        for row in &data.rows {
            let line: Vec<String> = row.iter().map(format_value).collect();
            writeln!(w, "{}", line.join("|")).unwrap();
        }
        w.flush().unwrap();
        println!("Wrote {}.tbl: {} rows", table, data.rows.len());
    }
}
