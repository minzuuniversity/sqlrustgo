//! Workflow definition types

#[derive(Debug, Clone)]
pub struct WorkflowDef {
    pub name: String,
    pub description: String,
    pub current_version: WorkflowVersion,
}

#[derive(Debug, Clone)]
pub struct StepDef {
    pub id: String,
    pub name: String,
    pub step_type: StepType,
    pub approvers: Vec<String>,
    pub timeout_hours: u32,
    pub requires_evidence: bool,
}

#[derive(Debug, Clone)]
pub enum StepType {
    Manual,
    Automated,
    Approval,
    Review,
    Signoff,
}

#[derive(Debug, Clone)]
pub struct GateDef {
    pub id: String,
    pub name: String,
    pub gate_type: GateType,
    pub conditions: Vec<GateCondition>,
    pub timeout_hours: u32,
}

#[derive(Debug, Clone)]
pub enum GateType {
    And,
    Or,
    Not,
}

#[derive(Debug, Clone)]
pub struct GateCondition {
    pub field: String,
    pub operator: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct WorkflowVersion {
    pub version: String,
    pub defined_at: chrono::DateTime<chrono::Utc>,
    pub steps: Vec<StepDef>,
    pub gates: Vec<GateDef>,
}

impl WorkflowVersion {
    pub fn new(version: &str) -> Self {
        Self {
            version: version.to_string(),
            defined_at: chrono::Utc::now(),
            steps: Vec::new(),
            gates: Vec::new(),
        }
    }

    pub fn add_step(&mut self, step: StepDef) {
        self.steps.push(step);
    }

    pub fn add_gate(&mut self, gate: GateDef) {
        self.gates.push(gate);
    }
}

impl WorkflowDef {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            description: String::new(),
            current_version: WorkflowVersion::new("1.0"),
        }
    }
}

impl StepDef {
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            step_type: StepType::Manual,
            approvers: Vec::new(),
            timeout_hours: 24,
            requires_evidence: false,
        }
    }

    pub fn with_approvers(mut self, approvers: Vec<&str>) -> Self {
        self.approvers = approvers.into_iter().map(String::from).collect();
        self
    }
}
