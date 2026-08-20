use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine, Value};
use std::sync::Arc;
use std::time::Instant;

type Engine = ExecutionEngine<MemoryStorage>;

const BATCH_SIZE: usize = 2000;

fn fresh_engine() -> (Engine, Arc<RwLock<MemoryStorage>>) {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let engine = ExecutionEngine::new(Arc::clone(&storage));
    (engine, storage)
}

fn create_tables(engine: &mut Engine) {
    engine
        .execute("CREATE TABLE region (r_regionkey INTEGER, r_name TEXT, r_comment TEXT)")
        .unwrap();
    engine.execute(
        "CREATE TABLE nation (n_nationkey INTEGER, n_regionkey INTEGER, n_name TEXT, n_comment TEXT)",
    ).unwrap();
    engine.execute(
        "CREATE TABLE supplier (s_suppkey INTEGER, s_nationkey INTEGER, s_name TEXT, s_address TEXT, s_phone TEXT, s_acctbal REAL, s_comment TEXT)",
    ).unwrap();
    engine.execute(
        "CREATE TABLE orders (o_orderkey INTEGER, o_custkey INTEGER, o_orderstatus TEXT, o_orderdate TEXT, o_orderpriority TEXT, o_clerk TEXT, o_shippriority INTEGER, o_comment TEXT)",
    ).unwrap();
    engine.execute(
        "CREATE TABLE lineitem (l_orderkey INTEGER, l_partkey INTEGER, l_suppkey INTEGER, l_linenumber INTEGER, l_quantity REAL, l_extendedprice REAL, l_discount REAL, l_tax REAL, l_returnflag TEXT, l_linestatus TEXT, l_shipdate TEXT, l_commitdate TEXT, l_receiptdate TEXT, l_shipinstruct TEXT, l_shipmode TEXT, l_comment TEXT)",
    ).unwrap();
}

fn parse_tpch_value(s: &str) -> Value {
    let s = s.trim();
    if s.is_empty() || s == "null" || s == "NULL" {
        return Value::Null;
    }
    if let Ok(n) = s.parse::<i64>() {
        return Value::Integer(n);
    }
    if let Ok(f) = s.parse::<f64>() {
        return Value::Float(f);
    }
    Value::Text(s.to_string())
}

fn load_tpch_table(
    storage: &Arc<RwLock<MemoryStorage>>,
    tbl: &str,
    ncols: usize,
    path: &std::path::Path,
) -> usize {
    let content =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let mut batch = Vec::with_capacity(BATCH_SIZE);
    let mut count = 0usize;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let raw: Vec<&str> = line.split('|').collect();
        let mut row = Vec::with_capacity(ncols);
        for i in 0..ncols {
            row.push(if i < raw.len() {
                parse_tpch_value(raw[i])
            } else {
                Value::Null
            });
        }
        batch.push(row);
        count += 1;
        if batch.len() >= BATCH_SIZE {
            let mut st = storage.write();
            let _ = st.insert(tbl, batch);
            batch = Vec::with_capacity(BATCH_SIZE);
        }
    }
    if !batch.is_empty() {
        let mut st = storage.write();
        let _ = st.insert(tbl, batch);
    }
    count
}

fn load_sf01(storage: &Arc<RwLock<MemoryStorage>>, data_dir: &std::path::Path) {
    let tables: &[(&str, usize, &str)] = &[
        ("region", 3, "region.tbl"),
        ("nation", 4, "nation.tbl"),
        ("supplier", 7, "supplier.tbl"),
        ("orders", 8, "orders.tbl"),
        ("lineitem", 16, "lineitem.tbl"),
    ];
    for (tbl, ncols, filename) in tables {
        let p = data_dir.join(filename);
        let n = load_tpch_table(storage, tbl, *ncols, &p);
        eprintln!("  loaded {tbl}: {n} rows");
    }
}

