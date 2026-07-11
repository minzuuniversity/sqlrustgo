use super::{DmlOperation, DriftDetector, DriftViolation, ExecutionEvent, GuardPolicy};
use std::sync::{Arc, Mutex};

pub struct EventBuffer {
    events: Vec<ExecutionEvent>,
    capacity: usize,
    seq: usize,
}

impl EventBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            events: Vec::with_capacity(capacity),
            capacity,
            seq: 0,
        }
    }

    pub fn push(&mut self, event: ExecutionEvent) -> Option<Vec<ExecutionEvent>> {
        self.seq += 1;
        self.events.push(event);
        if self.events.len() >= self.capacity {
            Some(self.drain())
        } else {
            None
        }
    }

    fn drain(&mut self) -> Vec<ExecutionEvent> {
        let events = std::mem::take(&mut self.events);
        self.events = Vec::with_capacity(self.capacity);
        events
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn seq(&self) -> usize {
        self.seq
    }

    pub fn reset_seq(&mut self) {
        self.seq = 0;
    }
}

impl Default for EventBuffer {
    fn default() -> Self {
        Self::new(256)
    }
}

pub struct TelemetryCollector {
    buffer: Arc<Mutex<EventBuffer>>,
    trace_id: String,
    enabled: bool,
    detector: DriftDetector,
    policy: GuardPolicy,
}

impl TelemetryCollector {
    pub fn new(trace_id: String) -> Self {
        Self {
            buffer: Arc::new(Mutex::new(EventBuffer::new(256))),
            trace_id: trace_id.clone(),
            enabled: true,
            detector: DriftDetector::new(trace_id),
            policy: GuardPolicy::new(),
        }
    }

    pub fn with_capacity(trace_id: String, capacity: usize) -> Self {
        Self {
            buffer: Arc::new(Mutex::new(EventBuffer::new(capacity))),
            trace_id: trace_id.clone(),
            enabled: true,
            detector: DriftDetector::new(trace_id),
            policy: GuardPolicy::new(),
        }
    }

    pub fn emit(&self, event: ExecutionEvent) {
        if !self.enabled {
            return;
        }

        let mut detector = self.detector.clone();
        detector.add_event(event.clone());

        if detector.has_critical() && self.policy.should_block(detector.violations()) {
            return;
        }

        let mut buf = match self.buffer.lock() {
            Ok(b) => b,
            Err(_) => return,
        };

        if let Some(batch) = buf.push(event) {
            drop(buf);
            let violation_stmts = detector.to_cypher_statements();
            if !violation_stmts.is_empty() {
                self.send_to_neo4j(&violation_stmts);
            }
            self.flush(batch);
        }
    }

    pub fn violations(&self) -> Vec<DriftViolation> {
        self.detector.violations().to_vec()
    }

    pub fn has_violations(&self) -> bool {
        self.detector.has_violations()
    }

    fn flush(&self, events: Vec<ExecutionEvent>) {
        if events.is_empty() {
            return;
        }
        let trace_node = self.build_trace_node();
        let event_nodes = self.build_linked_events(&events);
        let all_statements = trace_node
            .into_iter()
            .chain(event_nodes)
            .collect::<Vec<_>>();
        self.send_to_neo4j(&all_statements);
    }

    fn build_trace_node(&self) -> Vec<serde_json::Value> {
        vec![serde_json::json!({
            "statement": "MERGE (t:ExecutionTrace { trace_id: $trace_id }) ON CREATE SET t.created_at = timestamp()",
            "parameters": {
                "trace_id": self.trace_id,
            }
        })]
    }

