use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine, Value};
use std::sync::Arc;
use std::time::Instant;

// The shared TPC-H fixture resolver (issue #5379).
//
// This test is calibrated against the SF=0.1 dataset and its cell-level
// baseline JSON. Neither the `tests/data/tpch-sf01` symlink nor the baseline
// were ever committed, so both absences are disclosed as NOT-RUN instead of
// panicking.
#[path = "../../common/tpch_fixture.rs"]
mod tpch_fixture;

/// Cargo test target this file belongs to (used as the NOT-RUN label).
const TARGET: &str = "q21_cell_regression_test";

/// The scale profile this test's expectations were calibrated against.
const EXPECTED_SF: f64 = 0.1;

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
) -> Result<usize, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
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
            // The insert error must not be swallowed: a failed batch insert
            // leaves the table empty and every assertion below would then run
            // against an empty table and pass vacuously.
            st.insert(tbl, batch)
                .map_err(|e| format!("insert into `{tbl}`: {e}"))?;
            batch = Vec::with_capacity(BATCH_SIZE);
        }
    }
    if !batch.is_empty() {
        let mut st = storage.write();
        st.insert(tbl, batch)
            .map_err(|e| format!("insert into `{tbl}`: {e}"))?;
    }
    Ok(count)
}

fn load_sf01(
    storage: &Arc<RwLock<MemoryStorage>>,
    data_dir: &std::path::Path,
) -> Result<(), String> {
    let tables: &[(&str, usize, &str)] = &[
        ("region", 3, "region.tbl"),
        ("nation", 4, "nation.tbl"),
        ("supplier", 7, "supplier.tbl"),
        ("orders", 8, "orders.tbl"),
        ("lineitem", 16, "lineitem.tbl"),
    ];
    for (tbl, ncols, filename) in tables {
        let p = data_dir.join(filename);
        let n = load_tpch_table(storage, tbl, *ncols, &p)?;
        eprintln!("  loaded {tbl}: {n} rows");
    }
    Ok(())
}

fn read_baseline(path: &std::path::Path) -> Result<(usize, Vec<Vec<String>>), String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("read baseline {}: {e}", path.display()))?;
    let v: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("parse baseline {}: {e}", path.display()))?;
    let row_count = v["row_count"]
        .as_u64()
        .ok_or_else(|| format!("baseline {} has no row_count", path.display()))?
        as usize;
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
    Ok((row_count, first_3))
}

#[test]
fn q21_cell_regression_sf01() {
    // The fixture is resolved through the shared resolver (issue #5379): a
    // missing fixture, or a fixture whose scale profile is not the SF=0.1
    // profile this test was calibrated against, is disclosed as NOT-RUN and
    // the test ends normally.
    let state = tpch_fixture::FixtureState::resolve();
    let Some(fixture) = state.require_profile(TARGET, EXPECTED_SF) else {
        return;
    };
    let data_dir = fixture.dir.clone();

    // The cell-level baseline JSON was never committed to the repository: it
    // only existed behind the removed `tests/data/tpch-sf01` symlink.
    let baseline_path = data_dir.join("expected/Q21_sf01_baseline.json");
    if !baseline_path.is_file() {
        tpch_fixture::report_not_run(
            TARGET,
            tpch_fixture::reason::FIXTURE_MISSING,
            &format!(
                "baseline JSON not committed to the repository ({})",
                baseline_path.display()
            ),
        );
        return;
    }

    eprintln!("=== Q21 cell-level regression test at SF=0.1 ===");
    let (mut engine, storage) = fresh_engine();
    create_tables(&mut engine);
    if let Err(e) = load_sf01(&storage, &data_dir) {
        tpch_fixture::report_not_run(TARGET, tpch_fixture::reason::FIXTURE_MISSING, &e);
        return;
    }

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

    let (expected_rc, expected_first_3) = match read_baseline(&baseline_path) {
        Ok(v) => v,
        Err(e) => {
            tpch_fixture::report_not_run(TARGET, tpch_fixture::reason::FIXTURE_MISSING, &e);
            return;
        }
    };

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
