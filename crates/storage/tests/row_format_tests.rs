//! Row Format Integration Tests
//!
//! Tests for the Compact Row v1 format encoding and decoding.
//! Covers all SQL value types, null handling, and edge cases.

use sqlrustgo_storage::row_format::types::{ClusterKey, RowHeader};
use sqlrustgo_storage::row_format::{decode_row, encode_row};
use sqlrustgo_types::Value;

// ============================================================================
// Test Setup Helpers
// ============================================================================

/// Helper to encode and decode a row with given columns
fn encode_decode_row(
    cluster_key: i64,
    fixed_values: Vec<Value>,
    varlen_values: Vec<Option<Vec<u8>>>,
    nulls: Vec<bool>,
) -> std::io::Result<(Vec<Value>, Vec<Option<Vec<u8>>>, Vec<bool>)> {
    let key = ClusterKey::HiddenRowId(cluster_key as u64);
    let encoded = encode_row(&key, &fixed_values, &varlen_values, &nulls)?;
    let (_, decoded_fixed, decoded_varlen, decoded_nulls) =
        decode_row(&encoded, fixed_values.len(), varlen_values.len())?;
    Ok((decoded_fixed, decoded_varlen, decoded_nulls))
}

/// Helper to create a simple row with just fixed values
fn encode_decode_fixed_row(
    cluster_key: i64,
    fixed_values: Vec<Value>,
) -> std::io::Result<Vec<Value>> {
    let key = ClusterKey::HiddenRowId(cluster_key as u64);
    let encoded = encode_row(&key, &fixed_values, &[], &[])?;
    let (_, decoded_fixed, _, _) = decode_row(&encoded, fixed_values.len(), 0)?;
    Ok(decoded_fixed)
}

// ============================================================================
// Basic Encoding/Decoding Tests
// ============================================================================

#[test]
fn test_encode_decode_empty_row() {
    let key = ClusterKey::HiddenRowId(1);
    let encoded = encode_row(&key, &[], &[], &[]).unwrap();

    assert!(!encoded.is_empty());
    // First byte is format version
    assert_eq!(encoded[0], 1);
}

#[test]
fn test_encode_decode_single_integer() {
    let decoded = encode_decode_fixed_row(1, vec![Value::Integer(42)]).unwrap();
    assert_eq!(decoded.len(), 1);
    assert_eq!(decoded[0], Value::Integer(42));
}

#[test]
fn test_encode_decode_single_float() {
    let decoded = encode_decode_fixed_row(1, vec![Value::Float(3.14159)]).unwrap();
    assert_eq!(decoded.len(), 1);
    // Compare float values
    match decoded[0] {
        Value::Float(f) => assert!((f - 3.14159).abs() < 0.0001),
        _ => panic!("Expected Float value"),
    }
}

#[test]
fn test_encode_decode_single_text() {
    let key = ClusterKey::HiddenRowId(1);
    let text = "Hello, World!".to_string();
    let encoded = encode_row(&key, &[], &[Some(text.clone().into_bytes())], &[false]).unwrap();
    let (_, _, decoded_varlen, _) = decode_row(&encoded, 0, 1).unwrap();

    assert_eq!(decoded_varlen.len(), 1);
    let retrieved_text = String::from_utf8(decoded_varlen[0].clone().unwrap()).unwrap();
    assert_eq!(retrieved_text, text);
}

// ============================================================================
// Integer Value Tests
// ============================================================================

#[test]
fn test_encode_decode_integer_zero() {
    let decoded = encode_decode_fixed_row(1, vec![Value::Integer(0)]).unwrap();
    assert_eq!(decoded[0], Value::Integer(0));
}

#[test]
fn test_encode_decode_integer_positive() {
    let decoded = encode_decode_fixed_row(1, vec![Value::Integer(12345)]).unwrap();
    assert_eq!(decoded[0], Value::Integer(12345));
}

