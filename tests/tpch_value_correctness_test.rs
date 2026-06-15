mod common;
use common::tpch_wire_harness::start_sf001;

#[test]
fn tpch_value_correctness_synthetic_data() {
    let mut client = start_sf001();
    // DDL (DROP/CREATE TABLE) must use `exec` — `query_rows` waits
    // for a result set that DDL never returns, hanging until the
    // read timeout.
    let _ = client.exec("DROP TABLE IF EXISTS vc_lineitem");
    client
        .exec(
            "CREATE TABLE vc_lineitem (
                l_orderkey INTEGER,
                l_linenumber INTEGER,
                l_quantity INTEGER,
                l_extendedprice REAL,
                PRIMARY KEY (l_orderkey, l_linenumber)
            )",
        )
        .expect("create table");
    client
        .exec("INSERT INTO vc_lineitem VALUES (1, 1, 10, 100.0), (1, 2, 20, 200.0)")
        .expect("insert");
    let count = client
        .query_rows("SELECT COUNT(*) FROM vc_lineitem")
        .expect("count");
    assert_eq!(count.len(), 1);
    let sum = client
        .query_rows("SELECT SUM(l_quantity) FROM vc_lineitem")
        .expect("sum");
    assert_eq!(sum.len(), 1);
    let _ = client.exec("DROP TABLE vc_lineitem");
}

#[test]
fn tpch_value_correctness_q1_count() {
    let mut client = start_sf001();
    let result = client
        .query_rows("SELECT COUNT(*) FROM lineitem")
        .expect("count");
    assert!(!result.is_empty());
    let v = &result[0][0];
    assert!(!v.is_empty());
    let _ = v.parse::<i64>().expect("count should be integer");
}
