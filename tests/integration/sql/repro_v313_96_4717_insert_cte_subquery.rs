// V313-96 / Issue #4717: `INSERT INTO ... SELECT ... FROM (WITH RECURSIVE ...)`.

use sqlrustgo_parser::parse;

fn assert_parses(sql: &str) {
    let stmt = parse(sql).unwrap_or_else(|e| panic!("Expected parse OK for {sql:?}, got {e:?}"));
    let _ = stmt;
}

#[test]
fn insert_select_from_with_recursive_parses() {
    assert_parses(
        "INSERT INTO t \
         SELECT x, x * 10 \
         FROM (WITH RECURSIVE s(x) AS (VALUES (1) UNION ALL SELECT x + 1 FROM s WHERE x < 3) \
         SELECT * FROM s) AS sub",
    );
}

#[test]
fn insert_select_from_with_non_recursive_parses() {
    assert_parses(
        "INSERT INTO t \
         SELECT a, b \
         FROM (WITH c AS (SELECT 1 AS a, 2 AS b) SELECT * FROM c) sub",
    );
}

#[test]
fn select_from_with_recursive_subquery_parses() {
    assert_parses(
        "SELECT * \
         FROM (WITH RECURSIVE walk(n) AS (VALUES (1) UNION ALL SELECT n + 1 FROM walk WHERE n < 5) \
         SELECT * FROM walk) sub",
    );
}
