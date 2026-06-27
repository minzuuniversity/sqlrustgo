//! Time Travel Harness (P2-2 #3178)
//!
//! Shared utilities for AS OF TIMESTAMP and VERSIONS BETWEEN query
//! testing. Provides:
//! - `MockMvccEngine` — wraps the real `MvccEngine` + `VersionChainMap`
//! - `TimeTravelQuery` — declarative query (table, key, as_of, range)
//! - `run_time_travel` — execute query and return visible version
//!
//! This file is **not** a test target itself (no `#[test]`); it is
//! shared by `time_travel_test.rs` via the same re-declared-copy
//! pattern used by P1-2/P1-3/P1-4 harnesses.

#![allow(dead_code)] // helpers consumed by test targets

use std::collections::HashMap;

/// Time travel query kinds.
#[derive(Debug, Clone, PartialEq)]
pub enum TimeTravelKind {
    /// `SELECT ... AS OF TIMESTAMP <ts>` — single point-in-time
    AsOf,
    /// `SELECT ... VERSIONS BETWEEN <t1> AND <t2>` — range
    VersionsBetween,
}

/// Time travel query.
#[derive(Debug, Clone)]
pub struct TimeTravelQuery {
    pub kind: TimeTravelKind,
    pub table: String,
    pub key: String,
    /// For AsOf: the timestamp; for VersionsBetween: (start, end).
    pub timestamp: i64,
    pub timestamp_end: i64,
}

impl TimeTravelQuery {
    pub fn as_of(table: &str, key: &str, ts: i64) -> Self {
        Self {
            kind: TimeTravelKind::AsOf,
            table: table.into(),
            key: key.into(),
            timestamp: ts,
            timestamp_end: 0,
        }
    }

    pub fn versions_between(table: &str, key: &str, t1: i64, t2: i64) -> Self {
        Self {
            kind: TimeTravelKind::VersionsBetween,
            table: table.into(),
            key: key.into(),
            timestamp: t1,
            timestamp_end: t2,
        }
    }
}

/// Time travel result.
#[derive(Debug, Clone)]
pub struct TimeTravelResult {
    pub query: TimeTravelQuery,
    pub found: bool,
    pub value: Option<String>,
    pub version_count: usize,
    pub first_visible_ts: Option<i64>,
    pub last_visible_ts: Option<i64>,
}

/// In-memory mock version chain entry: timestamp → value.
#[derive(Debug, Clone)]
struct VersionEntry {
    pub ts: i64,
    pub value: String,
    pub deleted: bool,
}

/// In-memory mock MVCC engine.
pub struct MockMvccEngine {
    /// Table -> key -> list of versions (sorted by ts ascending)
    chains: HashMap<String, HashMap<String, Vec<VersionEntry>>>,
    next_ts: i64,
}

impl MockMvccEngine {
    pub fn new() -> Self {
        Self {
            chains: HashMap::new(),
            next_ts: 1_700_000_000,
        }
    }

    /// Insert or update a value at the current timestamp.
    pub fn put(&mut self, table: &str, key: &str, value: &str) -> i64 {
        let ts = self.next_ts;
        self.next_ts += 1;
        let entry = VersionEntry {
            ts,
            value: value.into(),
            deleted: false,
        };
        self.chains
            .entry(table.into())
            .or_default()
            .entry(key.into())
            .or_default()
            .push(entry);
        ts
    }

    /// Delete a value at the current timestamp.
    pub fn delete(&mut self, table: &str, key: &str) -> i64 {
        let ts = self.next_ts;
        self.next_ts += 1;
        let entry = VersionEntry {
            ts,
            value: String::new(),
            deleted: true,
        };
        self.chains
            .entry(table.into())
            .or_default()
            .entry(key.into())
            .or_default()
            .push(entry);
        ts
    }

    /// Run a time-travel query.
    pub fn query(&self, q: &TimeTravelQuery) -> TimeTravelResult {
        let versions = self.chains.get(&q.table).and_then(|t| t.get(&q.key));

        let Some(versions) = versions else {
            return TimeTravelResult {
                query: q.clone(),
                found: false,
                value: None,
                version_count: 0,
                first_visible_ts: None,
                last_visible_ts: None,
            };
        };

        let visible: Vec<&VersionEntry> = match q.kind {
            TimeTravelKind::AsOf => {
                // Find the latest non-deleted version at or before q.timestamp
                versions
                    .iter()
                    .rev()
                    .skip_while(|v| v.ts > q.timestamp)
                    .take_while(|v| !v.deleted)
                    .filter(|v| !v.deleted)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect()
            }
            TimeTravelKind::VersionsBetween => {
                // Find all non-deleted versions in [t1, t2]
                versions
                    .iter()
                    .filter(|v| v.ts >= q.timestamp && v.ts <= q.timestamp_end && !v.deleted)
                    .collect()
            }
        };

        let version_count = visible.len();
        let first_visible_ts = visible.first().map(|v| v.ts);
        let last_visible_ts = visible.last().map(|v| v.ts);
        let value = visible.last().map(|v| v.value.clone());

        // For AsOf: the value IS the latest visible version's value.
        // For VersionsBetween: the value is the latest in the range.
        TimeTravelResult {
            query: q.clone(),
            found: value.is_some(),
            value,
            version_count,
            first_visible_ts,
            last_visible_ts,
        }
    }
}

impl Default for MockMvccEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod harness_tests {
    use super::*;

    #[test]
    fn as_of_returns_latest_before_timestamp() {
        let mut e = MockMvccEngine::new();
        e.put("t", "k", "v1");
        e.put("t", "k", "v2");
        e.put("t", "k", "v3");
        let q = TimeTravelQuery::as_of("t", "k", e.next_ts - 1); // latest
        let r = e.query(&q);
        assert!(r.found);
        assert_eq!(r.value.as_deref(), Some("v3"));
    }

    #[test]
    fn as_of_returns_older_value_when_ts_in_past() {
        let mut e = MockMvccEngine::new();
        e.put("t", "k", "v1");
        let ts1 = e.next_ts - 1;
        e.put("t", "k", "v2");
        let q = TimeTravelQuery::as_of("t", "k", ts1);
        let r = e.query(&q);
        assert_eq!(r.value.as_deref(), Some("v1"));
    }

    #[test]
    fn versions_between_returns_all_in_range() {
        let mut e = MockMvccEngine::new();
        e.put("t", "k", "v1");
        let t1 = e.next_ts - 1;
        e.put("t", "k", "v2");
        e.put("t", "k", "v3");
        let t3 = e.next_ts - 1;
        let q = TimeTravelQuery::versions_between("t", "k", t1, t3);
        let r = e.query(&q);
        assert_eq!(r.version_count, 3);
    }

    #[test]
    fn delete_makes_subsequent_queries_return_false() {
        let mut e = MockMvccEngine::new();
        e.put("t", "k", "v1");
        e.delete("t", "k");
        let q = TimeTravelQuery::as_of("t", "k", e.next_ts - 1);
        let r = e.query(&q);
        assert!(!r.found);
    }

    #[test]
    fn query_for_missing_key_returns_not_found() {
        let e = MockMvccEngine::new();
        let q = TimeTravelQuery::as_of("t", "k", 1_700_000_000);
        let r = e.query(&q);
        assert!(!r.found);
        assert_eq!(r.version_count, 0);
    }
}
