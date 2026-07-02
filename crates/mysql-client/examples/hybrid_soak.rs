//! Hybrid OLTP + OLAP soak client.
//!
//! Connects to an existing sqlrustgo-mysql-server (default 127.0.0.1:13306)
//! and runs a mixed workload:
//!
//!   * OLTP: 7 op types — INSERT into orders / lineitem, UPDATE
//!     orders/customer/supplier/part/partsupp/nation, DELETE orders/lineitem
//!   * OLAP: 6 TPC-H Q1/Q4/Q6/Q15/Q20/Q22 (read-only aggregates)
//!
//! One persistent MySQL connection per worker thread. Connections are
//! NOT recycled mid-run — they live for the full duration so we
//! exercise server connection-management paths.
//!
//! Verifies the workload actually wrote by sampling `orders` and
//! `lineitem` row counts at start and end, and reports the delta.
//!
//! Run:
//!   cargo run --release -p sqlrustgo-mysql-client --example hybrid_soak -- \
//!
//! Env vars (CLI flags override):
//!   HYBRID_HOST, HYBRID_PORT, HYBRID_THREADS, HYBRID_DURATION,
//!   HYBRID_OLTP_RATIO (0.0..1.0, default 0.3), HYBRID_REPORT_INTERVAL

use rand::prelude::*;
use sqlrustgo_mysql_client::{MySqlConnection, ResultSet};
use std::env;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// OLAP: read-only TPC-H queries (no side effects)
// ---------------------------------------------------------------------------

const OLAP: &[(&str, &str)] = &[
    ("Q1",  "SELECT l_returnflag, l_linestatus, COUNT(*) AS cnt, \
             SUM(l_extendedprice) AS sum_base, SUM(l_discount) AS sum_disc \
             FROM lineitem WHERE l_shipdate <= '1998-12-01' \
             GROUP BY l_returnflag, l_linestatus ORDER BY l_returnflag"),
    ("Q4",  "SELECT o_orderpriority, COUNT(*) AS order_count \
             FROM orders WHERE o_orderdate >= '1993-07-01' \
               AND o_orderdate < '1993-10-01' \
             GROUP BY o_orderpriority ORDER BY o_orderpriority"),
    ("Q6",  "SELECT SUM(l_extendedprice * l_discount) AS revenue \
             FROM lineitem WHERE l_shipdate >= '1994-01-01' \
               AND l_shipdate < '1995-01-01' \
               AND l_quantity < 25 AND l_discount BETWEEN 0.05 AND 0.07"),
    ("Q15", "SELECT s_suppkey, s_name FROM supplier, lineitem \
             WHERE s_suppkey = l_suppkey \
               AND l_shipdate >= '1996-01-01' \
               AND l_shipdate < '1996-04-01' \
             GROUP BY s_suppkey, s_name LIMIT 5"),
    ("Q20", "SELECT s_name, s_address FROM supplier, nation \
             WHERE s_nationkey = n_nationkey AND n_name = 'GERMANY' \
             ORDER BY s_name LIMIT 5"),
    ("Q22", "SELECT SUBSTR(c_phone, 1, 2) AS cntrycode, COUNT(*) AS numcust \
             FROM customer WHERE c_acctbal > 0 \
             GROUP BY SUBSTR(c_phone, 1, 2) ORDER BY cntrycode"),
];

// ---------------------------------------------------------------------------
// OLTP: random CRUD generators
// ---------------------------------------------------------------------------

fn round2(v: f64) -> f64 { (v * 100.0).round() / 100.0 }

/// Orderkeys in range 20001..=100_000 are safe to INSERT (the live
/// server's fixture loaded 15000 orders with orderkeys 1..15000).
const SAFE_OK_RANGE: std::ops::RangeInclusive<u32> = 20001..=100_000;
const SAFE_LINENUM: std::ops::RangeInclusive<u32> = 1..=8;

