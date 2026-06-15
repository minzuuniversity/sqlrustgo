mod common;
use common::tpch_wire_harness::{start_sf01, run_query_timed};
use std::time::Duration;

#[test]
fn tpch_sf01_perf_smoke_q1_q6() {
    let mut client = start_sf01();
    let q1 = "SELECT l_returnflag, l_linestatus, COUNT(*) FROM lineitem \
              WHERE l_shipdate <= '1998-09-02' \
              GROUP BY l_returnflag, l_linestatus \
              ORDER BY l_returnflag, l_linestatus";
    let (r1, t1) = run_query_timed(&mut client, q1, 60);
    assert!(r1.is_ok(), "Q1: {r1:?}");
    println!("Q1 wall: {t1:?}");

    let q6 = "SELECT COUNT(*) FROM lineitem \
              WHERE l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01' \
                AND l_quantity < 25";
    let (r6, t6) = run_query_timed(&mut client, q6, 60);
    assert!(r6.is_ok(), "Q6: {r6:?}");
    println!("Q6 wall: {t6:?}");

    assert!(t1 < Duration::from_secs(30), "Q1 too slow");
    assert!(t6 < Duration::from_secs(30), "Q6 too slow");
}
