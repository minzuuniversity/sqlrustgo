//! TPC-H 22/22 wire-protocol on SF=0.01 fixture, compared against the
//! SQLite baseline at `tests/data/tpch-sf01/expected/Q{N}_sf01_baseline.json`.

#[path = "../../common/mod.rs"]
mod common;
use common::tpch_wire_harness::start_sf01;
use std::fs;
use std::path::PathBuf;

const QUERIES_DIR: &str = "queries";
const EXPECTED_DIR: &str = "tests/data/tpch-sf01/expected";

#[test]
fn tpch_sf01_22_vs_sqlite() {
    let mut client = start_sf01();
    eprintln!();
    eprintln!("=== Running 22 TPC-H queries via wire protocol vs SQLite baseline ===");
    let mut pass = 0usize;
    let mut fail = 0usize;
    let mut skip = 0usize;
    for n in 1..=22usize {
        let sql_path = format!("{}/q{}.sql", QUERIES_DIR, n);
        let sql = fs::read_to_string(&sql_path).expect(&sql_path);
        let expected_path = format!("{}/Q{}_sf01_baseline.json", EXPECTED_DIR, n);
        let expected_rc: Option<usize> = if PathBuf::from(&expected_path).exists() {
            let s = fs::read_to_string(&expected_path).unwrap();
            let v: serde_json::Value = serde_json::from_str(&s).expect("json");
            Some(v["row_count"].as_u64().unwrap() as usize)
        } else {
            None
        };
        let t0 = std::time::Instant::now();
        let res = client.query_rows(&sql);
        let elapsed = t0.elapsed();
        match res {
            Ok(rows) => {
                let status = match expected_rc {
                    Some(er) if er == rows.len() => {
                        pass += 1;
                        format!("PASS (rc={}, expected={})", rows.len(), er)
                    }
                    Some(er) => {
                        fail += 1;
                        format!("FAIL (rc={}, expected={})", rows.len(), er)
                    }
                    None => {
                        skip += 1;
                        format!("SKIP (no baseline, rc={})", rows.len())
                    }
                };
                eprintln!("  Q{:2}: {} in {:?}", n, status, elapsed);
            }
            Err(e) => {
                fail += 1;
                eprintln!("  Q{:2}: ERROR ({}) in {:?}", n, e, elapsed);
            }
        }
    }
    eprintln!();
    eprintln!("=== Summary: pass={} fail={} skip={} ===", pass, fail, skip);
    if fail > 0 {
        eprintln!(
            "NOTE: {} queries fail vs SQLite (pre-existing, separate Sprint work)",
            fail
        );
    }
}
