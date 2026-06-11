//! Sprint 5 v6: minimal Q21 perf isolation test.
//! Loads SF=0.1 fixture, runs Q21, measures time + row count.
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use sqlrustgo_storage::Record;
use sqlrustgo_types::Value as SqlValue;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Instant;

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
    let data = PathBuf::from("tests/data/tpch-sf01");
    if !data.exists() {
        eprintln!("[SKIP] {} not found", data.display());
        return;
    }
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut e = ExecutionEngine::new(storage);

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
    let t0 = Instant::now();
    for (name, ddl, ncols) in ddls {
        e.execute(ddl).unwrap();
        let path = data.join(format!("{}.tbl", name));
        let content = std::fs::read_to_string(&path).expect("read");
        let mut recs: Vec<Record> = Vec::with_capacity(1024);
        for line in content.lines() {
            if let Some(r) = parse(line, *ncols) {
                recs.push(r);
            }
        }
        e.bulk_insert_records(name, recs).unwrap();
        eprintln!("  {} loaded", name);
    }
    eprintln!("Load: {:?}", t0.elapsed());

    // Q21 (Germany version of the SF=0.1 fixture).
    let q21 = "SELECT s_name, COUNT(*) AS numwait FROM supplier, lineitem l1, orders, nation WHERE s_suppkey = l1.l_suppkey AND o_orderkey = l1.l_orderkey AND o_orderstatus = 'F' AND s_nationkey = n_nationkey AND n_name = 'GERMANY' AND EXISTS (SELECT * FROM lineitem l2 WHERE l2.l_orderkey = l1.l_orderkey AND l2.l_suppkey <> l1.l_suppkey) AND NOT EXISTS (SELECT * FROM lineitem l3 WHERE l3.l_orderkey = l1.l_orderkey AND l3.l_suppkey <> l1.l_suppkey AND l3.l_receiptdate > l3.l_commitdate) GROUP BY s_name ORDER BY numwait DESC, s_name LIMIT 100";
    let t0 = Instant::now();
    let r = e.execute(q21).expect("q21");
    eprintln!("Q21: rc={} in {:?}", r.rows.len(), t0.elapsed());
    for row in &r.rows {
        eprintln!("  {:?}", row);
    }
}
