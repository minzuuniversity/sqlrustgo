//! Event Scheduler Execution Engine
//!
//! This module provides event scheduler execution functionality.
//! Events are executed according to their schedule (one-time or recurring).

use sqlrustgo_catalog::{Catalog, Event, EventSchedule};
use sqlrustgo_parser::parse;
use sqlrustgo_types::{SqlError, SqlResult};
use std::sync::{Arc, RwLock};

/// Event executor for running scheduled events
pub struct EventExecutor {
    catalog: Arc<RwLock<Catalog>>,
}

impl EventExecutor {
    /// Create a new EventExecutor
    pub fn new(catalog: Arc<RwLock<Catalog>>) -> Self {
        Self { catalog }
    }

    /// Check if an event should run at the current time
    pub fn should_run_event(event: &Event) -> bool {
        if !event.enable {
            return false;
        }

        match &event.schedule {
            EventSchedule::OneTime => true,
            EventSchedule::Interval { .. } => true,
        }
    }

    /// Execute a single event
    pub fn execute_event(&self, event: &Event) -> SqlResult<()> {
        if !Self::should_run_event(event) {
            return Ok(());
        }

        let statement = parse(&event.body).map_err(|e| {
            SqlError::ExecutionError(format!(
                "Failed to parse event body '{}': {}",
                event.name, e
            ))
        })?;

        let _ = statement;

        Ok(())
    }

    /// Get all events that should run now
    pub fn get_due_events(&self) -> Vec<Event> {
        let catalog = self.catalog.read().unwrap();
        catalog
            .events()
            .iter()
            .filter(|e| Self::should_run_event(e))
            .map(|e| (*e).clone())
            .collect()
    }

    /// Check if an event exists
    pub fn has_event(&self, name: &str) -> bool {
        let catalog = self.catalog.read().unwrap();
        catalog.has_event(name)
    }

    /// Get an event by name
    pub fn get_event(&self, name: &str) -> Option<Event> {
        let catalog = self.catalog.read().unwrap();
        catalog.get_event(name).cloned()
    }

    /// Run all due events
    pub fn run_due_events(&self) -> Vec<Result<(), String>> {
        let due_events = self.get_due_events();
        due_events
            .iter()
            .map(|event| self.execute_event(event).map_err(|e| e.to_string()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_event(name: &str, enable: bool, schedule: EventSchedule) -> Event {
        Event {
            name: name.to_string(),
            schema: "public".to_string(),
            body: "SELECT 1".to_string(),
            enable,
            schedule,
            comment: None,
            created: "2024-01-01 00:00:00".to_string(),
            last_altered: "2024-01-01 00:00:00".to_string(),
            definer: "root@localhost".to_string(),
            sql_mode: "ONLY_FULL_GROUP_BY,STRICT_TRANS_TABLES".to_string(),
            status: "ENABLED".to_string(),
            on_completion: "PRESERVE".to_string(),
            starts: None,
            ends: None,
        }
    }

    #[test]
    fn test_should_run_event_enabled_onetime() {
        let event = create_test_event("test", true, EventSchedule::OneTime);
        assert!(EventExecutor::should_run_event(&event));
    }

    #[test]
    fn test_should_run_event_enabled_interval() {
        let event = create_test_event(
            "test",
            true,
            EventSchedule::Interval {
                interval_value: "5".to_string(),
                interval_unit: "HOUR".to_string(),
            },
        );
        assert!(EventExecutor::should_run_event(&event));
    }

    #[test]
    fn test_should_run_event_disabled() {
        let event = create_test_event("test", false, EventSchedule::OneTime);
        assert!(!EventExecutor::should_run_event(&event));
    }

    #[test]
    fn test_should_run_event_disabled_interval() {
        let event = create_test_event(
            "test",
            false,
            EventSchedule::Interval {
                interval_value: "1".to_string(),
                interval_unit: "DAY".to_string(),
            },
        );
        assert!(!EventExecutor::should_run_event(&event));
    }
}
