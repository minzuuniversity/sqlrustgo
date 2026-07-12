//! SIMD batch eval integration tests
//!
//! v3.10.0 Issue #3703 Phase 5: SIMD optimization

use sqlrustgo_executor::simd_eval::{
    apply_mask, extract_i64_column, BatchPredicate, BitMask, EqualsPredicate, GreaterThanPredicate,
    LessThanPredicate,
};
use sqlrustgo_types::Value;

#[test]
fn test_bitmask_all_true() {
    let mask = BitMask::all_true(8);
    assert_eq!(mask.count(), 8);
    for i in 0..8 {
        assert!(mask.is_set(i));
    }
}

#[test]
fn test_bitmask_all_true_64() {
    let mask = BitMask::all_true(64);
    assert_eq!(mask.count(), 64);
}

#[test]
fn test_bitmask_all_false() {
    let mask = BitMask::all_false();
    assert_eq!(mask.count(), 0);
}

#[test]
fn test_less_than_predicate() {
    let pred = LessThanPredicate { threshold: 5 };
    let values: Vec<i64> = (0..10).collect();
    let mask = pred.eval_batch_i64(&values);

    for i in 0..5 {
        assert!(mask.is_set(i), "Index {} should pass", i);
    }
    for i in 5..10 {
        assert!(!mask.is_set(i), "Index {} should fail", i);
    }
}

#[test]
fn test_greater_than_predicate() {
    let pred = GreaterThanPredicate { threshold: 5 };
    let values: Vec<i64> = (0..10).collect();
    let mask = pred.eval_batch_i64(&values);

    for i in 0..=5 {
        assert!(!mask.is_set(i), "Index {} should fail", i);
    }
    for i in 6..10 {
        assert!(mask.is_set(i), "Index {} should pass", i);
    }
}

#[test]
fn test_equals_predicate() {
    let pred = EqualsPredicate { value: 5 };
    let values: Vec<i64> = (0..10).collect();
    let mask = pred.eval_batch_i64(&values);

    for i in 0..10 {
        if i == 5 {
            assert!(mask.is_set(i), "Index 5 should pass");
        } else {
            assert!(!mask.is_set(i), "Index {} should fail", i);
        }
    }
}

#[test]
fn test_simd_chunk_alignment() {
    let pred = LessThanPredicate { threshold: 100 };

    // Size exactly 4
    let v4: Vec<i64> = (0..4).collect();
    assert_eq!(pred.eval_batch_i64(&v4).count(), 4);

    // Size 5 (4 + 1 tail)
    let v5: Vec<i64> = (0..5).collect();
    assert_eq!(pred.eval_batch_i64(&v5).count(), 5);

    // Size 8 (2 chunks of 4)
    let v8: Vec<i64> = (0..8).collect();
    assert_eq!(pred.eval_batch_i64(&v8).count(), 8);

    // Size 17 (4 chunks of 4 + 1 tail)
    let v17: Vec<i64> = (0..17).collect();
    assert_eq!(pred.eval_batch_i64(&v17).count(), 17);
}

#[test]
fn test_extract_i64_column() {
    let records = vec![
        vec![Value::Integer(1), Value::Integer(10)],
        vec![Value::Integer(2), Value::Integer(20)],
        vec![Value::Integer(3), Value::Integer(30)],
    ];
    let col = extract_i64_column(&records, 0);
    assert_eq!(col, Some(vec![1, 2, 3]));

    let col = extract_i64_column(&records, 1);
    assert_eq!(col, Some(vec![10, 20, 30]));
}

#[test]
fn test_extract_i64_column_invalid_type() {
    let records = vec![vec![Value::Text("hello".into())]];
    let col = extract_i64_column(&records, 0);
    assert_eq!(col, None);
}

#[test]
fn test_extract_i64_column_out_of_bounds() {
    let records = vec![vec![Value::Integer(1)]];
    let col = extract_i64_column(&records, 5);
    assert_eq!(col, None);
}

#[test]
fn test_apply_mask() {
    let records = vec![
        vec![Value::Integer(1)],
        vec![Value::Integer(2)],
        vec![Value::Integer(3)],
        vec![Value::Integer(4)],
    ];
    let custom = BitMask::from_bits(0b0101); // Pass indices 0 and 2

    let result = apply_mask(&records, custom);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0], records[0]);
    assert_eq!(result[1], records[2]);
}

#[test]
fn test_composed_predicates() {
    let values: Vec<i64> = (0..10).collect();

    let pred1 = GreaterThanPredicate { threshold: 2 };
    let mask1 = pred1.eval_batch_i64(&values);

    let pred2 = LessThanPredicate { threshold: 8 };
    let mask2 = pred2.eval_batch_i64(&values);

    let combined = mask1.and(mask2);

    // 3, 4, 5, 6, 7 should pass
    assert!(!combined.is_set(0));
    assert!(!combined.is_set(1));
    assert!(!combined.is_set(2));
    for i in 3..8 {
        assert!(combined.is_set(i), "Index {} should pass combined", i);
    }
    assert!(!combined.is_set(8));
    assert!(!combined.is_set(9));
}