fn oltp_insert_orders(rng: &mut ThreadRng) -> String {
    let ok = rng.gen_range(SAFE_OK_RANGE);
    let cust = rng.gen_range(1..=65);
    let status = ["O", "P", "F"][rng.gen_range(0..3)];
    let total = round2(rng.gen_range(100.0..100_000.0));
    let date = format!("2026-{:02}-{:02}", rng.gen_range(1..=12), rng.gen_range(1..=28));
    let priority = ["1-URGENT", "2-HIGH", "3-MEDIUM", "4-LOW", "5-NOT SPECIFIED"]
        [rng.gen_range(0..5)];
    let clerk = format!("Clerk#{:05}", rng.gen_range(0..1000));
    let shipp: i32 = rng.gen_range(0..=10);
    let comment = format!("hyb_{}", rng.gen::<u32>());
    format!(
        "INSERT INTO orders VALUES ({ok},{cust},'{status}',{total},'{date}','{priority}','{clerk}',{shipp},'{comment}')"
    )
}

fn oltp_insert_lineitem(rng: &mut ThreadRng) -> String {
    let ok = rng.gen_range(SAFE_OK_RANGE);
    let part = rng.gen_range(1..=2000);
    let supp = rng.gen_range(1..=100);
    let linenum = rng.gen_range(SAFE_LINENUM);
    let qty = round2(rng.gen_range(1.0..50.0));
    let price = round2(rng.gen_range(100.0..50_000.0));
    let disc = round2(rng.gen_range(0.0..0.15));
    let tax = round2(rng.gen_range(0.0..0.08));
    let ship = format!("2026-{:02}-{:02}", rng.gen_range(1..=12), rng.gen_range(1..=28));
    let commit = format!("2026-{:02}-{:02}", rng.gen_range(1..=12), rng.gen_range(1..=28));
    let receipt = format!("2026-{:02}-{:02}", rng.gen_range(1..=12), rng.gen_range(1..=28));
    let flag = ["N", "R"][rng.gen_range(0..2)];
    let stat = ["O", "F"][rng.gen_range(0..2)];
    let instr = format!("INST_{}", rng.gen::<u32>());
    let mode = ["REG AIR", "TRUCK", "SHIP", "RAIL", "MAIL"][rng.gen_range(0..5)];
    let comment = format!("li_{}", rng.gen::<u32>());
    format!(
        "INSERT INTO lineitem VALUES ({ok},{part},{supp},{linenum},{qty},{price},{disc},{tax},'{flag}','{stat}','{ship}','{commit}','{receipt}','{instr}','{mode}','{comment}')"
    )
}

fn oltp_update_orders(rng: &mut ThreadRng) -> String {
    let ok = rng.gen_range(1..=100_000u32);
    match rng.gen_range(0..3) {
        0 => format!("UPDATE orders SET o_orderstatus='F' WHERE o_orderkey={ok}"),
        1 => format!("UPDATE orders SET o_shippriority=9 WHERE o_orderkey={ok}"),
        _ => format!(
            "UPDATE orders SET o_comment='upd_{}' WHERE o_orderkey={ok}",
            rng.gen::<u32>()
        ),
    }
}

fn oltp_update_customer(rng: &mut ThreadRng) -> String {
    let cid = rng.gen_range(1..=65u32);
    let bal = round2(rng.gen_range(0.0..5000.0));
    format!("UPDATE customer SET c_acctbal={bal} WHERE c_custkey={cid}")
}

fn oltp_update_supplier(rng: &mut ThreadRng) -> String {
    let sid = rng.gen_range(1..=10u32);
    let bal = round2(rng.gen_range(1000.0..15000.0));
    format!("UPDATE supplier SET s_acctbal={bal} WHERE s_suppkey={sid}")
}

fn oltp_update_part(rng: &mut ThreadRng) -> String {
    let pid = rng.gen_range(1..=20u32);
    let price = round2(rng.gen_range(100.0..10000.0));
    format!("UPDATE part SET p_retailprice={price} WHERE p_partkey={pid}")
}

