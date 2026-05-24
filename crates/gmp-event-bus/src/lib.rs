//! GMP Event Bus - Unified event propagation for GMP platform
//!
//! Provides a channel-based event bus for decoupled event handling across
//! GMP platform modules (Device, Audit, Workflow, Document, Agent, System).
//!
//! # Architecture
//!
//! ```text
//! Publisher → EventBus → Handlers
//!                         ├── AuditHandler (logging/audit trail)
//!                         ├── WorkflowHandler (workflow triggers)
//!                         └── KnowledgeGraphHandler (graph updates)
//! ```
//!
//! # Usage
//!
//! ```
//! use gmp_event_bus::{GmpEvent, AuditEvent, AuditAction, InMemoryBus, EventBus};
//!
//! let bus = InMemoryBus::new();
//! bus.publish(GmpEvent::Audit(AuditEvent {
//!     event_id: "test-1".to_string(),
//!     action: AuditAction::Insert,
//!     entity_id: "doc-1".to_string(),
//!     timestamp: 0,
//!     actor: "system".to_string(),
//!     hash: "abc".to_string(),
//!     signature: None,
//!     trace_id: "trace-1".to_string(),
//!     metadata: None,
//! })).unwrap();
//! ```

pub mod bus;
pub mod events;
pub mod handlers;
pub mod middleware;
pub mod mqtt;

pub use bus::{EventBus, EventHandler, GmpEventType, InMemoryBus, SubscriptionId};
pub use events::{
    AgentEvent, AuditAction, AuditEvent, DeviceEvent, DeviceReading, DeviceReadingType,
    DocumentEvent, DocumentEventType, GmpEvent, SystemEvent, SystemEventType, WorkflowEvent,
    WorkflowStatus,
};
pub use handlers::AuditHandler;
pub use middleware::{AuditContext, AuditEmitter, hash_event, unix_timestamp};
pub use mqtt::{MqttGateway, MqttGatewayHandle};
