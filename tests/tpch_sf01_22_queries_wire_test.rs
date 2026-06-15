mod common;
use common::tpch_wire_harness::start_sf01;

const Q1_SF01: &str = "SELECT l_returnflag, l_linestatus, COUNT(*) \
                      FROM lineitem WHERE l_shipdate <= '1998-09-02' \
                      GROUP BY l_returnflag, l_linestatus \
                      ORDER BY l_returnflag, l_linestatus";

const Q6_SF01: &str = "SELECT COUNT(*) FROM lineitem \
                      WHERE l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01' \
                        AND l_quantity < 25";

#[ignore = "SF=0.1 LOAD DATA + queries — run with: cargo test --test <name> -- --ignored"]

#[test]
fn tpch_sf01_22_queries_wire_harness_q1_q6() {
    let mut client = start_sf01();
    let r1 = client.query_rows(Q1_SF01).expect("Q1");
    assert!(!r1.is_empty());
    let r6 = client.query_rows(Q6_SF01).expect("Q6");
    assert!(!r6.is_empty());
}
