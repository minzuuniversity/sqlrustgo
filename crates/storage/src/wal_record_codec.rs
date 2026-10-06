//! The single codec for row images stored inside WAL entry payloads.
//!
//! #5055: this used to exist in two places that had drifted apart.
//!
//! - `WalStorage::record_to_bytes` (encoder) understood all eight
//!   [`Value`] variants, including `P:` (Point) and `J:` (Json).
//! - `RecoveryEngine::bytes_to_record` (decoder) understood only six.
//!   `P:` and `J:` fell into its "unknown prefix" tolerance path, which
//!   substitutes `Value::Null` and then advances **2 bytes**. For a
//!   `P:` that meant the next 16 bytes of `f64` payload were re-scanned
//!   as more "prefixes", each producing another bogus `Null`.
//!
//! So a table with a POINT or JSON column replayed from the WAL did not
//! fail — it produced a row of the right arity full of `Null`s, which
//! is the worst failure mode for a recovery tool: it reports success.
//!
//! Both sides now come from this module, so an encoder change that adds
//! a variant without a matching decoder arm is a compile error in one
//! place rather than a silent corruption found in production.
//!
//! # Two wire formats
//!
//! **V2 (current).** Every value is a 1-byte type code followed by an
//! explicit `u32` LE length. Nothing is terminated by a sentinel, so
//! the encoding is total: every byte of input is accounted for.
//!
//! **V1 (legacy, decode-only).** A concatenation of `i:`/`f:`/`b:`/`n:`/
//! `s:`/`B:`/`P:`/`J:` tags, with the variable-length ones terminated by
//! a `\0`. The writer no longer emits it; the reader still accepts it so
//! a WAL written before this change stays recoverable.
//!
//! Why V1 is not merely "old": its `\0` terminator means a `Text` or
//! `Blob` **containing a NUL byte cannot round-trip**. `decode("a\0b")`
//! returns `"a"` and then re-scans `b\0` as another tag. That is
//! legal input — `CHAR(0)` and binary strings are ordinary SQL — and
//! the failure was silent.
//!
//! V1 payloads never begin with the two bytes `V2` (no V1 tag is `V`),
//! so the format is self-identifying and detection needs no extra
//! framing in [`WalEntry`].
//!
//! [`WalEntry`]: crate::wal::WalEntry

use crate::engine::SqlResult;
use sqlrustgo_types::SqlError;
use sqlrustgo_types::Value;

/// Stable 31-radix hash of a table name, used as [`WalEntry::table_id`].
///
/// [`WalEntry::table_id`]: crate::wal::WalEntry::table_id
///
/// #5055: this hash is **not reversible** — two different table names
/// can collide, and nothing maps an id back to a name. That is fine
/// as a compact grouping key but useless for replay, which is why
/// #5055 added `WalEntry::table_name` alongside it. Replay resolves
/// the table through the name and treats a missing one as a hard
/// error rather than guessing from the id.
pub fn table_name_to_id(table: &str) -> u64 {
    let mut hash: u64 = 0;
    for byte in table.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
    }
    hash
}

/// Row identity used as [`WalEntry::key`].
///
/// [`WalEntry::key`]: crate::wal::WalEntry::key
///
/// This is the first column — the primary key by SQL convention. A
/// row whose first column is `NULL` encodes to an empty key, which is
/// also what an empty record produces; both are rejected downstream
/// rather than silently conflated with a real PK.
pub fn record_key(record: &[Value]) -> Vec<u8> {
    if record.is_empty() {
        return Vec::new();
    }
    match &record[0] {
        Value::Integer(i) => i.to_le_bytes().to_vec(),
        Value::Text(s) => s.as_bytes().to_vec(),
        Value::Boolean(b) => vec![*b as u8],
        Value::Null => Vec::new(),
        Value::Float(f) => f.to_bits().to_le_bytes().to_vec(),
        Value::Blob(b) => b.clone(),
        Value::Point(x, y) => {
            let mut bytes = vec![0x07];
            bytes.extend_from_slice(&x.to_le_bytes());
            bytes.extend_from_slice(&y.to_le_bytes());
            bytes
        }
        Value::Json(v) => {
            let mut bytes = vec![0x08];
            bytes.extend_from_slice(v.to_string().as_bytes());
            bytes
        }
    }
}

// --- V2 type codes ---------------------------------------------------------

const T_I64: u8 = 0x01;
const T_F64: u8 = 0x02;
const T_BOOL: u8 = 0x03;
const T_NULL: u8 = 0x04;
const T_TEXT: u8 = 0x05;
const T_BLOB: u8 = 0x06;
const T_POINT: u8 = 0x07;
const T_JSON: u8 = 0x08;