    fn build_linked_events(&self, events: &[ExecutionEvent]) -> Vec<serde_json::Value> {
        let mut statements = Vec::new();
        let mut prev_id: Option<usize> = None;

        for (idx, e) in events.iter().enumerate() {
            let (event_type, table, rows, txn_id) = match e {
                ExecutionEvent::SqlReceived { sql } => ("SqlReceived", sql.clone(), 0, None),
                ExecutionEvent::TxnBegin { txn_id } => {
                    ("TxnBegin", String::new(), 0, Some(*txn_id))
                }
                ExecutionEvent::TxnCommit { txn_id } => {
                    ("TxnCommit", String::new(), 0, Some(*txn_id))
                }
                ExecutionEvent::TxnRollback { txn_id } => {
                    ("TxnRollback", String::new(), 0, Some(*txn_id))
                }
                ExecutionEvent::WalBegin { txn_id } => {
                    ("WalBegin", String::new(), 0, Some(*txn_id))
                }
                ExecutionEvent::WalWrite { txn_id, segment } => {
                    ("WalWrite", segment.clone(), 0, Some(*txn_id))
                }
                ExecutionEvent::WalCommit { txn_id } => {
                    ("WalCommit", String::new(), 0, Some(*txn_id))
                }
                ExecutionEvent::StorageRead { table, rows } => {
                    ("StorageRead", table.clone(), *rows, None)
                }
                ExecutionEvent::StorageWrite { table, rows } => {
                    ("StorageWrite", table.clone(), *rows, None)
                }
                ExecutionEvent::StorageMutation { table, op } => {
                    let op_name = match op {
                        DmlOperation::Insert => "INSERT",
                        DmlOperation::Update => "UPDATE",
                        DmlOperation::Delete => "DELETE",
                    };
                    (op_name, table.clone(), 0, None)
                }
                ExecutionEvent::BoundaryCheck { module, passed } => (
                    "BoundaryCheck",
                    module.clone(),
                    if *passed { 1 } else { 0 },
                    None,
                ),
                ExecutionEvent::VtuValidate { result } => (
                    "VtuValidate",
                    String::new(),
                    if *result { 1 } else { 0 },
                    None,
                ),
            };

            let event_id = format!("{}_{}", self.trace_id, idx);

            statements.push(serde_json::json!({
                "statement": "MATCH (t:ExecutionTrace {trace_id: $trace_id}) CREATE (t)-[:HAS_EVENT]->(e:ExecutionEvent {id: $id, type: $type, table: $table, rows: $rows, txn_id: $txn_id, seq: $seq, ts: timestamp()})",
                "parameters": {
                    "trace_id": self.trace_id,
                    "id": event_id,
                    "type": event_type,
                    "table": table,
                    "rows": rows,
                    "txn_id": txn_id,
                    "seq": idx,
                }
            }));

            if let Some(prev) = prev_id {
                let prev_event_id = format!("{}_{}", self.trace_id, prev);
                statements.push(serde_json::json!({
                    "statement": "MATCH (e1:ExecutionEvent {id: $prev_id}), (e2:ExecutionEvent {id: $curr_id}) CREATE (e1)-[:NEXT]->(e2)",
                    "parameters": {
                        "prev_id": prev_event_id,
                        "curr_id": event_id,
                    }
                }));

                if Self::is_causal_link(event_type) {
                    let cause_event_id = format!("{}_{}", self.trace_id, prev);
                    statements.push(serde_json::json!({
                        "statement": "MATCH (cause:ExecutionEvent {id: $cause_id}), (effect:ExecutionEvent {id: $effect_id}) CREATE (cause)-[:CAUSES]->(effect)",
                        "parameters": {
                            "cause_id": cause_event_id,
                            "effect_id": event_id,
                        }
                    }));
                }
            }

            prev_id = Some(idx);
        }

        statements
    }

    fn is_causal_link(event_type: &str) -> bool {
        matches!(event_type, "StorageMutation" | "WalCommit" | "TxnCommit")
    }

    fn send_to_neo4j(&self, statements: &[serde_json::Value]) {
        let payload = serde_json::json!({
            "statements": statements
        });
        let client = match reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
        {
            Ok(c) => c,
            Err(_) => return,
        };
        let _ = client
            .post("http://127.0.0.1:7474/db/neo4j/tx/commit")
            .header("Content-Type", "application/json")
            .basic_auth("neo4j", Some("Neo4jAdmin2026"))
            .json(&payload)
            .send();
    }

    pub fn trace_id(&self) -> &str {
        &self.trace_id
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

impl std::fmt::Debug for EventBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventBuffer")
            .field("events.len()", &self.events.len())
            .field("capacity", &self.capacity)
            .finish()
    }
}

impl std::fmt::Debug for TelemetryCollector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TelemetryCollector")
            .field("trace_id", &self.trace_id)
            .field("enabled", &self.enabled)
            .finish()
    }
}

impl Clone for TelemetryCollector {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer.clone(),
            trace_id: self.trace_id.clone(),
            enabled: self.enabled,
            detector: self.detector.clone(),
            policy: GuardPolicy::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_buffer_new() {
        let buf = EventBuffer::new(10);
        assert_eq!(buf.len(), 0);
        assert!(buf.is_empty());
        assert_eq!(buf.seq(), 0);
    }

