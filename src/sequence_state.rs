//! Sequence state registry — extracted from `StorageEngine` so that
//! SELECT projection can advance/read sequences without acquiring the
//! global `storage` write lock.
//!
//! ## Why
//!
//! In v3.11.0, `SequenceNextVal`/`SequenceCurrval` evaluation inside the
//! SELECT projection path went through `self.storage.read()/write()`,
//! which serialised every concurrent SELECT against every other SELECT,
//! INSERT, UPDATE, and DELETE — QPS dropped to ~1/3 at 8-16 threads
//! (see `/tmp/perf-evidence/report.md`).
//!
//! v3.12 introduced a `storage.write()` lock at the top of the
//! projection loop (V311-10 fix, `engine_select.rs:1900`). That
//! protected `SequenceNextVal`'s atomic increment but destroyed
//! concurrent SELECT scalability.
//!
//! ## How
//!
//! This module introduces `SequenceState`, an in-memory cache of
//! `SequenceInfo` keyed by name, guarded by a single `parking_lot::Mutex`.
//! `ExecutionEngine` holds an `Arc<Mutex<SequenceState>>` and a read-only
//! snapshot view that the projection loop can consult without touching
//! `storage`.
//!
//! DDL (`CREATE`/`ALTER`/`DROP SEQUENCE`) and explicit
//! `next_sequence_value`/`get_sequence` from outside the projection
//! loop still go through `storage` (so persistence/recovery sees
//! sequence changes), but they also publish the result into the
//! `SequenceState` cache.
//!
//! The projection path no longer holds the `storage` write lock for the
//! full row loop — only the brief `SequenceState.lock().get_mut(name)` for
//! the columns that actually contain `SequenceNextVal`/`SequenceCurrval`.

use parking_lot::Mutex;
use sqlrustgo_storage::engine::SequenceInfo;
use sqlrustgo_types::{SqlError, SqlResult};
use std::collections::HashMap;
use std::sync::Arc;

/// In-memory cache of all known sequences and their current values.
///
/// Thread-safe: protected by a single `parking_lot::Mutex`. The lock
/// is held only for `O(1)` HashMap lookups + an integer increment, so
/// critical sections are sub-microsecond.
///
/// Invariants:
/// - `name` is the fully-qualified sequence name as supplied by the
///   user (`CREATE SEQUENCE foo` ⇒ `"foo"`).
/// - `current_value` is the value most recently produced by
///   `next_value()`. The semantics follow the `SequenceInfo` doc:
///   after `create`, the first `next_value()` returns `start_with`.
pub struct SequenceState {
    sequences: Mutex<HashMap<String, SequenceInfo>>,
}

impl Default for SequenceState {
    fn default() -> Self {
        Self::new()
    }
}

impl SequenceState {
    /// Create an empty `SequenceState`.
    pub fn new() -> Self {
        Self {
            sequences: Mutex::new(HashMap::new()),
        }
    }

    /// Atomically increment and return the next value of the sequence.
    ///
    /// Returns `Err(SqlError::ExecutionError)` if the sequence does not
    /// exist, has not been advanced (empty `start_with`), or has
    /// overflowed its `[minvalue, maxvalue]` range (with the configured
    /// cycle behaviour applied).
    pub fn next_value(&self, name: &str) -> SqlResult<i64> {
        let mut guard = self.sequences.lock();
        let info = guard.get_mut(name).ok_or_else(|| {
            SqlError::ExecutionError(format!("Sequence '{name}' not found"))
        })?;
        advance_sequence(info)
    }

    /// Return the most recently produced value of the sequence without
    /// advancing it. Returns `Err` if the sequence does not exist or
    /// has not yet been advanced (`current_value == start_with - increment_by`).
    pub fn currval(&self, name: &str) -> SqlResult<i64> {
        let guard = self.sequences.lock();
        let info = guard.get(name).ok_or_else(|| {
            SqlError::ExecutionError(format!("Sequence '{name}' not found"))
        })?;
        if info.current_value == info.start_with - info.increment_by {
            return Err(SqlError::ExecutionError(format!(
                "Sequence '{name}' has not been advanced yet (CURRVAL before NEXTVAL)"
            )));
        }
        Ok(info.current_value)
    }

    /// Insert or replace the sequence definition. Used by DDL
    /// (`CREATE SEQUENCE`, `ALTER SEQUENCE`) and by the storage
    /// load path at startup.
    pub fn install(&self, info: SequenceInfo) {
        self.sequences.lock().insert(info.name.clone(), info);
    }

    /// Remove a sequence from the cache.
    pub fn drop_sequence(&self, name: &str) {
        self.sequences.lock().remove(name);
    }

