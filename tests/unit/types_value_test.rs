// Types Value Tests
use sqlrustgo_types::Value;

#[test]
fn test_value_integer_as_integer() {
    let v = Value::Integer(42);
    assert_eq!(v.as_integer(), Some(42));

    let v = Value::Text("hello".to_string());
    assert_eq!(v.as_integer(), None);
}

#[test]
fn test_value_to_bool() {
    // #3421: to_bool was removed from Value. Use type_name + as_integer
    // to verify Boolean variant exists and is distinct from Integer.
    assert_eq!(Value::Boolean(true).type_name(), "BOOLEAN");
    assert_eq!(Value::Boolean(false).type_name(), "BOOLEAN");
    assert_eq!(Value::Boolean(true).as_integer(), None);
}
#[test]
fn test_value_to_sql_string() {
    assert_eq!(Value::Null.to_sql_string(), "NULL");
    assert_eq!(Value::Boolean(true).to_sql_string(), "true");
    assert_eq!(Value::Integer(42).to_sql_string(), "42");
    assert_eq!(Value::Float(1.5).to_sql_string(), "1.5");
    assert_eq!(Value::Text("hello".to_string()).to_sql_string(), "hello");
}
#[test]
fn test_value_type_name() {
    assert_eq!(Value::Null.type_name(), "NULL");
    assert_eq!(Value::Boolean(true).type_name(), "BOOLEAN");
    assert_eq!(Value::Integer(1).type_name(), "INTEGER");
    assert_eq!(Value::Float(1.0).type_name(), "FLOAT");
    assert_eq!(Value::Text("x".to_string()).type_name(), "TEXT");
    assert_eq!(Value::Blob(vec![1, 2]).type_name(), "BLOB");
}
#[test]
fn test_value_to_index_key() {
    assert_eq!(Value::Integer(42).to_index_key(), Some(42));
    // Text returns a hash value, not None
    let text_key = Value::Text("hello".to_string()).to_index_key();
    assert!(text_key.is_some());
    assert_eq!(Value::Null.to_index_key(), None);
}

#[test]
fn test_value_estimate_memory_size() {
    assert_eq!(Value::Null.estimate_memory_size(), 0);
    assert_eq!(Value::Boolean(true).estimate_memory_size(), 1);
    assert_eq!(Value::Integer(42).estimate_memory_size(), 8);
    assert_eq!(Value::Float(1.5).estimate_memory_size(), 8);
    assert_eq!(Value::Text("hello".to_string()).estimate_memory_size(), 5);
    assert_eq!(Value::Blob(vec![1, 2, 3]).estimate_memory_size(), 3);
}
#[test]
fn test_value_timestamp_creation() {
    // #3421: Value::Timestamp was removed. Verify Integer/Float variants
    // still support numeric construction and type_name.
    let v = Value::Integer(1728000000);
    assert_eq!(v.type_name(), "INTEGER");
    assert_eq!(v.as_integer(), Some(1728000000));
}
#[test]
fn test_value_timestamp_to_string() {
    // #3421: Value::Timestamp was removed. Verify Integer to_sql_string
    // renders correctly.
    let v = Value::Integer(1728000000);
    assert_eq!(v.to_sql_string(), "1728000000");
}
#[test]
fn test_value_date_creation() {
    // #3421: Value::Date was removed. Verify Text variant still supports
    // string construction and type_name.
    let v = Value::Text("2026-01-01".to_string());
    assert_eq!(v.type_name(), "TEXT");
    assert_eq!(v.to_sql_string(), "2026-01-01");
}
#[test]
fn test_value_hash() {
    use std::collections::HashSet;

    let mut set = HashSet::new();
    set.insert(Value::Integer(1));
    set.insert(Value::Integer(1));
    set.insert(Value::Integer(2));
    assert_eq!(set.len(), 2);

    let mut set = HashSet::new();
    set.insert(Value::Text("hello".to_string()));
    set.insert(Value::Text("hello".to_string()));
    assert_eq!(set.len(), 1);
}

#[test]
fn test_value_equality() {
    // #3421: to_bool was removed. Verify PartialEq works across variants.
    assert_eq!(Value::Integer(1), Value::Integer(1));
    assert_ne!(Value::Integer(1), Value::Integer(2));
    assert_ne!(Value::Integer(1), Value::Text("1".to_string()));
    assert_eq!(Value::Null, Value::Null);
    assert_eq!(Value::Boolean(true), Value::Boolean(true));
    assert_ne!(Value::Boolean(true), Value::Boolean(false));
}
#[test]
fn test_value_clone() {
    let v1 = Value::Text("hello".to_string());
    let v2 = v1.clone();
    assert_eq!(v1, v2);
}

#[test]
fn test_value_debug() {
    let debug_str = format!("{:?}", Value::Integer(42));
    assert!(debug_str.contains("Integer"));
}
