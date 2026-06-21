//! Perf benchmark: 60000-row lineitem.tbl LOAD DATA via bulk_insert_records
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use sqlrustgo_storage::Record;
use sqlrustgo_types::Value as SqlValue;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Instant;

fn parse_tbl_line(line: &str, n: usize) -> Option<Record> {
    let s = line.trim_end_matches('\n').trim_end_matches('\r');
    let parts: Vec<&str> = s.split('|').collect();
    let parts: Vec<&str> = if parts.last() == Some(&"") {
        parts[..parts.len() - 1].to_vec()
    } else {
        parts
    };
    if parts.len() < n {
        return None;
    }
    let row: Vec<SqlValue> = parts[..n]
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
    Some(row)
}

#[test]
fn bench_bulk_insert_lineitem() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    engine.execute("CREATE TABLE lineitem (l_orderkey INTEGER, l_partkey INTEGER, l_suppkey INTEGER, l_linenumber INTEGER, l_quantity REAL, l_extendedprice REAL, l_discount REAL, l_tax REAL, l_returnflag TEXT, l_linestatus TEXT, l_shipdate TEXT, l_commitdate TEXT, l_receiptdate TEXT, l_shipinstruct TEXT, l_shipmode TEXT, l_comment TEXT)").unwrap();

    let path = resolve_lineitem_path();
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let n = 16;
    let mut records: Vec<Record> = Vec::with_capacity(content.lines().count());
    for line in content.lines() {
        if let Some(r) = parse_tbl_line(line, n) {
            records.push(r);
        }
    }
    println!("Parsed {} records from {}", records.len(), path.display());
    let expected = records.len() as u64;

    let start = Instant::now();
    let inserted = engine.bulk_insert_records("lineitem", records).unwrap();
    let dur = start.elapsed();
    println!("bulk_insert_records: inserted={} in {:?}", inserted, dur);
    assert_eq!(
        inserted, expected,
        "bulk_insert_records must insert all parsed records"
    );
}

fn resolve_lineitem_path() -> PathBuf {
    if let Ok(p) = std::env::var("LINEITEM_TBL_PATH") {
        return PathBuf::from(p);
    }
    for candidate in [
        "tests/data/lineitem.tbl",
        "data/lineitem.tbl",
        "tests/data/tpch-sf01/lineitem.tbl",
        "tests/data/tpch-sf001/lineitem.tbl",
    ] {
        let p = PathBuf::from(candidate);
        if p.exists() {
            return p;
        }
    }
    panic!(
        "lineitem.tbl not found: set LINEITEM_TBL_PATH or place file under tests/data/, data/, tests/data/tpch-sf01/, or tests/data/tpch-sf001/"
    );
}
