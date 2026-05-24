//! Event Bus core trait and in-memory implementation

use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

// ─── Public Types ─────────────────────────────────────────────────────────────

pub use crate::events::{GmpEvent, GmpEventType};

/// Subscription ID returned when subscribing to the bus
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubscriptionId(u64);

/// Event handler trait — implement this to handle specific event types
pub trait EventHandler: Send + Sync {
    /// Returns the event types this handler subscribes to
    fn event_types(&self) -> &[GmpEventType];

    /// Handle an event. Return Ok(()) on success.
    fn handle(&self, event: &GmpEvent) -> Result<()>;
}

/// Event Bus trait — publish-subscribe message bus
pub trait EventBus: Send + Sync {
    /// Publish an event to the bus
    fn publish(&self, event: GmpEvent) -> Result<()>;

    /// Subscribe to events matching the handler's event_types()
    fn subscribe(&self, handler: Box<dyn EventHandler>) -> Result<SubscriptionId>;

    /// Unsubscribe by ID
    fn unsubscribe(&self, id: SubscriptionId) -> Result<()>;

    /// Get current number of subscribers
    fn subscriber_count(&self) -> usize;
}

// ─── In-Memory Implementation ────────────────────────────────────────────────

/// Simple in-process event bus using Arc<dyn EventHandler> for cheap cloning
pub struct InMemoryBus {
    handlers: Arc<RwLock<HashMap<SubscriptionId, Arc<dyn EventHandler>>>>,
    next_id: Arc<RwLock<u64>>,
}

impl InMemoryBus {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
            next_id: Arc::new(RwLock::new(0)),
        }
    }

    fn next_id(&self) -> SubscriptionId {
        let mut next = self.next_id.write().unwrap();
        let id = SubscriptionId(*next);
        *next += 1;
        id
    }
}

impl Default for InMemoryBus {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for InMemoryBus {
    fn clone(&self) -> Self {
        Self {
            handlers: Arc::clone(&self.handlers),
            next_id: Arc::clone(&self.next_id),
        }
    }
}

impl EventBus for InMemoryBus {
    fn publish(&self, event: GmpEvent) -> Result<()> {
        let event_type = GmpEventType::from_event(&event);
        let handlers: Vec<Arc<dyn EventHandler>> = {
            let handlers = self.handlers.read().unwrap();
            handlers
                .values()
                .filter(|h| {
                    h.event_types().contains(&GmpEventType::All)
                        || h.event_types().contains(&event_type)
                })
                .cloned()
                .collect()
        };

        for handler in handlers {
            if let Err(e) = handler.handle(&event) {
                tracing::error!(error = %e, "Event handler error");
            }
        }
        Ok(())
    }

    fn subscribe(&self, handler: Box<dyn EventHandler>) -> Result<SubscriptionId> {
        let id = self.next_id();
        let mut handlers = self.handlers.write().unwrap();
        handlers.insert(id, Arc::from(handler));
        Ok(id)
    }

    fn unsubscribe(&self, id: SubscriptionId) -> Result<()> {
        let mut handlers = self.handlers.write().unwrap();
        handlers.remove(&id);
        Ok(())
    }

    fn subscriber_count(&self) -> usize {
        let handlers = self.handlers.read().unwrap();
        handlers.len()
    }
}