#[test]
fn test_encode_decode_integer_negative() {
    let decoded = encode_decode_fixed_row(1, vec![Value::Integer(-98765)]).unwrap();
    assert_eq!(decoded[0], Value::Integer(-98765));
}

#[test]
fn test_encode_decode_integer_max() {
    let decoded = encode_decode_fixed_row(1, vec![Value::Integer(i64::MAX)]).unwrap();
    assert_eq!(decoded[0], Value::Integer(i64::MAX));
}

#[test]
fn test_encode_decode_integer_min() {
    let decoded = encode_decode_fixed_row(1, vec![Value::Integer(i64::MIN)]).unwrap();
    assert_eq!(decoded[0], Value::Integer(i64::MIN));
}

#[test]
fn test_encode_decode_multiple_integers() {
    let values = vec![
        Value::Integer(1),
        Value::Integer(2),
        Value::Integer(-3),
        Value::Integer(0),
        Value::Integer(i64::MAX),
    ];
    let decoded = encode_decode_fixed_row(1, values.clone()).unwrap();
    assert_eq!(decoded, values);
}

// ============================================================================
// Float Value Tests
// ============================================================================

#[test]
fn test_encode_decode_float_zero() {
    let decoded = encode_decode_fixed_row(1, vec![Value::Float(0.0)]).unwrap();
    assert_eq!(decoded[0], Value::Float(0.0));
}

#[test]
fn test_encode_decode_float_positive() {
    let decoded = encode_decode_fixed_row(1, vec![Value::Float(123.456)]).unwrap();
    match decoded[0] {
        Value::Float(f) => assert!((f - 123.456).abs() < 0.001),
        _ => panic!("Expected Float value"),
    }
}

#[test]
fn test_encode_decode_float_negative() {
    let decoded = encode_decode_fixed_row(1, vec![Value::Float(-789.012)]).unwrap();
    match decoded[0] {
        Value::Float(f) => assert!((f + 789.012).abs() < 0.001),
        _ => panic!("Expected Float value"),
    }
}

// ============================================================================
// Boolean Value Tests
// ============================================================================

#[test]
fn test_encode_decode_boolean_true() {
    let decoded = encode_decode_fixed_row(1, vec![Value::Boolean(true)]).unwrap();
    assert_eq!(decoded[0], Value::Boolean(true));
}

#[test]
fn test_encode_decode_boolean_false() {
    let decoded = encode_decode_fixed_row(1, vec![Value::Boolean(false)]).unwrap();
    assert_eq!(decoded[0], Value::Boolean(false));
}

#[test]
fn test_encode_decode_multiple_booleans() {
    let values = vec![
        Value::Boolean(true),
        Value::Boolean(false),
        Value::Boolean(true),
        Value::Boolean(false),
    ];
    let decoded = encode_decode_fixed_row(1, values).unwrap();
    assert_eq!(decoded.len(), 4);
}

// ============================================================================
// Text Value Tests
// ============================================================================

#[test]
fn test_encode_decode_text_empty() {
    let key = ClusterKey::HiddenRowId(1);
    let encoded = encode_row(&key, &[], &[Some(vec![])], &[false]).unwrap();
    let (_, _, decoded_varlen, _) = decode_row(&encoded, 0, 1).unwrap();

    assert!(decoded_varlen[0].is_some());
    assert!(decoded_varlen[0].as_ref().unwrap().is_empty());
}

#[test]
fn test_encode_decode_text_ascii() {
    let key = ClusterKey::HiddenRowId(1);
    let text = "Hello, World!".to_string();
    let encoded = encode_row(&key, &[], &[Some(text.clone().into_bytes())], &[false]).unwrap();
    let (_, _, decoded_varlen, _) = decode_row(&encoded, 0, 1).unwrap();

    let decoded_text = String::from_utf8(decoded_varlen[0].clone().unwrap()).unwrap();
    assert_eq!(decoded_text, text);
}