    #[test]
    fn test_event_buffer_default() {
        let buf = EventBuffer::default();
        assert_eq!(buf.capacity, 256);
    }

    #[test]
    fn test_event_buffer_push_under_capacity() {
        let mut buf = EventBuffer::new(10);
        let batch = buf.push(ExecutionEvent::SqlReceived {
            sql: "SELECT 1".into(),
        });
        assert!(batch.is_none());
        assert_eq!(buf.len(), 1);
        assert_eq!(buf.seq(), 1);
    }

    #[test]
    fn test_event_buffer_push_at_capacity() {
        let mut buf = EventBuffer::new(2);
        buf.push(ExecutionEvent::SqlReceived { sql: "a".into() });
        let batch = buf.push(ExecutionEvent::SqlReceived { sql: "b".into() });
        assert!(batch.is_some());
        assert_eq!(batch.unwrap().len(), 2);
        assert!(buf.is_empty());
    }

    #[test]
    fn test_event_buffer_seq_increment() {
        let mut buf = EventBuffer::new(10);
        buf.push(ExecutionEvent::SqlReceived { sql: "1".into() });
        buf.push(ExecutionEvent::SqlReceived { sql: "2".into() });
        assert_eq!(buf.seq(), 2);
    }

    #[test]
    fn test_event_buffer_reset_seq() {
        let mut buf = EventBuffer::new(10);
        buf.push(ExecutionEvent::SqlReceived { sql: "1".into() });
        assert_eq!(buf.seq(), 1);
        buf.reset_seq();
        assert_eq!(buf.seq(), 0);
    }

    #[test]
    fn test_telemetry_collector_new() {
        let tc = TelemetryCollector::new("trace-1".into());
        assert_eq!(tc.trace_id(), "trace-1");
        assert!(!tc.has_violations());
    }

    #[test]
    fn test_telemetry_collector_with_capacity() {
        let tc = TelemetryCollector::with_capacity("trace-2".into(), 64);
        assert_eq!(tc.trace_id(), "trace-2");
    }

    #[test]
    fn test_telemetry_collector_emit_disabled() {
        let mut tc = TelemetryCollector::new("trace-1".into());
        tc.set_enabled(false);
        tc.emit(ExecutionEvent::SqlReceived {
            sql: "SELECT 1".into(),
        });
        assert!(!tc.has_violations());
    }

    #[test]
    fn test_telemetry_collector_emit_valid_sequence() {
        let tc = TelemetryCollector::new("trace-1".into());
        tc.emit(ExecutionEvent::TxnBegin { txn_id: 1 });
        tc.emit(ExecutionEvent::WalBegin { txn_id: 1 });
        tc.emit(ExecutionEvent::StorageMutation {
            table: "t".into(),
            op: DmlOperation::Insert,
        });
        tc.emit(ExecutionEvent::WalCommit { txn_id: 1 });
        tc.emit(ExecutionEvent::TxnCommit { txn_id: 1 });
        assert!(!tc.has_violations());
    }

    #[test]
    fn test_telemetry_collector_emit_storage_events() {
        let tc = TelemetryCollector::new("trace-1".into());
        tc.emit(ExecutionEvent::StorageRead {
            table: "t".into(),
            rows: 100,
        });
        tc.emit(ExecutionEvent::StorageWrite {
            table: "t".into(),
            rows: 5,
        });
        assert!(!tc.has_violations());
    }

    #[test]
    fn test_telemetry_collector_boundary_and_vtu() {
        let tc = TelemetryCollector::new("trace-1".into());
        tc.emit(ExecutionEvent::BoundaryCheck {
            module: "wal".into(),
            passed: true,
        });
        tc.emit(ExecutionEvent::VtuValidate { result: false });
        assert!(!tc.has_violations());
    }

    #[test]
    fn test_telemetry_collector_clone() {
        let tc = TelemetryCollector::new("trace-1".into());
        let tc2 = tc.clone();
        assert_eq!(tc2.trace_id(), "trace-1");
    }

    #[test]
    fn test_telemetry_collector_violations_accessor() {
        let tc = TelemetryCollector::new("trace-1".into());
        let _v = tc.violations();
    }

    #[test]
    fn test_is_causal_link() {
        assert!(TelemetryCollector::is_causal_link("StorageMutation"));
        assert!(TelemetryCollector::is_causal_link("WalCommit"));
        assert!(TelemetryCollector::is_causal_link("TxnCommit"));
        assert!(!TelemetryCollector::is_causal_link("SqlReceived"));
        assert!(!TelemetryCollector::is_causal_link("StorageRead"));
    }

