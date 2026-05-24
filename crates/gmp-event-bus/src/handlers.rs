//! Built-in event handlers

use anyhow::Result;

use crate::{EventHandler, GmpEvent, GmpEventType};

/// AuditHandler — logs all events to tracing for audit trail
///
/// In v1.3.0 this is the only built-in handler.
/// Future versions will write to persistent AuditStore.
pub struct AuditHandler {
    _priv: (),
}

impl AuditHandler {
    pub fn new() -> Self {
        Self { _priv: () }
    }
}

impl Default for AuditHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for AuditHandler {
    fn clone(&self) -> Self {
        Self::new()
    }
}

impl EventHandler for AuditHandler {
    fn event_types(&self) -> &[GmpEventType] {
        &[GmpEventType::All]
    }

    fn handle(&self, event: &GmpEvent) -> Result<()> {
        match event {
            GmpEvent::Audit(ae) => {
                tracing::info!(
                    event_type = "audit",
                    event_id = %ae.event_id,
                    action = ?ae.action,
                    entity_id = %ae.entity_id,
                    actor = %ae.actor,
                    trace_id = %ae.trace_id,
                    "Audit event: {} on {} by {}",
                    format!("{:?}", ae.action),
                    ae.entity_id,
                    ae.actor,
                );
            }
            GmpEvent::Device(de) => {
                tracing::info!(
                    event_type = "device",
                    device_id = %de.device_id,
                    reading_type = ?de.reading_type,
                    value = %de.value,
                    unit = %de.unit,
                    trace_id = %de.trace_id,
                    "Device event: {} = {} {}",
                    de.device_id,
                    de.value,
                    de.unit,
                );
            }
            GmpEvent::Workflow(we) => {
                tracing::info!(
                    event_type = "workflow",
                    workflow_id = %we.workflow_id,
                    step = %we.step,
                    status = ?we.status,
                    trace_id = %we.trace_id,
                    "Workflow event: {} {}",
                    we.workflow_id,
                    format!("{:?}", we.status),
                );
            }
            GmpEvent::Document(de) => {
                tracing::info!(
                    event_type = "document",
                    doc_id = %de.doc_id,
                    event_type_tag = de.event_type.tag(),
                    actor = %de.actor,
                    trace_id = %de.trace_id,
                    "Document event: {} {}",
                    de.doc_id,
                    de.event_type.tag(),
                );
            }
            GmpEvent::Agent(ae) => {
                tracing::info!(
                    event_type = "agent",
                    agent_id = %ae.agent_id,
                    action = %ae.action,
                    trace_id = %ae.trace_id,
                    "Agent event: {} {}",
                    ae.agent_id,
                    ae.action,
                );
            }
            GmpEvent::System(se) => {
                tracing::info!(
                    event_type = "system",
                    system_type = ?se.event_type,
                    message = %se.message,
                    "System event: {:?} - {}",
                    se.event_type,
                    se.message,
                );
            }
        }
        Ok(())
    }
}

/// Extension trait for DocumentEvent to get string tag
trait EventTypeTag {
    fn tag(&self) -> &'static str;
}

impl EventTypeTag for super::DocumentEventType {
    fn tag(&self) -> &'static str {
        match self {
            super::DocumentEventType::Created => "created",
            super::DocumentEventType::Updated => "updated",
            super::DocumentEventType::Deleted => "deleted",
            super::DocumentEventType::Classified => "classified",
            super::DocumentEventType::Embedded => "embedded",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AuditAction, AuditEvent, SystemEvent, SystemEventType};

    #[test]
    fn test_audit_handler_system_event() {
        let handler = AuditHandler::new();
        assert_eq!(handler.event_types(), &[GmpEventType::All]);

        let result = handler.handle(&GmpEvent::System(SystemEvent {
            event_type: SystemEventType::Startup,
            timestamp: 0,
            message: "test".to_string(),
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_audit_handler_audit_event() {
        let handler = AuditHandler::new();
        let result = handler.handle(&GmpEvent::Audit(AuditEvent {
            event_id: "e1".to_string(),
            action: AuditAction::Query,
            entity_id: "doc1".to_string(),
            timestamp: 0,
            actor: "test".to_string(),
            hash: "abc".to_string(),
            signature: None,
            trace_id: "t1".to_string(),
            metadata: None,
        }));
        assert!(result.is_ok());
    }
}