#[test]
fn test_encode_decode_text_unicode() {
    let key = ClusterKey::HiddenRowId(1);
    let text = "你好世界".to_string();
    let encoded = encode_row(&key, &[], &[Some(text.clone().into_bytes())], &[false]).unwrap();
    let (_, _, decoded_varlen, _) = decode_row(&encoded, 0, 1).unwrap();

    let decoded_text = String::from_utf8(decoded_varlen[0].clone().unwrap()).unwrap();
    assert_eq!(decoded_text, text);
}

#[test]
fn test_encode_decode_text_long() {
    let key = ClusterKey::HiddenRowId(1);
    let text = "A".repeat(10000);
    let encoded = encode_row(&key, &[], &[Some(text.clone().into_bytes())], &[false]).unwrap();
    let (_, _, decoded_varlen, _) = decode_row(&encoded, 0, 1).unwrap();

    let decoded_text = String::from_utf8(decoded_varlen[0].clone().unwrap()).unwrap();
    assert_eq!(decoded_text.len(), 10000);
}

#[test]
fn test_encode_decode_text_special_chars() {
    let key = ClusterKey::HiddenRowId(1);
    let text = "Line1\nLine2\tTabbed\r\nWindows\rUnix\n".to_string();
    let encoded = encode_row(&key, &[], &[Some(text.clone().into_bytes())], &[false]).unwrap();
    let (_, _, decoded_varlen, _) = decode_row(&encoded, 0, 1).unwrap();

    let decoded_text = String::from_utf8(decoded_varlen[0].clone().unwrap()).unwrap();
    assert_eq!(decoded_text, text);
}

// ============================================================================
// Blob Value Tests
// ============================================================================

#[test]
fn test_encode_decode_blob_empty() {
    let key = ClusterKey::HiddenRowId(1);
    let encoded = encode_row(&key, &[], &[Some(vec![])], &[false]).unwrap();
    let (_, _, decoded_varlen, _) = decode_row(&encoded, 0, 1).unwrap();

    assert!(decoded_varlen[0].is_some());
    assert!(decoded_varlen[0].as_ref().unwrap().is_empty());
}

#[test]
fn test_encode_decode_blob_binary() {
    let key = ClusterKey::HiddenRowId(1);
    let blob = vec![0x00, 0xFF, 0x42, 0x13, 0x37, 0xDE, 0xAD, 0xBE, 0xEF];
    let encoded = encode_row(&key, &[], &[Some(blob.clone())], &[false]).unwrap();
    let (_, _, decoded_varlen, _) = decode_row(&encoded, 0, 1).unwrap();

    assert_eq!(decoded_varlen[0].clone().unwrap(), blob);
}

#[test]
fn test_encode_decode_blob_large() {
    let key = ClusterKey::HiddenRowId(1);
    let blob: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();
    let encoded = encode_row(&key, &[], &[Some(blob.clone())], &[false]).unwrap();
    let (_, _, decoded_varlen, _) = decode_row(&encoded, 0, 1).unwrap();

    assert_eq!(decoded_varlen[0].clone().unwrap().len(), 10000);
}

// ============================================================================
// Null Value Tests
// ============================================================================

#[test]
fn test_encode_decode_null_integer() {
    let decoded = encode_decode_fixed_row(1, vec![Value::Null]).unwrap();
    assert_eq!(decoded[0], Value::Null);
}

#[test]
fn test_encode_decode_null_in_fixed_columns() {
    let key = ClusterKey::HiddenRowId(1);
    let fixed_values = vec![Value::Integer(42), Value::Null, Value::Float(3.14)];
    let nulls = vec![false, true, false];

    let encoded = encode_row(&key, &fixed_values, &[], &nulls).unwrap();
    let (_, decoded_fixed, _, decoded_nulls) = decode_row(&encoded, 3, 0).unwrap();

    assert_eq!(decoded_fixed[0], Value::Integer(42));
    assert_eq!(decoded_fixed[1], Value::Null);
    match decoded_fixed[2] {
        Value::Float(_) => {}
        _ => panic!("Expected Float value"),
    }
    assert_eq!(decoded_nulls[0], false);
    assert_eq!(decoded_nulls[1], true);
    assert_eq!(decoded_nulls[2], false);
}