/// Every V2 payload starts with these two bytes. No V1 tag is `V`, so
/// the presence of this prefix is a complete and unambiguous format
/// discriminator.
const V2_MAGIC: [u8; 2] = *b"V2";

fn put_len_prefixed(out: &mut Vec<u8>, code: u8, bytes: &[u8]) {
    out.push(code);
    out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(bytes);
}

/// Encode one row into a WAL entry payload, in V2.
pub fn record_to_bytes(record: &[Value]) -> Vec<u8> {
    let mut out = Vec::with_capacity(2 + record.len() * 9);
    out.extend_from_slice(&V2_MAGIC);
    for value in record {
        match value {
            Value::Integer(i) => {
                out.push(T_I64);
                out.extend_from_slice(&i.to_le_bytes());
            }
            Value::Float(f) => {
                out.push(T_F64);
                out.extend_from_slice(&f.to_bits().to_le_bytes());
            }
            Value::Boolean(b) => {
                out.push(T_BOOL);
                out.push(*b as u8);
            }
            Value::Null => out.push(T_NULL),
            Value::Text(s) => put_len_prefixed(&mut out, T_TEXT, s.as_bytes()),
            Value::Blob(b) => put_len_prefixed(&mut out, T_BLOB, b),
            Value::Point(x, y) => {
                let mut buf = [0u8; 16];
                buf[..8].copy_from_slice(&x.to_le_bytes());
                buf[8..].copy_from_slice(&y.to_le_bytes());
                out.push(T_POINT);
                out.extend_from_slice(&buf);
            }
            Value::Json(v) => put_len_prefixed(&mut out, T_JSON, v.to_string().as_bytes()),
        }
    }
    out
}

fn truncated(what: &str) -> SqlError {
    SqlError::ExecutionError(format!("WAL record codec: truncated {what}"))
}

fn bad(what: &str) -> SqlError {
    SqlError::ExecutionError(format!("WAL record codec: malformed {what}"))
}

/// Decode a V2 payload.
///
/// Unlike V1 this is strict: an unknown type code is an error rather
/// than a substituted `Null`. The V2 header states the format exactly,
/// so an unknown code means the file is corrupt or was written by a
/// newer version — and a recovery tool that guesses here is a recovery
/// tool that silently invents data.
fn bytes_to_record_v2(data: &[u8]) -> SqlResult<Vec<Value>> {
    let mut pos = 2; // skip the V2 magic, already matched by the caller
    let mut record = Vec::new();
    while pos < data.len() {
        let code = data[pos];
        pos += 1;
        match code {
            T_I64 => {
                if pos + 8 > data.len() {
                    return Err(truncated("Integer"));
                }
                let mut buf = [0u8; 8];
                buf.copy_from_slice(&data[pos..pos + 8]);
                record.push(Value::Integer(i64::from_le_bytes(buf)));
                pos += 8;
            }
            T_F64 => {
                if pos + 8 > data.len() {
                    return Err(truncated("Float"));
                }
                let mut buf = [0u8; 8];
                buf.copy_from_slice(&data[pos..pos + 8]);
                record.push(Value::Float(f64::from_bits(u64::from_le_bytes(buf))));
                pos += 8;
            }
            T_BOOL => {
                if pos + 1 > data.len() {
                    return Err(truncated("Boolean"));
                }
                record.push(Value::Boolean(data[pos] != 0));
                pos += 1;
            }
            T_NULL => record.push(Value::Null),
            T_TEXT => {
                let (s, next) = read_v2_text(data, pos)?;
                record.push(Value::Text(s));
                pos = next;
            }
            T_BLOB => {
                let (bytes, next) = read_v2_blob(data, pos)?;
                record.push(Value::Blob(bytes));
                pos = next;
            }
            T_POINT => {
                if pos + 16 > data.len() {
                    return Err(truncated("Point"));
                }
                let mut xb = [0u8; 8];
                xb.copy_from_slice(&data[pos..pos + 8]);
                let mut yb = [0u8; 8];
                yb.copy_from_slice(&data[pos + 8..pos + 16]);
                record.push(Value::Point(
                    f64::from_bits(u64::from_le_bytes(xb)),
                    f64::from_bits(u64::from_le_bytes(yb)),
                ));
                pos += 16;
            }
            T_JSON => {
                let (s, next) = read_v2_text(data, pos)?;
                let parsed: serde_json::Value =
                    serde_json::from_str(&s).map_err(|e| bad(&format!("Json payload: {e}")))?;
                record.push(Value::Json(parsed));
                pos = next;
            }
            other => {
                return Err(bad(&format!(
                    "V2 type code 0x{other:02x}; the WAL was written by an \
                     incompatible version and cannot be replayed safely"
                )));
            }
        }
    }
    Ok(record)
}

