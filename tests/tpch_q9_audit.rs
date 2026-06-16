mod common;
use common::tpch_wire_harness::{start_sf01, read_baseline, compare_cells};
use std::path::Path;

#[ignore = "needs tests/data/tpch-sf01/baseline/Q09_three_way.json (not generated)"]
#[test]
fn tpch_q9_audit_wire() {
    let mut client = start_sf01();
    let q9 = "SELECT n_name, EXTRACT(YEAR FROM o_orderdate) AS o_year, \
              SUM(l_extendedprice * (1 - l_discount) - l_quantity * ps_supplycost) AS sum_profit \
              FROM part, supplier, lineitem, partsupp, orders, nation \
              WHERE s_suppkey = l_suppkey \
                AND ps_suppkey = l_suppkey \
                AND ps_partkey = l_partkey \
                AND p_partkey = l_partkey \
                AND o_orderkey = l_orderkey \
                AND s_nationkey = n_nationkey \
                AND p_name LIKE '%green%' \
              GROUP BY n_name, o_year \
              ORDER BY n_name, o_year DESC";
    let rows = client.query_rows(q9).expect("Q9 must run");
    assert!(!rows.is_empty(), "Q9 must return rows");
    let baseline = read_baseline(Path::new("tests/data/tpch-sf01/baseline/Q09_three_way.json"));
    let res = compare_cells(&rows, &baseline, 9.0);
    assert!(res.is_ok(), "Q9 cell compare: {res:?}");
}
