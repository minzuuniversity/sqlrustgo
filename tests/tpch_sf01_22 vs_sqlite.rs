mod common;
use common::tpch_wire_harness::start_sf01;

#[test]
fn tpch_sf01_sqlite_baseline_queries_run() {
    let mut client = start_sf01();
    let q1 = "SELECT l_returnflag, COUNT(*) FROM lineitem GROUP BY l_returnflag";
    let rows = client.query_rows(q1).expect("Q1 must run");
    assert!(!rows.is_empty());

    let q6 = "SELECT COUNT(*) FROM lineitem WHERE l_quantity < 25";
    let rows = client.query_rows(q6).expect("Q6 must run");
    assert!(!rows.is_empty());
}