fn oltp_update_partsupp(rng: &mut ThreadRng) -> String {
    let part = rng.gen_range(1..=20u32);
    let supp = rng.gen_range(1..=10u32);
    let cost = round2(rng.gen_range(1.0..2000.0));
    format!(
        "UPDATE partsupp SET ps_supplycost={cost} WHERE ps_partkey={part} AND ps_suppkey={supp}"
    )
}

fn oltp_update_nation(rng: &mut ThreadRng) -> String {
    let nid = rng.gen_range(1..=25u32);
    let comment = format!("n_upd_{}", rng.gen::<u32>());
    format!("UPDATE nation SET n_comment='{comment}' WHERE n_nationkey={nid}")
}

fn oltp_delete_orders(rng: &mut ThreadRng) -> String {
    let ok = rng.gen_range(SAFE_OK_RANGE);
    format!("DELETE FROM orders WHERE o_orderkey={ok}")
}

fn oltp_delete_lineitem(rng: &mut ThreadRng) -> String {
    let ok = rng.gen_range(SAFE_OK_RANGE);
    format!("DELETE FROM lineitem WHERE l_orderkey={ok}")
}

fn oltp_select_orders(rng: &mut ThreadRng) -> String {
    let ok = rng.gen_range(1..=15_000u32);
    format!("SELECT * FROM orders WHERE o_orderkey={ok}")
}

fn oltp_select_customer(rng: &mut ThreadRng) -> String {
    let cid = rng.gen_range(1..=65u32);
    format!("SELECT * FROM customer WHERE c_custkey={cid}")
}

fn oltp_select_supplier(rng: &mut ThreadRng) -> String {
    let sid = rng.gen_range(1..=10u32);
    format!("SELECT * FROM supplier WHERE s_suppkey={sid}")
}

fn oltp_select_part(rng: &mut ThreadRng) -> String {
    let pid = rng.gen_range(1..=20u32);
    format!("SELECT * FROM part WHERE p_partkey={pid}")
}

fn oltp_select_nation(rng: &mut ThreadRng) -> String {
    let nid = rng.gen_range(1..=25u32);
    format!("SELECT * FROM nation WHERE n_nationkey={nid}")
}

fn oltp_select_region(_rng: &mut ThreadRng) -> String {
    "SELECT * FROM region ORDER BY r_regionkey".to_string()
}

fn oltp_count_orders(_rng: &mut ThreadRng) -> String {
    "SELECT COUNT(*) FROM orders".to_string()
}

fn oltp_count_lineitem(_rng: &mut ThreadRng) -> String {
    "SELECT COUNT(*) FROM lineitem".to_string()
}

// ---------------------------------------------------------------------------
// Counters
// ---------------------------------------------------------------------------

#[derive(Default, Debug)]
struct Counters {
    oltp_ok: AtomicU64,
    oltp_err: AtomicU64,
    olap_ok: AtomicU64,
    olap_err: AtomicU64,
    // Disambiguate INSERT vs UPDATE vs DELETE for write rate analysis
    oltp_insert_ok: AtomicU64,
    oltp_update_ok: AtomicU64,
    oltp_delete_ok: AtomicU64,
    oltp_select_ok: AtomicU64,
    oltp_insert_err: AtomicU64,
    oltp_update_err: AtomicU64,
    oltp_delete_err: AtomicU64,
    oltp_select_err: AtomicU64,
}

impl Counters {
    fn total(&self) -> u64 {
        self.oltp_ok.load(Ordering::Relaxed)
            + self.oltp_err.load(Ordering::Relaxed)
            + self.olap_ok.load(Ordering::Relaxed)
            + self.olap_err.load(Ordering::Relaxed)
    }
    fn snapshot(&self) -> (u64, u64, u64, u64) {
        (
            self.oltp_ok.load(Ordering::Relaxed),
            self.oltp_err.load(Ordering::Relaxed),
            self.olap_ok.load(Ordering::Relaxed),
            self.olap_err.load(Ordering::Relaxed),
        )
    }
}

