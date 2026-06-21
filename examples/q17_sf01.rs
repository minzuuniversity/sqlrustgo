// Q17 SF=0.1 in-process performance test
use std::sync::{Arc, RwLock};
use std::time::Instant;

use sqlrustgo::{ExecutionEngine, MemoryStorage};
use sqlrustgo_storage::Record;
use sqlrustgo_types::Value as SqlValue;

fn parse(line: &str, n: usize) -> Option<Record> {
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

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "tests/data/tpch-sf01".to_string());

    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);

    // Load all tables
    let ddls: &[(&str, &str, usize)] = &[
        ("region",   "CREATE TABLE region(r_regionkey INTEGER PRIMARY KEY, r_name TEXT NOT NULL, r_comment TEXT)", 3),
        ("nation",   "CREATE TABLE nation(n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, n_regionkey INTEGER NOT NULL, n_comment TEXT)", 4),
        ("supplier", "CREATE TABLE supplier(s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)", 7),
        ("customer", "CREATE TABLE customer(c_custkey INTEGER PRIMARY KEY, c_name TEXT NOT NULL, c_address TEXT NOT NULL, c_nationkey INTEGER NOT NULL, c_phone TEXT NOT NULL, c_acctbal REAL NOT NULL, c_mktsegment TEXT, c_comment TEXT)", 8),
        ("part",     "CREATE TABLE part(p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT)", 9),
        ("partsupp", "CREATE TABLE partsupp(ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, ps_comment TEXT)", 5),
        ("orders",   "CREATE TABLE orders(o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus TEXT NOT NULL, o_totalprice REAL NOT NULL, o_orderdate TEXT NOT NULL, o_orderpriority TEXT NOT NULL, o_clerk TEXT NOT NULL, o_shippriority INTEGER NOT NULL, o_comment TEXT)", 9),
        ("lineitem", "CREATE TABLE lineitem(l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity REAL NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT NOT NULL, l_linestatus TEXT NOT NULL, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT NOT NULL, l_shipmode TEXT NOT NULL, l_comment TEXT)", 16),
    ];
    for (name, ddl, ncols) in ddls {
        engine.execute(ddl).unwrap();
        let p = std::path::Path::new(&path).join(format!("{name}.tbl"));
        if !p.exists() {
            eprintln!("  skip {name}: {}", p.display());
            continue;
        }
        let content = std::fs::read_to_string(&p).expect("read");
        let mut recs: Vec<Record> = Vec::with_capacity(1024);
        for line in content.lines() {
            if let Some(r) = parse(line, *ncols) {
                recs.push(r);
            }
        }
        engine.bulk_insert_records(name, recs).unwrap();
        eprintln!("  {name} loaded");
    }

    let q17 = "SELECT SUM(l_extendedprice) / 7.0 AS avg_yearly \
               FROM lineitem, part \
               WHERE p_partkey = l_partkey \
                 AND p_brand = 'Brand#23' \
                 AND p_container = 'LG CASE' \
                 AND l_quantity < (SELECT 0.2 * AVG(l_quantity) FROM lineitem WHERE l_partkey = p_partkey)";

    let start = Instant::now();
    let result = engine.execute(q17);
    let elapsed = start.elapsed();
    match result {
        Ok(r) => println!("Q17 OK in {:?}, rows={}", elapsed, r.rows.len()),
        Err(e) => println!("Q17 ERR in {:?}: {}", elapsed, e),
    }
}
