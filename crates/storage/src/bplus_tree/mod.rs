//! B+ Tree implementation - simplified version for storage crate

pub mod index;

use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;

/// B+ Tree index - simplified for storage crate
///
/// #5209: the value is a `Vec<u32>`, one entry per row carrying that key.
///
/// It used to be a single `u32`, which made a non-unique column
/// impossible to index correctly: `insert` overwrote on a duplicate key,
/// so with 142 rows sharing `k = 0` only the last row's id survived, and
/// `search_all` — despite its name — could return at most one row.
/// Primary-key indexes were unaffected because their keys are unique,
/// which is exactly why the defect stayed hidden.
///
/// # On-disk compatibility
///
/// Persisted `.idx` files written before this change contain
/// `{"map": {"42": 7}}`. Deserialisation accepts both that legacy scalar
/// form and the new `{"map": {"42": [7]}}` form, so an existing data
/// directory keeps working after an upgrade instead of failing to load or
/// — worse — loading an index whose contents no longer match its shape.
#[derive(Debug, Clone, Serialize, Default)]
pub struct BPlusTree {
    /// Map from key to the row ids carrying that key (sorted by key).
    map: BTreeMap<i64, Vec<u32>>,
}

impl<'de> Deserialize<'de> for BPlusTree {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        // Convert value-by-value rather than with an `untagged` enum: the
        // two shapes differ only in whether the value is a number or a
        // one-element array, and `untagged` buffers into `Content` where
        // map-key deserialization through `BTreeMap` is unreliable.
        // The persisted document is `{"map": {...}}` — `serde_json::
        // to_string_pretty(index)` serialises the struct, not its field.
        #[derive(Deserialize)]
        struct Repr {
            map: BTreeMap<String, serde_json::Value>,
        }
        let Repr { map: raw } = Repr::deserialize(d)?;
        let mut map = BTreeMap::new();
        for (k, v) in raw {
            let key: i64 = k.parse().map_err(serde::de::Error::custom)?;
            let ids: Vec<u32> = match v {
                // Legacy: `{"map": {"42": 7}}`.
                serde_json::Value::Number(n) => vec![n
                    .as_u64()
                    .ok_or_else(|| serde::de::Error::custom("index value out of range"))?
                    as u32],
                // Current: `{"map": {"42": [7, 9]}}`.
                serde_json::Value::Array(items) => items
                    .iter()
                    .map(|i| {
                        i.as_u64()
                            .map(|n| n as u32)
                            .ok_or_else(|| serde::de::Error::custom("index value out of range"))
                    })
                    .collect::<Result<Vec<u32>, _>>()?,
                other => {
                    return Err(serde::de::Error::custom(format!(
                        "unexpected index entry shape: {other}"
                    )))
                }
            };
            map.insert(key, ids);
        }
        Ok(BPlusTree { map })
    }
}

