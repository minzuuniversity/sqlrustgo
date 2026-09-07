//! V312-88 / Issue #4755: `JSON_EACH` and `JSON_TREE` table-valued
//! function tests. Verifies the SQLite-compatible column shape, the
//! recursive vs one-level semantics, and basic path filtering.
//!
//! Note: column names like `key`, `parent` are reserved words in
//! sqlrustgo's parser, so we use quoted identifiers (`"key"`) in the
//! SELECT lists. The underlying column data is unaffected.

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use std::sync::Arc;

fn e() -> ExecutionEngine<MemoryStorage> {
    let s = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(s)
}

/// JSON_EACH on a JSON array flattens each element to one row, plus a
/// single root row whose `key` is NULL and whose `value` is the array
/// itself.
#[test]
fn test_json_each_array_basic() {
    let mut x = e();
    let r = x
        .execute("SELECT \"key\", value, type FROM JSON_EACH('[1,2,3]')")
        .unwrap();
    assert_eq!(r.rows.len(), 4, "root + 3 elements");
    // Root row.
    assert!(matches!(&r.rows[0][0], sqlrustgo::Value::Null));
    assert_eq!(r.rows[0][1], sqlrustgo::Value::Text("[1,2,3]".to_string()));
    assert_eq!(r.rows[0][2], sqlrustgo::Value::Text("array".to_string()));
    // Element rows.
    assert_eq!(r.rows[1][0], sqlrustgo::Value::Text("0".to_string()));
    assert_eq!(r.rows[1][1], sqlrustgo::Value::Text("1".to_string()));
    assert_eq!(r.rows[1][2], sqlrustgo::Value::Text("integer".to_string()));
    assert_eq!(r.rows[2][0], sqlrustgo::Value::Text("1".to_string()));
    assert_eq!(r.rows[3][0], sqlrustgo::Value::Text("2".to_string()));
}

/// JSON_EACH on a JSON object flattens each member to one row.
#[test]
fn test_json_each_object_basic() {
    let mut x = e();
    let r = x
        .execute("SELECT \"key\", value, type FROM JSON_EACH('{\"a\":1,\"b\":2}')")
        .unwrap();
    assert_eq!(r.rows.len(), 3, "root + 2 members");
    // Root row.
    assert!(matches!(&r.rows[0][0], sqlrustgo::Value::Null));
    assert_eq!(
        r.rows[0][1],
        sqlrustgo::Value::Text("{\"a\":1,\"b\":2}".to_string())
    );
    assert_eq!(r.rows[0][2], sqlrustgo::Value::Text("object".to_string()));
    // Member rows.
    let keys: Vec<String> = r
        .rows
        .iter()
        .skip(1)
        .map(|row| match &row[0] {
            sqlrustgo::Value::Text(s) => s.clone(),
            other => panic!("expected text key, got {:?}", other),
        })
        .collect();
    assert!(keys.contains(&"a".to_string()));
    assert!(keys.contains(&"b".to_string()));
}

/// JSON_TREE recurses into nested structures, producing a row for
/// every leaf and every intermediate container.
#[test]
fn test_json_tree_recursive() {
    let mut x = e();
    let r = x
        .execute("SELECT \"key\", value, type, id, parent FROM JSON_TREE('[1,[2,3],{\"x\":4}]')")
        .unwrap();
    // Expected: root, [0]=1, [1]=[2,3], [1][0]=2, [1][1]=3, [2]={"x":4}, [2].x=4 → 7 rows
    let expected_msg = "root + 3 elements + [2,3] children + {x:4} member";
    assert_eq!(r.rows.len(), 7, "{}", expected_msg);
    // id column should be strictly increasing.
    let ids: Vec<i64> = r
        .rows
        .iter()
        .map(|row| match &row[3] {
            sqlrustgo::Value::Integer(i) => *i,
            other => panic!("expected int id, got {:?}", other),
        })
        .collect();
    for w in ids.windows(2) {
        assert!(w[1] > w[0], "ids should be strictly increasing");
    }
}

/// JSON_EACH on a primitive value returns one row whose `atom` matches
/// the input and `type` matches the JSON type.
#[test]
fn test_json_each_primitive() {
    let mut x = e();
    let r = x
        .execute("SELECT \"key\", value, type, atom FROM JSON_EACH('\"hello\"')")
        .unwrap();
    assert_eq!(r.rows.len(), 1);
    // For a primitive root, key is NULL but value/type/atom are set.
    assert!(matches!(&r.rows[0][0], sqlrustgo::Value::Null));
    assert_eq!(
        r.rows[0][1],
        sqlrustgo::Value::Text("\"hello\"".to_string())
    );
    assert_eq!(r.rows[0][2], sqlrustgo::Value::Text("text".to_string()));
    assert_eq!(r.rows[0][3], sqlrustgo::Value::Text("hello".to_string()));
}

/// JSON_EACH on a non-array, non-object value also returns one row
/// (the primitive itself), with `atom` set to the typed value.
#[test]
fn test_json_each_integer_primitive() {
    let mut x = e();
    let r = x
        .execute("SELECT \"key\", value, type, atom FROM JSON_EACH('42')")
        .unwrap();
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0][1], sqlrustgo::Value::Text("42".to_string()));
    assert_eq!(r.rows[0][2], sqlrustgo::Value::Text("integer".to_string()));
    assert_eq!(r.rows[0][3], sqlrustgo::Value::Integer(42));
}

