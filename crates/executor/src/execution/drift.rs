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
            .find(|e| e.event_type() == "TxnCommit")
            .and_then(|e| e.txn_id());
        let txn_begin_id = self
            .events
            .iter()
            .find(|e| e.event_type() == "TxnBegin")
            .and_then(|e| e.txn_id());

        if let (Some(commit_id), Some(begin_id)) = (txn_id, txn_begin_id) {
            if commit_id != begin_id {
                return Some(DriftViolation::new(
                    self.trace_id.clone(),
                    DriftViolationType::TxnDrift,
                    DriftSeverity::Medium,
                    format!(
                        "TxnCommit id {} does not match TxnBegin id {}",
                        commit_id, begin_id
                    ),
                ));
            }
        }

        None
    }

    pub fn violations(&self) -> &[DriftViolation] {
        &self.violations
    }

    pub fn has_critical(&self) -> bool {
        self.violations
            .iter()
            .any(|v| v.severity == DriftSeverity::Critical)
    }

    pub fn has_violations(&self) -> bool {
        !self.violations.is_empty()
    }

    pub fn clear(&mut self) {
        self.events.clear();
        self.violations.clear();
    }

    pub fn to_cypher_statements(&self) -> Vec<serde_json::Value> {
        self.violations.iter().map(|v| {
            let (violation_type, severity) = match (&v.violation_type, &v.severity) {
                (DriftViolationType::WalDrift, DriftSeverity::Critical) => ("WAL_DRIFT", "CRITICAL"),
                (DriftViolationType::WalDrift, DriftSeverity::Medium) => ("WAL_DRIFT", "MEDIUM"),
                (DriftViolationType::WalDrift, DriftSeverity::Low) => ("WAL_DRIFT", "LOW"),
                (DriftViolationType::TxnDrift, DriftSeverity::Critical) => ("TXN_DRIFT", "CRITICAL"),
                (DriftViolationType::TxnDrift, DriftSeverity::Medium) => ("TXN_DRIFT", "MEDIUM"),
                (DriftViolationType::TxnDrift, DriftSeverity::Low) => ("TXN_DRIFT", "LOW"),
                (DriftViolationType::GraphDrift, DriftSeverity::Critical) => ("GRAPH_DRIFT", "CRITICAL"),
                (DriftViolationType::GraphDrift, DriftSeverity::Medium) => ("GRAPH_DRIFT", "MEDIUM"),
                (DriftViolationType::GraphDrift, DriftSeverity::Low) => ("GRAPH_DRIFT", "LOW"),
            };

            serde_json::json!({
                "statement": "CREATE (v:DriftViolation {violation_id: $id, trace_id: $trace_id, type: $type, severity: $severity, description: $desc, ts: $ts})",
                "parameters": {
                    "id": v.violation_id,
                    "trace_id": v.trace_id,
                    "type": violation_type,
                    "severity": severity,
                    "desc": v.description,
                    "ts": v.detected_at,
                }
            })
        }).collect()
    }
}

pub struct GuardPolicy {
    block_on_critical: bool,
    mark_degraded_on_medium: bool,
}

impl GuardPolicy {
    pub fn new() -> Self {
        Self {
            block_on_critical: true,
            mark_degraded_on_medium: true,
        }
    }

    pub fn should_block(&self, violations: &[DriftViolation]) -> bool {
        self.block_on_critical
            && violations
                .iter()
                .any(|v| v.severity == DriftSeverity::Critical)
    }

    pub fn should_mark_degraded(&self, violations: &[DriftViolation]) -> bool {
        self.mark_degraded_on_medium
            && violations
                .iter()
                .any(|v| v.severity == DriftSeverity::Medium)
    }
}

impl Default for GuardPolicy {
    fn default() -> Self {
        Self::new()
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
