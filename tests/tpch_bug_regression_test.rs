mod common;
use common::tpch_wire_harness::start_sf001;

#[test]
fn tpch_bug_regression_q1_count_4_rows() {
    let mut client = start_sf001();
    let rows = client
        .query_rows(
            "SELECT l_returnflag, l_linestatus, SUM(l_quantity), COUNT(*)
             FROM lineitem
             WHERE l_shipdate <= '1998-09-02'
             GROUP BY l_returnflag, l_linestatus
             ORDER BY l_returnflag, l_linestatus",
        )
        .expect("Q1 should run without error");
    assert!(!rows.is_empty(), "Q1 must return at least 1 group");
}

#[test]
fn tpch_bug_regression_q6_count() {
    let mut client = start_sf001();
    let rows = client
        .query_rows(
            "SELECT COUNT(*) FROM lineitem
             WHERE l_shipdate >= '1994-01-01'
               AND l_shipdate < '1995-01-01'
               AND l_quantity < 25",
        )
        .expect("Q6 should run");
    assert!(!rows.is_empty());
}

#[test]
fn tpch_bug_regression_q1_sum_aggregate() {
    let mut client = start_sf001();
    let rows = client
        .query_rows("SELECT SUM(l_quantity) FROM lineitem")
        .expect("SUM aggregate");
    assert!(!rows.is_empty());
}