/// The path argument restricts the expansion to a sub-path. A missing
/// path produces zero rows.
#[test]
fn test_json_each_path_argument() {
    let mut x = e();
    let r = x
        .execute("SELECT \"key\", value FROM JSON_EACH('{\"items\":[1,2,3]}', '$.items')")
        .unwrap();
    // Root is the items array, plus 3 children.
    assert_eq!(r.rows.len(), 4);
    assert_eq!(r.rows[1][0], sqlrustgo::Value::Text("0".to_string()));
    assert_eq!(r.rows[2][0], sqlrustgo::Value::Text("1".to_string()));
    assert_eq!(r.rows[3][0], sqlrustgo::Value::Text("2".to_string()));
}

/// Missing path returns zero rows.
#[test]
fn test_json_each_path_missing() {
    let mut x = e();
    let r = x
        .execute("SELECT \"key\" FROM JSON_EACH('{\"a\":1}', '$.nonexistent')")
        .unwrap();
    assert_eq!(r.rows.len(), 0);
}

/// JSON_TREE on an object value.
#[test]
fn test_json_tree_object() {
    let mut x = e();
    let r = x
        .execute("SELECT \"key\", value, type FROM JSON_TREE('{\"a\":1,\"b\":2}')")
        .unwrap();
    // root + 2 members = 3 rows
    assert_eq!(r.rows.len(), 3);
}

/// JSON_TREE on a nested object.
#[test]
fn test_json_tree_nested_object() {
    let mut x = e();
    let r = x
        .execute("SELECT \"key\", type FROM JSON_TREE('{\"a\":{\"b\":1}}')")
        .unwrap();
    // root + "a" object + "b" integer = 3 rows
    assert_eq!(r.rows.len(), 3);
    // Find the "a" row and verify it points to the nested object.
    let a_row = r
        .rows
        .iter()
        .find(|row| matches!(&row[0], sqlrustgo::Value::Text(k) if k == "a"))
        .expect("a row");
    assert_eq!(a_row[1], sqlrustgo::Value::Text("object".to_string()));
    let b_row = r
        .rows
        .iter()
        .find(|row| matches!(&row[0], sqlrustgo::Value::Text(k) if k == "b"))
        .expect("b row");
    assert_eq!(b_row[1], sqlrustgo::Value::Text("integer".to_string()));
}

/// `parent` correctly links child rows to their parent row.
#[test]
fn test_json_each_parent_id() {
    let mut x = e();
    let r = x
        .execute("SELECT id, parent FROM JSON_EACH('{\"a\":1,\"b\":2}')")
        .unwrap();
    // First row (root) has parent 0; subsequent rows have parent = 1.
    assert_eq!(
        r.rows[0][1],
        sqlrustgo::Value::Integer(0),
        "root has parent 0"
    );
    for row in &r.rows[1..] {
        assert_eq!(
            row[1],
            sqlrustgo::Value::Integer(1),
            "children of root have parent 1"
        );
    }
}

/// `fullkey` and `path` for an object member.
#[test]
fn test_json_each_fullkey_and_path() {
    let mut x = e();
    let r = x
        .execute("SELECT \"key\", fullkey, path FROM JSON_EACH('{\"a\":1}')")
        .unwrap();
    // First row: root, key NULL, fullkey '$', path ''.
    assert!(matches!(&r.rows[0][0], sqlrustgo::Value::Null));
    assert_eq!(r.rows[0][1], sqlrustgo::Value::Text("$".to_string()));
    assert_eq!(r.rows[0][2], sqlrustgo::Value::Text("".to_string()));
    // Second row: member 'a', fullkey '$.a', path '.a'.
    let a_row = r
        .rows
        .iter()
        .find(|row| matches!(&row[0], sqlrustgo::Value::Text(k) if k == "a"))
        .expect("a row");
    assert_eq!(a_row[1], sqlrustgo::Value::Text("$.a".to_string()));
    assert_eq!(a_row[2], sqlrustgo::Value::Text(".a".to_string()));
}

/// Invalid JSON in the first argument should return an error.
#[test]
fn test_json_each_invalid_json_errors() {
    let mut x = e();
    let r = x.execute("SELECT \"key\" FROM JSON_EACH('not-json')");
    assert!(r.is_err(), "invalid JSON should produce an error");
}

/// JSON_EACH with no arguments returns a clear error (the TVF needs
/// at least the JSON document).
#[test]
fn test_json_each_no_args_errors() {
    let mut x = e();
    let r = x.execute("SELECT \"key\" FROM JSON_EACH()");
    assert!(
        r.is_err(),
        "JSON_EACH with no args should produce a clear error"
    );
}

/// Cross-check: `SELECT * FROM JSON_EACH(...)` returns 8 columns
/// (key, value, type, atom, id, parent, fullkey, path) per SQLite.
#[test]
fn test_json_each_star_returns_eight_columns() {
    let mut x = e();
    let r = x.execute("SELECT * FROM JSON_EACH('[1]')").unwrap();
    // 1 root + 1 element = 2 rows
    assert_eq!(r.rows.len(), 2);
    // Each row has 8 columns.
    for row in &r.rows {
        assert_eq!(row.len(), 8, "row should have 8 columns");
    }
}
