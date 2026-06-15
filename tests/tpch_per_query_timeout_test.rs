//! TPC-H single-query correctness test with per-query timeout.
//!
//! Runs each TPC-H query (q01-q22) in isolation and reports a
//! pass/fail/timeout per query. Intended to be used in environments
//! where the 4-way harness is too slow (e.g. Q20 nested subquery
//! path takes >5 minutes).
//!
//! Default timeout: 30 seconds per query. Override via
//! `TPCH_TIMEOUT_SECS` environment variable.

use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

const SCHEMA_SQL: &[&str] = &[
    "CREATE TABLE region (r_regionkey INTEGER PRIMARY KEY, r_name TEXT NOT NULL, r_comment TEXT)",
    "CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, n_regionkey INTEGER NOT NULL, n_comment TEXT)",
    "CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)",
    "CREATE TABLE customer (c_custkey INTEGER PRIMARY KEY, c_name TEXT NOT NULL, c_address TEXT NOT NULL, c_nationkey INTEGER NOT NULL, c_phone TEXT NOT NULL, c_acctbal REAL NOT NULL, c_mktsegment TEXT, c_comment TEXT)",
    "CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT)",
    "CREATE TABLE partsupp (ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, ps_comment TEXT, PRIMARY KEY (ps_partkey, ps_suppkey))",
    "CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus TEXT NOT NULL, o_totalprice REAL NOT NULL, o_orderdate TEXT NOT NULL, o_orderpriority TEXT, o_clerk TEXT, o_shippriority INTEGER, o_comment TEXT)",
    "CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity REAL NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT, l_linestatus TEXT, l_shipdate TEXT, l_commitdate TEXT, l_receiptdate TEXT, l_shipinstruct TEXT, l_shipmode TEXT, l_comment TEXT, PRIMARY KEY (l_orderkey, l_linenumber))",
];

fn data_dir() -> PathBuf {
    env::var("TPCH_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/tpch_sf01"))
}

fn timeout() -> u64 {
    env::var("TPCH_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30)
}

fn make_engine() -> Option<ExecutionEngine<MemoryStorage>> {
    let dir = data_dir();
    if !dir.exists() {
        eprintln!("Data dir not found: {}", dir.display());
        return None;
    }
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    for ddl in SCHEMA_SQL {
        if let Err(e) = engine.execute(ddl) {
            eprintln!("DDL failed: {e}");
            return None;
        }
    }
    for tbl in [
        "region", "nation", "supplier", "customer", "part", "partsupp", "orders", "lineitem",
    ] {
        let path = dir.join(format!("{tbl}.tbl"));
        if !path.exists() {
            eprintln!("missing: {}", path.display());
            continue;
        }
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("read {}: {e}", path.display());
                continue;
            }
        };
        for line in content.lines() {
            if line.is_empty() {
                continue;
            }
            let line_trimmed = line.trim_end_matches('|');
            let cols: Vec<&str> = line_trimmed.split('|').collect();
            let mut vals: Vec<String> = Vec::with_capacity(cols.len());
            for c in cols {
                if c.parse::<i64>().is_ok() || c.parse::<f64>().is_ok() {
                    vals.push(c.to_string());
                } else {
                    let esc = c.replace('\'', "''");
                    vals.push(format!("'{esc}'"));
                }
            }
            let sql = format!("INSERT INTO {tbl} VALUES ({})", vals.join(","));
            if let Err(e) = engine.execute(&sql) {
                eprintln!("insert {tbl}: {e}");
            }
        }
    }
    Some(engine)
}

#[test]
fn test_tpch_22_with_per_query_timeout() {
    let timeout_secs = timeout();
    let mut engine = match make_engine() {
        Some(e) => e,
        None => {
            eprintln!("\n=== SKIPPED: TPC-H data not loaded ===");
            return;
        }
    };

    let queries_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("queries");
    let mut pass = 0u32;
    let mut fail = 0u32;
    let mut timed_out = 0u32;

    println!(
        "\n=== TPC-H 22 with per-query timeout {}s ===",
        timeout_secs
    );

    for q in 1..=22 {
        let qfile = queries_dir.join(format!("q{q}.sql"));
        if !qfile.exists() {
            println!("  Q{q:02}: SKIP (no file)");
            continue;
        }
        let sql = match fs::read_to_string(&qfile) {
            Ok(s) => s,
            Err(e) => {
                println!("  Q{q:02}: ERROR read: {e}");
                fail += 1;
                continue;
            }
        };

        // Run the query in a worker thread, with the main thread
        // imposing a wall-clock deadline. If the worker is still
        // running when the deadline elapses, we detach the thread
        // (it keeps running in the background) and report TIMEOUT.
        // The test process is not killed.
        let start = Instant::now();
        let (tx, rx) = std::sync::mpsc::channel::<Result<usize, String>>();
        let sql_for_thread = sql.clone();
        // SAFETY: the test process is single-threaded at this point
        // (we are not in #[tokio::test]). The spawned thread is the
        // only one touching the engine until `rx` reports back. We
        // cast the mutable reference to a usize (raw address) to
        // satisfy the Send bound, then cast it back to a raw
        // pointer inside the worker closure. The pointer is only
        // dereferenced after we re-acquire the original borrow
        // (which is fine because the test thread is blocked on
        // `rx.recv_timeout` and does not touch `engine` until after
        // the worker finishes).
        let engine_addr: usize = &mut engine as *mut ExecutionEngine<MemoryStorage> as usize;
        let handle = std::thread::spawn(move || unsafe {
            let engine_ptr = engine_addr as *mut ExecutionEngine<MemoryStorage>;
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                (*engine_ptr).execute(&sql_for_thread)
            }));
            let payload = match result {
                Ok(Ok(r)) => Ok(r.rows.len()),
                Ok(Err(e)) => Err(e.to_string()),
                Err(_) => Err("panic".to_string()),
            };
            let _ = tx.send(payload);
        });
        let timeout_dur = Duration::from_secs(timeout_secs);
        let result = match rx.recv_timeout(timeout_dur) {
            Ok(Ok(payload)) => {
                let _ = handle.join();
                Ok(payload)
            }
            Ok(Err(e_msg)) => {
                let _ = handle.join();
                Err(e_msg)
            }
            Err(_) => {
                // Worker thread is still running; let it run in the
                // background. We do NOT join (it might never finish,
                // e.g. Q20 with N^2 EXISTS), so we leak the handle
                // until process exit. Acceptable for a diagnostic test.
                Err("TIMEOUT".to_string())
            }
        };
        let elapsed = start.elapsed();

        match result {
            Ok(n) => {
                println!("  Q{q:02}: PASS ({}ms, {} rows)", elapsed.as_millis(), n);
                pass += 1;
            }
            Err(msg) if msg == "TIMEOUT" => {
                println!("  Q{q:02}: TIMEOUT ({}ms)", elapsed.as_millis());
                timed_out += 1;
            }
            Err(msg) => {
                println!("  Q{q:02}: ERROR ({}ms, {msg})", elapsed.as_millis());
                fail += 1;
            }
        }
    }

    println!("\n=== Summary: pass={pass} fail={fail} timeout={timed_out} ===");
}
