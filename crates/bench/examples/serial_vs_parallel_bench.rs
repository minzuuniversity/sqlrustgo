//! Serial vs Parallel Benchmark — Issue #3792
//!
//! Compares serial (degree=1) vs parallel (degree=N) execution for:
//! - TPC-H queries (OLAP)
//! - Microbenchmarks (OLTP)
//!
//! Outputs structured JSON + Markdown report.
//!
//! Usage:
//!   cargo run --example serial_vs_parallel_bench -- \
//!     --sf 0.1 --degrees 1,4 --runs 3
//!   QUICK=1 cargo run --example serial_vs_parallel_bench -- \
//!     --sf 0.1 --degrees 1,4 --runs 1

use clap::Parser;
use parking_lot::RwLock;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};
use std::fs;
use std::sync::Arc;
use std::time::Instant;

// ── TPC-H query SQL (abbreviated set for benchmark) ──────────────────────────

const TPC_H_QUERIES: &[(&str, &str)] = &[
    ("Q1",  "SELECT l_returnflag, l_linestatus, SUM(l_quantity) AS sum_qty, SUM(l_extendedprice) AS sum_base_price, SUM(l_extendedprice * (1 - l_discount)) AS sum_disc_price, SUM(l_extendedprice * (1 - l_discount) * (1 + l_tax)) AS sum_charge, AVG(l_quantity) AS avg_qty, AVG(l_extendedprice) AS avg_price, AVG(l_discount) AS avg_disc, COUNT(*) AS count_order FROM lineitem WHERE l_shipdate <= '1998-09-02' GROUP BY l_returnflag, l_linestatus ORDER BY l_returnflag, l_linestatus"),
    ("Q3",  "SELECT l_orderkey, SUM(l_extendedprice * (1 - l_discount)) AS revenue, o_orderdate, o_shippriority FROM customer, orders, lineitem WHERE c_custkey = o_custkey AND l_orderkey = o_orderkey AND c_mktsegment = 'BUILDING' AND o_orderdate < '1995-03-15' AND l_shipdate > '1995-03-15' GROUP BY l_orderkey, o_orderdate, o_shippriority ORDER BY revenue DESC, o_orderdate LIMIT 10"),
    ("Q4",  "SELECT o_orderpriority, COUNT(*) AS order_count FROM orders WHERE o_orderdate >= '1993-07-01' AND o_orderdate < '1993-10-01' AND EXISTS (SELECT * FROM lineitem WHERE l_orderkey = o_orderkey AND l_commitdate < l_receiptdate) GROUP BY o_orderpriority ORDER BY o_orderpriority"),
    ("Q5",  "SELECT n_name, SUM(l_extendedprice * (1 - l_discount)) AS revenue FROM customer, orders, lineitem, supplier, nation, region WHERE c_custkey = o_custkey AND l_orderkey = o_orderkey AND s_suppkey = l_suppkey AND c_nationkey = s_nationkey AND s_nationkey = n_nationkey AND n_regionkey = r_regionkey AND r_name = 'ASIA' AND o_orderdate >= '1994-01-01' AND o_orderdate < '1995-01-01' GROUP BY n_name ORDER BY revenue DESC"),
    ("Q6",  "SELECT SUM(l_extendedprice * l_discount) AS revenue FROM lineitem WHERE l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01' AND l_discount >= 0.05 AND l_discount <= 0.07 AND l_quantity < 24"),
    ("Q7",  "SELECT supp_nation, cust_nation, l_year, SUM(volume) AS revenue FROM (SELECT s_nationkey AS supp_nation, c_nationkey AS cust_nation, YEAR(l_shipdate) AS l_year, l_extendedprice * (1 - l_discount) AS volume FROM supplier, lineitem, orders, customer, nation n1, nation n2 WHERE s_suppkey = l_suppkey AND o_orderkey = l_orderkey AND c_custkey = o_custkey AND s_nationkey = n1.n_nationkey AND c_nationkey = n2.n_nationkey AND ((n1.n_name = 'FRANCE' AND n2.n_name = 'GERMANY') OR (n1.n_name = 'GERMANY' AND n2.n_name = 'FRANCE')) AND l_shipdate >= '1995-01-01' AND l_shipdate <= '1996-12-31') AS shipping GROUP BY supp_nation, cust_nation, l_year ORDER BY supp_nation, cust_nation, l_year"),
    ("Q10", "SELECT c_custkey, c_name, SUM(l_extendedprice * (1 - l_discount)) AS revenue, c_acctbal, n_name, c_address, c_phone, c_comment FROM customer, orders, lineitem, nation WHERE c_custkey = o_custkey AND l_orderkey = o_orderkey AND o_orderdate >= '1993-10-01' AND o_orderdate < '1994-01-01' AND l_returnflag = 'R' AND c_nationkey = n_nationkey GROUP BY c_custkey, c_name, c_acctbal, c_phone, n_name, c_address, c_comment ORDER BY revenue DESC LIMIT 20"),
    ("Q12", "SELECT l_shipmode, SUM(CASE WHEN o_orderpriority = '1-URGENT' OR o_orderpriority = '2-HIGH' THEN 1 ELSE 0 END) AS high_line_count, SUM(CASE WHEN o_orderpriority <> '1-URGENT' AND o_orderpriority <> '2-HIGH' THEN 1 ELSE 0 END) AS low_line_count FROM orders, lineitem WHERE l_orderkey = o_orderkey AND (l_shipmode = 'MAIL' OR l_shipmode = 'SHIP') AND l_commitdate < l_receiptdate AND l_shipdate < l_commitdate AND l_receiptdate >= '1994-01-01' AND l_receiptdate < '1995-01-01' GROUP BY l_shipmode ORDER BY l_shipmode"),
    ("Q14", "SELECT 100.00 * SUM(CASE WHEN p_type LIKE 'PROMO%' THEN l_extendedprice * (1 - l_discount) ELSE 0 END) / SUM(l_extendedprice * (1 - l_discount)) AS promo_revenue FROM lineitem, part WHERE l_partkey = p_partkey AND l_shipdate >= '1995-09-01' AND l_shipdate < '1995-10-01'"),
    ("Q17", "SELECT SUM(l_extendedprice) / 7.0 AS avg_yearly FROM lineitem, part WHERE p_partkey = l_partkey AND p_brand = 'Brand#23' AND p_container = 'MED JAR' AND l_quantity < (SELECT 0.2 * AVG(l_quantity) FROM lineitem WHERE l_partkey = p_partkey)"),
    ("Q18", "SELECT c_name, c_custkey, o_orderkey, o_orderdate, o_totalprice, SUM(l_quantity) AS col1 FROM customer, orders, lineitem WHERE o_orderkey IN (SELECT l_orderkey FROM lineitem GROUP BY l_orderkey HAVING SUM(l_quantity) > 300) AND c_custkey = o_custkey AND l_orderkey = o_orderkey GROUP BY c_name, c_custkey, o_orderkey, o_orderdate, o_totalprice ORDER BY o_totalprice DESC, o_orderdate LIMIT 100"),
    ("Q19", "SELECT SUM(l_extendedprice * (1 - l_discount)) AS revenue FROM lineitem, part WHERE p_partkey = l_partkey AND p_brand = 'Brand#12' AND p_container IN ('SM CASE', 'SM BOX', 'SM PACK', 'SM PKG') AND l_quantity >= 1 AND l_quantity <= 11 AND p_size BETWEEN 1 AND 5 AND l_shipmode IN ('AIR', 'AIR REG') AND l_shipinstruct = 'DELIVER IN PERSON' UNION ALL SELECT SUM(l_extendedprice * (1 - l_discount)) AS revenue FROM lineitem, part WHERE p_partkey = l_partkey AND p_brand = 'Brand#23' AND p_container IN ('MED BAG', 'MED BOX', 'MED PACK', 'MED PKG') AND l_quantity >= 10 AND l_quantity <= 20 AND p_size BETWEEN 1 AND 10 AND l_shipmode IN ('AIR', 'AIR REG') AND l_shipinstruct = 'DELIVER IN PERSON' UNION ALL SELECT SUM(l_extendedprice * (1 - l_discount)) AS revenue FROM lineitem, part WHERE p_partkey = l_partkey AND p_brand = 'Brand#34' AND p_container IN ('LG CASE', 'LG BOX', 'LG PACK', 'LG PKG') AND l_quantity >= 20 AND l_quantity <= 30 AND p_size BETWEEN 1 AND 15 AND l_shipmode IN ('AIR', 'AIR REG') AND l_shipinstruct = 'DELIVER IN PERSON'"),
    ("Q20", "SELECT s_name, s_address FROM supplier, nation WHERE s_suppkey IN (SELECT ps_suppkey FROM partsupp WHERE ps_partkey IN (SELECT p_partkey FROM part WHERE p_name LIKE 'forest%') AND ps_availqty > (SELECT 0.5 * SUM(l_quantity) FROM lineitem WHERE l_partkey = ps_partkey AND l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01')) AND s_nationkey = n_nationkey AND n_name = 'CANADA' ORDER BY s_name"),
    ("Q22", "SELECT cntrycode, COUNT(*) AS numcust, SUM(c_acctbal) AS totacctbal FROM (SELECT SUBSTRING(c_phone, 1, 2) AS cntrycode, c_acctbal FROM customer WHERE SUBSTRING(c_phone, 1, 2) IN ('13', '31', '23', '29', '30', '18', '17') AND c_acctbal > (SELECT AVG(c_acctbal) FROM customer WHERE c_acctbal > 0.00 AND SUBSTRING(c_phone, 1, 2) IN ('13', '31', '23', '29', '30', '18', '17')) AND NOT EXISTS (SELECT * FROM orders WHERE o_custkey = c_custkey)) AS custsale GROUP BY cntrycode ORDER BY cntrycode"),
];

