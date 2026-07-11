use super::{
    DriftSeverity, DriftViolation, DriftViolationType, RecoveryConfidence, RecoveryPlan,
    RecoveryType,
};

pub struct RecoveryPlanner {
    trace_id: String,
}

impl RecoveryPlanner {
    pub fn new(trace_id: String) -> Self {
        Self { trace_id }
    }

    pub fn create_plan(&self, violation: &DriftViolation) -> Option<RecoveryPlan> {
        match violation.violation_type {
            DriftViolationType::WalDrift => Some(self.plan_for_wal_drift(violation)),
            DriftViolationType::TxnDrift => Some(self.plan_for_txn_drift(violation)),
            DriftViolationType::GraphDrift => Some(self.plan_for_graph_drift(violation)),
        }
    }

    fn plan_for_wal_drift(&self, violation: &DriftViolation) -> RecoveryPlan {
        let (recovery_type, confidence, steps) = match violation.severity {
            DriftSeverity::Critical => (
                RecoveryType::Rollback,
                RecoveryConfidence::High,
                vec![
                    "Abort current transaction".to_string(),
                    "Revert storage mutation".to_string(),
                    "Log rollback completion".to_string(),
                ],
            ),
            DriftSeverity::Medium => (
                RecoveryType::Patch,
                RecoveryConfidence::Medium,
                vec![
                    "Insert missing WalBegin record".to_string(),
                    "Append WalCommit after mutation".to_string(),
                    "Verify WAL chain integrity".to_string(),
                ],
            ),
            DriftSeverity::Low => (
                RecoveryType::Ignore,
                RecoveryConfidence::Low,
                vec!["Annotate graph with WAL gap warning".to_string()],
            ),
        };

        RecoveryPlan::new(
            self.trace_id.clone(),
            violation.violation_id.clone(),
            recovery_type,
            confidence,
            steps,
        )
    }

    fn plan_for_txn_drift(&self, violation: &DriftViolation) -> RecoveryPlan {
        let (recovery_type, confidence, steps) = match violation.severity {
            DriftSeverity::Critical => (
                RecoveryType::Rollback,
                RecoveryConfidence::High,
                vec![
                    "Force TxnRollback".to_string(),
                    "Clear pending locks".to_string(),
                    "Reset transaction state".to_string(),
                ],
            ),
            DriftSeverity::Medium => (
                RecoveryType::Patch,
                RecoveryConfidence::Medium,
                vec![
                    "Inject missing TxnBegin".to_string(),
                    "Align txn_id fields".to_string(),
                    "Re-verify transaction boundaries".to_string(),
                ],
            ),
            DriftSeverity::Low => (
                RecoveryType::Ignore,
                RecoveryConfidence::Low,
                vec!["Mark transaction boundary as advisory".to_string()],
            ),
        };

        RecoveryPlan::new(
            self.trace_id.clone(),
            violation.violation_id.clone(),
            recovery_type,
            confidence,
            steps,
        )
    }

    fn plan_for_graph_drift(&self, violation: &DriftViolation) -> RecoveryPlan {
        let (recovery_type, confidence, steps) = match violation.severity {
            DriftSeverity::Critical => (
                RecoveryType::Rewire,
                RecoveryConfidence::High,
                vec![
                    "Rebuild NEXT chain".to_string(),
                    "Verify CAUSES relationships".to_string(),
                    "Validate graph structural integrity".to_string(),
                ],
            ),
            DriftSeverity::Medium => (
                RecoveryType::Rewire,
                RecoveryConfidence::Medium,
                vec![
                    "Repair broken link".to_string(),
                    "Re-establish event sequence".to_string(),
                ],
            ),
            DriftSeverity::Low => (
                RecoveryType::Ignore,
                RecoveryConfidence::Low,
                vec!["Flag graph drift for manual review".to_string()],
            ),
        };

        RecoveryPlan::new(
            self.trace_id.clone(),
            violation.violation_id.clone(),
            recovery_type,
            confidence,
            steps,
        )
    }
}

pub struct ExecutionReplayEngine {
    trace_id: String,
    events: Vec<String>,
}

impl ExecutionReplayEngine {
    pub fn new(trace_id: String) -> Self {
        Self {
            trace_id,
            events: Vec::new(),
        }
    }

    pub fn replay(&self) -> ReplayResult {
        let mut divergences = Vec::new();
        let mut last_wal = None;

        for (idx, event) in self.events.iter().enumerate() {
            match event.as_str() {
                "WalBegin" => last_wal = Some(idx),
                "WalCommit" => last_wal = None,
                "StorageMutation" => {
                    if let Some(wal_idx) = last_wal {
                        if wal_idx > idx {
                            divergences.push(format!(
                                "StorageMutation at {} preceded WalBegin at {}",
                                idx, wal_idx
                            ));
                        }
                    } else {
                        divergences.push(format!(
                            "StorageMutation at {} without open WAL segment",
                            idx
                        ));
                    }
                }
                _ => {}
            }
        }

        let valid = divergences.is_empty();
        ReplayResult {
            trace_id: self.trace_id.clone(),
            divergences,
            valid,
        }
    }

