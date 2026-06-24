mod common;
use common::tpch_wire_harness::start_sf01;

#[test]
fn tpch_22_hash_smoke() {
    let mut client = start_sf01();
    let queries = [
        ("Q1", "SELECT COUNT(*) FROM lineitem"),
        ("Q6", "SELECT SUM(l_quantity) FROM lineitem"),
        ("Q14", "SELECT SUM(l_extendedprice * (1 - l_discount)) FROM lineitem"),
    ];
    for (name, q) in queries {
        let rows = client.query_rows(q).expect(&format!("{name} must run"));
        assert!(!rows.is_empty(), "{name} must return rows");
    }
}
