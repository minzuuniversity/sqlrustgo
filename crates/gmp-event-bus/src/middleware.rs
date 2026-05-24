//! Audit Middleware — AuditContext and AuditEmitter for CLI/API layer
//!
//! Provides request-level audit context and convenient APIs for emitting
//! audit events through the Event Bus.

use std::sync::Arc;

use crate::{AuditAction, AuditEvent, GmpEvent};

// ─── Audit Context ─────────────────────────────────────────────────────────────

/// Request-level audit context — carries trace_id and actor through an operation
#[derive(Debug, Clone)]
pub struct AuditContext {
    /// Global trace ID for event correlation
    pub trace_id: String,
    /// Actor who initiated the operation
    pub actor: String,
    /// Optional session ID
    pub session_id: Option<String>,
}

impl AuditContext {
    /// Create a new context with a generated trace_id
    pub fn new(actor: &str) -> Self {
        Self {
            trace_id: uuid::Uuid::new_v4().to_string(),
            actor: actor.to_string(),
            session_id: None,
        }
    }

    /// Create a new context with a session ID
    pub fn with_session(actor: &str, session_id: &str) -> Self {
        Self {
            trace_id: uuid::Uuid::new_v4().to_string(),
            actor: actor.to_string(),
            session_id: Some(session_id.to_string()),
        }
    }

    /// Create a context with a known trace_id (for distributed tracing)
    pub fn with_trace(actor: &str, trace_id: &str) -> Self {
        Self {
            trace_id: trace_id.to_string(),
            actor: actor.to_string(),
            session_id: None,
        }
    }
}

// ─── Hash Utilities ─────────────────────────────────────────────────────────────

/// Hash an audit event using std DefaultHasher
pub fn hash_event(trace_id: &str, entity_id: &str, action: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    trace_id.hash(&mut h);
    entity_id.hash(&mut h);
    action.hash(&mut h);
    format!("{:016x}", h.finish())
}

/// Get current Unix timestamp in seconds
pub fn unix_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

// ─── Audit Emitter ─────────────────────────────────────────────────────────────

/// Convenient API for emitting audit events through the Event Bus
pub struct AuditEmitter {
    bus: Arc<dyn crate::EventBus>,
}

impl AuditEmitter {
    /// Create a new emitter backed by an event bus
    pub fn new(bus: Arc<dyn crate::EventBus>) -> Self {
        Self { bus }
    }

    /// Emit a search/query audit event
    pub fn emit_search(
        &self,
        ctx: &AuditContext,
        query: &str,
        result_count: usize,
    ) -> anyhow::Result<()> {
        let meta = serde_json::json!({
            "query": query,
            "result_count": result_count,
        })
        .to_string();
        self.publish(AuditAction::Query, "search", ctx, Some(meta))
    }

    /// Emit an insert audit event
    pub fn emit_insert(
        &self,
        ctx: &AuditContext,
        entity_id: &str,
        entity_type: &str,
    ) -> anyhow::Result<()> {
        let meta = serde_json::json!({ "entity_type": entity_type }).to_string();
        self.publish(AuditAction::Insert, entity_id, ctx, Some(meta))
    }

    /// Emit an update audit event
    pub fn emit_update(
        &self,
        ctx: &AuditContext,
        entity_id: &str,
        entity_type: &str,
    ) -> anyhow::Result<()> {
        let meta = serde_json::json!({ "entity_type": entity_type }).to_string();
        self.publish(AuditAction::Update, entity_id, ctx, Some(meta))
    }

    /// Emit a delete audit event
    pub fn emit_delete(
        &self,
        ctx: &AuditContext,
        entity_id: &str,
        entity_type: &str,
    ) -> anyhow::Result<()> {
        let meta = serde_json::json!({ "entity_type": entity_type }).to_string();
        self.publish(AuditAction::Delete, entity_id, ctx, Some(meta))
    }

    /// Emit a signature/approval audit event
    pub fn emit_sign(
        &self,
        ctx: &AuditContext,
        entity_id: &str,
        signature: &str,
    ) -> anyhow::Result<()> {
        let meta = serde_json::json!({ "signature": signature }).to_string();
        self.publish(AuditAction::Sign, entity_id, ctx, Some(meta))
    }

    fn publish(
        &self,
        action: AuditAction,
        entity_id: &str,
        ctx: &AuditContext,
        metadata: Option<String>,
    ) -> anyhow::Result<()> {
        let event = GmpEvent::Audit(AuditEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            action,
            entity_id: entity_id.to_string(),
            timestamp: unix_timestamp(),
            actor: ctx.actor.clone(),
            hash: hash_event(&ctx.trace_id, entity_id, &format!("{:?}", action)),
            signature: None,
            trace_id: ctx.trace_id.clone(),
            metadata,
        });
        self.bus.publish(event)
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AuditHandler, EventBus, InMemoryBus};

    #[test]
    fn test_audit_context_new() {
        let ctx = AuditContext::new("alice");
        assert_eq!(ctx.actor, "alice");
        assert!(!ctx.trace_id.is_empty());
        assert!(ctx.session_id.is_none());
    }

    #[test]
    fn test_audit_context_with_session() {
        let ctx = AuditContext::with_session("alice", "sess-123");
        assert_eq!(ctx.actor, "alice");
        assert_eq!(ctx.session_id, Some("sess-123".to_string()));
    }

    #[test]
    fn test_audit_context_with_trace() {
        let ctx = AuditContext::with_trace("alice", "trace-abc");
        assert_eq!(ctx.trace_id, "trace-abc");
    }

    #[test]
    fn test_hash_event_deterministic() {
        let h1 = hash_event("trace", "entity", "Insert");
        let h2 = hash_event("trace", "entity", "Insert");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_event_different_inputs() {
        let h1 = hash_event("trace", "entity", "Insert");
        let h2 = hash_event("trace", "entity", "Delete");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_audit_emitter_publish() {
        let bus = Arc::new(InMemoryBus::new());
        bus.subscribe(Box::new(AuditHandler::new())).unwrap();
        let emitter = AuditEmitter::new(bus);
        let ctx = AuditContext::new("test");
        emitter.emit_search(&ctx, "SELECT * FROM", 10).unwrap();
    }
}
