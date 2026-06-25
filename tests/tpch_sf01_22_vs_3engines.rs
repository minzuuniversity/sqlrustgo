mod common;
use common::tpch_wire_harness::start_sf01;

#[test]
fn tpch_sf01_3engine_smoke() {
    let mut client = start_sf01();
    let queries = [
        "SELECT COUNT(*) FROM lineitem",
        "SELECT COUNT(*) FROM orders",
        "SELECT COUNT(*) FROM customer",
        "SELECT SUM(l_quantity) FROM lineitem",
    ];
    for (i, q) in queries.iter().enumerate() {
        let rows = client.query_rows(q).expect(&format!("Q{i} must run"));
        assert!(!rows.is_empty());
    }
}
