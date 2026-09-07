// V312-86 / Issue #4809: INDEXED BY hint parsing integration tests
// Tests SQLite-style INDEXED BY and NOT INDEXED syntax parsing

use sqlrustgo::parse;

#[test]
fn test_indexed_by_basic() {
    let sql = "SELECT * FROM t1 INDEXED BY idx_name WHERE x = 1";
    let result = parse(sql);
    assert!(result.is_ok(), "INDEXED BY parsing failed: {:?}", result);
}

#[test]
fn test_not_indexed_basic() {
    let sql = "SELECT * FROM t1 NOT INDEXED WHERE x = 1";
    let result = parse(sql);
    assert!(result.is_ok(), "NOT INDEXED parsing failed: {:?}", result);
}

#[test]
fn test_indexed_by_update() {
    let sql = "UPDATE t1 INDEXED BY idx_name SET col = 1 WHERE x = 2";
    let result = parse(sql);
    assert!(
        result.is_ok(),
        "INDEXED BY UPDATE parsing failed: {:?}",
        result
    );
}

#[test]
fn test_not_indexed_update() {
    let sql = "UPDATE t1 NOT INDEXED SET col = 1 WHERE x = 2";
    let result = parse(sql);
    assert!(
        result.is_ok(),
        "NOT INDEXED UPDATE parsing failed: {:?}",
        result
    );
}

#[test]
fn test_indexed_by_delete() {
    let sql = "DELETE FROM t1 INDEXED BY idx_name WHERE x = 3";
    let result = parse(sql);
    assert!(
        result.is_ok(),
        "INDEXED BY DELETE parsing failed: {:?}",
        result
    );
}

#[test]
fn test_not_indexed_delete() {
    let sql = "DELETE FROM t1 NOT INDEXED WHERE x = 3";
    let result = parse(sql);
    assert!(
        result.is_ok(),
        "NOT INDEXED DELETE parsing failed: {:?}",
        result
    );
}

#[test]
fn test_indexed_by_with_alias() {
    let sql = "SELECT a.* FROM t1 AS a INDEXED BY idx_name WHERE a.x > 10";
    let result = parse(sql);
    assert!(result.is_ok(), "INDEXED BY with alias failed: {:?}", result);
}

#[test]
fn test_multiple_tables_indexed_by() {
    let sql = "SELECT * FROM t1 INDEXED BY idx1, t2 INDEXED BY idx2 WHERE t1.id = t2.id";
    let result = parse(sql);
    assert!(
        result.is_ok(),
        "Multiple INDEXED BY parsing failed: {:?}",
        result
    );
}
