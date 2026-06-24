// Q17 SF=0.1 in-process performance test
use std::time::Instant;

fn main() {
    println!("Q17 SF=0.1 in-process perf test starting...");
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "tests/data/tpch-sf01".to_string());

    // Use the public EngineBuilder API (similar to other perf examples)
    use sqlrustgo::{executor::Value, EngineBuilder};
    let mut engine = EngineBuilder::new()
        .data_dir(format!("{path}_q17"))
        .build()
        .expect("build engine");

    // Load all tables
    for table in &[
        "region", "nation", "supplier", "customer", "part", "partsupp", "orders", "lineitem",
    ] {
        let p = format!("{path}/{table}.tbl");
        if std::path::Path::new(&p).exists() {
            let sql = format!(
                "COPY {} FROM '{}' WITH (FORMAT text, DELIMITER '|')",
                table, p
            );
            let _ = engine.execute(&sql);
        }
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
        Ok(rows) => println!("Q17 OK in {:?}, rows={}", elapsed, rows.len()),
        Err(e) => println!("Q17 ERR in {:?}: {}", elapsed, e),
    }
}
