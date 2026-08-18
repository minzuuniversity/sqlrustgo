//! TPC-H SF=0.1 sqlrustgo Oracle Dump
//!
//! Runs all 22 TPC-H queries against the SF=0.1 fixture via the wire
//! protocol and dumps per-query TSVs + row counts to
//! `docs/releases/v3.12.0/evidence/tpch/cross_engine_sf01/sqlrustgo/`.
//!
//! The captured `.tsv` files can be diffed against the PostgreSQL
//! oracle at `cross_engine_sf01/postgres/` to validate sqlrustgo
//! semantics.
//!
//! Run:
//!   cargo test --test tpch_sf01_oracle_dump --all-features -- --nocapture

#[path = "../../common/mod.rs"]
mod common;

use common::tpch_wire_harness::{run_query_timed, start_sf01};
use std::fs;
use std::path::PathBuf;

const QUERIES: &[(&str, &str, u64)] = &[
    ("Q1",  include_str!("../../../queries/q1.sql"),  60),
    ("Q2",  include_str!("../../../queries/q2.sql"),  60),
    ("Q3",  include_str!("../../../queries/q3.sql"),  60),
    ("Q4",  include_str!("../../../queries/q4.sql"),  60),
    ("Q5",  include_str!("../../../queries/q5.sql"),  60),
    ("Q6",  include_str!("../../../queries/q6.sql"),  60),
    ("Q7",  include_str!("../../../queries/q7.sql"),  60),
    ("Q8",  include_str!("../../../queries/q8.sql"),  60),
    ("Q9",  include_str!("../../../queries/q9.sql"),  60),
    ("Q10", include_str!("../../../queries/q10.sql"), 60),
    ("Q11", include_str!("../../../queries/q11.sql"), 60),
    ("Q12", include_str!("../../../queries/q12.sql"), 60),
    ("Q13", include_str!("../../../queries/q13.sql"), 60),
    ("Q14", include_str!("../../../queries/q14.sql"), 60),
    ("Q15", include_str!("../../../queries/q15.sql"), 60),
    ("Q16", include_str!("../../../queries/q16.sql"), 60),
    ("Q17", include_str!("../../../queries/q17.sql"), 60),
    ("Q18", include_str!("../../../queries/q18.sql"), 60),
    ("Q19", include_str!("../../../queries/q19.sql"), 60),
    ("Q20", include_str!("../../../queries/q20.sql"), 60),
    ("Q21", include_str!("../../../queries/q21.sql"), 60),
    ("Q22", include_str!("../../../queries/q22.sql"), 60),
];

#[test]
fn dump_sqlrustgo_sf01_oracle_tsvs() {
    let out_dir = PathBuf::from(
        "docs/releases/v3.12.0/evidence/tpch/cross_engine_sf01/sqlrustgo",
    );
    fs::create_dir_all(&out_dir).expect("create out dir");

    let mut client = start_sf01();
    println!("=== sqlrustgo SF=0.1 Oracle Dump ===");
    let mut passed = 0;
    let mut errors = Vec::new();
    for (name, sql, timeout) in QUERIES {
        let sql = sql.trim();
        let (res, elapsed) = run_query_timed(&mut client, sql, *timeout);
        print!("{:>3}: ", name);
        match res {
            Ok(rows) => {
                let mut sorted = rows.clone();
                sorted.sort();
                let tsv: Vec<String> = sorted
                    .iter()
                    .map(|r| {
                        r.iter()
                            .map(|c| c.replace('\t', " ").replace('\n', " "))
                            .collect::<Vec<_>>()
                            .join("\t")
                    })
                    .collect();
                let tsv_text = tsv.join("\n") + "\n";
                fs::write(out_dir.join(format!("{}.tsv", name.to_lowercase())), tsv_text)
                    .expect("write tsv");
                println!(
                    "{} rows, {:.2}s",
                    rows.len(),
                    elapsed.as_secs_f64()
                );
                passed += 1;
            }
            Err(e) => {
                println!("ERROR: {} ({:.2}s)", e, elapsed.as_secs_f64());
                errors.push((name, e));
            }
        }
    }
    println!("\n[OK] {}/22 queries", passed);
    assert!(errors.is_empty(), "{} queries failed", errors.len());
}