    /// Check whether a sequence exists.
    pub fn contains(&self, name: &str) -> bool {
        self.sequences.lock().contains_key(name)
    }

    /// Return a cloned `SnapshotInfo` (read-only view).
    pub fn get(&self, name: &str) -> Option<SequenceInfo> {
        self.sequences.lock().get(name).cloned()
    }

    /// Snapshot the full state for persistence / introspection.
    pub fn snapshot(&self) -> HashMap<String, SequenceInfo> {
        self.sequences.lock().clone()
    }

    /// Replace the entire cache contents (used at startup to hydrate
    /// from `storage`).
    pub fn hydrate(&self, sequences: HashMap<String, SequenceInfo>) {
        *self.sequences.lock() = sequences;
    }
}

/// Shared, cloneable handle to the engine's sequence cache.
pub type SharedSeqState = Arc<SequenceState>;

/// Helper: apply the increment + min/max/cycle policy to a sequence.
/// Caller must already hold the lock on `info`.
fn advance_sequence(info: &mut SequenceInfo) -> SqlResult<i64> {
    // First advance: current_value was initialised to `start_with - increment_by`,
    // so the first `next_value` returns `start_with`.
    let next = info.current_value + info.increment_by;
    if next > info.maxvalue {
        if info.cycle {
            // Wrap around to minvalue.
            info.current_value = info.minvalue - info.increment_by;
            let wrapped = info.current_value + info.increment_by;
            info.current_value = wrapped;
            return Ok(wrapped);
        }
        return Err(SqlError::ExecutionError(format!(
            "Sequence '{}' exceeded MAXVALUE ({})",
            info.name, info.maxvalue
        )));
    }
    if next < info.minvalue {
        // Defensive path for negative-increment sequences with cycle.
        if info.cycle {
            info.current_value = info.maxvalue - info.increment_by;
            let wrapped = info.current_value + info.increment_by;
            info.current_value = wrapped;
            return Ok(wrapped);
        }
        return Err(SqlError::ExecutionError(format!(
            "Sequence '{}' went below MINVALUE ({})",
            info.name, info.minvalue
        )));
    }
    info.current_value = next;
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make(name: &str, start: i64, inc: i64, min: i64, max: i64, cycle: bool) -> SequenceInfo {
        SequenceInfo {
            name: name.to_string(),
            start_with: start,
            increment_by: inc,
            minvalue: min,
            maxvalue: max,
            current_value: start - inc,
            cache: 1,
            cycle,
        }
    }

    #[test]
    fn next_value_basic_increment() {
        let s = SequenceState::new();
        s.install(make("s", 1, 1, i64::MIN, i64::MAX, false));
        assert_eq!(s.next_value("s").unwrap(), 1);
        assert_eq!(s.next_value("s").unwrap(), 2);
        assert_eq!(s.next_value("s").unwrap(), 3);
        assert_eq!(s.currval("s").unwrap(), 3);
    }

    #[test]
    fn next_value_unknown_errors() {
        let s = SequenceState::new();
        assert!(s.next_value("missing").is_err());
    }

    #[test]
    fn currval_before_nextval_errors() {
        let s = SequenceState::new();
        s.install(make("s", 10, 1, i64::MIN, i64::MAX, false));
        assert!(s.currval("s").is_err());
    }

    #[test]
    fn next_value_overflow_no_cycle_errors() {
        let s = SequenceState::new();
        s.install(make("s", 9, 1, 0, 10, false));
        for _ in 0..2 {
            s.next_value("s").unwrap();
        }
        assert!(s.next_value("s").is_err());
    }

    #[test]
    fn next_value_overflow_cycle_wraps() {
        let s = SequenceState::new();
        s.install(make("s", 9, 1, 0, 10, true));
        for _ in 0..2 {
            s.next_value("s").unwrap();
        }
        // 10 -> wrap -> 0 (per cycle policy)
        assert_eq!(s.next_value("s").unwrap(), 0);
    }

    #[test]
    fn drop_sequence_removes_entry() {
        let s = SequenceState::new();
        s.install(make("s", 1, 1, i64::MIN, i64::MAX, false));
        assert!(s.contains("s"));
        s.drop_sequence("s");
        assert!(!s.contains("s"));
    }

    #[test]
    fn hydrate_replaces_state() {
        let s = SequenceState::new();
        let mut initial = HashMap::new();
        initial.insert("a".to_string(), make("a", 5, 1, i64::MIN, i64::MAX, false));
        s.hydrate(initial);
        assert_eq!(s.next_value("a").unwrap(), 5);
    }
}