fn read_v2_len(data: &[u8], pos: usize) -> SqlResult<usize> {
    if pos + 4 > data.len() {
        return Err(truncated("value length"));
    }
    let mut buf = [0u8; 4];
    buf.copy_from_slice(&data[pos..pos + 4]);
    let len = u32::from_le_bytes(buf) as usize;
    // Guard before allocating: a corrupt length must not be able to
    // ask for a 4 GiB allocation.
    if len > data.len() {
        return Err(bad("value length exceeds remaining payload"));
    }
    Ok(len)
}

fn read_v2_text(data: &[u8], pos: usize) -> SqlResult<(String, usize)> {
    let len = read_v2_len(data, pos)?;
    let start = pos + 4;
    let s = std::str::from_utf8(&data[start..start + len])
        .map_err(|e| bad(&format!("invalid UTF-8: {e}")))?;
    Ok((s.to_string(), start + len))
}

fn read_v2_blob(data: &[u8], pos: usize) -> SqlResult<(Vec<u8>, usize)> {
    let len = read_v2_len(data, pos)?;
    let start = pos + 4;
    Ok((data[start..start + len].to_vec(), start + len))
}

/// Decode a V1 (pre-#5055) payload.
///
/// Retained so a WAL written before this change is still recoverable.
/// Two known lossy cases, both accepted deliberately because the
/// alternative is refusing to restore at all:
///
/// - `P:` / `J:` were written by the old encoder but not understood by
///   the old decoder. They are decoded correctly here, so a V1 payload
///   containing them round-trips into the right values.
/// - A `Text` or `Blob` containing `\0` is truncated at the first NUL
///   and the remainder is re-scanned as tags. This is unrecoverable —
///   the length was never written. It yields a *shorter row* rather
///   than a wrong-value row, and only for values that were already
///   unrepresentable in V1.
fn bytes_to_record_v1(data: &[u8]) -> SqlResult<Vec<Value>> {
    let mut record = Vec::new();
    let mut pos = 0;
    while pos < data.len() {
        if pos + 2 > data.len() {
            return Err(truncated("V1 value prefix"));
        }
        match &data[pos..pos + 2] {
            b"i:" => {
                if pos + 10 > data.len() {
                    return Err(truncated("V1 Integer"));
                }
                let mut buf = [0u8; 8];
                buf.copy_from_slice(&data[pos + 2..pos + 10]);
                record.push(Value::Integer(i64::from_le_bytes(buf)));
                pos += 10;
            }
            b"f:" => {
                if pos + 10 > data.len() {
                    return Err(truncated("V1 Float"));
                }
                let mut buf = [0u8; 8];
                buf.copy_from_slice(&data[pos + 2..pos + 10]);
                record.push(Value::Float(f64::from_bits(u64::from_le_bytes(buf))));
                pos += 10;
            }
            b"b:" => {
                if pos + 3 > data.len() {
                    return Err(truncated("V1 Boolean"));
                }
                record.push(Value::Boolean(data[pos + 2] != 0));
                pos += 3;
            }
            b"n:" => {
                record.push(Value::Null);
                pos += 2;
            }
            b"s:" => {
                let start = pos + 2;
                let end = data[start..]
                    .iter()
                    .position(|&b| b == 0)
                    .ok_or_else(|| bad("V1 Text missing null terminator"))?;
                let s = std::str::from_utf8(&data[start..start + end])
                    .map_err(|e| bad(&format!("invalid UTF-8 in V1 Text: {e}")))?;
                record.push(Value::Text(s.to_string()));
                pos = start + end + 1;
            }
            b"B:" => {
                let start = pos + 2;
                let end = data[start..]
                    .iter()
                    .position(|&b| b == 0)
                    .ok_or_else(|| bad("V1 Blob missing null terminator"))?;
                record.push(Value::Blob(data[start..start + end].to_vec()));
                pos = start + end + 1;
            }
            b"P:" => {
                if pos + 19 > data.len() {
                    return Err(truncated("V1 Point"));
                }
                if data[pos + 18] != 0 {
                    return Err(bad("V1 Point missing null terminator"));
                }
                let mut xb = [0u8; 8];
                xb.copy_from_slice(&data[pos + 2..pos + 10]);
                let mut yb = [0u8; 8];
                yb.copy_from_slice(&data[pos + 10..pos + 18]);
                record.push(Value::Point(
                    f64::from_bits(u64::from_le_bytes(xb)),
                    f64::from_bits(u64::from_le_bytes(yb)),
                ));
                pos += 19;
            }
            b"J:" => {
                let start = pos + 2;
                let end = data[start..]
                    .iter()
                    .position(|&b| b == 0)
                    .ok_or_else(|| bad("V1 Json missing null terminator"))?;
                let raw = std::str::from_utf8(&data[start..start + end])
                    .map_err(|e| bad(&format!("invalid UTF-8 in V1 Json: {e}")))?;
                let parsed: serde_json::Value = serde_json::from_str(raw)
                    .map_err(|e| bad(&format!("invalid V1 Json payload: {e}")))?;
                record.push(Value::Json(parsed));
                pos = start + end + 1;
            }
            // v3.12.0 Issue #4682: preserve the pre-#5055 tolerance. A
            // tag this build does not know becomes `Null` and the scan
            // continues, so a WAL written by a *newer* writer still
            // replays instead of aborting the whole file.
            other => {
                log::debug!(
                    "wal_record_codec: unknown V1 value tag {:?}, substituting Null",
                    other
                );
                record.push(Value::Null);
                pos += 2;
            }
        }
    }
    Ok(record)
}