    #[test]
    fn test_event_buffer_with_default_size() {
        let buf = EventBuffer::default();
        assert_eq!(buf.capacity, 256);
    }

    #[test]
    fn test_event_buffer_push_many() {
        let mut buf = EventBuffer::new(3);
        for i in 0..5 {
            buf.push(ExecutionEvent::SqlReceived {
                sql: format!("q{}", i),
            });
        }
        assert!(buf.len() <= 3);
    }

    #[test]
    fn test_telemetry_collector_trace_id() {
        let tc = TelemetryCollector::new("trace-X".into());
        assert_eq!(tc.trace_id(), "trace-X");
    }

    #[test]
    fn test_telemetry_collector_has_violations_false() {
        let tc = TelemetryCollector::new("trace".into());
        assert!(!tc.has_violations());
    }

    #[test]
    fn test_telemetry_collector_emit_wal_events() {
        let tc = TelemetryCollector::new("trace".into());
        tc.emit(ExecutionEvent::WalBegin { txn_id: 1 });
        tc.emit(ExecutionEvent::WalWrite {
            txn_id: 1,
            segment: "seg1".into(),
        });
        tc.emit(ExecutionEvent::WalCommit { txn_id: 1 });
        assert!(!tc.has_violations());
    }

    #[test]
    fn test_telemetry_collector_emit_txn_events() {
        let tc = TelemetryCollector::new("trace".into());
        tc.emit(ExecutionEvent::TxnBegin { txn_id: 1 });
        tc.emit(ExecutionEvent::TxnRollback { txn_id: 1 });
        assert!(!tc.has_violations());
    }

    #[test]
    fn test_event_buffer_zero_capacity() {
        let mut buf = EventBuffer::new(0);
        let batch = buf.push(ExecutionEvent::SqlReceived { sql: "q".into() });
        assert!(batch.is_some());
    }

    #[test]
    fn test_event_buffer_drain_returns_events_in_order() {
        let mut buf = EventBuffer::new(2);
        buf.push(ExecutionEvent::SqlReceived { sql: "a".into() });
        let batch = buf.push(ExecutionEvent::SqlReceived { sql: "b".into() });
        let drained = batch.unwrap();
        if let ExecutionEvent::SqlReceived { sql } = &drained[0] {
            assert_eq!(sql, "a");
        } else {
            panic!("Expected SqlReceived");
        }
        if let ExecutionEvent::SqlReceived { sql } = &drained[1] {
            assert_eq!(sql, "b");
        } else {
            panic!("Expected SqlReceived");
        }
    }

    #[test]
    fn test_event_buffer_drain_creates_new_buffer() {
        let mut buf = EventBuffer::new(2);
        buf.push(ExecutionEvent::SqlReceived { sql: "a".into() });
        let batch = buf.push(ExecutionEvent::SqlReceived { sql: "b".into() });
        assert!(batch.is_some());
        assert_eq!(buf.len(), 0);
        assert!(buf.is_empty());
    }

    #[test]
    fn test_telemetry_collector_emit_capacity_3() {
        let tc = TelemetryCollector::with_capacity("trace".into(), 3);
        tc.emit(ExecutionEvent::SqlReceived { sql: "q1".into() });
        tc.emit(ExecutionEvent::SqlReceived { sql: "q2".into() });
        tc.emit(ExecutionEvent::SqlReceived { sql: "q3".into() });
    }

    #[test]
    fn test_telemetry_collector_emit_all_dml_ops() {
        let tc = TelemetryCollector::new("trace".into());
        tc.emit(ExecutionEvent::StorageMutation {
            table: "t".into(),
            op: DmlOperation::Insert,
        });
        tc.emit(ExecutionEvent::StorageMutation {
            table: "t".into(),
            op: DmlOperation::Update,
        });
        tc.emit(ExecutionEvent::StorageMutation {
            table: "t".into(),
            op: DmlOperation::Delete,
        });
    }

    #[test]
    fn test_telemetry_collector_emit_with_storage_read() {
        let tc = TelemetryCollector::new("trace".into());
        tc.emit(ExecutionEvent::StorageRead {
            table: "t".into(),
            rows: 10,
        });
        tc.emit(ExecutionEvent::StorageWrite {
            table: "t".into(),
            rows: 5,
        });
        assert!(!tc.has_violations());
    }
}