#[test]
fn test_encode_decode_null_in_varlen_columns() {
    let key = ClusterKey::HiddenRowId(1);
    let fixed_values = vec![Value::Integer(100)];
    let varlen_values = vec![None, Some(b"hello".to_vec())];
    let nulls = vec![false, true, false]; // [fixed_null, varlen_null, extra]

    let encoded = encode_row(&key, &fixed_values, &varlen_values, &nulls).unwrap();
    let (_, decoded_fixed, decoded_varlen, decoded_nulls) = decode_row(&encoded, 1, 2).unwrap();

    assert_eq!(decoded_fixed[0], Value::Integer(100));
    assert!(decoded_varlen[0].is_none()); // Null varlen
    assert!(decoded_varlen[1].is_some()); // Non-null varlen
    assert_eq!(decoded_nulls[0], false);
    assert_eq!(decoded_nulls[1], true); // varlen column is null
    assert_eq!(decoded_nulls[2], false);
}

#[test]
fn test_encode_decode_all_nulls() {
    let key = ClusterKey::HiddenRowId(1);
    let fixed_values = vec![Value::Null, Value::Null, Value::Null];
    let varlen_values = vec![None, None];
    let nulls = vec![true, true, true, true, true];

    let encoded = encode_row(&key, &fixed_values, &varlen_values, &nulls).unwrap();
    let (_, decoded_fixed, decoded_varlen, decoded_nulls) = decode_row(&encoded, 3, 2).unwrap();

    for val in decoded_fixed {
        assert_eq!(val, Value::Null);
    }
    for varlen in decoded_varlen {
        assert!(varlen.is_none());
    }
    for null_flag in decoded_nulls {
        assert!(null_flag);
    }
}

// ============================================================================
// Cluster Key Tests
// ============================================================================

#[test]
fn test_encode_decode_hidden_row_id_key() {
    let key = ClusterKey::HiddenRowId(12345);
    let encoded = encode_row(&key, &[Value::Integer(42)], &[], &[]).unwrap();
    let (_, _, _, _) = decode_row(&encoded, 1, 0).unwrap();
}

#[test]
fn test_encode_decode_primary_key() {
    let key = ClusterKey::PrimaryKey(Value::Integer(999));
    let encoded = encode_row(&key, &[Value::Integer(42)], &[], &[]).unwrap();
    let (_, _, _, _) = decode_row(&encoded, 1, 0).unwrap();
}

// ============================================================================
// Mixed Type Tests
// ============================================================================

#[test]
fn test_encode_decode_mixed_fixed_types() {
    let values = vec![
        Value::Integer(42),
        Value::Float(3.14),
        Value::Boolean(true),
        Value::Integer(-100),
        Value::Float(-2.718),
        Value::Boolean(false),
    ];
    let decoded = encode_decode_fixed_row(1, values.clone()).unwrap();
    assert_eq!(decoded, values);
}

