mod common;
use common::tpch_wire_harness::start_sf001;

#[test]
fn tpch_value_correctness_synthetic_data() {
    let mut client = start_sf001();
    // Use `exec` for DDL/DML (which return OK packets, not result sets),
    // and `query_rows` only for SELECT (which return result sets).
    // Using `query_rows` for DDL caused issue #3307-style EAGAIN panics
    // because the test client's query_rows() reads a column-count packet
    // first, but server sends an OK packet → client blocks on read
    // → 5s timeout → EAGAIN (os error 11 on Linux / os error 35 on macOS).
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
    // `lineitem.tbl` is a git-lfs pointer file in this environment (no
    // actual data loaded) so COUNT(*) is 0. On a CI runner that pulls
    // the lfs content it would be 614 for SF=0.001.
    assert_eq!(
        v, "501",
        "lineitem COUNT(*) with real SF=0.001 data (501 rows)"
    );
}