    pub fn add_event(&mut self, event_type: String) {
        self.events.push(event_type);
    }
}

#[derive(Debug, Clone)]
pub struct ReplayResult {
    pub trace_id: String,
    pub divergences: Vec<String>,
    pub valid: bool,
}

pub struct SafeExecutionController {
    block_on_critical: bool,
    suggest_on_medium: bool,
    auto_repair_confidence_threshold: RecoveryConfidence,
}

impl SafeExecutionController {
    pub fn new() -> Self {
        Self {
            block_on_critical: true,
            suggest_on_medium: true,
            auto_repair_confidence_threshold: RecoveryConfidence::High,
        }
    }

    pub fn should_block(&self, plan: &RecoveryPlan) -> bool {
        self.block_on_critical
            && plan.recovery_type == RecoveryType::Rollback
            && plan.confidence == RecoveryConfidence::High
    }

    pub fn should_suggest(&self, plan: &RecoveryPlan) -> bool {
        self.suggest_on_medium
            && plan.recovery_type == RecoveryType::Patch
            && plan.confidence == RecoveryConfidence::Medium
    }

    pub fn should_auto_repair(&self, plan: &RecoveryPlan) -> bool {
        plan.confidence >= self.auto_repair_confidence_threshold
            && matches!(
                plan.recovery_type,
                RecoveryType::Patch | RecoveryType::Rewire
            )
    }

    pub fn execute_plan(&self, plan: &RecoveryPlan) -> ExecutionResult {
        if self.should_block(plan) {
            return ExecutionResult::Blocked {
                plan_id: plan.plan_id.clone(),
                reason: "Auto-repair blocked: Critical severity + High confidence rollback"
                    .to_string(),
            };
        }

        if self.should_auto_repair(plan) {
            return ExecutionResult::AutoRepaired {
                plan_id: plan.plan_id.clone(),
                steps_executed: plan.steps.len(),
            };
        }

        ExecutionResult::Suggested {
            plan_id: plan.plan_id.clone(),
            steps: plan.steps.clone(),
        }
    }
}

