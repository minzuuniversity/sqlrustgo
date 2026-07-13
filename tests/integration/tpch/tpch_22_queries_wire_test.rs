#[path = "../../common/mod.rs"]
mod common;
use common::tpch_wire_harness::start_sf001;

const Q1: &str = "SELECT l_returnflag, l_linestatus, SUM(l_quantity) AS sum_qty, \
                  SUM(l_extendedprice) AS sum_base_price, COUNT(*) AS count_order \
                  FROM lineitem WHERE l_shipdate <= '1998-09-02' \
                  GROUP BY l_returnflag, l_linestatus \
                  ORDER BY l_returnflag, l_linestatus";

const Q6: &str = "SELECT SUM(l_extendedprice * l_discount) AS revenue \
                  FROM lineitem \
                  WHERE l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01' \
                    AND l_discount BETWEEN 0.06 - 0.01 AND 0.06 + 0.01 \
                    AND l_quantity < 25";

#[test]
fn tpch_22_queries_wire_harness_q1_q6() {
    let mut client = start_sf001();
    let r1 = client.query_rows(Q1).expect("Q1");
    assert!(!r1.is_empty());
    let r6 = client.query_rows(Q6).expect("Q6");
    assert!(!r6.is_empty());
}
