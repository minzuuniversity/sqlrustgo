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
    // TODO #3421: body uses removed Value API (Date/Timestamp/to_bool)
    unimplemented!()
}
#[test]
fn test_value_to_sql_string() {
    // TODO #3421: body uses removed Value API (Date/Timestamp/to_bool)
    unimplemented!()
}
#[test]
fn test_value_type_name() {
    // TODO #3421: body uses removed Value API (Date/Timestamp/to_bool)
    unimplemented!()
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
    // TODO #3421: body uses removed Value API (Date/Timestamp/to_bool)
    unimplemented!()
}
#[test]
fn test_value_timestamp_creation() {
    // TODO #3421: body uses removed Value API (Date/Timestamp/to_bool)
    unimplemented!()
}
#[test]
fn test_value_timestamp_to_string() {
    // TODO #3421: body uses removed Value API (Date/Timestamp/to_bool)
    unimplemented!()
}
#[test]
fn test_value_date_creation() {
    // TODO #3421: body uses removed Value API (Date/Timestamp/to_bool)
    unimplemented!()
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
    // TODO #3421: body uses removed Value API (Date/Timestamp/to_bool)
    unimplemented!()
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