fn read_baseline(path: &std::path::Path) -> (usize, Vec<Vec<String>>) {
    let content = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("read baseline {}: {e}", path.display()));
    let v: serde_json::Value = serde_json::from_str(&content).unwrap();
    let row_count = v["row_count"].as_u64().unwrap() as usize;
    let mut first_3: Vec<Vec<String>> = Vec::new();
    if let Some(arr) = v["first_3_rows"].as_array() {
        for row in arr {
            let mut r = Vec::new();
            if let Some(cells) = row.as_array() {
                for cell in cells {
                    r.push(cell.as_str().unwrap_or("").to_string());
                }
            }
            first_3.push(r);
        }
    }
    (row_count, first_3)
}

#[test]
fn q21_cell_regression_sf01() {
    let data_dir = std::path::PathBuf::from("tests/data/tpch-sf01");
    if !data_dir.exists() {
        panic!("SF=0.1 fixture missing at {}", data_dir.display());
    }

    eprintln!("=== Q21 cell-level regression test at SF=0.1 ===");
    let (mut engine, storage) = fresh_engine();
    create_tables(&mut engine);
    load_sf01(&storage, &data_dir);

    let q21_sql = "SELECT s_name, COUNT(*) AS numwait \
        FROM supplier, lineitem l1, orders, nation \
        WHERE s_suppkey = l1.l_suppkey \
          AND o_orderkey = l1.l_orderkey \
          AND o_orderstatus = 'F' \
          AND s_nationkey = n_nationkey \
          AND n_name = 'GERMANY' \
          AND EXISTS (SELECT * FROM lineitem l2 \
                      WHERE l2.l_orderkey = l1.l_orderkey \
                        AND l2.l_suppkey <> l1.l_suppkey) \
          AND NOT EXISTS (SELECT * FROM lineitem l3 \
                          WHERE l3.l_orderkey = l1.l_orderkey \
                            AND l3.l_suppkey <> l1.l_suppkey \
                            AND l3.l_receiptdate > l3.l_commitdate) \
        GROUP BY s_name \
        ORDER BY numwait DESC, s_name \
        LIMIT 100";

    let start = Instant::now();
    let result = engine
        .execute(q21_sql)
        .unwrap_or_else(|e| panic!("Q21 failed: {e}"));
    let secs = start.elapsed().as_secs_f64();
    let rows = result.rows.len();

    eprintln!("Q21 completed in {:.3}s, returned {} rows", secs, rows);

    let baseline_path = data_dir.join("expected/Q21_sf01_baseline.json");
    let (expected_rc, expected_first_3) = read_baseline(&baseline_path);

    assert_eq!(
        rows, expected_rc,
        "row_count mismatch: engine={} baseline={}",
        rows, expected_rc
    );

    let actual_first_3: Vec<Vec<String>> = result
        .rows
        .iter()
        .take(3)
        .map(|r| {
            r.iter()
                .map(|v| match v {
                    Value::Text(s) => s.clone(),
                    Value::Integer(n) => n.to_string(),
                    Value::Float(f) => format!("{:.2}", f),
                    Value::Boolean(b) => b.to_string(),
                    Value::Blob(_) => "[blob]".to_string(),
                    Value::Point(x, y) => format!("POINT({:.2},{:.2})", x, y),
                    Value::Null => String::new(),
                    Value::Json(j) => format!("[json:{}]", j.to_string().len()),
                })
                .collect()
        })
        .collect();

    assert_eq!(
        actual_first_3.len(),
        expected_first_3.len(),
        "first-3-rows count mismatch: engine={}, baseline={}",
        actual_first_3.len(),
        expected_first_3.len()
    );

    for (i, (actual_row, expected_row)) in actual_first_3
        .iter()
        .zip(expected_first_3.iter())
        .enumerate()
    {
        assert_eq!(
            actual_row.len(),
            expected_row.len(),
            "row {} column count mismatch",
            i
        );
        for (j, (a, e)) in actual_row.iter().zip(expected_row.iter()).enumerate() {
            assert_eq!(
                a, e,
                "cell mismatch at row {} col {}: engine='{}' baseline='{}'",
                i, j, a, e
            );
        }
    }

    eprintln!("PASS: Q21 cell-level matches baseline ({} rows)", rows);
}
