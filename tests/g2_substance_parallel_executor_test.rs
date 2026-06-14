mod common;
use common::tpch_wire_harness::start_sf01;
use std::time::Instant;

#[test]
fn test_concurrent_wire_clients_against_tpch() {
    let n_clients = 4;
    let n_queries = 5;

    let start = Instant::now();
    let mut handles = vec![];
    for _ in 0..n_clients {
        let handle = std::thread::spawn(move || {
            let mut client = start_sf01();
            let mut ok = 0;
            let mut err = 0;
            for _ in 0..n_queries {
                match client.query_rows("SELECT COUNT(*) FROM lineitem") {
                    Ok(_) => ok += 1,
                    Err(_) => err += 1,
                }
            }
            (ok, err)
        });
        handles.push(handle);
    }
    let mut total_ok = 0;
    let mut total_err = 0;
    for h in handles {
        let (ok, err) = h.join().unwrap();
        total_ok += ok;
        total_err += err;
    }
    let wall = start.elapsed();
    println!(
        "Concurrent: {n_clients} clients × {n_queries} queries = {total_ok} OK, {total_err} err in {wall:?}"
    );
    assert_eq!(total_err, 0, "concurrent client errors");
    assert_eq!(total_ok, n_clients * n_queries);
}
