//! Workflow engine

use crate::definition::{StepDef, WorkflowDef};
use crate::state::{InstanceStatus, StepInstance, StepStatus, WorkflowInstance};
use crate::transitions::TransitionResult;

pub struct WorkflowEngine;

impl WorkflowEngine {
    pub fn start_instance(def: &WorkflowDef, instance_id: &str) -> WorkflowInstance {
        let mut instance =
            WorkflowInstance::new(instance_id, &def.name, &def.current_version.version);

        // Initialize step instances
        for step in &def.current_version.steps {
            instance.steps.push(StepInstance::new(&step.id));
        }

        // Start first step
        if let Some(first_step) = def.current_version.steps.first() {
            instance.advance_to_step(&first_step.id);
            if let Some(first) = instance.steps.first_mut() {
                first.status = StepStatus::InProgress;
            }
        }

        instance
    }

    pub fn advance_step(
        instance: &mut WorkflowInstance,
        step_id: &str,
        action: &str,
    ) -> TransitionResult {
        let step_idx = match instance.steps.iter().position(|s| s.step_id == step_id) {
            Some(idx) => idx,
            None => return TransitionResult::InvalidAction(format!("Step {} not found", step_id)),
        };
        let step = &mut instance.steps[step_idx];

        match action {
            "approve" => {
                step.status = StepStatus::Approved;
                step.completed_at = Some(chrono::Utc::now());
                instance.updated_at = chrono::Utc::now();

                if step_idx + 1 < instance.steps.len() {
                    let next_id = instance.steps[step_idx + 1].step_id.clone();
                    instance.advance_to_step(&next_id);
                    instance.steps[step_idx + 1].status = StepStatus::InProgress;
                    return TransitionResult::Advanced(next_id);
                } else {
                    instance.status = InstanceStatus::Completed;
                    return TransitionResult::Completed;
                }
            }
            "reject" => {
                step.status = StepStatus::Rejected;
                step.completed_at = Some(chrono::Utc::now());
                instance.status = InstanceStatus::Rejected;
                instance.updated_at = chrono::Utc::now();
                TransitionResult::Rejected(step_id.to_string())
            }
            _ => TransitionResult::InvalidAction(action.to_string()),
        }
    }
}
