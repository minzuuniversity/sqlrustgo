//! Partition key hashing.
//!
//! Twox-hash (64-bit) provides cheap, high-quality distribution. The
//! hash modulo shard count gives a stable shard index for any input
//! value. We deliberately do NOT use `std::collections::hash_map::DefaultHasher`
//! because its output is not specified to be stable across Rust versions
//! or runs — tests rely on deterministic routing.

use std::hash::Hasher;
use twox_hash::XxHash64;

/// Hash any byte sequence into a stable 64-bit fingerprint, then
/// fold into `[0, num_shards)`. Returns 0 when `num_shards == 0`
/// (caller is responsible for validating this).
pub fn shard_index_for(bytes: &[u8], num_shards: usize) -> usize {
    if num_shards == 0 {
        return 0;
    }
    let mut h = XxHash64::with_seed(0xC1B2_A2D8_FA57_1E2B);
    h.write(bytes);
    let digest = h.finish();
    (digest as usize) % num_shards
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        let a = shard_index_for(b"42", 4);
        let b = shard_index_for(b"42", 4);
        assert_eq!(a, b);
    }

    #[test]
    fn distribution_reasonable() {
        let n = 4;
        let mut counts = [0usize; 4];
        for i in 0..10_000u32 {
            let s = i.to_string();
            let idx = shard_index_for(s.as_bytes(), n);
            counts[idx] += 1;
        }
        // Roughly uniform — each shard should get 2000-3000 out of 10000
        for (i, &c) in counts.iter().enumerate() {
            assert!(
                c > 1500 && c < 3500,
                "shard {i} got {c}, expected ~2500 (uneven distribution)"
            );
        }
    }

    #[test]
    fn zero_shards_safe() {
        // Must not panic; returns 0
        assert_eq!(shard_index_for(b"x", 0), 0);
    }
}
