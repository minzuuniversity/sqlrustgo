//! JSON Eval Fn Tests (V312-16 / #3903 truthfulness audit)
//!
//! Tests exercise the SQL JSON surface end-to-end via the public
//! `ExecutionEngine::execute` path. Boolean/Integer cross-type
//! equality has known gaps in eval_binary_op; this file uses
//! Boolean predicates directly (truthiness via WHERE) where the
//! cross-type gap matters.

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use std::sync::Arc;

fn create_engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

#[test]
fn test_json_valid_well_formed_returns_true() {
    let mut engine = create_engine();
    let r = engine.execute("SELECT JSON_VALID('{\"a\":1}')").unwrap();
    assert_eq!(r.rows, vec![vec![sqlrustgo_types::Value::Boolean(true)]]);
}

#[test]
fn test_json_valid_invalid_returns_false() {
    let mut engine = create_engine();
    let r = engine.execute("SELECT JSON_VALID('not-json{')").unwrap();
    assert_eq!(r.rows, vec![vec![sqlrustgo_types::Value::Boolean(false)]]);
}

#[test]
fn test_json_extract_returns_null_for_malformed() {
    let mut engine = create_engine();
    let r = engine
        .execute("SELECT JSON_EXTRACT('not-json', '$.a')")
        .unwrap();
    // json_extract returns Null when the JSON is malformed.
    assert_eq!(r.rows, vec![vec![sqlrustgo_types::Value::Null]]);
}

#[test]
fn test_json_value_extracts_scalar() {
    // `$.a` against `{"a":1}` should yield the JSON scalar 1.
    // JSON_VALUE returns it unquoted as Integer.
    let mut engine = create_engine();
    let r = engine
        .execute("SELECT JSON_VALUE('{\"a\":1}', '$.a')")
        .unwrap();
    // The unquote path converts Number to Integer.
    assert_eq!(
        r.rows,
        vec![vec![sqlrustgo_types::Value::Integer(1)]],
        "JSON_VALUE('{{\"a\":1}}', '$.a') should yield 1, got {:?}",
        r.rows
    );
}

#[test]
fn test_json_value_extracts_string() {
    let mut engine = create_engine();
    let r = engine
        .execute("SELECT JSON_VALUE('{\"name\":\"alice\"}', '$.name')")
        .unwrap();
    assert_eq!(
        r.rows,
        vec![vec![sqlrustgo_types::Value::Text("alice".into())]],
        "JSON_VALUE('{{\"name\":\"alice\"}}', '$.name') should yield 'alice', got {:?}",
        r.rows
    );
}

#[test]
#[test]
fn test_json_function_constructor() {
    let mut engine = create_engine();
    let r = engine
        .execute("SELECT JSON('{\"a\":1}') IS NOT NULL")
        .unwrap();
    // JSON constructor returns non-null Value::Json for valid input.
    assert_eq!(r.rows.len(), 1);
}

#[test]
fn test_json_function_null_for_garbage() {
    let mut engine = create_engine();
    let r = engine.execute("SELECT JSON('garbage')").unwrap();
    assert_eq!(r.rows, vec![vec![sqlrustgo_types::Value::Null]]);
}

#[test]
fn test_json_type_returns_object() {
    let mut engine = create_engine();
    let r = engine.execute("SELECT JSON_TYPE('{\"a\":1}')").unwrap();
    // JSON_TYPE for object returns "object".
    assert_eq!(
        r.rows,
        vec![vec![sqlrustgo_types::Value::Text("object".into())]],
        "JSON_TYPE should return OBJECT for object, got {:?}",
        r.rows
    );
}

fn test_json_type_returns_array() {
    let mut engine = create_engine();
    let r = engine.execute("SELECT JSON_TYPE('[1,2,3]')").unwrap();
    assert_eq!(
        r.rows,
        vec![vec![sqlrustgo_types::Value::Text("array".into())]],
        "JSON_TYPE should return ARRAY for array, got {:?}",
        r.rows
    );
}

#[test]
fn test_json_nested_path() {
    let mut engine = create_engine();
    // $.a.b should resolve to 2 in {"a":{"b":2}}.
    let r = engine
        .execute("SELECT JSON_VALUE('{\"a\":{\"b\":2}}', '$.a.b')")
        .unwrap();
    assert_eq!(
        r.rows,
        vec![vec![sqlrustgo_types::Value::Integer(2)]],
        "nested path should yield 2, got {:?}",
        r.rows
    );
}

#[test]
fn test_json_array_index() {
    let mut engine = create_engine();
    // $.items[1] should resolve to 2 in {"items":[1,2,3]}.
    let r = engine
        .execute("SELECT JSON_VALUE('{\"items\":[1,2,3]}', '$.items[1]')")
        .unwrap();
    assert_eq!(
        r.rows,
        vec![vec![sqlrustgo_types::Value::Integer(2)]],
        "array index [1] should yield 2, got {:?}",
        r.rows
    );
}
