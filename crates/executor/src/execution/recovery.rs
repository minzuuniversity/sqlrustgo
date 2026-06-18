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
    use crate::execution::drift_gate::{DriftSeverity, DriftViolation, DriftViolationType};
    use crate::execution::events::{RecoveryConfidence, RecoveryType};

    fn make_violation(vt: DriftViolationType, sev: DriftSeverity) -> DriftViolation {
        DriftViolation {
            violation_id: format!("v_{:?}_{:?}", vt, sev),
            trace_id: "trace-test".to_string(),
            violation_type: vt,
            severity: sev,
            description: "test violation".to_string(),
            detected_at: 0,
        }
    }

    #[test]
    fn test_recovery_planner_new() {
        let planner = RecoveryPlanner::new("trace-1".to_string());
        assert_eq!(planner.trace_id, "trace-1");
    }

    #[test]
    fn test_create_plan_wal_drift_severities() {
        let planner = RecoveryPlanner::new("trace-1".to_string());
        for sev in [
            DriftSeverity::Critical,
            DriftSeverity::Medium,
            DriftSeverity::Low,
        ] {
            let v = make_violation(DriftViolationType::WalDrift, sev);
            let plan = planner.create_plan(&v).expect("plan");
            assert_eq!(plan.recovery_type, match sev {
                DriftSeverity::Critical => RecoveryType::Rollback,
                DriftSeverity::Medium => RecoveryType::Patch,
                DriftSeverity::Low => RecoveryType::Ignore,
            });
            assert!(!plan.steps.is_empty());
        }
    }

    #[test]
    fn test_create_plan_txn_drift_severities() {
        let planner = RecoveryPlanner::new("trace-1".to_string());
        for sev in [
            DriftSeverity::Critical,
            DriftSeverity::Medium,
            DriftSeverity::Low,
        ] {
            let v = make_violation(DriftViolationType::TxnDrift, sev);
            let plan = planner.create_plan(&v).expect("plan");
            assert_eq!(plan.recovery_type, match sev {
                DriftSeverity::Critical => RecoveryType::Rollback,
                DriftSeverity::Medium => RecoveryType::Patch,
                DriftSeverity::Low => RecoveryType::Ignore,
            });
        }
    }

    #[test]
    fn test_create_plan_graph_drift_severities() {
        let planner = RecoveryPlanner::new("trace-1".to_string());
        for sev in [
            DriftSeverity::Critical,
            DriftSeverity::Medium,
            DriftSeverity::Low,
        ] {
            let v = make_violation(DriftViolationType::GraphDrift, sev);
            let plan = planner.create_plan(&v).expect("plan");
            assert_eq!(plan.recovery_type, match sev {
                DriftSeverity::Critical => RecoveryType::Rewire,
                DriftSeverity::Medium => RecoveryType::Rewire,
                DriftSeverity::Low => RecoveryType::Ignore,
            });
        }
    }

    #[test]
    fn test_replay_engine_empty_events_valid() {
        let engine = ExecutionReplayEngine::new("trace-1".to_string());
        let result = engine.replay();
        assert!(result.valid);
        assert!(result.divergences.is_empty());
    }

    #[test]
    fn test_replay_engine_valid_wal_sequence() {
        let mut engine = ExecutionReplayEngine::new("trace-1".to_string());
        engine.add_event("WalBegin".to_string());
        engine.add_event("StorageMutation".to_string());
        engine.add_event("WalCommit".to_string());
        let result = engine.replay();
        assert!(result.valid, "valid WAL sequence: {:?}", result.divergences);
    }

    #[test]
    fn test_replay_engine_mutation_without_open_wal() {
        let mut engine = ExecutionReplayEngine::new("trace-1".to_string());
        engine.add_event("StorageMutation".to_string());
        let result = engine.replay();
        assert!(!result.valid);
        assert_eq!(result.divergences.len(), 1);
        assert!(result.divergences[0].contains("without open WAL"));
    }

    #[test]
    fn test_replay_engine_mutation_after_commit() {
        let mut engine = ExecutionReplayEngine::new("trace-1".to_string());
        engine.add_event("WalBegin".to_string());
        engine.add_event("WalCommit".to_string());
        engine.add_event("StorageMutation".to_string());
        let result = engine.replay();
        assert!(!result.valid);
        assert_eq!(result.divergences.len(), 1);
    }

    #[test]
    fn test_replay_engine_non_wal_events_ignored() {
        let mut engine = ExecutionReplayEngine::new("trace-1".to_string());
        engine.add_event("SqlReceived".to_string());
        engine.add_event("StorageRead".to_string());
        let result = engine.replay();
        assert!(result.valid);
    }

    #[test]
    fn test_safe_controller_new_and_default() {
        let c = SafeExecutionController::new();
        assert!(c.block_on_critical);
        assert!(c.suggest_on_medium);
        assert_eq!(
            c.auto_repair_confidence_threshold,
            RecoveryConfidence::High
        );

        let default = SafeExecutionController::default();
        assert_eq!(
            default.auto_repair_confidence_threshold,
            RecoveryConfidence::High
        );
    }

    #[test]
    fn test_safe_controller_should_block_rollback() {
        let c = SafeExecutionController::new();
        let plan = RecoveryPlan::new(
            "trace-1".to_string(),
            "v-1".to_string(),
            RecoveryType::Rollback,
            RecoveryConfidence::High,
            vec!["step".to_string()],
        );
        assert!(c.should_block(&plan));
    }

    #[test]
    fn test_safe_controller_should_suggest_patch() {
        let c = SafeExecutionController::new();
        let plan = RecoveryPlan::new(
            "trace-1".to_string(),
            "v-1".to_string(),
            RecoveryType::Patch,
            RecoveryConfidence::Medium,
            vec!["step".to_string()],
        );
        assert!(c.should_suggest(&plan));
    }

    #[test]
    fn test_safe_controller_should_auto_repair_high_confidence_patch() {
        let c = SafeExecutionController::new();
        let plan = RecoveryPlan::new(
            "trace-1".to_string(),
            "v-1".to_string(),
            RecoveryType::Patch,
            RecoveryConfidence::High,
            vec!["step1".to_string(), "step2".to_string()],
        );
        assert!(c.should_auto_repair(&plan));
    }

    #[test]
    fn test_safe_controller_should_auto_repair_high_confidence_rewire() {
        let c = SafeExecutionController::new();
        let plan = RecoveryPlan::new(
            "trace-1".to_string(),
            "v-1".to_string(),
            RecoveryType::Rewire,
            RecoveryConfidence::High,
            vec!["rewire".to_string()],
        );
        assert!(c.should_auto_repair(&plan));
    }

    #[test]
    fn test_safe_controller_execute_plan_blocked() {
        let c = SafeExecutionController::new();
        let plan = RecoveryPlan::new(
            "trace-1".to_string(),
            "v-blocked".to_string(),
            RecoveryType::Rollback,
            RecoveryConfidence::High,
            vec!["abort".to_string()],
        );
        match c.execute_plan(&plan) {
            ExecutionResult::Blocked { plan_id, reason } => {
                assert_eq!(plan_id, "RP-v-blocked-trace-1");
                assert!(reason.contains("Critical"));
            }
            _ => panic!("expected Blocked"),
        }
    }

    #[test]
    fn test_safe_controller_execute_plan_auto_repaired() {
        let c = SafeExecutionController::new();
        let plan = RecoveryPlan::new(
            "trace-1".to_string(),
            "v-auto".to_string(),
            RecoveryType::Patch,
            RecoveryConfidence::High,
            vec!["s1".to_string(), "s2".to_string()],
        );
        match c.execute_plan(&plan) {
            ExecutionResult::AutoRepaired {
                plan_id,
                steps_executed,
            } => {
                assert_eq!(plan_id, "RP-v-auto-trace-1");
                assert_eq!(steps_executed, 2);
            }
            _ => panic!("expected AutoRepaired"),
        }
    }

    #[test]
    fn test_safe_controller_execute_plan_suggested() {
        let c = SafeExecutionController::new();
        let plan = RecoveryPlan::new(
            "trace-1".to_string(),
            "v-sug".to_string(),
            RecoveryType::Ignore,
            RecoveryConfidence::Low,
            vec!["hint".to_string()],
        );
        match c.execute_plan(&plan) {
            ExecutionResult::Suggested { plan_id, steps } => {
                assert_eq!(plan_id, "RP-v-sug-trace-1");
                assert_eq!(steps, vec!["hint".to_string()]);
            }
            _ => panic!("expected Suggested"),
        }
    }

    #[test]
    fn test_replay_result_clone() {
        let r = ReplayResult {
            trace_id: "t".to_string(),
            divergences: vec!["d1".to_string()],
            valid: false,
        };
        let clone = r.clone();
        assert_eq!(clone.trace_id, "t");
        assert_eq!(clone.divergences.len(), 1);
        assert!(!clone.valid);
    }
}