// ── Microbenchmark SQL templates ─────────────────────────────────────────────

const MICRO_BENCHMARKS: &[(&str, &str)] = &[
    ("point_select_pk",   "SELECT * FROM lineitem WHERE l_orderkey = $1 AND l_linenumber = 1"),
    ("range_select",      "SELECT * FROM lineitem WHERE l_quantity > 10 AND l_quantity < 20"),
    ("aggregate_sum",     "SELECT SUM(l_extendedprice), AVG(l_discount) FROM lineitem"),
    ("filter_aggregate",  "SELECT l_returnflag, COUNT(*), SUM(l_quantity) FROM lineitem WHERE l_discount > 0.05 GROUP BY l_returnflag"),
    ("simple_join",       "SELECT COUNT(*) FROM orders o, lineitem l WHERE o.o_orderkey = l.l_orderkey"),
    ("order_limit",       "SELECT * FROM lineitem ORDER BY l_orderkey LIMIT 100"),
];

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(author, version, about = "Serial vs Parallel benchmark — Issue #3792", long_about = None)]
struct Args {
    /// TPC-H scale factor (0.1 = ~10K lineitem rows)
    #[arg(long, default_value_t = 1.0)]
    sf: f64,

    /// Comma-separated parallel degrees to test (e.g. "1,4,8")
    #[arg(long, default_value = "1,4")]
    degrees: String,

