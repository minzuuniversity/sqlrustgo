#[path = "../../common/mod.rs"]
mod common;
#[path = "../../common/tpch_fixture.rs"]
mod tpch_fixture;
use common::tpch_wire_harness::start_sf001;

#[test]
fn tpch_value_correctness_synthetic_data() {
    let Some(mut client) = start_sf001() else {
        return;
    };
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
    let Some(mut client) = start_sf001() else {
        return;
    };
    let result = client
        .query_rows("SELECT COUNT(*) FROM lineitem")
        .expect("count");
    assert!(!result.is_empty());
    let v = &result[0][0];
    // The expected count comes from the **fixture manifest**, not from a
    // number hard-coded in this test. The test used to assert "501", the
    // lineitem count of a dbgen SF=0.001 dataset that the repository never
    // carried; the fixture is now generated at run time, so a hard-coded
    // count is either wrong or right by accident. Asserting against the
    // manifest checks the property that matters — the engine sees every row
    // the fixture file holds — and fails loudly if the storage backend drops
    // rows (issue #5392).
    let state = tpch_fixture::FixtureState::resolve();
    let Some(fixture) = state.require("tpch_value_correctness_q1_count") else {
        return;
    };
    let expected = fixture
        .row_count("lineitem")
        .expect("the fixture manifest has no lineitem row count");
    assert_eq!(
        v,
        &expected.to_string(),
        "the engine sees {v} lineitem rows but the fixture file holds {expected}"
    );
}
