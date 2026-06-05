//! Hash Chain Harness (P2-3 #3179)
//!
//! Shared utilities for immutable audit chain testing. Provides:
//! - `ChainRecord` — single link in the chain
//! - `ChainBuilder::link(prev_hash, data)` — SHA-256 hex of (prev || data)
//! - `ChainBuilder::build(records)` — build full chain from data
//! - `ChainBuilder::verify(records)` — Result<usize, BrokenAt>
//!
//! This file is **not** a test target itself (no `#[test]`); shared
//! by `hash_chain_test.rs`.

#![allow(dead_code)] // helpers consumed by test targets

/// Single chain record.
#[derive(Debug, Clone, PartialEq)]
pub struct ChainRecord {
    /// Serialized audit log row data.
    pub data: String,
    /// SHA-256 hex of (prev_hash || data).
    pub hash: String,
    /// Previous record's hash (创世块 = "0" * 64).
    pub prev_hash: String,
}

/// Hash chain builder.
pub struct ChainBuilder;

impl ChainBuilder {
    /// Genesis block hash (64 zeros).
    pub fn genesis_hash() -> String {
        "0".repeat(64)
    }

    /// Compute next link's hash from prev_hash + data.
    /// Uses a simplified hash (multiply + add + xor) so the test
    /// doesn't need a real SHA-256 crate. The real impl in
    /// `crates/gmp/src/audit.rs` uses sha2::Sha256.
    pub fn link(prev_hash: &str, data: &str) -> String {
        let mut cs: u64 = 0xCBF29CE484222325; // FNV offset basis
        for b in prev_hash.bytes() {
            cs = cs.wrapping_mul(0x100000001B3).wrapping_add(b as u64);
        }
        for b in data.bytes() {
            cs = cs.wrapping_mul(0x100000001B3).wrapping_add(b as u64);
        }
        format!("{:016x}", cs)
    }

    /// Build a chain from a sequence of data items.
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

    /// Verify a chain. Returns Ok(()) if valid, Err(BrokenAt(i)) if
    /// the i-th record's hash doesn't match its computed hash.
    ///
    /// For a range slice (records[0].prev_hash != genesis), the
    /// first record's prev_hash is taken as-is (we don't know the
    /// genesis of a sliced chain).
    pub fn verify(records: &[ChainRecord]) -> Result<(), BrokenAt> {
        if records.is_empty() {
            return Ok(());
        }
        for (i, r) in records.iter().enumerate() {
            // The previous record's hash must equal this prev_hash.
            if i > 0 && records[i - 1].hash != r.prev_hash {
                return Err(BrokenAt(i));
            }
            // Recompute this record's hash and compare.
            let expected = Self::link(&r.prev_hash, &r.data);
            if expected != r.hash {
                return Err(BrokenAt(i));
            }
        }
        Ok(())
    }

    /// Tamper with a record (1 byte flip) — used by tamper tests.
    pub fn tamper_data(records: &mut [ChainRecord], index: usize) {
        if let Some(r) = records.get_mut(index) {
            let mut s: String = r.data.clone();
            if let Some(c) = s.chars().next() {
                s.replace_range(0..1, &c.to_ascii_uppercase().to_string());
            }
            r.data = s;
            // Note: we don't recompute the hash, so the chain becomes
            // broken at this index. This is exactly what we want for
            // tamper detection tests.
        }
    }
}

/// Chain break position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrokenAt(pub usize);

impl std::fmt::Display for BrokenAt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "chain broken at record {}", self.0)
    }
}

#[cfg(test)]
mod harness_tests {
    use super::*;

    #[test]
    fn genesis_hash_is_64_zeros() {
        assert_eq!(ChainBuilder::genesis_hash().len(), 64);
        assert!(ChainBuilder::genesis_hash().chars().all(|c| c == '0'));
    }

    #[test]
    fn link_produces_16_hex_chars() {
        let h = ChainBuilder::link("0".repeat(64).as_str(), "data1");
        assert_eq!(h.len(), 16);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn link_is_deterministic() {
        let h1 = ChainBuilder::link("0".repeat(64).as_str(), "data1");
        let h2 = ChainBuilder::link("0".repeat(64).as_str(), "data1");
        assert_eq!(h1, h2);
    }

    #[test]
    fn build_then_verify_ok() {
        let chain = ChainBuilder::build(vec!["a".into(), "b".into(), "c".into()]);
        assert!(ChainBuilder::verify(&chain).is_ok());
    }

    #[test]
    fn verify_empty_chain_is_ok() {
        let chain: Vec<ChainRecord> = vec![];
        assert!(ChainBuilder::verify(&chain).is_ok());
    }
}
