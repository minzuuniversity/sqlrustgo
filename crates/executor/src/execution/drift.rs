use super::{DriftSeverity, DriftViolation, DriftViolationType, ExecutionEvent};

pub struct DriftDetector {
    trace_id: String,
    events: Vec<ExecutionEvent>,
    violations: Vec<DriftViolation>,
}

impl DriftDetector {
    pub fn new(trace_id: String) -> Self {
        Self {
            trace_id,
            events: Vec::new(),
            violations: Vec::new(),
        }
    }

    pub fn add_event(&mut self, event: ExecutionEvent) {
        if let Some(violation) = self.check_rule_violation(&event) {
            self.violations.push(violation);
        }
        self.events.push(event);
    }

    pub fn add_events(&mut self, events: Vec<ExecutionEvent>) {
        for event in events {
            self.add_event(event);
        }
    }

    fn check_rule_violation(&self, event: &ExecutionEvent) -> Option<DriftViolation> {
        let event_type = event.event_type();

        match event_type {
            "StorageMutation" => self.check_wal_before_mutation(),
            "TxnCommit" => self.check_txn_boundary(),
            _ => None,
        }
    }

    fn check_wal_before_mutation(&self) -> Option<DriftViolation> {
        let has_wal_begin = self.events.iter().any(|e| e.event_type() == "WalBegin");
        if !has_wal_begin {
            return Some(DriftViolation::new(
                self.trace_id.clone(),
                DriftViolationType::WalDrift,
                DriftSeverity::Critical,
                "StorageMutation without preceding WalBegin".to_string(),
            ));
        }

        let last_wal = self
            .events
            .iter()
            .rev()
            .find(|e| e.event_type() == "WalBegin" || e.event_type() == "WalCommit");
        match last_wal {
            Some(e) if e.event_type() == "WalBegin" => None,
            Some(_) | None => Some(DriftViolation::new(
                self.trace_id.clone(),
                DriftViolationType::WalDrift,
                DriftSeverity::Medium,
                "StorageMutation without open WAL segment".to_string(),
            )),
        }
    }

    fn check_txn_boundary(&self) -> Option<DriftViolation> {
        let has_txn_begin = self.events.iter().any(|e| e.event_type() == "TxnBegin");
        if !has_txn_begin {
            return Some(DriftViolation::new(
                self.trace_id.clone(),
                DriftViolationType::TxnDrift,
                DriftSeverity::Critical,
                "TxnCommit without preceding TxnBegin".to_string(),
            ));
        }

        let txn_id = self
            .events
            .iter()
            .rev()
            .find(|e| e.event_type() == "TxnBegin" || e.event_type() == "TxnCommit");
        match txn_id {
            Some(e) if e.event_type() == "TxnBegin" => None,
            Some(_) | None => Some(DriftViolation::new(
                self.trace_id.clone(),
                DriftViolationType::TxnDrift,
                DriftSeverity::Medium,
                "TxnCommit without open transaction".to_string(),
            )),
        }
    }

    pub fn has_violations(&self) -> bool {
        !self.violations.is_empty()
    }

    pub fn has_critical(&self) -> bool {
        self.violations
            .iter()
            .any(|v| v.severity == DriftSeverity::Critical)
    }

    pub fn violations(&self) -> &[DriftViolation] {
        &self.violations
    }

    pub fn clear(&mut self) {
        self.events.clear();
        self.violations.clear();
    }

    pub fn to_cypher_statements(&self) -> Vec<serde_json::Value> {
        vec![]
    }
}

impl Clone for DriftDetector {
    fn clone(&self) -> Self {
        Self {
            trace_id: self.trace_id.clone(),
            events: self.events.clone(),
            violations: self.violations.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_event(typ: &str) -> ExecutionEvent {
        match typ {
            "TxnBegin" => ExecutionEvent::TxnBegin { txn_id: 1 },
            "TxnCommit" => ExecutionEvent::TxnCommit { txn_id: 1 },
            "WalBegin" => ExecutionEvent::WalBegin { txn_id: 1 },
            "WalCommit" => ExecutionEvent::WalCommit { txn_id: 1 },
            "Mutation" => ExecutionEvent::StorageMutation {
                table: "t".into(),
                op: super::super::DmlOperation::Insert,
            },
            "Sql" => ExecutionEvent::SqlReceived {
                sql: "SELECT 1".into(),
            },
            _ => ExecutionEvent::SqlReceived {
                sql: "SELECT 1".into(),
            },
        }
    }

    #[test]
    fn test_detector_new() {
        let d = DriftDetector::new("trace-1".into());
        assert!(!d.has_violations());
        assert!(!d.has_critical());
    }

    #[test]
    fn test_detector_no_violation_wal_before_mutation() {
        let mut d = DriftDetector::new("trace-1".into());
        d.add_event(make_event("WalBegin"));
        d.add_event(make_event("Mutation"));
        assert!(!d.has_violations());
    }

    #[test]
    fn test_detector_critical_mutation_without_wal() {
        let mut d = DriftDetector::new("trace-1".into());
        d.add_event(make_event("Mutation"));
        assert!(d.has_critical());
        assert_eq!(d.violations().len(), 1);
        let v = &d.violations()[0];
        assert!(matches!(v.violation_type, DriftViolationType::WalDrift));
        assert!(matches!(v.severity, DriftSeverity::Critical));
    }

    #[test]
    fn test_detector_txn_commit_without_begin() {
        let mut d = DriftDetector::new("trace-1".into());
        d.add_event(make_event("TxnCommit"));
        assert!(d.has_critical());
        let v = &d.violations()[0];
        assert!(matches!(v.violation_type, DriftViolationType::TxnDrift));
    }

    #[test]
    fn test_detector_txn_commit_after_begin_no_violation() {
        let mut d = DriftDetector::new("trace-1".into());
        d.add_event(make_event("TxnBegin"));
        d.add_event(make_event("TxnCommit"));
        assert!(!d.has_violations());
    }

    #[test]
    fn test_detector_add_events_batch() {
        let mut d = DriftDetector::new("trace-1".into());
        d.add_events(vec![make_event("WalBegin"), make_event("Mutation")]);
        assert!(!d.has_violations());
    }

    #[test]
    fn test_detector_mutation_after_wal_commit_violation() {
        let mut d = DriftDetector::new("trace-1".into());
        d.add_event(make_event("WalBegin"));
        d.add_event(make_event("WalCommit"));
        d.add_event(make_event("Mutation"));
        assert!(d.has_violations());
    }

    #[test]
    fn test_detector_mixed_events() {
        let mut d = DriftDetector::new("trace-1".into());
        d.add_event(make_event("Sql"));
        d.add_event(make_event("TxnBegin"));
        d.add_event(make_event("WalBegin"));
        d.add_event(make_event("Mutation"));
        d.add_event(make_event("WalCommit"));
        d.add_event(make_event("TxnCommit"));
        assert!(!d.has_violations());
    }

    #[test]
    fn test_detector_clear() {
        let mut d = DriftDetector::new("trace-1".into());
        d.add_event(make_event("Mutation"));
        assert!(d.has_violations());
        d.clear();
        assert!(!d.has_violations());
    }

    #[test]
    fn test_detector_clone() {
        let mut d = DriftDetector::new("trace-1".into());
        d.add_event(make_event("WalBegin"));
        d.add_event(make_event("Mutation"));
        let d2 = d.clone();
        assert!(!d2.has_violations());
    }
}
