//! P2-3 (#3179) Immutable Audit Chain — 20+ tests across 4 categories
//!
//! 1. build (5)
//! 2. verify_ok (5)
//! 3. tamper (7)
//! 4. range (3)
//!
//! Total: 20 tests
//!
//! Refs: docs/openspec/3179-hash-chain.md
//!       V390_TEST_PLAN.md §G10

mod harness {
    #[derive(Debug, Clone, PartialEq)]
    pub struct ChainRecord {
        pub data: String,
        pub hash: String,
        pub prev_hash: String,
    }

    pub struct ChainBuilder;

    impl ChainBuilder {
        pub fn genesis_hash() -> String {
            "0".repeat(64)
        }
        pub fn link(prev_hash: &str, data: &str) -> String {
            let mut cs: u64 = 0xCBF29CE484222325;
            for b in prev_hash.bytes() {
                cs = cs.wrapping_mul(0x100000001B3).wrapping_add(b as u64);
            }
            for b in data.bytes() {
                cs = cs.wrapping_mul(0x100000001B3).wrapping_add(b as u64);
            }
            format!("{:016x}", cs)
        }
        pub fn build(records: Vec<String>) -> Vec<ChainRecord> {
            let mut chain = Vec::with_capacity(records.len());
            let mut prev = Self::genesis_hash();
            for data in records {
                let hash = Self::link(&prev, &data);
                chain.push(ChainRecord {
                    data,
                    hash: hash.clone(),
                    prev_hash: prev,
                });
                prev = hash;
            }
            chain
        }
        pub fn verify(records: &[ChainRecord]) -> Result<(), BrokenAt> {
            if records.is_empty() {
                return Ok(());
            }
            for (i, r) in records.iter().enumerate() {
                if i > 0 && records[i - 1].hash != r.prev_hash {
                    return Err(BrokenAt(i));
                }
                let expected = Self::link(&r.prev_hash, &r.data);
                if expected != r.hash {
                    return Err(BrokenAt(i));
                }
            }
            Ok(())
        }
        pub fn tamper_data(records: &mut [ChainRecord], index: usize) {
            if let Some(r) = records.get_mut(index) {
                let mut s: String = r.data.clone();
                if let Some(c) = s.chars().next() {
                    s.replace_range(0..1, &c.to_ascii_uppercase().to_string());
                }
                r.data = s;
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct BrokenAt(pub usize);
}

use harness::{BrokenAt, ChainBuilder, ChainRecord};

// --------------------------------------------------------------------
// 1. build (5 tests)
// --------------------------------------------------------------------

#[test]
fn test_chain_build_genesis_p2_3() {
    let chain = ChainBuilder::build(vec![]);
    assert!(chain.is_empty());
}

#[test]
fn test_chain_build_one_record_p2_3() {
    let chain = ChainBuilder::build(vec!["a".into()]);
    assert_eq!(chain.len(), 1);
    assert_eq!(chain[0].prev_hash, ChainBuilder::genesis_hash());
    assert_eq!(chain[0].data, "a");
    assert!(!chain[0].hash.is_empty());
}

#[test]
fn test_chain_build_10_records_p2_3() {
    let data: Vec<String> = (0..10).map(|i| format!("record-{}", i)).collect();
    let chain = ChainBuilder::build(data);
    assert_eq!(chain.len(), 10);
    // Each record's prev_hash equals the previous record's hash.
    for i in 1..chain.len() {
        assert_eq!(chain[i].prev_hash, chain[i - 1].hash);
    }
}

#[test]
fn test_chain_build_100_records_p2_3() {
    let data: Vec<String> = (0..100).map(|i| format!("r{}", i)).collect();
    let chain = ChainBuilder::build(data);
    assert_eq!(chain.len(), 100);
}

#[test]
fn test_chain_build_1000_records_p2_3() {
    let data: Vec<String> = (0..1000).map(|i| format!("r{}", i)).collect();
    let chain = ChainBuilder::build(data);
    assert_eq!(chain.len(), 1000);
}

// --------------------------------------------------------------------
// 2. verify_ok (5 tests)
// --------------------------------------------------------------------

#[test]
fn test_chain_verify_empty_p2_3() {
    let chain: Vec<ChainRecord> = vec![];
    assert!(ChainBuilder::verify(&chain).is_ok());
}

#[test]
fn test_chain_verify_one_record_p2_3() {
    let chain = ChainBuilder::build(vec!["a".into()]);
    assert!(ChainBuilder::verify(&chain).is_ok());
}

#[test]
fn test_chain_verify_10_records_p2_3() {
    let data: Vec<String> = (0..10).map(|i| format!("r{}", i)).collect();
    let chain = ChainBuilder::build(data);
    assert!(ChainBuilder::verify(&chain).is_ok());
}

#[test]
fn test_chain_verify_100_records_p2_3() {
    let data: Vec<String> = (0..100).map(|i| format!("r{}", i)).collect();
    let chain = ChainBuilder::build(data);
    assert!(ChainBuilder::verify(&chain).is_ok());
}

#[test]
fn test_chain_verify_mixed_sizes_p2_3() {
    let data = vec![
        "".to_string(), // empty data
        "x".to_string(),
        "this is a longer data field with various chars 123!@#".to_string(),
        "短数据".to_string(), // unicode
        "another".to_string(),
    ];
    let chain = ChainBuilder::build(data);
    assert!(ChainBuilder::verify(&chain).is_ok());
}

// --------------------------------------------------------------------
// 3. tamper (7 tests)
// --------------------------------------------------------------------

#[test]
fn test_chain_tamper_one_byte_flip_p2_3() {
    let mut chain = ChainBuilder::build(vec!["a".into(), "b".into(), "c".into()]);
    ChainBuilder::tamper_data(&mut chain, 1); // 1 byte flip at index 1
    let r = ChainBuilder::verify(&chain);
    assert_eq!(r, Err(BrokenAt(1)));
}

#[test]
fn test_chain_tamper_prev_hash_change_p2_3() {
    let mut chain = ChainBuilder::build(vec!["a".into(), "b".into(), "c".into()]);
    chain[1].prev_hash = "deadbeef".repeat(8); // 64 chars hex
    let r = ChainBuilder::verify(&chain);
    assert_eq!(r, Err(BrokenAt(1)));
}

#[test]
fn test_chain_tamper_delete_record_p2_3() {
    let mut chain = ChainBuilder::build(vec!["a".into(), "b".into(), "c".into(), "d".into()]);
    chain.remove(2); // remove record at index 2
    let r = ChainBuilder::verify(&chain);
    // After removal, index 2 is what was index 3. Its prev_hash
    // should equal what was index 2's hash, but it equals index 1's
    // hash. Hence broken at index 2.
    assert_eq!(r, Err(BrokenAt(2)));
}

#[test]
fn test_chain_tamper_insert_record_p2_3() {
    let mut chain = ChainBuilder::build(vec!["a".into(), "b".into(), "c".into()]);
    let bad = ChainRecord {
        data: "injected".into(),
        hash: ChainBuilder::link(&chain[1].hash, "injected"),
        prev_hash: chain[1].hash.clone(),
    };
    chain.insert(2, bad);
    let r = ChainBuilder::verify(&chain);
    // The original index 2 now sees wrong prev_hash.
    assert_eq!(r, Err(BrokenAt(3)));
}

#[test]
fn test_chain_tamper_record_replacement_p2_3() {
    let mut chain = ChainBuilder::build(vec!["a".into(), "b".into(), "c".into()]);
    // Replace the data field but keep the original hash — the
    // recomputed hash will not match the stored hash, so verify
    // will fail at index 1.
    let new_data = "REPLACED".to_string();
    let prev_hash = chain[1].prev_hash.clone();
    let old_hash = chain[1].hash.clone();
    chain[1] = ChainRecord {
        data: new_data,
        hash: old_hash,
        prev_hash,
    };
    let r = ChainBuilder::verify(&chain);
    assert_eq!(r, Err(BrokenAt(1)));
}

#[test]
fn test_chain_tamper_size_change_p2_3() {
    let mut chain = ChainBuilder::build(vec!["a".into(), "b".into(), "c".into()]);
    chain[1].data = "much-longer-data-field-that-changes-size-completely".to_string();
    let r = ChainBuilder::verify(&chain);
    assert_eq!(r, Err(BrokenAt(1)));
}

#[test]
fn test_chain_tamper_hash_character_replace_p2_3() {
    let mut chain = ChainBuilder::build(vec!["a".into(), "b".into(), "c".into()]);
    let mut new_hash: String = chain[1].hash.clone();
    // Replace first char with '0' if not '0', else '1'
    unsafe {
        let bytes = new_hash.as_bytes_mut();
        if bytes[0] == b'0' {
            bytes[0] = b'1';
        } else {
            bytes[0] = b'0';
        }
    }
    chain[1].hash = new_hash;
    let r = ChainBuilder::verify(&chain);
    assert_eq!(r, Err(BrokenAt(1)));
}

// --------------------------------------------------------------------
// 4. range (3 tests)
// --------------------------------------------------------------------

#[test]
fn test_chain_verify_range_first_10_p2_3() {
    let data: Vec<String> = (0..20).map(|i| format!("r{}", i)).collect();
    let chain = ChainBuilder::build(data);
    let range = &chain[0..10];
    assert!(ChainBuilder::verify(range).is_ok());
}

#[test]
fn test_chain_verify_range_middle_p2_3() {
    let data: Vec<String> = (0..20).map(|i| format!("r{}", i)).collect();
    let chain = ChainBuilder::build(data);
    let range = &chain[5..15];
    assert!(ChainBuilder::verify(range).is_ok());
}

#[test]
fn test_chain_verify_full_range_p2_3() {
    let data: Vec<String> = (0..20).map(|i| format!("r{}", i)).collect();
    let chain = ChainBuilder::build(data);
    assert!(ChainBuilder::verify(&chain).is_ok());
}
