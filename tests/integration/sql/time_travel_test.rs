//! P2-2 (#3178) Time Travel Query — 20+ tests across 5 categories
//!
//! 1. basic (AS OF) (5)
//! 2. range (VERSIONS BETWEEN) (4)
//! 3. no_match (3)
//! 4. multi_table (4)
//! 5. edge_cases (4)
//!
//! Total: 20 tests
//!
//! Refs: docs/openspec/3178-time-travel.md
//!       V390_TEST_PLAN.md §G10

#![allow(dead_code)]

mod harness {
    use std::collections::HashMap;

    #[derive(Debug, Clone, PartialEq)]
    pub enum TimeTravelKind {
        AsOf,
        VersionsBetween,
    }

    #[derive(Debug, Clone)]
    pub struct TimeTravelQuery {
        pub kind: TimeTravelKind,
        pub table: String,
        pub key: String,
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

    #[derive(Debug, Clone)]
    pub struct TimeTravelResult {
        pub query: TimeTravelQuery,
        pub found: bool,
        pub value: Option<String>,
        pub version_count: usize,
        pub first_visible_ts: Option<i64>,
        pub last_visible_ts: Option<i64>,
    }

    #[derive(Debug, Clone)]
    struct VersionEntry {
        pub ts: i64,
        pub value: String,
        pub deleted: bool,
    }