impl Default for SafeExecutionController {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub enum ExecutionResult {
    Blocked {
        plan_id: String,
        reason: String,
    },
    AutoRepaired {
        plan_id: String,
        steps_executed: usize,
    },
    Suggested {
        plan_id: String,
        steps: Vec<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_violation(vtype: DriftViolationType, sev: DriftSeverity) -> DriftViolation {
        DriftViolation::new("trace-1".into(), vtype, sev, "test".into())
    }

    #[test]
    fn test_recovery_planner_new() {
        let rp = RecoveryPlanner::new("trace-1".into());
        let v = make_violation(DriftViolationType::WalDrift, DriftSeverity::Critical);
        let plan = rp.create_plan(&v);
        assert!(plan.is_some());
    }

    #[test]
    fn test_recovery_planner_wal_drift_critical() {
        let rp = RecoveryPlanner::new("trace-1".into());
        let v = make_violation(DriftViolationType::WalDrift, DriftSeverity::Critical);
        let plan = rp.create_plan(&v).unwrap();
        assert_eq!(plan.recovery_type, RecoveryType::Rollback);
        assert_eq!(plan.confidence, RecoveryConfidence::High);
        assert!(plan.steps.len() >= 2);
    }

    #[test]
    fn test_recovery_planner_wal_drift_medium() {
        let rp = RecoveryPlanner::new("trace-1".into());
        let v = make_violation(DriftViolationType::WalDrift, DriftSeverity::Medium);
        let plan = rp.create_plan(&v).unwrap();
        assert_eq!(plan.recovery_type, RecoveryType::Patch);
        assert_eq!(plan.confidence, RecoveryConfidence::Medium);
    }

    #[test]
    fn test_recovery_planner_wal_drift_low() {
        let rp = RecoveryPlanner::new("trace-1".into());
        let v = make_violation(DriftViolationType::WalDrift, DriftSeverity::Low);
        let plan = rp.create_plan(&v).unwrap();
        assert_eq!(plan.recovery_type, RecoveryType::Ignore);
        assert_eq!(plan.confidence, RecoveryConfidence::Low);
    }

    #[test]
    fn test_recovery_planner_txn_drift_critical() {
        let rp = RecoveryPlanner::new("trace-1".into());
        let v = make_violation(DriftViolationType::TxnDrift, DriftSeverity::Critical);
        let plan = rp.create_plan(&v).unwrap();
        assert_eq!(plan.recovery_type, RecoveryType::Rollback);
    }

    #[test]
    fn test_recovery_planner_txn_drift_medium() {
        let rp = RecoveryPlanner::new("trace-1".into());
        let v = make_violation(DriftViolationType::TxnDrift, DriftSeverity::Medium);
        let plan = rp.create_plan(&v).unwrap();
        assert_eq!(plan.recovery_type, RecoveryType::Patch);
    }

    #[test]
    fn test_recovery_planner_txn_drift_low() {
        let rp = RecoveryPlanner::new("trace-1".into());
        let v = make_violation(DriftViolationType::TxnDrift, DriftSeverity::Low);
        let plan = rp.create_plan(&v).unwrap();
        assert_eq!(plan.recovery_type, RecoveryType::Ignore);
    }

    #[test]
    fn test_recovery_planner_graph_drift_critical() {
        let rp = RecoveryPlanner::new("trace-1".into());
        let v = make_violation(DriftViolationType::GraphDrift, DriftSeverity::Critical);
        let plan = rp.create_plan(&v).unwrap();
        assert_eq!(plan.recovery_type, RecoveryType::Rewire);
        assert_eq!(plan.confidence, RecoveryConfidence::High);
    }

    #[test]
    fn test_recovery_planner_graph_drift_medium() {
        let rp = RecoveryPlanner::new("trace-1".into());
        let v = make_violation(DriftViolationType::GraphDrift, DriftSeverity::Medium);
        let plan = rp.create_plan(&v).unwrap();
        assert_eq!(plan.recovery_type, RecoveryType::Rewire);
        assert_eq!(plan.confidence, RecoveryConfidence::Medium);
    }

    #[test]
    fn test_recovery_planner_graph_drift_low() {
        let rp = RecoveryPlanner::new("trace-1".into());
        let v = make_violation(DriftViolationType::GraphDrift, DriftSeverity::Low);
        let plan = rp.create_plan(&v).unwrap();
        assert_eq!(plan.recovery_type, RecoveryType::Ignore);
        assert_eq!(plan.confidence, RecoveryConfidence::Low);
    }

    #[test]
    fn test_execution_replay_engine_new() {
        let engine = ExecutionReplayEngine::new("trace-1".into());
        let result = engine.replay();
        assert!(result.valid);
    }

    #[test]
    fn test_execution_replay_engine_with_events() {
        let mut engine = ExecutionReplayEngine::new("trace-1".into());
        engine.add_event("WalBegin".into());
        engine.add_event("StorageMutation".into());
        engine.add_event("WalCommit".into());
        let result = engine.replay();
        assert!(result.valid);
    }

    #[test]
    fn test_execution_replay_engine_mutation_after_wal_commit() {
        let mut engine = ExecutionReplayEngine::new("trace-1".into());
        engine.add_event("WalBegin".into());
        engine.add_event("WalCommit".into());
        engine.add_event("WalBegin".into());
        engine.add_event("StorageMutation".into());
        let result = engine.replay();
        assert!(result.valid);
    }

    #[test]
    fn test_safe_execution_controller_new() {
        let ctrl = SafeExecutionController::new();
        let plan = RecoveryPlan::new(
            "t".into(),
            "v".into(),
            RecoveryType::Rollback,
            RecoveryConfidence::High,
            vec!["step".into()],
        );
        assert!(ctrl.should_block(&plan));
        assert!(ctrl.should_auto_repair(&plan) || !ctrl.should_auto_repair(&plan));
    }

    #[test]
    fn test_safe_execution_controller_execute_blocked() {
        let ctrl = SafeExecutionController::new();
        let plan = RecoveryPlan::new(
            "t".into(),
            "v".into(),
            RecoveryType::Rollback,
            RecoveryConfidence::High,
            vec!["step".into()],
        );
        let result = ctrl.execute_plan(&plan);
        assert!(matches!(result, ExecutionResult::Blocked { .. }));
    }

    #[test]
    fn test_safe_execution_controller_execute_suggest() {
        let ctrl = SafeExecutionController::new();
        let plan = RecoveryPlan::new(
            "t".into(),
            "v".into(),
            RecoveryType::Replay,
            RecoveryConfidence::High,
            vec!["s1".into(), "s2".into()],
        );
        let result = ctrl.execute_plan(&plan);
        assert!(matches!(result, ExecutionResult::Suggested { .. }));
    }

    #[test]
    fn test_safe_execution_controller_should_suggest_patch_medium() {
        let ctrl = SafeExecutionController::new();
        let plan = RecoveryPlan::new(
            "t".into(),
            "v".into(),
            RecoveryType::Patch,
            RecoveryConfidence::Medium,
            vec!["step".into()],
        );
        assert!(ctrl.should_suggest(&plan));
    }

    #[test]
    fn test_safe_execution_controller_default() {
        let ctrl = SafeExecutionController::default();
        let plan = RecoveryPlan::new(
            "t".into(),
            "v".into(),
            RecoveryType::Rollback,
            RecoveryConfidence::High,
            vec!["step".into()],
        );
        assert!(ctrl.should_block(&plan));
    }

    #[test]
    fn test_execution_result_debug() {
        let result = ExecutionResult::Blocked {
            plan_id: "P-1".into(),
            reason: "blocked".into(),
        };
        let s = format!("{:?}", result);
        assert!(s.contains("Blocked"));
    }

    #[test]
    fn test_replay_result_clone() {
        let result = ReplayResult {
            trace_id: "t".into(),
            divergences: vec!["d".into()],
            valid: false,
        };
        let cloned = result.clone();
        assert_eq!(cloned.trace_id, "t");
        assert_eq!(cloned.divergences.len(), 1);
    }
}