// ---------------------------------------------------------------------------
// Worker
// ---------------------------------------------------------------------------

/// Returns (op_category, op_label) so caller can dispatch.
fn pick_oltp_op(rng: &mut ThreadRng) -> (&'static str, fn(&mut ThreadRng) -> String) {
    // 16 op variants; bias toward writes.
    match rng.gen_range(0u32..16) {
        0  => ("insert",  oltp_insert_orders  as fn(&mut ThreadRng) -> String),
        1  => ("insert",  oltp_insert_lineitem as fn(&mut ThreadRng) -> String),
        2  => ("update",  oltp_update_orders),
        3  => ("update",  oltp_update_customer),
        4  => ("update",  oltp_update_supplier),
        5  => ("update",  oltp_update_part),
        6  => ("update",  oltp_update_partsupp),
        7  => ("update",  oltp_update_nation),
        8  => ("delete",  oltp_delete_orders),
        9  => ("delete",  oltp_delete_lineitem),
        10 => ("select",  oltp_select_orders),
        11 => ("select",  oltp_select_customer),
        12 => ("select",  oltp_select_supplier),
        13 => ("select",  oltp_select_part),
        14 => ("select",  oltp_select_nation),
        _  => ("select",  oltp_select_region),
    }
}

fn worker(
    wid: u32,
    addr: SocketAddr,
    duration: Duration,
    oltp_ratio: f64,
    target_qps_per_thread: f64,
    counters: Arc<Counters>,
) {
    let mut conn = match MySqlConnection::connect(&addr, "root", "", "") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[worker {wid}] connect failed: {e}");
            return;
        }
    };
    // Use database so unqualified table refs work (the schema
    // autoloads region/nation/... but orders need a default db).
    let _ = conn.execute("USE default");

    let mut rng = rand::thread_rng();
    let deadline = Instant::now() + duration;
    let start = Instant::now();
    let mut next_tick = start + Duration::from_secs(1);
    let per_op_nanos = if target_qps_per_thread > 0.0 {
        (1_000_000_000.0 / target_qps_per_thread) as u64
    } else {
        0
    };
    let mut next_op_at = Instant::now();

    while Instant::now() < deadline {
        if per_op_nanos > 0 {
            let now = Instant::now();
            if now < next_op_at {
                let wait = next_op_at - now;
                if wait > Duration::from_millis(2) {
                    thread::sleep(wait);
                }
                continue;
            }
            next_op_at = now + Duration::from_nanos(per_op_nanos);
        }

        let is_oltp = rng.gen::<f64>() < oltp_ratio;
        if is_oltp {
            let (cat, gen) = pick_oltp_op(&mut rng);
            let sql = gen(&mut rng);
            match conn.execute(&sql) {
                Ok(_) => {
                    counters.oltp_ok.fetch_add(1, Ordering::Relaxed);
                    match cat {
                        "insert" => counters.oltp_insert_ok.fetch_add(1, Ordering::Relaxed),
                        "update" => counters.oltp_update_ok.fetch_add(1, Ordering::Relaxed),
                        "delete" => counters.oltp_delete_ok.fetch_add(1, Ordering::Relaxed),
                        _        => counters.oltp_select_ok.fetch_add(1, Ordering::Relaxed),
                    };
                }
                Err(e) => {
                    counters.oltp_err.fetch_add(1, Ordering::Relaxed);
                    match cat {
                        "insert" => counters.oltp_insert_err.fetch_add(1, Ordering::Relaxed),
                        "update" => counters.oltp_update_err.fetch_add(1, Ordering::Relaxed),
                        "delete" => counters.oltp_delete_err.fetch_add(1, Ordering::Relaxed),
                        _        => counters.oltp_select_err.fetch_add(1, Ordering::Relaxed),
                    };
                    let n = counters.oltp_err.load(Ordering::Relaxed);
                    if n <= 3 {
                        eprintln!("[worker {wid}] OLTP err: {e}  sql={sql}");
                    }
                }
            }
        } else {
            let (qname, qsql) = OLAP[rng.gen_range(0..OLAP.len())];
            match conn.execute(qsql) {
                Ok(_) => {
                    counters.olap_ok.fetch_add(1, Ordering::Relaxed);
                }
                Err(e) => {
                    counters.olap_err.fetch_add(1, Ordering::Relaxed);
                    let n = counters.olap_err.load(Ordering::Relaxed);
                    if n <= 3 {
                        eprintln!("[worker {wid}] OLAP {qname} err: {e}");
                    }
                }
            }
        }
    }
    let _ = conn.execute("COM_QUIT").ok();
}

