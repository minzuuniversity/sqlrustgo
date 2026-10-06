//! #5055: `WalEntry` must carry the table's **name**.
//!
//! `table_id` is a 31-radix hash of the name (`WalStorage::table_name_to_id`),
//! so a replay cannot recover the name from it — and `StorageEngine::insert`
//! takes a name. An entry that knows *that* a row changed but not *which
//! table it belongs to* cannot be applied.
//!
//! The field is an optional trailing field so that WAL files written
//! before #5055 still parse; they simply come back with `None`.

use sqlrustgo_storage::wal_legacy::{WalEntry, WalEntryType};

fn entry() -> WalEntry {
    WalEntry {
        tx_id: 42,
        entry_type: WalEntryType::Insert,
        table_id: 0x1234_5678_9abc_def0,
        key: Some(b"key-bytes".to_vec()),
        data: Some(b"i:\x01\x00\x00\x00\x00\x00\x00\x00".to_vec()),
        lsn: 7,
        timestamp: 1_700_000_000,
        table_name: Some("users".to_string()),
    }
}

#[test]
fn issue_5055_table_name_survives_a_round_trip() {
    let e = entry();
    let bytes = e.to_bytes();
    let back = WalEntry::from_bytes(&bytes).expect("entry must parse");

    assert_eq!(
        back.table_name.as_deref(),
        Some("users"),
        "the table name is what makes the entry replayable; it must survive \
         serialization"
    );
    // Everything else must be untouched too — the field was appended, not
    // substituted for something.
    assert_eq!(back.tx_id, e.tx_id);
    assert_eq!(back.lsn, e.lsn);
    assert_eq!(back.table_id, e.table_id);
    assert_eq!(back.key, e.key);
    assert_eq!(back.data, e.data);
}

/// A WAL written before #5055 has no trailing field. It must still parse —
/// losing the ability to read existing WAL files would be a far worse
/// regression than not being able to replay them.
#[test]
fn issue_5055_entries_without_a_table_name_still_parse() {
    let mut e = entry();
    e.table_name = None;
    let bytes = e.to_bytes();
    let back = WalEntry::from_bytes(&bytes).expect("a name-less entry must parse");

    assert_eq!(
        back.table_name, None,
        "no name was written, so none may be invented"
    );
    assert_eq!(back.tx_id, e.tx_id);
    assert_eq!(back.data, e.data);
}

/// The magic guard exists so a reader that mis-computed a field length does
/// not silently interpret arbitrary bytes as a table name.
#[test]
fn issue_5055_a_stray_tail_is_not_mistaken_for_a_table_name() {
    let mut bytes = entry().to_bytes();
    // Append bytes that are not the magic. They must be ignored, and the
    // entry must still parse with its own name intact.
    bytes.extend_from_slice(b"XXXXXXXXtrailing garbage");

    let back = WalEntry::from_bytes(&bytes).expect("must still parse");
    assert_eq!(
        back.table_name.as_deref(),
        Some("users"),
        "only a TNAM-tagged tail is a table name; anything else is ignored"
    );
}

/// A table name containing non-ASCII and the magic bytes themselves must
/// round-trip — the name is length-prefixed, not null-terminated.
#[test]
fn issue_5055_non_ascii_and_embedded_magic_names_survive() {
    for name in ["表名", "a\x00b", "", "TNAM", "x".repeat(300).as_str()] {
        let mut e = entry();
        e.table_name = if name.is_empty() {
            None
        } else {
            Some(name.to_string())
        };
        let back = WalEntry::from_bytes(&e.to_bytes()).expect("parse");
        assert_eq!(
            back.table_name.as_deref(),
            e.table_name.as_deref(),
            "name {name:?} must round-trip"
        );
    }
}
