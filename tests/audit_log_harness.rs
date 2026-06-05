//! Audit Log Harness (P2-1 #3177)
//!
//! Shared utilities for audit logging tests. Provides:
//! - AuditEventBuilder — declarative test event construction
//! - AuditReport — captured metrics (events_recorded, events_queried,
//!   checksum_verified, overhead_percent)
//! - run_audit_test — execute a sequence of audit events against
//!   the in-memory audit store
//!
//! This file is **not** a test target itself (no `#[test]`); it is
//! shared by `audit_log_test.rs` via the same re-declared-copy pattern
//! used by P1-2/P1-3/P1-4 harnesses.

#![allow(dead_code)] // helpers consumed by test targets

/// Audit log action types (mirrors `crates/gmp/src/audit.rs::AuditAction`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditAction {
    Create,
    Update,
    Delete,
}

impl AuditAction {
    pub fn as_str(self) -> &'static str {
        match self {
            AuditAction::Create => "CREATE",
            AuditAction::Update => "UPDATE",
            AuditAction::Delete => "DELETE",
        }
    }
}

/// In-memory audit event (mirrors `AuditLog` for test purposes; the
/// real `AuditLog` lives in crates/gmp/src/audit.rs).
#[derive(Debug, Clone, PartialEq)]
pub struct AuditEvent {
    pub id: i64,
    pub timestamp: i64,
    pub user: String,
    pub action: AuditAction,
    pub table: String,
    pub row_id: Option<String>,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub tx_id: Option<String>,
    pub source: Option<String>,
    pub checksum: String,
}

/// Builder for AuditEvent.
pub struct AuditEventBuilder {
    pub user: String,
    pub action: AuditAction,
    pub table: String,
    pub row_id: Option<String>,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub tx_id: Option<String>,
    pub source: Option<String>,
}

impl AuditEventBuilder {
    pub fn new(user: &str, action: AuditAction, table: &str) -> Self {
        Self {
            user: user.into(),
            action,
            table: table.into(),
            row_id: None,
            old_value: None,
            new_value: None,
            tx_id: None,
            source: Some("TCP".into()),
        }
    }

    pub fn row_id(mut self, id: &str) -> Self {
        self.row_id = Some(id.into());
        self
    }

    pub fn old(mut self, value: &str) -> Self {
        self.old_value = Some(value.into());
        self
    }

    pub fn new_val(mut self, value: &str) -> Self {
        self.new_value = Some(value.into());
        self
    }

    pub fn tx(mut self, id: &str) -> Self {
        self.tx_id = Some(id.into());
        self
    }

    pub fn source(mut self, s: &str) -> Self {
        self.source = Some(s.into());
        self
    }

    pub fn build(self, id: i64, timestamp: i64) -> AuditEvent {
        // Simulated checksum: SHA-256-like hex (real impl in gmp/audit.rs).
        let mut cs: u64 = 0;
        for b in self.user.bytes() {
            cs = cs.wrapping_mul(31).wrapping_add(b as u64);
        }
        for b in self.table.bytes() {
            cs = cs.wrapping_mul(31).wrapping_add(b as u64);
        }
        cs ^= id as u64;
        let checksum = format!("{:016x}", cs);

        AuditEvent {
            id,
            timestamp,
            user: self.user,
            action: self.action,
            table: self.table,
            row_id: self.row_id,
            old_value: self.old_value,
            new_value: self.new_value,
            tx_id: self.tx_id,
            source: self.source,
            checksum,
        }
    }
}

/// Result of running audit tests.
#[derive(Debug, Clone)]
pub struct AuditReport {
    pub events_recorded: u32,
    pub events_queried: u32,
    pub checksum_verified: bool,
    pub overhead_percent: f64,
}

impl AuditReport {
    pub fn passed(&self) -> bool {
        self.checksum_verified
            && self.events_recorded > 0
            && self.events_recorded == self.events_queried
    }
}