    pub struct MockMvccEngine {
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
        pub fn next_ts_after(&self) -> i64 {
            self.next_ts
        }
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
                TimeTravelKind::VersionsBetween => versions
                    .iter()
                    .filter(|v| v.ts >= q.timestamp && v.ts <= q.timestamp_end && !v.deleted)
                    .collect(),
            };
            let version_count = visible.len();
            let first_visible_ts = visible.first().map(|v| v.ts);
            let last_visible_ts = visible.last().map(|v| v.ts);
            let value = visible.last().map(|v| v.value.clone());
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
}

use harness::{MockMvccEngine, TimeTravelQuery};

// --------------------------------------------------------------------
// 1. basic (AS OF) (5 tests)
// --------------------------------------------------------------------

#[test]
fn test_tt_as_of_single_row_p2_2() {
    let mut e = MockMvccEngine::new();
    e.put("users", "alice", "v1");
    let q = TimeTravelQuery::as_of("users", "alice", e.next_ts_after() - 1);
    let r = e.query(&q);
    assert!(r.found);
    assert_eq!(r.value.as_deref(), Some("v1"));
    assert_eq!(r.version_count, 1);
}

#[test]
fn test_tt_as_of_multi_updates_p2_2() {
    let mut e = MockMvccEngine::new();
    e.put("users", "alice", "v1");
    e.put("users", "alice", "v2");
    e.put("users", "alice", "v3");
    let q = TimeTravelQuery::as_of("users", "alice", e.next_ts_after() - 1);
    let r = e.query(&q);
    assert_eq!(r.value.as_deref(), Some("v3"));
}

#[test]
fn test_tt_as_of_before_insert_p2_2() {
    // Query at ts=0 (before any insert) — must return not_found.
    let e = MockMvccEngine::new();
    let q = TimeTravelQuery::as_of("users", "alice", 0);
    let r = e.query(&q);
    assert!(!r.found);
}

#[test]
fn test_tt_as_of_after_delete_p2_2() {
    let mut e = MockMvccEngine::new();
    e.put("users", "alice", "v1");
    e.delete("users", "alice");
    let q = TimeTravelQuery::as_of("users", "alice", e.next_ts_after() - 1);
    let r = e.query(&q);
    assert!(!r.found);
}

#[test]
fn test_tt_as_of_restored_after_delete_p2_2() {
    // insert, delete, insert (same key) — should find the new value.
    let mut e = MockMvccEngine::new();
    e.put("users", "alice", "v1");
    e.delete("users", "alice");
    e.put("users", "alice", "v2-restored");
    let q = TimeTravelQuery::as_of("users", "alice", e.next_ts_after() - 1);
    let r = e.query(&q);
    assert!(r.found);
    assert_eq!(r.value.as_deref(), Some("v2-restored"));
}

// --------------------------------------------------------------------
// 2. range (VERSIONS BETWEEN) (4 tests)
// --------------------------------------------------------------------

#[test]
fn test_tt_versions_between_one_day_p2_2() {
    let mut e = MockMvccEngine::new();
    e.put("users", "alice", "v1");
    let t1 = e.next_ts_after() - 1;
    e.put("users", "alice", "v2");
    e.put("users", "alice", "v3");
    let t3 = e.next_ts_after() - 1;
    let q = TimeTravelQuery::versions_between("users", "alice", t1, t3);
    let r = e.query(&q);
    assert_eq!(r.version_count, 3);
}

#[test]
fn test_tt_versions_between_one_week_p2_2() {
    let mut e = MockMvccEngine::new();
    for i in 0..7 {
        e.put("users", "alice", &format!("day-{}", i));
    }
    let (t_start, t_end) = (e.next_ts_after() - 7, e.next_ts_after() - 1);
    let q = TimeTravelQuery::versions_between("users", "alice", t_start, t_end);
    let r = e.query(&q);
    assert_eq!(r.version_count, 7);
}

#[test]
fn test_tt_versions_between_empty_range_p2_2() {
    let mut e = MockMvccEngine::new();
    e.put("users", "alice", "v1");
    // Query in the past, before any insert.
    let q = TimeTravelQuery::versions_between("users", "alice", 0, 1000);
    let r = e.query(&q);
    assert_eq!(r.version_count, 0);
    assert!(!r.found);
}

#[test]
fn test_tt_versions_between_full_range_p2_2() {
    let mut e = MockMvccEngine::new();
    for i in 0..10 {
        e.put("users", "alice", &format!("v{}", i));
    }
    let (t_start, t_end) = (e.next_ts_after() - 10, e.next_ts_after() - 1);
    let q = TimeTravelQuery::versions_between("users", "alice", t_start, t_end);
    let r = e.query(&q);
    assert_eq!(r.version_count, 10);
}

// --------------------------------------------------------------------
// 3. no_match (3 tests)
// --------------------------------------------------------------------

#[test]
fn test_tt_no_match_future_timestamp_p2_2() {
    let mut e = MockMvccEngine::new();
    e.put("users", "alice", "v1");
    let q = TimeTravelQuery::as_of("users", "alice", i64::MAX - 1);
    let r = e.query(&q);
    // Future ts: latest is at insert ts which is < i64::MAX-1
    assert!(r.found);
    assert_eq!(r.value.as_deref(), Some("v1"));
}

#[test]
fn test_tt_no_match_very_old_timestamp_p2_2() {
    let mut e = MockMvccEngine::new();
    e.put("users", "alice", "v1");
    let q = TimeTravelQuery::as_of("users", "alice", 0); // before insert
    let r = e.query(&q);
    assert!(!r.found);
}

#[test]
fn test_tt_no_match_nonexistent_key_p2_2() {
    let mut e = MockMvccEngine::new();
    e.put("users", "alice", "v1");
    let q = TimeTravelQuery::as_of("users", "bob", e.next_ts_after() - 1);
    let r = e.query(&q);
    assert!(!r.found);
    assert_eq!(r.version_count, 0);
}

// --------------------------------------------------------------------
// 4. multi_table (4 tests)
// --------------------------------------------------------------------

#[test]
fn test_tt_multi_table_independent_p2_2() {
    let mut e = MockMvccEngine::new();
    e.put("users", "alice", "user-v1");
    e.put("orders", "alice", "order-v1");
    let q1 = TimeTravelQuery::as_of("users", "alice", e.next_ts_after() - 1);
    let q2 = TimeTravelQuery::as_of("orders", "alice", e.next_ts_after() - 1);
    let r1 = e.query(&q1);
    let r2 = e.query(&q2);
    assert_eq!(r1.value.as_deref(), Some("user-v1"));
    assert_eq!(r2.value.as_deref(), Some("order-v1"));
}

#[test]
fn test_tt_multi_table_join_p2_2() {
    let mut e = MockMvccEngine::new();
    e.put("users", "alice", "Alice User");
    e.put("orders", "alice", "Order 1");
    let q1 = TimeTravelQuery::as_of("users", "alice", e.next_ts_after() - 1);
    let q2 = TimeTravelQuery::as_of("orders", "alice", e.next_ts_after() - 1);
    let r1 = e.query(&q1);
    let r2 = e.query(&q2);
    // JOIN would be in the SQL layer; here we just verify both lookups work.
    assert!(r1.found);
    assert!(r2.found);
    assert_eq!(r1.value.as_deref(), Some("Alice User"));
    assert_eq!(r2.value.as_deref(), Some("Order 1"));
}

#[test]
fn test_tt_multi_table_same_key_p2_2() {
    let mut e = MockMvccEngine::new();
    e.put("users", "key1", "users-value");
    e.put("orders", "key1", "orders-value");
    let q1 = TimeTravelQuery::as_of("users", "key1", e.next_ts_after() - 1);
    let q2 = TimeTravelQuery::as_of("orders", "key1", e.next_ts_after() - 1);
    let r1 = e.query(&q1);
    let r2 = e.query(&q2);
    assert_eq!(r1.value.as_deref(), Some("users-value"));
    assert_eq!(r2.value.as_deref(), Some("orders-value"));
}

#[test]
fn test_tt_multi_table_isolation_p2_2() {
    // Inserting into one table must NOT affect queries on another.
    let mut e = MockMvccEngine::new();
    e.put("users", "alice", "v1");
    e.put("orders", "bob", "o1");
    let q = TimeTravelQuery::as_of("users", "bob", e.next_ts_after() - 1);
    let r = e.query(&q);
    assert!(!r.found, "users.bob must not exist");
}

// --------------------------------------------------------------------
// 5. edge_cases (4 tests)
// --------------------------------------------------------------------

#[test]
fn test_tt_same_ts_insert_and_update_p2_2() {
    // We can simulate same-ts operations by using the same put() call
    // in sequence; this test pins that two consecutive puts at adjacent
    // ts are both visible at the latest ts.
    let mut e = MockMvccEngine::new();
    e.put("t", "k", "v1");
    e.put("t", "k", "v2");
    let q = TimeTravelQuery::as_of("t", "k", e.next_ts_after() - 1);
    let r = e.query(&q);
    assert_eq!(r.value.as_deref(), Some("v2"));
    assert_eq!(r.version_count, 2);
}

#[test]
fn test_tt_many_versions_p2_2() {
    let mut e = MockMvccEngine::new();
    for i in 0..100 {
        e.put("t", "k", &format!("v{}", i));
    }
    let q = TimeTravelQuery::as_of("t", "k", e.next_ts_after() - 1);
    let r = e.query(&q);
    assert_eq!(r.value.as_deref(), Some("v99"));
}

#[test]
fn test_tt_delete_then_restore_p2_2() {
    let mut e = MockMvccEngine::new();
    e.put("t", "k", "v1");
    e.delete("t", "k");
    e.put("t", "k", "v2-restored");
    // Three operations; v2-restored is the latest.
    let q = TimeTravelQuery::as_of("t", "k", e.next_ts_after() - 1);
    let r = e.query(&q);
    assert_eq!(r.value.as_deref(), Some("v2-restored"));
    // Versions-between yields 2 (v1 + v2-restored, v1's delete is
    // filtered out).
    let t1 = 1_700_000_000 - 1; // just before first insert
    let t2 = e.next_ts_after() - 1;
    let q2 = TimeTravelQuery::versions_between("t", "k", t1, t2);
    let r2 = e.query(&q2);
    assert_eq!(r2.version_count, 2);
}

#[test]
fn test_tt_first_and_last_timestamps_p2_2() {
    let mut e = MockMvccEngine::new();
    e.put("t", "k", "v1");
    let t1 = e.next_ts_after() - 1;
    e.put("t", "k", "v2");
    e.put("t", "k", "v3");
    let t3 = e.next_ts_after() - 1;
    let q = TimeTravelQuery::versions_between("t", "k", t1, t3);
    let r = e.query(&q);
    assert_eq!(r.first_visible_ts, Some(t1));
    assert_eq!(r.last_visible_ts, Some(t3));
    assert_eq!(r.version_count, 3);
}
