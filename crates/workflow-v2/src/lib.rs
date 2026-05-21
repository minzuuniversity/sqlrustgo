//! Workflow V2 Engine for SQLRustGo GMP
//!
//! Next-generation workflow engine with state machines, parallel gates, and full audit trail.

pub mod definition;
pub mod engine;
pub mod state;
pub mod transitions;

pub use definition::{GateDef, StepDef, WorkflowDef, WorkflowVersion};
pub use engine::WorkflowEngine;
pub use state::{InstanceStatus, StepStatus, WorkflowInstance};
pub use transitions::TransitionResult;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definition::{GateCondition, GateType, StepType};
    use crate::state::StepInstance;

    // ============ WorkflowDef Tests ============

    #[test]
    fn test_workflow_def_new() {
        let wf = WorkflowDef::new("TestWorkflow");
        assert_eq!(wf.name, "TestWorkflow");
        assert_eq!(wf.description, "");
        assert_eq!(wf.current_version.version, "1.0");
    }

    #[test]
    fn test_workflow_def_with_version() {
        let mut wf = WorkflowDef::new("TestWorkflow");
        let mut version = WorkflowVersion::new("2.0");
        version.add_step(StepDef::new("step1", "First Step"));
        wf.current_version = version;
        assert_eq!(wf.current_version.version, "2.0");
        assert_eq!(wf.current_version.steps.len(), 1);
    }

    // ============ WorkflowVersion Tests ============

    #[test]
    fn test_workflow_version_new() {
        let version = WorkflowVersion::new("1.5");
        assert_eq!(version.version, "1.5");
        assert!(version.steps.is_empty());
        assert!(version.gates.is_empty());
    }

    #[test]
    fn test_workflow_version_add_step() {
        let mut version = WorkflowVersion::new("1.0");
        let step = StepDef::new("step1", "Documentation");
        version.add_step(step.clone());
        assert_eq!(version.steps.len(), 1);
        assert_eq!(version.steps[0].id, "step1");
    }

    #[test]
    fn test_workflow_version_add_gate() {
        let mut version = WorkflowVersion::new("1.0");
        let gate = GateDef {
            id: "gate1".to_string(),
            name: "Review Gate".to_string(),
            gate_type: GateType::And,
            conditions: vec![],
            timeout_hours: 48,
        };
        version.add_gate(gate);
        assert_eq!(version.gates.len(), 1);
        assert_eq!(version.gates[0].id, "gate1");
    }

    // ============ StepDef Tests ============

    #[test]
    fn test_step_def_new() {
        let step = StepDef::new("review", "Code Review");
        assert_eq!(step.id, "review");
        assert_eq!(step.name, "Code Review");
        assert!(matches!(step.step_type, StepType::Manual));
        assert!(step.approvers.is_empty());
        assert_eq!(step.timeout_hours, 24);
        assert!(!step.requires_evidence);
    }

    #[test]
    fn test_step_def_with_approvers() {
        let step = StepDef::new("approve", "Approval Step").with_approvers(vec!["alice", "bob"]);
        assert_eq!(step.approvers.len(), 2);
        assert_eq!(step.approvers[0], "alice");
        assert_eq!(step.approvers[1], "bob");
    }

    #[test]
    fn test_step_def_clone() {
        let step = StepDef::new("clone", "Clone Test").with_approvers(vec!["alice"]);
        let cloned = step.clone();
        assert_eq!(cloned.id, step.id);
        assert_eq!(cloned.approvers, step.approvers);
    }

    // ============ GateDef Tests ============

    #[test]
    fn test_gate_def_and_type() {
        let gate = GateDef {
            id: "gate_and".to_string(),
            name: "AND Gate".to_string(),
            gate_type: GateType::And,
            conditions: vec![GateCondition {
                field: "approved".to_string(),
                operator: "eq".to_string(),
                value: serde_json::json!(true),
            }],
            timeout_hours: 72,
        };
        assert!(matches!(gate.gate_type, GateType::And));
        assert_eq!(gate.conditions.len(), 1);
    }

    #[test]
    fn test_gate_def_or_type() {
        let gate = GateDef {
            id: "gate_or".to_string(),
            name: "OR Gate".to_string(),
            gate_type: GateType::Or,
            conditions: vec![],
            timeout_hours: 24,
        };
        assert!(matches!(gate.gate_type, GateType::Or));
    }

    #[test]
    fn test_gate_def_not_type() {
        let gate = GateDef {
            id: "gate_not".to_string(),
            name: "NOT Gate".to_string(),
            gate_type: GateType::Not,
            conditions: vec![],
            timeout_hours: 12,
        };
        assert!(matches!(gate.gate_type, GateType::Not));
    }

    // ============ StepType Tests ============

    #[test]
    fn test_step_type_variants() {
        let step_manual = StepDef::new("1", "Manual Step");
        assert!(matches!(step_manual.step_type, StepType::Manual));

        // Test that step types are cloneable
        let _ = StepType::Manual;
        let _ = StepType::Automated;
        let _ = StepType::Approval;
        let _ = StepType::Review;
        let _ = StepType::Signoff;
    }

    // ============ InstanceStatus Tests ============

    #[test]
    fn test_instance_status_all_variants() {
        let statuses = vec![
            InstanceStatus::Pending,
            InstanceStatus::Running,
            InstanceStatus::Waiting,
            InstanceStatus::Completed,
            InstanceStatus::Rejected,
            InstanceStatus::Cancelled,
            InstanceStatus::TimedOut,
        ];
        assert_eq!(statuses.len(), 7);
    }

    #[test]
    fn test_instance_status_as_str() {
        assert_eq!(InstanceStatus::Pending.as_str(), "pending");
        assert_eq!(InstanceStatus::Running.as_str(), "running");
        assert_eq!(InstanceStatus::Waiting.as_str(), "waiting");
        assert_eq!(InstanceStatus::Completed.as_str(), "completed");
        assert_eq!(InstanceStatus::Rejected.as_str(), "rejected");
        assert_eq!(InstanceStatus::Cancelled.as_str(), "cancelled");
        assert_eq!(InstanceStatus::TimedOut.as_str(), "timed_out");
    }

    #[test]
    fn test_instance_status_equality() {
        assert_eq!(InstanceStatus::Pending, InstanceStatus::Pending);
        assert_ne!(InstanceStatus::Pending, InstanceStatus::Running);
    }

    // ============ StepStatus Tests ============

    #[test]
    fn test_step_status_all_variants() {
        let statuses = vec![
            StepStatus::NotStarted,
            StepStatus::InProgress,
            StepStatus::Approved,
            StepStatus::Rejected,
            StepStatus::Skipped,
            StepStatus::TimedOut,
        ];
        assert_eq!(statuses.len(), 6);
    }

    #[test]
    fn test_step_status_as_str() {
        assert_eq!(StepStatus::NotStarted.as_str(), "not_started");
        assert_eq!(StepStatus::InProgress.as_str(), "in_progress");
        assert_eq!(StepStatus::Approved.as_str(), "approved");
        assert_eq!(StepStatus::Rejected.as_str(), "rejected");
        assert_eq!(StepStatus::Skipped.as_str(), "skipped");
        assert_eq!(StepStatus::TimedOut.as_str(), "timed_out");
    }

    #[test]
    fn test_step_status_equality() {
        assert_eq!(StepStatus::NotStarted, StepStatus::NotStarted);
        assert_ne!(StepStatus::NotStarted, StepStatus::InProgress);
    }

    // ============ StepInstance Tests ============

    #[test]
    fn test_step_instance_new() {
        let instance = StepInstance::new("review_step");
        assert_eq!(instance.step_id, "review_step");
        assert!(matches!(instance.status, StepStatus::NotStarted));
        assert!(instance.assignee.is_none());
        assert!(instance.started_at.is_none());
        assert!(instance.completed_at.is_none());
        assert!(instance.evidence_ids.is_empty());
    }

    // ============ WorkflowInstance Tests ============

    #[test]
    fn test_workflow_instance_new() {
        let instance = WorkflowInstance::new("wf-001", "MyWorkflow", "1.0");
        assert_eq!(instance.id, "wf-001");
        assert_eq!(instance.workflow_name, "MyWorkflow");
        assert_eq!(instance.version, "1.0");
        assert!(matches!(instance.status, InstanceStatus::Pending));
        assert!(instance.current_step.is_none());
        assert!(instance.steps.is_empty());
    }

    #[test]
    fn test_workflow_instance_advance_to_step() {
        let mut instance = WorkflowInstance::new("wf-002", "TestWorkflow", "1.0");
        instance.advance_to_step("step_1");
        assert_eq!(instance.current_step, Some("step_1".to_string()));
        assert!(matches!(instance.status, InstanceStatus::Running));
    }

    // ============ TransitionResult Tests ============

    #[test]
    fn test_transition_result_is_success_advanced() {
        let result = TransitionResult::Advanced("step2".to_string());
        assert!(result.is_success());
    }

    #[test]
    fn test_transition_result_is_success_completed() {
        let result = TransitionResult::Completed;
        assert!(result.is_success());
    }

    #[test]
    fn test_transition_result_is_not_success_rejected() {
        let result = TransitionResult::Rejected("step1".to_string());
        assert!(!result.is_success());
    }

    #[test]
    fn test_transition_result_is_not_success_invalid() {
        let result = TransitionResult::InvalidAction("unknown".to_string());
        assert!(!result.is_success());
    }

    #[test]
    fn test_transition_result_is_not_success_timeout() {
        let result = TransitionResult::Timeout("step1".to_string());
        assert!(!result.is_success());
    }

    #[test]
    fn test_transition_result_variants() {
        let _ = TransitionResult::Advanced("id".to_string());
        let _ = TransitionResult::Completed;
        let _ = TransitionResult::Rejected("id".to_string());
        let _ = TransitionResult::InvalidAction("action".to_string());
        let _ = TransitionResult::Timeout("id".to_string());
    }

    // ============ WorkflowEngine Tests ============

    #[test]
    fn test_workflow_engine_start_instance_empty() {
        let def = WorkflowDef::new("EmptyWorkflow");
        let instance = WorkflowEngine::start_instance(&def, "inst-001");
        assert_eq!(instance.id, "inst-001");
        assert_eq!(instance.workflow_name, "EmptyWorkflow");
        assert!(instance.steps.is_empty());
        assert!(instance.current_step.is_none());
    }

    #[test]
    fn test_workflow_engine_start_instance_with_steps() {
        let mut wf = WorkflowDef::new("MultiStepWorkflow");
        wf.current_version.add_step(StepDef::new("step1", "First"));
        wf.current_version.add_step(StepDef::new("step2", "Second"));
        wf.current_version.add_step(StepDef::new("step3", "Third"));

        let instance = WorkflowEngine::start_instance(&wf, "inst-002");

        assert_eq!(instance.steps.len(), 3);
        assert_eq!(instance.current_step, Some("step1".to_string()));
        assert!(matches!(instance.status, InstanceStatus::Running));
        assert!(matches!(instance.steps[0].status, StepStatus::InProgress));
        assert!(matches!(instance.steps[1].status, StepStatus::NotStarted));
    }

    #[test]
    fn test_workflow_engine_advance_step_approve() {
        let mut wf = WorkflowDef::new("ApproveWorkflow");
        wf.current_version.add_step(StepDef::new("step1", "First"));
        wf.current_version.add_step(StepDef::new("step2", "Second"));

        let mut instance = WorkflowEngine::start_instance(&wf, "inst-003");
        let result = WorkflowEngine::advance_step(&mut instance, "step1", "approve");

        assert!(matches!(result, TransitionResult::Advanced(id) if id == "step2"));
        assert!(matches!(instance.steps[0].status, StepStatus::Approved));
        assert!(instance.steps[0].completed_at.is_some());
        assert_eq!(instance.current_step, Some("step2".to_string()));
    }

    #[test]
    fn test_workflow_engine_advance_step_approve_last_completes() {
        let mut wf = WorkflowDef::new("SingleStepWorkflow");
        wf.current_version
            .add_step(StepDef::new("step1", "Only Step"));

        let mut instance = WorkflowEngine::start_instance(&wf, "inst-004");
        let result = WorkflowEngine::advance_step(&mut instance, "step1", "approve");

        assert!(matches!(result, TransitionResult::Completed));
        assert!(matches!(instance.status, InstanceStatus::Completed));
    }

    #[test]
    fn test_workflow_engine_advance_step_reject() {
        let mut wf = WorkflowDef::new("RejectWorkflow");
        wf.current_version.add_step(StepDef::new("step1", "First"));

        let mut instance = WorkflowEngine::start_instance(&wf, "inst-005");
        let result = WorkflowEngine::advance_step(&mut instance, "step1", "reject");

        assert!(matches!(result, TransitionResult::Rejected(id) if id == "step1"));
        assert!(matches!(instance.steps[0].status, StepStatus::Rejected));
        assert!(matches!(instance.status, InstanceStatus::Rejected));
    }

    #[test]
    fn test_workflow_engine_advance_step_invalid_action() {
        let mut wf = WorkflowDef::new("InvalidActionWorkflow");
        wf.current_version.add_step(StepDef::new("step1", "First"));

        let mut instance = WorkflowEngine::start_instance(&wf, "inst-006");
        let result = WorkflowEngine::advance_step(&mut instance, "step1", "unknown_action");

        assert!(matches!(result, TransitionResult::InvalidAction(msg) if msg == "unknown_action"));
        assert!(matches!(instance.steps[0].status, StepStatus::InProgress));
    }

    #[test]
    fn test_workflow_engine_advance_step_not_found() {
        let mut wf = WorkflowDef::new("NotFoundWorkflow");
        wf.current_version.add_step(StepDef::new("step1", "First"));

        let mut instance = WorkflowEngine::start_instance(&wf, "inst-007");
        let result = WorkflowEngine::advance_step(&mut instance, "nonexistent", "approve");

        assert!(
            matches!(result, TransitionResult::InvalidAction(msg) if msg.contains("not found"))
        );
    }

    #[test]
    fn test_workflow_engine_full_happy_path() {
        let mut wf = WorkflowDef::new("FullPathWorkflow");
        wf.current_version.add_step(StepDef::new("step1", "Start"));
        wf.current_version.add_step(StepDef::new("step2", "Middle"));
        wf.current_version.add_step(StepDef::new("step3", "End"));

        let mut instance = WorkflowEngine::start_instance(&wf, "inst-008");

        // Step 1
        let result1 = WorkflowEngine::advance_step(&mut instance, "step1", "approve");
        assert!(matches!(result1, TransitionResult::Advanced(id) if id == "step2"));

        // Step 2
        let result2 = WorkflowEngine::advance_step(&mut instance, "step2", "approve");
        assert!(matches!(result2, TransitionResult::Advanced(id) if id == "step3"));

        // Step 3 (last)
        let result3 = WorkflowEngine::advance_step(&mut instance, "step3", "approve");
        assert!(matches!(result3, TransitionResult::Completed));
        assert!(matches!(instance.status, InstanceStatus::Completed));
    }

    // ============ Clone and Debug Tests ============

    #[test]
    fn test_workflow_def_clone() {
        let wf = WorkflowDef::new("CloneTest");
        let cloned = wf.clone();
        assert_eq!(cloned.name, wf.name);
    }

    #[test]
    fn test_workflow_version_clone() {
        let mut version = WorkflowVersion::new("1.0");
        version.add_step(StepDef::new("s1", "Step 1"));
        let cloned = version.clone();
        assert_eq!(cloned.version, "1.0");
        assert_eq!(cloned.steps.len(), 1);
    }

    #[test]
    fn test_gate_def_clone() {
        let gate = GateDef {
            id: "g1".to_string(),
            name: "Gate 1".to_string(),
            gate_type: GateType::And,
            conditions: vec![],
            timeout_hours: 24,
        };
        let cloned = gate.clone();
        assert_eq!(cloned.id, "g1");
        assert_eq!(cloned.name, "Gate 1");
    }

    #[test]
    fn test_workflow_instance_clone() {
        let instance = WorkflowInstance::new("i1", "W1", "1.0");
        let cloned = instance.clone();
        assert_eq!(cloned.id, "i1");
        assert_eq!(cloned.workflow_name, "W1");
    }

    #[test]
    fn test_step_instance_clone() {
        let instance = StepInstance::new("s1");
        let cloned = instance.clone();
        assert_eq!(cloned.step_id, "s1");
    }

    #[test]
    fn test_debug_format() {
        let wf = WorkflowDef::new("DebugTest");
        let debug_str = format!("{:?}", wf);
        assert!(debug_str.contains("DebugTest"));
    }
}