// ---------------------------------------------------------------------------
// Row-count baseline (proves writes happened)
// ---------------------------------------------------------------------------

fn count_table(conn: &mut MySqlConnection, tbl: &str) -> u64 {
    let sql = format!("SELECT COUNT(*) FROM {tbl}");
    match conn.execute(&sql) {
        Ok(ResultSet::Select { rows, .. }) if !rows.is_empty() => {
            rows[0][0].parse().unwrap_or(0)
        }
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// Args
// ---------------------------------------------------------------------------

struct Args {
    host: String,
    port: u16,
    threads: usize,
    duration_secs: u64,
    oltp_ratio: f64,
    target_qps: f64,
    report_interval: u64,
}

fn env_u64(name: &str, default: u64) -> u64 {
    env::var(name).ok().and_then(|s| s.parse().ok()).unwrap_or(default)
}
fn env_usize(name: &str, default: usize) -> usize {
    env::var(name).ok().and_then(|s| s.parse().ok()).unwrap_or(default)
}
fn env_f64(name: &str, default: f64) -> f64 {
    env::var(name).ok().and_then(|s| s.parse().ok()).unwrap_or(default)
}
fn env_string(name: &str, default: &str) -> String {
    env::var(name).unwrap_or_else(|_| default.to_string())
}

fn parse_args() -> Args {
    let mut host = env_string("HYBRID_HOST", "127.0.0.1");
    let mut port: u16 = env_u64("HYBRID_PORT", 13306) as u16;
    let mut threads = env_usize("HYBRID_THREADS", 4);
    let mut duration_secs = env_u64("HYBRID_DURATION", 60);
    let mut oltp_ratio = env_f64("HYBRID_OLTP_RATIO", 0.32);
    let mut target_qps = env_f64("HYBRID_TARGET_QPS", 20.0);
    let mut report_interval = env_u64("HYBRID_REPORT_INTERVAL", 5);

    let mut args = env::args().skip(1);
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--host"    => host = args.next().unwrap_or(host),
            "--port"    => port = args.next().and_then(|s| s.parse().ok()).unwrap_or(port),
            "--threads" | "-t" => threads = args.next().and_then(|s| s.parse().ok()).unwrap_or(threads),
            "--duration" | "-d" => duration_secs = args.next().and_then(|s| s.parse().ok()).unwrap_or(duration_secs),
            "--oltp-ratio" => oltp_ratio = args.next().and_then(|s| s.parse().ok()).unwrap_or(oltp_ratio),
            "--target-qps" => target_qps = args.next().and_then(|s| s.parse().ok()).unwrap_or(target_qps),
            "--report-interval" => report_interval = args.next().and_then(|s| s.parse().ok()).unwrap_or(report_interval),
            "--help" | "-h" => {
                println!("hybrid_soak — OLTP+OLAP workload against an existing server");
                println!("  --host HOST              default: 127.0.0.1");
                println!("  --port PORT              default: 13306");
                println!("  --threads N              default: 4");
                println!("  --oltp-ratio R           0.0..1.0   default: 0.32 (mix ~20%)");

                println!("  --target-qps QPS         per-thread target, default: 20");
                println!("  --report-interval SECS   default: 5");
                std::process::exit(0);
            }
            other => eprintln!("[warn] unknown flag: {other}"),
        }
    }
    Args { host, port, threads, duration_secs, oltp_ratio, target_qps, report_interval }
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    let args = parse_args();
    let addr: SocketAddr = format!("{}:{}", args.host, args.port)
        .parse()
        .expect("parse host:port");

    eprintln!("=== hybrid_soak ===");
    eprintln!("  target     : {addr}");
    eprintln!("  threads    : {}", args.threads);
    eprintln!("  duration   : {}s", args.duration_secs);
    eprintln!("  oltp_ratio : {}", args.oltp_ratio);
    eprintln!("  target_qps : {} per thread ({} total)", args.target_qps, args.target_qps * args.threads as f64);
    eprintln!("  report_int : {}s", args.report_interval);

    // Baseline row counts (on a fresh connection)
    let mut baseline_conn = MySqlConnection::connect(&addr, "root", "", "")
        .expect("baseline connect");
    let _ = baseline_conn.execute("USE default");
    let baseline_orders    = count_table(&mut baseline_conn, "orders");
    let baseline_lineitem  = count_table(&mut baseline_conn, "lineitem");
    let baseline_customer  = count_table(&mut baseline_conn, "customer");
    let baseline_supplier  = count_table(&mut baseline_conn, "supplier");
    let baseline_part      = count_table(&mut baseline_conn, "part");
    let baseline_nation    = count_table(&mut baseline_conn, "nation");
    eprintln!("  baseline   : orders={} lineitem={} customer={} supplier={} part={} nation={}",
        baseline_orders, baseline_lineitem, baseline_customer, baseline_supplier, baseline_part, baseline_nation);

    // Spawn workers
    let duration = Duration::from_secs(args.duration_secs);
    let counters = Arc::new(Counters::default());
    let report_interval = Duration::from_secs(args.report_interval);
    let qps_per_thread = args.target_qps / args.threads as f64;

    let start = Instant::now();
    let handles: Vec<_> = (0..args.threads)
        .map(|wid| {
            let ctr = Arc::clone(&counters);
            let addr = addr;
            thread::spawn(move || worker(wid as u32, addr, duration, args.oltp_ratio, qps_per_thread, ctr))
        })
        .collect();

    // Progress reporter
    let report_handle = {
        let ctr = Arc::clone(&counters);
        thread::spawn(move || {
            let mut last_total = 0u64;
            let mut last_oltp_ok = 0u64;
            let mut last_olap_ok = 0u64;
            let mut last_oltp_err = 0u64;
            let mut last_olap_err = 0u64;
            let mut last_ins = 0u64;
            let mut last_upd = 0u64;
            let mut last_del = 0u64;
            let mut last_sel = 0u64;
            let mut next = Instant::now() + report_interval;
            let rep_secs = report_interval.as_secs_f64();
            loop {
                if Instant::now() >= start + duration + Duration::from_secs(2) {
                    break;
                }
                let now = Instant::now();
                if now < next {
                    thread::sleep(next - now);
                }
                next += report_interval;
                let (oltp_ok, oltp_err, olap_ok, olap_err) = ctr.snapshot();
                let ins = ctr.oltp_insert_ok.load(Ordering::Relaxed);
                let upd = ctr.oltp_update_ok.load(Ordering::Relaxed);
                let del = ctr.oltp_delete_ok.load(Ordering::Relaxed);
                let sel = ctr.oltp_select_ok.load(Ordering::Relaxed);
                let total = ctr.total();
                let dt = total - last_total;
                let qps = dt as f64 / rep_secs;
                let oi = ins - last_ins;
                let ou = upd - last_upd;
                let od = del - last_del;
                let os = sel - last_sel;
                let writes = oi + ou + od;
                eprintln!(
                    "[{:>4.0}s] qps={:>6.0}  oltp={}/{}  olap={}/{}  tot_ops={}  writes(+{}/+{}/+{})  select(+{})",
                    start.elapsed().as_secs_f64(),
                    qps,
                    oltp_ok, oltp_err,
                    olap_ok, olap_err,
                    total,
                    oi, ou, od, os,
                );
                last_oltp_ok = oltp_ok;
                last_olap_ok = olap_ok;
                last_oltp_err = oltp_err;
                last_olap_err = olap_err;
                last_ins = ins;
                last_upd = upd;
                last_del = del;
                last_sel = sel;
            }
        })
    };

    for h in handles { let _ = h.join(); }
    let _ = report_handle.join();
    let wall = start.elapsed();

    // Final row counts
    let final_orders    = count_table(&mut baseline_conn, "orders");
    let final_lineitem  = count_table(&mut baseline_conn, "lineitem");
    let final_customer  = count_table(&mut baseline_conn, "customer");
    let final_supplier  = count_table(&mut baseline_conn, "supplier");
    let final_part      = count_table(&mut baseline_conn, "part");
    let final_nation    = count_table(&mut baseline_conn, "nation");

    let (oltp_ok, oltp_err, olap_ok, olap_err) = counters.snapshot();
    let ins_ok  = counters.oltp_insert_ok.load(Ordering::Relaxed);
    let upd_ok  = counters.oltp_update_ok.load(Ordering::Relaxed);
    let del_ok  = counters.oltp_delete_ok.load(Ordering::Relaxed);
    let sel_ok  = counters.oltp_select_ok.load(Ordering::Relaxed);
    let ins_err = counters.oltp_insert_err.load(Ordering::Relaxed);
    let upd_err = counters.oltp_update_err.load(Ordering::Relaxed);
    let del_err = counters.oltp_delete_err.load(Ordering::Relaxed);
    let sel_err = counters.oltp_select_err.load(Ordering::Relaxed);

    let total = oltp_ok + oltp_err + olap_ok + olap_err;
    let qps = total as f64 / wall.as_secs_f64();

    eprintln!();
    eprintln!("═══════════════════════════════════════════════════════════════");
    eprintln!("  FINAL: hybrid_soak done in {:.1}s", wall.as_secs_f64());
    eprintln!("  QPS (total)             : {:.1}", qps);
    eprintln!("  OLTP  ok/err            : {} / {}", oltp_ok, oltp_err);
    eprintln!("    insert ok/err         : {} / {}", ins_ok, ins_err);
    eprintln!("    update ok/err         : {} / {}", upd_ok, upd_err);
    eprintln!("    delete ok/err         : {} / {}", del_ok, del_err);
    eprintln!("    select ok/err         : {} / {}", sel_ok, sel_err);
    eprintln!("  OLAP  ok/err            : {} / {}", olap_ok, olap_err);
    eprintln!();
    eprintln!("  Row count deltas (proves writes happened):");
    eprintln!("    orders   : {:>6} -> {:>6}  (Δ {:+})", baseline_orders,    final_orders,    final_orders    as i64 - baseline_orders    as i64);
    eprintln!("    lineitem : {:>6} -> {:>6}  (Δ {:+})", baseline_lineitem,  final_lineitem,  final_lineitem  as i64 - baseline_lineitem  as i64);
    eprintln!("    customer : {:>6} -> {:>6}  (Δ {:+})", baseline_customer,  final_customer,  final_customer  as i64 - baseline_customer  as i64);
    eprintln!("    supplier : {:>6} -> {:>6}  (Δ {:+})", baseline_supplier,  final_supplier,  final_supplier  as i64 - baseline_supplier  as i64);
    eprintln!("    part     : {:>6} -> {:>6}  (Δ {:+})", baseline_part,      final_part,      final_part      as i64 - baseline_part      as i64);
    eprintln!("    nation   : {:>6} -> {:>6}  (Δ {:+})", baseline_nation,    final_nation,    final_nation    as i64 - baseline_nation    as i64);
    eprintln!("═══════════════════════════════════════════════════════════════");
    let _ = baseline_conn.execute("COM_QUIT");
}
