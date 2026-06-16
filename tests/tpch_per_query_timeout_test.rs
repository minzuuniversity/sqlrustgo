mod common;
use common::tpch_wire_harness::{start_sf001, run_query_timed};
use std::time::Duration;

#[test]
fn tpch_per_query_timeout_q1() {
    let mut client = start_sf001();
    let q1 = "SELECT l_returnflag, l_linestatus, COUNT(*) \
              FROM lineitem \
              WHERE l_shipdate <= '1998-09-02' \
              GROUP BY l_returnflag, l_linestatus \
              ORDER BY l_returnflag, l_linestatus";
    let (result, elapsed) = run_query_timed(&mut client, q1, 30);
    assert!(result.is_ok(), "Q1 must complete: {result:?}");
    assert!(elapsed < Duration::from_secs(30), "Q1 too slow: {elapsed:?}");
}

#[test]
fn tpch_per_query_timeout_q6() {
    let mut client = start_sf001();
    let q6 = "SELECT COUNT(*) FROM lineitem \
              WHERE l_shipdate >= '1994-01-01' \
                AND l_shipdate < '1995-01-01' \
                AND l_quantity < 25";
    let (result, elapsed) = run_query_timed(&mut client, q6, 30);
    assert!(result.is_ok(), "Q6: {result:?}");
    assert!(elapsed < Duration::from_secs(30));
}