/// In-memory audit store + operations.
pub struct AuditStore {
    events: Vec<AuditEvent>,
    next_id: i64,
}

impl AuditStore {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            next_id: 1,
        }
    }

    pub fn record(&mut self, builder: AuditEventBuilder) -> &AuditEvent {
        let ts = 1_700_000_000 + self.next_id;
        let ev = builder.build(self.next_id, ts);
        self.next_id += 1;
        self.events.push(ev);
        self.events.last().unwrap()
    }

    pub fn query_all(&self) -> &[AuditEvent] {
        &self.events
    }

    pub fn query_by_user(&self, user: &str) -> Vec<AuditEvent> {
        self.events.iter().filter(|e| e.user == user).cloned().collect()
    }

    pub fn query_by_table(&self, table: &str) -> Vec<AuditEvent> {
        self.events
            .iter()
            .filter(|e| e.table == table)
            .cloned()
            .collect()
    }

    pub fn count(&self) -> u32 {
        self.events.len() as u32
    }

    /// Verify all checksums (simulated; real impl uses SHA-256).
    pub fn verify_all(&self) -> bool {
        self.events.iter().all(|e| {
            // Recompute the simplified checksum
            let mut cs: u64 = 0;
            for b in e.user.bytes() {
                cs = cs.wrapping_mul(31).wrapping_add(b as u64);
            }
            for b in e.table.bytes() {
                cs = cs.wrapping_mul(31).wrapping_add(b as u64);
            }
            cs ^= e.id as u64;
            let expected = format!("{:016x}", cs);
            expected == e.checksum
        })
    }
}

impl Default for AuditStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Run a complete audit test scenario: record N events then query.
pub fn run_audit_test(events: Vec<AuditEventBuilder>) -> AuditReport {
    let mut store = AuditStore::new();
    for b in events {
        store.record(b);
    }
    let queried = store.query_all().len() as u32;
    let checksum_verified = store.verify_all();
    AuditReport {
        events_recorded: store.count(),
        events_queried: queried,
        checksum_verified,
        overhead_percent: 0.0, // not measured in this PR
    }
}

#[cfg(test)]
mod harness_tests {
    use super::*;

    #[test]
    fn builder_basic_event() {
        let ev = AuditEventBuilder::new("alice", AuditAction::Create, "users")
            .row_id("42")
            .build(1, 1700000000);
        assert_eq!(ev.id, 1);
        assert_eq!(ev.user, "alice");
        assert_eq!(ev.action, AuditAction::Create);
        assert_eq!(ev.table, "users");
        assert_eq!(ev.row_id.as_deref(), Some("42"));
        assert!(!ev.checksum.is_empty());
    }

    #[test]
    fn store_record_and_query() {
        let mut store = AuditStore::new();
        store.record(AuditEventBuilder::new("alice", AuditAction::Create, "users"));
        store.record(AuditEventBuilder::new("bob", AuditAction::Update, "orders"));
        assert_eq!(store.count(), 2);
        assert_eq!(store.query_by_user("alice").len(), 1);
        assert_eq!(store.query_by_table("orders").len(), 1);
    }

    #[test]
    fn checksum_verified_after_record() {
        let mut store = AuditStore::new();
        store.record(AuditEventBuilder::new("alice", AuditAction::Create, "users"));
        assert!(store.verify_all());
    }

    #[test]
    fn run_audit_test_passed() {
        let report = run_audit_test(vec![
            AuditEventBuilder::new("alice", AuditAction::Create, "users"),
            AuditEventBuilder::new("bob", AuditAction::Update, "orders"),
        ]);
        assert!(report.passed());
        assert_eq!(report.events_recorded, 2);
        assert_eq!(report.events_queried, 2);
    }

    #[test]
    fn audit_action_as_str() {
        assert_eq!(AuditAction::Create.as_str(), "CREATE");
        assert_eq!(AuditAction::Update.as_str(), "UPDATE");
        assert_eq!(AuditAction::Delete.as_str(), "DELETE");
    }
}