impl BPlusTree {
    /// Create a new B+ Tree
    pub fn new() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }

    /// Get the number of entries
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Insert a key-value pair, appending when the key already exists.
    ///
    /// #5209: this used to be `self.map.insert(key, value)`, which
    /// silently discarded every previously stored row id for that key.
    pub fn insert(&mut self, key: i64, value: u32) {
        self.map.entry(key).or_default().push(value);
    }

    /// Search for a key. Returns the **first** row id stored under it.
    ///
    /// Prefer [`BPlusTree::search_all`] unless the caller genuinely wants
    /// one arbitrary row: on a non-unique column this silently hides
    /// every other matching row.
    pub fn search(&self, key: i64) -> Option<u32> {
        self.map.get(&key).and_then(|v| v.first()).copied()
    }

    /// Search for all values matching a key.
    ///
    /// #5209: this returned at most one element regardless of how many
    /// rows carried the key.
    pub fn search_all(&self, key: i64) -> Vec<u32> {
        self.map.get(&key).cloned().unwrap_or_default()
    }

    /// Query all values in range [start, end)
    pub fn range_query(&self, start: i64, end: i64) -> Vec<u32> {
        self.map
            .range(start..end)
            .flat_map(|(_, v)| v.iter().copied())
            .collect()
    }

    /// Return all keys in sorted order
    pub fn keys(&self) -> Vec<i64> {
        self.map.keys().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #5209: a non-unique column must keep every row id, not just the
    /// last one inserted for a key. Before this the map was
    /// `BTreeMap<i64, u32>` and `insert` overwrote, so 142 rows sharing
    /// `k = 0` left exactly one behind and `search_all` could never
    /// return more than one row.
    #[test]
    fn duplicate_keys_keep_every_row_id() {
        let mut tree = BPlusTree::new();
        for row_id in 0..5u32 {
            tree.insert(42, row_id);
        }
        let found = tree.search_all(42);
        assert_eq!(
            found,
            vec![0, 1, 2, 3, 4],
            "#5209: every row carrying key 42 must survive insertion"
        );
        assert_eq!(
            tree.search(42),
            Some(0),
            "#5209: `search` documents itself as returning the first row id"
        );
        assert_eq!(tree.len(), 1, "one distinct key");
        assert!(tree.search_all(43).is_empty());
    }

    /// A pre-#5209 `.idx` file stored a bare `u32`. Loading one must not
    /// fail, and must not lose the row it named.
    #[test]
    fn legacy_scalar_form_still_deserializes() {
        let legacy = r#"{"map": {"42": 7, "99": 3}}"#;
        let tree: BPlusTree = serde_json::from_str(legacy).expect("legacy index must load");
        assert_eq!(tree.search_all(42), vec![7]);
        assert_eq!(tree.search_all(99), vec![3]);
        assert!(tree.search_all(1).is_empty());
    }

    /// Current form, and a round trip through the new serialisation.
    #[test]
    fn current_list_form_round_trips() {
        let mut tree = BPlusTree::new();
        tree.insert(1, 10);
        tree.insert(1, 11);
        tree.insert(2, 20);
        let text = serde_json::to_string(&tree).expect("serialize");
        let back: BPlusTree = serde_json::from_str(&text).expect("deserialize");
        assert_eq!(back.search_all(1), vec![10, 11]);
        assert_eq!(back.search_all(2), vec![20]);
    }

    /// `range_query` must not drop rows on duplicate keys either — it
    /// flattens the per-key lists now.
    #[test]
    fn range_query_returns_all_rows_in_range() {
        let mut tree = BPlusTree::new();
        tree.insert(1, 100);
        tree.insert(1, 101);
        tree.insert(2, 200);
        tree.insert(3, 300);
        assert_eq!(
            tree.range_query(1, 3),
            vec![100, 101, 200],
            "#5209: both rows under key 1 must appear"
        );
    }

    #[test]
    fn test_bplus_tree_insert_and_search() {
        let mut tree = BPlusTree::new();
        tree.insert(1, 100);
        tree.insert(2, 200);
        tree.insert(3, 300);

        assert_eq!(tree.search(1), Some(100));
        assert_eq!(tree.search(2), Some(200));
        assert_eq!(tree.search(3), Some(300));
        assert_eq!(tree.search(4), None);
    }

    #[test]
    fn test_bplus_tree_range_query() {
        let mut tree = BPlusTree::new();
        tree.insert(1, 100);
        tree.insert(2, 200);
        tree.insert(3, 300);
        tree.insert(4, 400);

        let results = tree.range_query(2, 4);
        assert_eq!(results.len(), 2);
        assert!(results.contains(&200));
        assert!(results.contains(&300));
    }

    #[test]
    fn test_simple_bplus_tree_keys() {
        let mut tree = BPlusTree::new();
        tree.insert(3, 300);
        tree.insert(1, 100);
        tree.insert(2, 200);

        let keys = tree.keys();
        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&1));
        assert!(keys.contains(&2));
        assert!(keys.contains(&3));
    }

    #[test]
    fn test_simple_bplus_tree_is_empty() {
        let tree = BPlusTree::new();
        assert!(tree.is_empty());
        assert_eq!(tree.len(), 0);

        let mut tree = BPlusTree::new();
        tree.insert(1, 100);
        assert!(!tree.is_empty());
        assert_eq!(tree.len(), 1);
    }

    #[test]
    fn test_simple_bplus_tree_search_all() {
        let mut tree = BPlusTree::new();
        tree.insert(1, 100);
        tree.insert(2, 200);

        let all = tree.search_all(1);
        assert_eq!(all.len(), 1);
        assert_eq!(all[0], 100);

        let none = tree.search_all(999);
        assert!(none.is_empty());
    }
}
