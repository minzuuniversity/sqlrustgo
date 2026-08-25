//! Q17/Q20/Q22 SF=1 hang watchdog — captures DIAG state during wall-clock
//! hang to identify the precise bottleneck (vs running the test to
//! completion which times out and discards the state).
//!
//! Run: cargo test --release --test q17_sf1_watchdog --all-features -- --ignored --nocapture

use parking_lot::RwLock;
use sqlrustgo::{
    dump_v312_58_sprint3_diag, reset_v312_58_sprint3_diag, ExecutionEngine, MemoryStorage,
};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

const DATA_DIR: &str = "/tmp/tpch-sf1";

fn resolve_data_dir() -> String {
    std::env::var("TPCH_SF1_DIR").unwrap_or_else(|_| DATA_DIR.to_string())
}

fn setup() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    let data_dir = resolve_data_dir();

    engine.execute("CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT NOT NULL)").unwrap();
    engine.execute("CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity INTEGER NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT NOT NULL, l_linestatus TEXT NOT NULL, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT NOT NULL, l_shipmode TEXT NOT NULL, l_comment TEXT NOT NULL)").unwrap();

    eprintln!("=== Loading fixtures ===");
    {
        let mut st = storage.write();
        let t0 = Instant::now();
        st.bulk_load_tbl_file("part", &format!("{}/part.tbl", data_dir))
            .unwrap();
        eprintln!("part loaded: {:?}", t0.elapsed());
        let t0 = Instant::now();
        st.bulk_load_tbl_file("lineitem", &format!("{}/lineitem.tbl", data_dir))
            .unwrap();
        eprintln!("lineitem loaded: {:?}", t0.elapsed());
    }
    engine
}

#[test]
#[ignore]
fn q17_sf1_watchdog() {
    reset_v312_58_sprint3_diag();
    let mut engine = setup();

    const Q17_SQL: &str = "SELECT SUM(l_extendedprice) / 7.0 AS avg_yearly \
         FROM lineitem, part \
         WHERE p_partkey = l_partkey \
           AND p_brand = 'Brand#23' \
           AND p_container = 'LG CASE' \
           AND l_quantity < (SELECT 0.2 * AVG(l_quantity) \
                             FROM lineitem \
                             WHERE l_partkey = p_partkey)";

    let (tx, rx) = mpsc::channel();
    let start = Instant::now();
    // Thread is intentionally detached: if watchdog times out we don't wait.
    thread::spawn(move || {
        let result = engine.execute(Q17_SQL);
        let _ = tx.send(result);
    });

    // Watchdog: dump DIAG every 10s for up to 90s
    let mut last_dump = Instant::now();
    let mut dump_count = 0;
    loop {
        match rx.recv_timeout(Duration::from_secs(5)) {
            Ok(result) => {
                let elapsed = start.elapsed();
                eprintln!("=== Q17 COMPLETED in {:?} ===", elapsed);
                eprintln!("=== FINAL DIAG ===");
                eprintln!("{}", dump_v312_58_sprint3_diag());
                match result {
                    Ok(r) => eprintln!("rows={}, value={:?}", r.rows.len(), r.rows.first()),
                    Err(e) => eprintln!("error: {}", e),
                }
                return;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let elapsed = start.elapsed();
                if elapsed >= Duration::from_secs(90) {
                    eprintln!("=== Q17 WATCHDOG TIMEOUT at {:?} ===", elapsed);
                    eprintln!("=== LAST DIAG ===");
                    eprintln!("{}", dump_v312_58_sprint3_diag());
                    eprintln!("=== END WATCHDOG ===");
                    // Don't wait for thread; it'll keep running but test exits
                    return;
                }
                if last_dump.elapsed() >= Duration::from_secs(10) {
                    dump_count += 1;
                    eprintln!(
                        "=== Q17 WATCHDOG #{dump_count} at {elapsed:?} ===\n{}\n=== END SNAPSHOT ===",
                        dump_v312_58_sprint3_diag()
                    );
                    last_dump = Instant::now();
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                eprintln!(
                    "=== Q17 thread disconnected (panicked?) at {:?} ===",
                    start.elapsed()
                );
                eprintln!("{}", dump_v312_58_sprint3_diag());
                return;
            }
        }
    }
}