    /// Number of runs per query (median reported)
    #[arg(long, default_value_t = 3)]
    runs: u32,

    /// Data directory for TPC-H .tbl files (skipped if absent — uses synthetic)
    #[arg(long, default_value = "/tmp/svp_tpch_data")]
    data_dir: String,

    /// Quick mode: fewer queries, fewer runs
    #[arg(long, default_value = "false")]
    quick: bool,

    /// Output directory for JSON/MD reports
    #[arg(long, default_value = "/tmp/svp_reports")]
    output_dir: String,
}

// ── Report types ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PlatformInfo {
    hostname: String,
    cpu_cores: usize,
    ram_gb: f64,
    os: String,
    kernel: String,
    rust_version: String,
    git_commit: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    sf: f64,
    degrees: Vec<usize>,
    runs: u32,
    quick: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct OlapQueryResult {
    query: String,
    serial_ms: u128,
    parallel_ms: Vec<(usize, u128)>,
    speedup_4: Option<f64>,
    speedup_8: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OlapSummary {
    total_serial_ms: u128,
    total_parallel_4_ms: u128,
    speedup_4: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct OlapResults {
    queries: Vec<OlapQueryResult>,
    summary: OlapSummary,
}

#[derive(Debug, Serialize, Deserialize)]
struct OltpBenchmarkResult {
    name: String,
    serial_ops_sec: f64,
    parallel_ops_sec: Vec<(usize, f64)>,
    speedup_4: Option<f64>,
    triggered_parallel: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct OltpResults {
    benchmarks: Vec<OltpBenchmarkResult>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BenchmarkReport {
    platform: PlatformInfo,
    config: Config,
    olap: OlapResults,
    oltp: OltpResults,
    timestamp: String,
}

// ── Platform info ─────────────────────────────────────────────────────────────

fn get_platform_info() -> PlatformInfo {
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "unknown".to_string());

    let cpu_cores = std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(1);

    let ram_gb = sys_info::mem_info()
        .map(|m| m.total as f64 / 1_048_576.0)
        .unwrap_or(0.0);

    let os = std::env::consts::OS.to_string();
    let kernel = std::process::Command::new("uname")
        .arg("-r")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    let rust_version = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    let git_commit = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    PlatformInfo {
        hostname,
        cpu_cores,
        ram_gb,
        os,
        kernel,
        rust_version,
        git_commit: git_commit[..8.min(git_commit.len())].to_string(),
    }
}

// ── Schema ────────────────────────────────────────────────────────────────────

fn create_tables(engine: &mut ExecutionEngine<MemoryStorage>) {
    let schema = r#"
        CREATE TABLE region    (r_regionkey INTEGER, r_name TEXT, r_comment TEXT);
        CREATE TABLE nation    (n_nationkey INTEGER, n_name TEXT, n_regionkey INTEGER, n_comment TEXT);
        CREATE TABLE supplier  (s_suppkey INTEGER, s_name TEXT, s_address TEXT, s_nationkey INTEGER, s_phone TEXT, s_acctbal REAL, s_comment TEXT);
        CREATE TABLE customer  (c_custkey INTEGER, c_name TEXT, c_address TEXT, c_nationkey INTEGER, c_phone TEXT, c_acctbal REAL, c_mktsegment TEXT, c_comment TEXT);
        CREATE TABLE part      (p_partkey INTEGER, p_name TEXT, p_mfgr TEXT, p_brand TEXT, p_type TEXT, p_size INTEGER, p_container TEXT, p_retailprice REAL, p_comment TEXT);
        CREATE TABLE partsupp  (ps_partkey INTEGER, ps_suppkey INTEGER, ps_availqty INTEGER, ps_supplycost REAL, ps_comment TEXT);
        CREATE TABLE orders    (o_orderkey INTEGER, o_custkey INTEGER, o_orderstatus TEXT, o_totalprice REAL, o_orderdate TEXT, o_orderpriority TEXT, o_clerk TEXT, o_shippriority INTEGER, o_comment TEXT);
        CREATE TABLE lineitem  (l_orderkey INTEGER, l_partkey INTEGER, l_suppkey INTEGER, l_linenumber INTEGER, l_quantity REAL, l_extendedprice REAL, l_discount REAL, l_tax REAL, l_returnflag TEXT, l_linestatus TEXT, l_shipdate TEXT, l_commitdate TEXT, l_receiptdate TEXT, l_shipinstruct TEXT, l_shipmode TEXT, l_comment TEXT);
    "#;
    for stmt in schema.split(';').filter(|s| !s.trim().is_empty()) {
        let _ = engine.execute(stmt);
    }
}

// ── Load .tbl data ────────────────────────────────────────────────────────────


/// v3.10.0: Fast .tbl loader — bypasses SQL parser for bulk inserts.
/// Reads .tbl files and inserts directly via StorageEngine::insert().
/// ~100x faster than per-row INSERT execution.
fn fast_load_tbl_data(storage: &Arc<RwLock<MemoryStorage>>, data_dir: &str, _sf: f64) {
    use sqlrustgo::Value;
    let tables = [
        "region", "nation", "supplier", "customer", "part", "partsupp", "orders", "lineitem",
    ];
    for table in tables {
        let path = format!("{}/{}.tbl", data_dir, table);
        if !std::path::Path::new(&path).exists() {
            continue;
        }
        let Ok(content) = fs::read_to_string(&path) else { continue };
        let mut batch: Vec<Vec<Value>> = Vec::with_capacity(100_000);
        for line in content.lines() {
            if line.is_empty() {
                continue;
            }
            let cols: Vec<&str> = line.trim_end_matches('|').split('|').collect();
            let mut record = Vec::with_capacity(cols.len());
            for c in &cols {
                if let Ok(n) = c.parse::<i64>() {
                    record.push(Value::Integer(n));
                } else if let Ok(f) = c.parse::<f64>() {
                    record.push(Value::Float(f));
                } else {
                    record.push(Value::Text(c.to_string()));
                }
            }
            batch.push(record);
            if batch.len() >= 100_000 {
                let mut g = storage.write();
                let _ = g.insert(table, std::mem::take(&mut batch));
            }
        }
        if !batch.is_empty() {
            let mut g = storage.write();
            let _ = g.insert(table, batch);
        }
        let row_count = storage.read().scan(table).map(|v| v.len()).unwrap_or(0);
        eprintln!("  loaded {}.tbl: {} rows", table, row_count);
    }
}

fn load_tbl_data(engine: &mut ExecutionEngine<MemoryStorage>, data_dir: &str, sf: f64) {
    let tables = [
        "region", "nation", "supplier", "customer", "part", "partsupp", "orders", "lineitem",
    ];
    for table in tables {
        let path = format!("{}/{}.tbl", data_dir, table);
        if !std::path::Path::new(&path).exists() {
            continue;
        }
        if let Ok(content) = fs::read_to_string(&path) {
            let row_limit = if table == "lineitem" {
                (10_000.0_f64.min(1_000_000.0 / sf.max(1.0))) as usize
            } else {
                usize::MAX
            };
            for line in content.lines().take(row_limit) {
                if line.is_empty() {
                    continue;
                }
                let cols: Vec<&str> = line.trim_end_matches('|').split('|').collect();
                let vals: Vec<String> = cols
                    .iter()
                    .map(|c| {
                        if c.parse::<i64>().is_ok() || c.parse::<f64>().is_ok() {
                            c.to_string()
                        } else {
                            format!("'{}'", c.replace('\'', "''"))
                        }
                    })
                    .collect();
                let sql = format!("INSERT INTO {} VALUES ({})", table, vals.join(","));
                let _ = engine.execute(&sql);
            }
        }
    }
}

// ── Generate synthetic data ───────────────────────────────────────────────────

fn generate_synthetic_data(engine: &mut ExecutionEngine<MemoryStorage>, sf: f64) {
    // v3.10.0 Issue #3792: remove .min(1.0) cap — real TPC-H scaling.
    // SF=1.0 → 100K rows, SF=3.0 → 300K rows (capped at 500K to fit reasonable runtime).
    let mut rng = rand::thread_rng();

    let target_lineitem = ((100_000.0 * sf) as usize).min(500_000).max(1_000);
    for i in 0..target_lineitem {
        let qty = rng.gen_range(1.0..50.0);
        let price = rng.gen_range(100.0..50_000.0);
        let discount = rng.gen_range(0.0..0.15);
        let tax = rng.gen_range(0.0..0.10);
        let sql = format!(
            "INSERT INTO lineitem VALUES ({}, {}, {}, {}, {:.2}, {:.2}, {:.4}, {:.4}, 'R', 'O', '1998-01-01', '1998-01-01', '1998-01-01', 'NONE', 'SHIP', '')",
            (i % 1_000) as i64 + 1,
            (i % 2_000) as i64 + 1,
            (i % 100) as i64 + 1,
            (i % 5) as i64 + 1,
            qty, price, discount, tax
        );
        let _ = engine.execute(&sql);
    }

    // orders
    for i in 0..((1_000.0 * sf) as usize).min(5_000).max(100) {
        let _ = engine.execute(&format!(
            "INSERT INTO orders VALUES ({}, {}, 'O', 1000.0, '1998-01-01', '1-URGENT', 'Clerk#001', 0, '')",
            i as i64 + 1,
            (i % 100) as i64 + 1
        ));
    }

    // partsupp
    for i in 0..((500.0 * sf) as usize).min(2_500).max(50) {
        let _ = engine.execute(&format!(
            "INSERT INTO partsupp VALUES ({}, {}, {}, 10.0, '')",
            (i % 2_000) as i64 + 1,
            (i % 100) as i64 + 1,
            (i % 99) as i64 + 1
        ));
    }

    // part
    for i in 0..((200.0 * sf) as usize).min(1_000).max(20) {
        let _ = engine.execute(&format!(
            "INSERT INTO part VALUES ({}, 'part name', 'mfgr', 'Brand#{}', 'ECONOMY ANODIZED STEEL', {}, 'MED JAR', 100.0, '')",
            i as i64 + 1,
            (i % 5) as i64 + 1,
            (i % 50) as i64 + 1
        ));
    }

    // customer, supplier, nation, region
    for i in 0..((10.0 * sf) as usize).min(50).max(2) {
        let _ = engine.execute(&format!(
            "INSERT INTO customer VALUES ({}, 'customer{}', 'addr', 1, '13-111-111', 1000.0, 'BUILDING', '')",
            i as i64 + 1, i as i64 + 1
        ));
        let _ = engine.execute(&format!(
            "INSERT INTO supplier VALUES ({}, 'supplier{}', 'addr', 1, '13-111-111', 1000.0, '')",
            i as i64 + 1,
            i as i64 + 1
        ));
    }
    let _ = engine.execute("INSERT INTO nation VALUES (1, 'FRANCE', 1, ''), (2, 'GERMANY', 1, ''), (3, 'CANADA', 1, ''), (4, 'BRAZIL', 1, ''), (5, 'PERU', 1, ''), (6, 'INDIA', 1, '')");
    let _ = engine.execute("INSERT INTO region VALUES (1, 'EUROPE', ''), (2, 'AMERICA', '')");
}


// ── Run a single query, return (duration_ms, row_count, error) ───────────────

fn run_query(
    engine: &mut ExecutionEngine<MemoryStorage>,
    sql: &str,
) -> (u128, usize, Option<String>) {
    let start = Instant::now();
    match engine.execute(sql) {
        Ok(result) => (start.elapsed().as_millis(), result.rows.len(), None),
        Err(e) => (start.elapsed().as_millis(), 0, Some(e.to_string())),
    }
}

// ── Median of a slice ─────────────────────────────────────────────────────────

fn median(vals: &mut Vec<u128>) -> u128 {
    vals.sort();
    vals[vals.len() / 2]
}

// ── OLAP benchmark ────────────────────────────────────────────────────────────

fn run_olap_benchmark(sf: f64, degrees: &[usize], runs: u32, data_dir: &str) -> OlapResults {
    let queries: Vec<&(&str, &str)> = if std::env::var("QUICK").is_ok() {
        TPC_H_QUERIES.iter().take(5).collect()
    } else {
        TPC_H_QUERIES.iter().collect()
    };

    let mut query_results = Vec::new();
    let mut total_serial = 0u128;

    // v3.10.0: load data ONCE before the query loop (was loading per-query before)
    let storage: Arc<RwLock<MemoryStorage>> = Arc::new(RwLock::new(MemoryStorage::new()));
    {
        let mut engine = ExecutionEngine::new(storage.clone());
        create_tables(&mut engine);
        if std::path::Path::new(data_dir).exists() {
            drop(engine);
            fast_load_tbl_data(&storage, data_dir, sf);
        } else {
            generate_synthetic_data(&mut engine, sf);
        }
    }

    eprintln!("[OLAP] Running {} queries", queries.len());
    for (qname, sql) in queries {

        eprintln!("[OLAP] {} starting...", qname);
        // Serial (degree=1)
        let mut serial_times = Vec::new();
        for _ in 0..runs {
            let mut engine = ExecutionEngine::new(storage.clone());
            engine.set_parallel_degree(1);
            let (ms, _, _) = run_query(&mut engine, sql);
            serial_times.push(ms);
        }
        let serial_ms = median(&mut serial_times);
        total_serial += serial_ms;
        eprintln!("[OLAP] {} serial done in {}ms", qname, serial_ms);

        // Parallel runs
        let mut parallel_results = Vec::new();
        for &deg in degrees.iter().filter(|&&d| d > 1) {
            let mut times = Vec::new();
            for _ in 0..runs {
                let mut engine = ExecutionEngine::new(storage.clone());
                engine.set_parallel_degree(deg);
                let (ms, _, _) = run_query(&mut engine, sql);
                times.push(ms);
            }
            parallel_results.push((deg, median(&mut times)));
        }

        let speedup_4 = parallel_results
            .iter()
            .find(|(d, _)| *d == 4)
            .map(|(_, pm)| serial_ms as f64 / *pm as f64);
        let speedup_8 = parallel_results
            .iter()
            .find(|(d, _)| *d == 8)
            .map(|(_, pm)| serial_ms as f64 / *pm as f64);

        query_results.push(OlapQueryResult {
            query: qname.to_string(),
            serial_ms,
            parallel_ms: parallel_results,
            speedup_4,
            speedup_8,
        });
    }

    let total_parallel_4 = query_results
        .iter()
        .map(|r| {
            r.parallel_ms
                .iter()
                .find(|(d, _)| *d == 4)
                .map(|(_, m)| *m)
                .unwrap_or(r.serial_ms)
        })
        .sum::<u128>();

    OlapResults {
        queries: query_results,
        summary: OlapSummary {
            total_serial_ms: total_serial,
            total_parallel_4_ms: total_parallel_4,
            speedup_4: total_serial as f64 / total_parallel_4.max(1) as f64,
        },
    }
}

// ── OLTP benchmark ────────────────────────────────────────────────────────────

fn run_oltp_benchmark(sf: f64, degrees: &[usize]) -> OltpResults {
    let benchmarks: Vec<OltpBenchmarkResult> = MICRO_BENCHMARKS
        .iter()
        .map(|(name, sql_template)| {
            let storage: Arc<RwLock<MemoryStorage>> = Arc::new(RwLock::new(MemoryStorage::new()));
            {
                let mut engine = ExecutionEngine::new(storage.clone());
                create_tables(&mut engine);
                generate_synthetic_data(&mut engine, sf);
            }

            // Check row count to determine if parallel path triggers
            let row_count = {
                let mut engine = ExecutionEngine::new(storage.clone());
                engine
                    .execute("SELECT COUNT(*) FROM lineitem")
                    .ok()
                    .and_then(|r| r.rows.first()?.get(0)?.as_integer())
                    .map(|n| n as usize)
                    .unwrap_or(0)
            };
            let triggered_parallel = row_count >= 100_000;

            // Serial baseline: 200 iterations
            let mut engine = ExecutionEngine::new(storage.clone());
            let start = Instant::now();
            for _ in 0..200 {
                let sql = sql_template.replace("$1", "1");
                let _ = engine.execute(&sql);
            }
            let serial_ms = start.elapsed().as_millis();
            let serial_ops_sec = if serial_ms > 0 {
                200.0 * 1000.0 / serial_ms as f64
            } else {
                0.0
            };

            // Parallel runs
            let mut parallel_results = Vec::new();
            for &deg in degrees.iter().filter(|&&d| d > 1) {
                let mut engine = ExecutionEngine::new(storage.clone());
                engine.set_parallel_degree(deg);
                let start = Instant::now();
                for _ in 0..200 {
                    let sql = sql_template.replace("$1", "1");
                    let _ = engine.execute(&sql);
                }
                let ms = start.elapsed().as_millis();
                let ops_sec = if ms > 0 {
                    200.0 * 1000.0 / ms as f64
                } else {
                    0.0
                };
                parallel_results.push((deg, ops_sec));
            }

            let speedup_4 = parallel_results
                .iter()
                .find(|(d, _)| *d == 4)
                .map(|(_, ops)| ops / serial_ops_sec.max(1.0));

            OltpBenchmarkResult {
                name: name.to_string(),
                serial_ops_sec,
                parallel_ops_sec: parallel_results,
                speedup_4,
                triggered_parallel,
            }
        })
        .collect();

    OltpResults { benchmarks }
}

// ── Markdown report ───────────────────────────────────────────────────────────

fn generate_markdown_report(report: &BenchmarkReport) -> String {
    let mut md = String::new();
    md.push_str("# Serial vs Parallel Benchmark Report\n\n");
    md.push_str(&format!(
        "**Platform:** {} ({} cores, {:.1} GB RAM)\n\n",
        report.platform.hostname, report.platform.cpu_cores, report.platform.ram_gb
    ));
    md.push_str(&format!("**Rust:** {}\n\n", report.platform.rust_version));
    md.push_str(&format!(
        "**Config:** SF={}, degrees={:?}, {} runs{}\n\n",
        report.config.sf,
        report.config.degrees,
        report.config.runs,
        if report.config.quick {
            " (QUICK mode)"
        } else {
            ""
        }
    ));

    md.push_str("## OLAP: TPC-H Queries\n\n");
    md.push_str(
        "| Query | Serial (ms) | Parallel 4 (ms) | Speedup 4x | Parallel 8 (ms) | Speedup 8x |\n",
    );
    md.push_str(
        "|-------|-------------|----------------|-------------|----------------|-------------|\n",
    );
    for q in &report.olap.queries {
        let p4 = q.parallel_ms.iter().find(|(d, _)| *d == 4);
        let p8 = q.parallel_ms.iter().find(|(d, _)| *d == 8);
        let p4s = p4
            .map(|(_, m)| m.to_string())
            .unwrap_or_else(|| "N/A".to_string());
        let p8s = p8
            .map(|(_, m)| m.to_string())
            .unwrap_or_else(|| "N/A".to_string());
        let s4s = q
            .speedup_4
            .map(|s| format!("{:.2}x", s))
            .unwrap_or_else(|| "N/A".to_string());
        let s8s = q
            .speedup_8
            .map(|s| format!("{:.2}x", s))
            .unwrap_or_else(|| "N/A".to_string());
        md.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            q.query, q.serial_ms, p4s, s4s, p8s, s8s
        ));
    }
    md.push_str("\n**Summary:**\n\n");
    md.push_str(&format!(
        "- Total Serial: {} ms\n  - Total Parallel 4: {} ms ({:.2}x speedup)\n\n",
        report.olap.summary.total_serial_ms,
        report.olap.summary.total_parallel_4_ms,
        report.olap.summary.speedup_4
    ));

    md.push_str("## OLTP: Microbenchmarks\n\n");
    md.push_str(
        "| Benchmark | Serial (ops/s) | Parallel 4 (ops/s) | Speedup 4x | Triggered Parallel |\n",
    );
    md.push_str("|------------|----------------|----------------|---------|-------------------|\n");
    for b in &report.oltp.benchmarks {
        let p4 = b.parallel_ops_sec.iter().find(|(d, _)| *d == 4);
        let p4s = p4
            .map(|(_, o)| format!("{:.1}", o))
            .unwrap_or_else(|| "N/A".to_string());
        let s4s = b
            .speedup_4
            .map(|s| format!("{:.2}x", s))
            .unwrap_or_else(|| "N/A".to_string());
        let triggered = if b.triggered_parallel {
            "Yes"
        } else {
            "No (<100K rows)"
        };
        md.push_str(&format!(
            "| {} | {:.1} | {} | {} | {} |\n",
            b.name, b.serial_ops_sec, p4s, s4s, triggered
        ));
    }
    md.push_str("\n---\n*Generated by serial_vs_parallel_bench (Issue #3792)*\n");
    md
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    let args = Args::parse();

    println!("=== Serial vs Parallel Benchmark ===");
    println!(
        "SF={}, degrees={}, runs={}, quick={}",
        args.sf, args.degrees, args.runs, args.quick
    );

    let degrees: Vec<usize> = args
        .degrees
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    let platform = get_platform_info();
    println!(
        "Platform: {} ({} cores, {:.1} GB RAM)",
        platform.hostname, platform.cpu_cores, platform.ram_gb
    );

    fs::create_dir_all(&args.output_dir).ok();

    println!("\nRunning OLAP (TPC-H) benchmarks...");
    let olap = run_olap_benchmark(args.sf, &degrees, args.runs, &args.data_dir);

    println!("Running OLTP (microbenchmark) benchmarks...");
    let oltp = run_oltp_benchmark(args.sf, &degrees);

    let report = BenchmarkReport {
        platform: platform.clone(),
        config: Config {
            sf: args.sf,
            degrees: degrees.clone(),
            runs: args.runs,
            quick: args.quick,
        },
        olap,
        oltp,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    let json_path = format!("{}/svp_sf{}.json", args.output_dir, args.sf);
    let json = serde_json::to_string_pretty(&report).unwrap();
    fs::write(&json_path, &json).ok();
    println!("\nJSON: {}", json_path);

    let md_path = format!("{}/SERIAL_VS_PARALLEL_REPORT.md", args.output_dir);
    let md = generate_markdown_report(&report);
    fs::write(&md_path, &md).ok();
    println!("Markdown: {}", md_path);

    println!("\n=== Summary ===");
    println!(
        "OLAP total: {} ms (serial) vs {:.2}x (parallel 4)",
        report.olap.summary.total_serial_ms, report.olap.summary.speedup_4
    );
    if let Some(b) = report.oltp.benchmarks.first() {
        println!(
            "OLTP {}: {:.1} ops/s (serial) vs {:.2}x (parallel 4)",
            b.name,
            b.serial_ops_sec,
            b.speedup_4.unwrap_or(1.0)
        );
    }
}