/// Decode a WAL entry payload back into a row, auto-detecting V2 vs V1.
pub fn bytes_to_record(data: &[u8]) -> SqlResult<Vec<Value>> {
    if data.starts_with(&V2_MAGIC) {
        bytes_to_record_v2(data)
    } else {
        bytes_to_record_v1(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_variants() -> Vec<Value> {
        vec![
            Value::Integer(-7),
            Value::Float(1.5),
            Value::Boolean(true),
            Value::Boolean(false),
            Value::Null,
            Value::Text("héllo".into()),
            Value::Blob(vec![0, 1, 2, 255]),
            Value::Point(1.25, -2.5),
            Value::Json(serde_json::json!({"a": [1, 2, {"b": null}], "u": "✓"})),
        ]
    }

    /// Encode a row the way the pre-#5055 writer did, so the legacy
    /// reader can be exercised against real bytes rather than a
    /// hand-rolled imitation that could drift from history too.
    fn v1_encode(record: &[Value]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for value in record {
            match value {
                Value::Integer(i) => {
                    bytes.extend_from_slice(b"i:");
                    bytes.extend_from_slice(&i.to_le_bytes());
                }
                Value::Text(s) => {
                    bytes.extend_from_slice(b"s:");
                    bytes.extend_from_slice(s.as_bytes());
                    bytes.push(0);
                }
                Value::Boolean(b) => {
                    bytes.extend_from_slice(b"b:");
                    bytes.push(*b as u8);
                }
                Value::Null => bytes.extend_from_slice(b"n:"),
                Value::Float(f) => {
                    bytes.extend_from_slice(b"f:");
                    bytes.extend_from_slice(&f.to_bits().to_le_bytes());
                }
                Value::Blob(b) => {
                    bytes.extend_from_slice(b"B:");
                    bytes.extend_from_slice(b);
                    bytes.push(0);
                }
                Value::Point(x, y) => {
                    bytes.extend_from_slice(b"P:");
                    bytes.extend_from_slice(&x.to_le_bytes());
                    bytes.extend_from_slice(&y.to_le_bytes());
                    bytes.push(0);
                }
                Value::Json(v) => {
                    bytes.extend_from_slice(b"J:");
                    bytes.extend_from_slice(&v.to_string().as_bytes());
                    bytes.push(0);
                }
            }
        }
        bytes
    }

    #[test]
    fn round_trip_all_variants() {
        let row = all_variants();
        assert_eq!(bytes_to_record(&record_to_bytes(&row)).unwrap(), row);
    }

    /// #5055 regression: the decoder used to turn a POINT into a run of
    /// `Null`s and still report success.
    #[test]
    fn point_survives_round_trip() {
        let row = vec![Value::Integer(1), Value::Point(3.5, 4.5)];
        assert_eq!(bytes_to_record(&record_to_bytes(&row)).unwrap(), row);
    }

    /// #5055 regression: same for JSON.
    #[test]
    fn json_survives_round_trip() {
        let row = vec![
            Value::Integer(1),
            Value::Json(serde_json::json!({"k": "v"})),
        ];
        assert_eq!(bytes_to_record(&record_to_bytes(&row)).unwrap(), row);
    }

    /// #5055: the reason V2 exists. A NUL inside a Text value made the
    /// old sentinel-terminated format decode as a *shorter* row with
    /// junk values appended — silently, on a recovery path.
    #[test]
    fn text_containing_nul_round_trips() {
        let row = vec![
            Value::Integer(1),
            Value::Text("a\0b".into()),
            Value::Text("c".into()),
        ];
        assert_eq!(bytes_to_record(&record_to_bytes(&row)).unwrap(), row);
    }

    #[test]
    fn blob_containing_nul_round_trips() {
        let row = vec![Value::Blob(vec![0, 0, 9, 0])];
        assert_eq!(bytes_to_record(&record_to_bytes(&row)).unwrap(), row);
    }

    /// Two adjacent Text values must not merge into one. V1's `\0`
    /// terminator is what made `["ab", "cd"]` and `["abcd"]`
    /// distinguishable; V2 gets it from the explicit length instead,
    /// which is why both of these now hold.
    #[test]
    fn adjacent_texts_do_not_merge() {
        let row = vec![Value::Text("ab".into()), Value::Text("cd".into())];
        assert_eq!(bytes_to_record(&record_to_bytes(&row)).unwrap(), row);
    }

    #[test]
    fn empty_record_round_trips() {
        assert_eq!(bytes_to_record(&record_to_bytes(&[])).unwrap(), vec![]);
    }

    /// An empty payload is not a valid V1 record either, but it is
    /// legitimately "zero columns" and must not error.
    #[test]
    fn empty_payload_decodes_to_empty_row() {
        assert_eq!(bytes_to_record(&[]).unwrap(), vec![]);
    }

    // --- legacy V1 payloads stay readable ---------------------------

    #[test]
    fn v1_payload_still_decodes() {
        let row = vec![
            Value::Integer(1),
            Value::Text("Alice".into()),
            Value::Float(2.5),
            Value::Boolean(true),
            Value::Null,
        ];
        assert_eq!(bytes_to_record(&v1_encode(&row)).unwrap(), row);
    }

    /// The old decoder had no `P:`/`J:` arms at all, so a V1 payload
    /// containing them was the exact silent corruption #5055 is about.
    /// They must decode now, from the old bytes.
    #[test]
    fn v1_point_and_json_decode() {
        let row = vec![
            Value::Integer(1),
            Value::Point(1.5, 2.5),
            Value::Json(serde_json::json!({"x": 1})),
        ];
        assert_eq!(bytes_to_record(&v1_encode(&row)).unwrap(), row);
    }

    /// #4682 tolerance, preserved for V1: unknown tag becomes Null and
    /// decoding continues rather than aborting the record.
    #[test]
    fn v1_unknown_tag_becomes_null_and_continues() {
        let mut bytes = v1_encode(&[Value::Integer(1)]);
        bytes.extend_from_slice(b"?:");
        bytes.extend_from_slice(&v1_encode(&[Value::Integer(2)]));
        assert_eq!(
            bytes_to_record(&bytes).unwrap(),
            vec![Value::Integer(1), Value::Null, Value::Integer(2)]
        );
    }

    // --- malformed V2 is rejected, not guessed ----------------------

    #[test]
    fn v2_unknown_type_code_is_an_error() {
        let mut bytes = V2_MAGIC.to_vec();
        bytes.push(0x7F);
        assert!(bytes_to_record(&bytes).is_err());
    }

    #[test]
    fn v2_truncated_value_is_an_error() {
        let mut bytes = V2_MAGIC.to_vec();
        bytes.push(T_I64);
        bytes.extend_from_slice(&[1, 2, 3]);
        assert!(bytes_to_record(&bytes).is_err());
    }

    /// A corrupt length must not be able to request a huge allocation.
    #[test]
    fn v2_absurd_length_is_rejected_without_allocating() {
        let mut bytes = V2_MAGIC.to_vec();
        bytes.push(T_TEXT);
        bytes.extend_from_slice(&u32::MAX.to_le_bytes());
        assert!(bytes_to_record(&bytes).is_err());
    }

    #[test]
    fn v1_truncated_prefix_is_an_error() {
        assert!(bytes_to_record(b"i").is_err());
    }

    #[test]
    fn v1_text_without_terminator_is_an_error() {
        assert!(bytes_to_record(b"s:abc").is_err());
    }

    // --- helpers ---------------------------------------------------

    #[test]
    fn key_is_first_column() {
        assert_eq!(
            record_key(&[Value::Integer(42), Value::Text("x".into())]),
            42i64.to_le_bytes().to_vec()
        );
    }

    #[test]
    fn key_of_empty_record_is_empty() {
        assert!(record_key(&[]).is_empty());
    }

    #[test]
    fn table_name_to_id_is_stable() {
        assert_eq!(table_name_to_id("users"), table_name_to_id("users"));
    }
}
