use sqlrustgo_types::Value;

fn make_value(s: &str) -> Value {
    Value::Text(s.to_string())
}

#[test]
fn test_semantic_null_propagation() {
    let null_val = Value::Null;
    let int_val = Value::Integer(42);
    assert_eq!(null_val, Value::Null);
    assert_ne!(null_val, int_val);
}

#[test]
fn test_semantic_integer_comparison() {
    let a = Value::Integer(42);
    let b = Value::Integer(42);
    let c = Value::Integer(43);
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn test_semantic_text_comparison() {
    let a = make_value("hello");
    let b = make_value("hello");
    let c = make_value("world");
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn test_semantic_type_discrimination() {
    let int_val = Value::Integer(42);
    let text_val = Value::Text("42".to_string());
    assert_ne!(int_val, text_val);
}

#[test]
fn test_semantic_hash_consistency() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let v1 = Value::Integer(42);
    let v2 = Value::Integer(42);
    let mut h1 = DefaultHasher::new();
    let mut h2 = DefaultHasher::new();
    v1.hash(&mut h1);
    v2.hash(&mut h2);
    assert_eq!(h1.finish(), h2.finish());
}

#[test]
fn test_semantic_text_hash_consistency() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let v1 = Value::Text("hello".to_string());
    let v2 = Value::Text("hello".to_string());
    let mut h1 = DefaultHasher::new();
    let mut h2 = DefaultHasher::new();
    v1.hash(&mut h1);
    v2.hash(&mut h2);
    assert_eq!(h1.finish(), h2.finish());
}

#[test]
fn test_semantic_null_hash_consistency() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let v1 = Value::Null;
    let v2 = Value::Null;
    let mut h1 = DefaultHasher::new();
    let mut h2 = DefaultHasher::new();
    v1.hash(&mut h1);
    v2.hash(&mut h2);
    assert_eq!(h1.finish(), h2.finish());
}

#[test]
fn test_semantic_ordering_total() {
    let values = vec![
        Value::Null,
        Value::Boolean(true),
        Value::Integer(1),
        Value::Float(1.0),
        Value::Text("a".to_string()),
    ];
    let mut sorted = values.clone();
    sorted.sort();
    assert_eq!(sorted.len(), values.len());
}

#[test]
fn test_semantic_type_mismatch_not_equal() {
    let int_val = Value::Integer(0);
    let text_val = Value::Text("0".to_string());
    let bool_val = Value::Boolean(false);
    assert_ne!(int_val, text_val);
    assert_ne!(int_val, bool_val);
    assert_ne!(text_val, bool_val);
}

#[test]
fn test_semantic_blob_equality() {
    let a = Value::Blob(vec![1, 2, 3]);
    let b = Value::Blob(vec![1, 2, 3]);
    let c = Value::Blob(vec![1, 2, 4]);
    assert_eq!(a, b);
    assert_ne!(a, c);
}