#[test]
fn test_encode_decode_all_sql_types() {
    let key = ClusterKey::HiddenRowId(1);
    let fixed_values = vec![
        Value::Integer(42),
        Value::Float(3.14),
        Value::Boolean(true),
        Value::Null,
    ];
    let varlen_values = vec![Some(b"hello".to_vec()), None, Some(b"".to_vec())];
    let nulls = vec![false, false, false, true, false, true, false];

    let encoded = encode_row(&key, &fixed_values, &varlen_values, &nulls).unwrap();
    let (_, decoded_fixed, decoded_varlen, decoded_nulls) = decode_row(&encoded, 4, 3).unwrap();

    assert_eq!(decoded_fixed.len(), 4);
    assert_eq!(decoded_varlen.len(), 3);
    assert_eq!(decoded_nulls.len(), 7);

    // Verify fixed values
    assert_eq!(decoded_fixed[0], Value::Integer(42));
    match decoded_fixed[1] {
        Value::Float(_) => {}
        _ => panic!("Expected Float"),
    }
    assert_eq!(decoded_fixed[2], Value::Boolean(true));
    assert_eq!(decoded_fixed[3], Value::Null);

    // Verify varlen
    assert!(decoded_varlen[0].is_some());
    assert_eq!(decoded_varlen[0].clone().unwrap(), b"hello");
    assert!(decoded_varlen[1].is_none());
    assert!(decoded_varlen[2].is_some());

    // Verify nulls
    assert_eq!(decoded_nulls[0], false);
    assert_eq!(decoded_nulls[1], false);
    assert_eq!(decoded_nulls[2], false);
    assert_eq!(decoded_nulls[3], true);
    assert_eq!(decoded_nulls[4], false);
    assert_eq!(decoded_nulls[5], true);
    assert_eq!(decoded_nulls[6], false);
}

// ============================================================================
// Row Header Tests
// ============================================================================

