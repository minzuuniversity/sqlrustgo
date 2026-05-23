//! Null bitmap utilities for Compact Row v1 format.

/// Get the number of bytes needed for a null bitmap with n columns.
pub fn null_bitmap_size(num_columns: usize) -> usize {
    num_columns.div_ceil(8)
}

/// Encode a null bitmap from a vector of booleans.
/// Each true value means NULL, false means NOT NULL.
pub fn encode_null_bitmap(nulls: &[bool]) -> Vec<u8> {
    let size = null_bitmap_size(nulls.len());
    let mut bitmap = vec![0u8; size];
    for (i, &is_null) in nulls.iter().enumerate() {
        if is_null {
            let byte_index = i / 8;
            let bit_index = i % 8;
            bitmap[byte_index] |= 1 << bit_index;
        }
    }
    bitmap
}

/// Check if a column is null given the null bitmap.
pub fn is_null(bitmap: &[u8], column_index: usize) -> bool {
    if column_index >= bitmap.len() * 8 {
        return false;
    }
    let byte_index = column_index / 8;
    let bit_index = column_index % 8;
    (bitmap[byte_index] & (1 << bit_index)) != 0
}

/// Decode a null bitmap to a vector of booleans.
/// Each true value means NULL, false means NOT NULL.
/// Returns exactly `expected_columns` elements.
pub fn decode_null_bitmap(bitmap: Vec<u8>, expected_columns: usize) -> Vec<bool> {
    let mut nulls = Vec::with_capacity(expected_columns);
    for &byte in &bitmap {
        for bit in 0..8 {
            if nulls.len() >= expected_columns {
                break;
            }
            nulls.push((byte & (1 << bit)) != 0);
        }
    }
    nulls.truncate(expected_columns);
    nulls
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null_bitmap_size() {
        assert_eq!(null_bitmap_size(0), 0);
        assert_eq!(null_bitmap_size(1), 1);
        assert_eq!(null_bitmap_size(7), 1);
        assert_eq!(null_bitmap_size(8), 1);
        assert_eq!(null_bitmap_size(9), 2);
        assert_eq!(null_bitmap_size(16), 2);
        assert_eq!(null_bitmap_size(17), 3);
    }

    #[test]
    fn test_encode_null_bitmap_empty() {
        assert_eq!(encode_null_bitmap(&[]), vec![0u8; 0]);
    }

    #[test]
    fn test_encode_null_bitmap_no_nulls() {
        assert_eq!(encode_null_bitmap(&[false, false, false]), vec![0u8]);
    }

    #[test]
    fn test_encode_null_bitmap_all_nulls() {
        assert_eq!(
            encode_null_bitmap(&[true, true, true, true, true, true, true, true]),
            [0xFF]
        );
    }

    #[test]
    fn test_encode_null_bitmap_mixed() {
        // [N, N, V, V, N, V, V, V] where N=null, V=not null
        let bitmap = encode_null_bitmap(&[true, true, false, false, true, false, false, false]);
        assert_eq!(bitmap[0], 0b00010011); // bits 0,1,4 set
    }

    #[test]
    fn test_encode_null_bitmap_multi_byte() {
        // 9 columns: 8 nulls + 1 not null
        let bitmap2 = encode_null_bitmap(&[true; 9]);
        assert_eq!(bitmap2, vec![0xFF, 0x01]); // second byte bit 0 = column 8 is null
    }

    #[test]
    fn test_is_null_out_of_bounds() {
        assert_eq!(is_null(&[0], 8), false);
        assert_eq!(is_null(&[0], 100), false);
    }

    #[test]
    fn test_is_null_not_set() {
        assert_eq!(is_null(&[0b00000000], 0), false);
        assert_eq!(is_null(&[0b00000000], 7), false);
    }

    #[test]
    fn test_is_null_set() {
        assert_eq!(is_null(&[0b00000001], 0), true);
        assert_eq!(is_null(&[0b00000010], 1), true);
        assert_eq!(is_null(&[0b10000000], 7), true);
    }

    #[test]
    fn test_is_null_second_byte() {
        assert_eq!(is_null(&[0, 0b00000001], 8), true);
        assert_eq!(is_null(&[0, 0b00000000], 8), false);
    }

    #[test]
    fn test_decode_null_bitmap_empty() {
        let result: Vec<bool> = decode_null_bitmap(vec![], 0);
        assert!(result.is_empty());
    }

    #[test]
    fn test_decode_null_bitmap_all_not_null() {
        assert_eq!(decode_null_bitmap(vec![0], 3), [false, false, false]);
    }

    #[test]
    fn test_decode_null_bitmap_all_null() {
        assert_eq!(
            decode_null_bitmap(vec![0xFF], 8),
            [true, true, true, true, true, true, true, true]
        );
    }

    #[test]
    fn test_decode_null_bitmap_truncates() {
        // 8 bytes but only ask for 3
        assert_eq!(decode_null_bitmap(vec![0xFF], 3).len(), 3);
    }

    #[test]
    fn test_decode_null_bitmap_mixed() {
        // Byte 0x33 = 0b00110011: bits 0,1,4,5 are set
        let nulls = decode_null_bitmap(vec![0b00110011], 8);
        assert_eq!(nulls[0], true); // bit 0
        assert_eq!(nulls[1], true); // bit 1
        assert_eq!(nulls[2], false); // bit 2
        assert_eq!(nulls[3], false); // bit 3
        assert_eq!(nulls[4], true); // bit 4
        assert_eq!(nulls[5], true); // bit 5
        assert_eq!(nulls[6], false); // bit 6
        assert_eq!(nulls[7], false); // bit 7
    }
}
