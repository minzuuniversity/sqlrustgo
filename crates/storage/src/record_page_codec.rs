//! PR-A.5 + PR-B / #5305 — shared record ↔ page-bytes codec.
//!
//! Centralised so `FileStorage` (PR-A.5) and `MemoryStorage` (PR-B) can both
//! route their `insert` / `scan` paths through `BufferPool` without
//! duplicating the serde layout. Lifted out of `file_storage.rs` at PR-B
//! T4 Step 1.

use sqlrustgo_types::Value;

/// Serialise a `Record` (a row) into a self-describing byte slice for
/// storage in a `Page.data` field. Layout is a 4-byte little-endian length
/// prefix followed by the `serde_json` body. Length-prefixed so we can
/// extend the format later (e.g., add a schema-id or a checksum) without
/// breaking already-stored pages.
pub(crate) fn serialize(record: &[Value]) -> Vec<u8> {
    let body = serde_json::to_vec(record).expect("Record serialization must not fail");
    let mut out = Vec::with_capacity(4 + body.len());
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&body);
    out
}

/// Inverse of [`serialize`]. Returns `None` for truncated / non-JSON
/// payloads; callers should silently skip those (a freshly-allocated
/// `Page::new(page_id)` has zero data and would otherwise be reported as a
/// real row).
pub(crate) fn deserialize(data: &[u8]) -> Option<Vec<Value>> {
    if data.len() < 4 {
        return None;
    }
    let len = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
    if data.len() < 4 + len {
        return None;
    }
    serde_json::from_slice(&data[4..4 + len]).ok()
}