#[test]
fn test_row_header_format_version() {
    let key = ClusterKey::HiddenRowId(1);
    let encoded = encode_row(&key, &[], &[], &[]).unwrap();

    // First byte is format version
    assert_eq!(encoded[0], 1);
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_encode_decode_many_columns() {
    let key = ClusterKey::HiddenRowId(1);
    let fixed_values: Vec<Value> = (0..100).map(|i| Value::Integer(i)).collect();
    let nulls = vec![false; 100];

    let encoded = encode_row(&key, &fixed_values, &[], &nulls).unwrap();
    let (_, decoded_fixed, _, _) = decode_row(&encoded, 100, 0).unwrap();

    assert_eq!(decoded_fixed.len(), 100);
    for (i, val) in decoded_fixed.iter().enumerate() {
        assert_eq!(*val, Value::Integer(i as i64));
    }
}

#[test]
fn test_encode_decode_many_varlen_columns() {
    let key = ClusterKey::HiddenRowId(1);
    let varlen_values: Vec<Option<Vec<u8>>> = (0..50).map(|i| Some(vec![i as u8; 10])).collect();
    let nulls = vec![false; 50];

    let encoded = encode_row(&key, &[], &varlen_values, &nulls).unwrap();
    let (_, _, decoded_varlen, _) = decode_row(&encoded, 0, 50).unwrap();

    assert_eq!(decoded_varlen.len(), 50);
    for (i, varlen) in decoded_varlen.iter().enumerate() {
        assert!(varlen.is_some());
        let data = varlen.clone().unwrap();
        assert_eq!(data.len(), 10);
        assert_eq!(data[0], i as u8);
    }
}

#[test]
fn test_encode_decode_single_column() {
    // Test single column edge case
    let decoded = encode_decode_fixed_row(1, vec![Value::Integer(1)]).unwrap();
    assert_eq!(decoded.len(), 1);
    assert_eq!(decoded[0], Value::Integer(1));
}

#[test]
fn test_encode_decode_zero_columns() {
    let key = ClusterKey::HiddenRowId(1);
    let encoded = encode_row(&key, &[], &[], &[]).unwrap();
    let (_, fixed, varlen, nulls) = decode_row(&encoded, 0, 0).unwrap();

    assert!(fixed.is_empty());
    assert!(varlen.is_empty());
    assert!(nulls.is_empty());
}

// ============================================================================
// Null Bitmap Tests
// ============================================================================

#[test]
fn test_null_bitmap_no_nulls() {
    let key = ClusterKey::HiddenRowId(1);
    let fixed_values = vec![Value::Integer(1), Value::Integer(2), Value::Integer(3)];
    let nulls = vec![false, false, false];

    let encoded = encode_row(&key, &fixed_values, &[], &nulls).unwrap();
    let (_, _, _, decoded_nulls) = decode_row(&encoded, 3, 0).unwrap();

    assert_eq!(decoded_nulls, vec![false, false, false]);
}

#[test]
fn test_null_bitmap_some_nulls() {
    let key = ClusterKey::HiddenRowId(1);
    let fixed_values = vec![Value::Integer(1), Value::Integer(2), Value::Integer(3)];
    let nulls = vec![true, false, true];

    let encoded = encode_row(&key, &fixed_values, &[], &nulls).unwrap();
    let (_, _, _, decoded_nulls) = decode_row(&encoded, 3, 0).unwrap();

    assert_eq!(decoded_nulls, vec![true, false, true]);
}

#[test]
fn test_null_bitmap_all_nulls() {
    let key = ClusterKey::HiddenRowId(1);
    let fixed_values = vec![Value::Integer(1), Value::Integer(2)];
    let nulls = vec![true, true];

    let encoded = encode_row(&key, &fixed_values, &[], &nulls).unwrap();
    let (_, _, _, decoded_nulls) = decode_row(&encoded, 2, 0).unwrap();

    assert_eq!(decoded_nulls, vec![true, true]);
}

// ============================================================================
// Data Integrity Tests
// ============================================================================

#[test]
fn test_roundtrip_small_integers() {
    for i in -128..=127 {
        let decoded = encode_decode_fixed_row(1, vec![Value::Integer(i)]).unwrap();
        assert_eq!(decoded[0], Value::Integer(i));
    }
}

#[test]
fn test_roundtrip_powers_of_two() {
    let powers: Vec<i64> = vec![
        1,
        2,
        4,
        8,
        16,
        32,
        64,
        128,
        256,
        512,
        1024,
        2048,
        4096,
        8192,
        i64::MAX / 2 + 1,
    ];

    for p in powers {
        let decoded = encode_decode_fixed_row(1, vec![Value::Integer(p)]).unwrap();
        assert_eq!(decoded[0], Value::Integer(p));
    }
}

#[test]
fn test_roundtrip_common_floats() {
    let floats = vec![
        0.0, 1.0, -1.0, 0.5, -0.5, 0.25, 0.75, 1.5, 2.5, 10.0, 100.0, 1000.0,
    ];

    for f in floats {
        let decoded = encode_decode_fixed_row(1, vec![Value::Float(f)]).unwrap();
        match decoded[0] {
            Value::Float(decoded_f) => assert!((decoded_f - f).abs() < 0.0001),
            _ => panic!("Expected Float"),
        }
    }
}

// ============================================================================
// Performance-Characterization Tests
// ============================================================================

#[test]
fn test_encode_decode_many_small_rows() {
    let key = ClusterKey::HiddenRowId(1);

    for row_num in 0..1000 {
        let fixed_values = vec![
            Value::Integer(row_num as i64),
            Value::Text(format!("row_{}", row_num)),
            Value::Integer(row_num as i64 * 2),
        ];
        let varlen_values = vec![Some(vec![row_num as u8; 10])];
        let nulls = vec![false, false, false, false];

        let encoded = encode_row(&key, &fixed_values, &varlen_values, &nulls).unwrap();
        let (_, decoded_fixed, _, _) = decode_row(&encoded, 3, 1).unwrap();

        assert_eq!(decoded_fixed[0], Value::Integer(row_num as i64));
    }
}

#[test]
fn test_encode_decode_repeated_null_patterns() {
    let key = ClusterKey::HiddenRowId(1);

    // Repeated null pattern: [val, null, val, null, ...]
    for iteration in 0..100 {
        let fixed_values = vec![
            Value::Integer(iteration as i64),
            Value::Null,
            Value::Float(iteration as f64),
            Value::Null,
        ];
        let nulls = vec![false, true, false, true];

        let encoded = encode_row(&key, &fixed_values, &[], &nulls).unwrap();
        let (_, decoded_fixed, _, _) = decode_row(&encoded, 4, 0).unwrap();

        assert_eq!(decoded_fixed[0], Value::Integer(iteration as i64));
        assert_eq!(decoded_fixed[1], Value::Null);
    }
}
