use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub condition: RuleCondition,
    pub severity: RuleSeverity,
    pub status: RuleStatus,
    pub version: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleCondition {
    pub expression: String,
    pub operator: ConditionOperator,
    pub value: ConditionValue,
    pub field: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ConditionOperator {
    Equals,
    NotEquals,
    GreaterThan,
    GreaterThanOrEquals,
    LessThan,
    LessThanOrEquals,
    Contains,
    StartsWith,
    EndsWith,
    Matches,
    In,
    NotIn,
}

impl ConditionOperator {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Equals => "eq",
            Self::NotEquals => "ne",
            Self::GreaterThan => "gt",
            Self::GreaterThanOrEquals => "gte",
            Self::LessThan => "lt",
            Self::LessThanOrEquals => "lte",
            Self::Contains => "contains",
            Self::StartsWith => "starts_with",
            Self::EndsWith => "ends_with",
            Self::Matches => "matches",
            Self::In => "in",
            Self::NotIn => "not_in",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConditionValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Array(Vec<ConditionValue>),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum RuleSeverity {
    Critical,
    Error,
    Warning,
    Info,
}

impl RuleSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum RuleStatus {
    Draft,
    Active,
    Suspended,
    Archived,
}

impl RuleStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Active => "active",
            Self::Suspended => "suspended",
            Self::Archived => "archived",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: RuleCategory,
    pub condition_template: String,
    pub default_severity: RuleSeverity,
    pub parameters: Vec<TemplateParameter>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum RuleCategory {
    GMPCompliance,
    Temperature,
    Pressure,
    Hygiene,
    Equipment,
    Quality,
    Safety,
    Custom,
}

impl RuleCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::GMPCompliance => "gmp_compliance",
            Self::Temperature => "temperature",
            Self::Pressure => "pressure",
            Self::Hygiene => "hygiene",
            Self::Equipment => "equipment",
            Self::Quality => "quality",
            Self::Safety => "safety",
            Self::Custom => "custom",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateParameter {
    pub name: String,
    pub param_type: ParameterType,
    pub required: bool,
    pub default_value: Option<ConditionValue>,
    pub description: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ParameterType {
    String,
    Number,
    Boolean,
    Array,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleEvaluation {
    pub rule_id: String,
    pub passed: bool,
    pub evaluated_at: DateTime<Utc>,
    pub context: EvaluationContext,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationContext {
    pub entity_type: String,
    pub entity_id: String,
    pub attributes: std::collections::HashMap<String, ConditionValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRuleRequest {
    pub name: String,
    pub description: String,
    pub condition: RuleCondition,
    pub severity: RuleSeverity,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRuleRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub condition: Option<RuleCondition>,
    pub severity: Option<RuleSeverity>,
    pub status: Option<RuleStatus>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleTestRequest {
    pub rule_id: String,
    pub context: EvaluationContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleTestResult {
    pub rule_id: String,
    pub passed: bool,
    pub message: String,
    pub evaluated_at: DateTime<Utc>,
}

pub struct RuleService {
    rules: Vec<Rule>,
    templates: Vec<RuleTemplate>,
    versions: Vec<RuleVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleVersion {
    pub rule_id: String,
    pub version: u32,
    pub rule: Rule,
    pub changed_at: DateTime<Utc>,
    pub changed_by: String,
    pub change_reason: String,
}

impl Default for RuleService {
    fn default() -> Self {
        Self::new()
    }
}

impl RuleService {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            templates: Vec::new(),
            versions: Vec::new(),
        }
    }

    pub fn create_rule(
        &mut self,
        request: CreateRuleRequest,
        user_id: &str,
    ) -> Result<Rule, RuleError> {
        if request.name.is_empty() {
            return Err(RuleError::ValidationError(
                "Rule name cannot be empty".to_string(),
            ));
        }

        let now = Utc::now();
        let rule = Rule {
            id: Uuid::new_v4().to_string(),
            name: request.name,
            description: request.description,
            condition: request.condition,
            severity: request.severity,
            status: RuleStatus::Draft,
            version: 1,
            created_at: now,
            updated_at: now,
            created_by: user_id.to_string(),
            tags: request.tags,
        };

        self.rules.push(rule.clone());
        Ok(rule)
    }

    pub fn get_rule(&self, id: &str) -> Option<&Rule> {
        self.rules.iter().find(|r| r.id == id)
    }

    pub fn update_rule(
        &mut self,
        id: &str,
        request: UpdateRuleRequest,
        user_id: &str,
    ) -> Result<Rule, RuleError> {
        let rule = self
            .rules
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or(RuleError::RuleNotFound)?;

        let old_version = rule.version;
        let new_version = old_version + 1;

        self.versions.push(RuleVersion {
            rule_id: id.to_string(),
            version: old_version,
            rule: rule.clone(),
            changed_at: Utc::now(),
            changed_by: user_id.to_string(),
            change_reason: "Update".to_string(),
        });

        if let Some(name) = request.name {
            rule.name = name;
        }
        if let Some(desc) = request.description {
            rule.description = desc;
        }
        if let Some(condition) = request.condition {
            rule.condition = condition;
        }
        if let Some(severity) = request.severity {
            rule.severity = severity;
        }
        if let Some(status) = request.status {
            rule.status = status;
        }
        if let Some(tags) = request.tags {
            rule.tags = tags;
        }

        rule.version = new_version;
        rule.updated_at = Utc::now();

        Ok(rule.clone())
    }

    pub fn delete_rule(&mut self, id: &str) -> Result<(), RuleError> {
        let pos = self.rules.iter().position(|r| r.id == id);
        match pos {
            Some(idx) => {
                self.rules.remove(idx);
                Ok(())
            }
            None => Err(RuleError::RuleNotFound),
        }
    }

    pub fn list_rules(&self, status: Option<RuleStatus>, category: Option<&str>) -> Vec<&Rule> {
        self.rules
            .iter()
            .filter(|r| {
                let status_match = status.is_none_or(|s| r.status == s);
                let category_match = category.is_none_or(|c| r.tags.iter().any(|t| t == c));
                status_match && category_match
            })
            .collect()
    }

    pub fn activate_rule(&mut self, id: &str) -> Result<Rule, RuleError> {
        let rule = self
            .rules
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or(RuleError::RuleNotFound)?;

        if rule.status == RuleStatus::Active {
            return Err(RuleError::RuleAlreadyActive);
        }

        rule.status = RuleStatus::Active;
        rule.updated_at = Utc::now();
        Ok(rule.clone())
    }

    pub fn suspend_rule(&mut self, id: &str) -> Result<Rule, RuleError> {
        let rule = self
            .rules
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or(RuleError::RuleNotFound)?;

        rule.status = RuleStatus::Suspended;
        rule.updated_at = Utc::now();
        Ok(rule.clone())
    }

    pub fn evaluate_rule(&self, rule_id: &str, context: &EvaluationContext) -> RuleEvaluation {
        let rule = match self.rules.iter().find(|r| r.id == rule_id) {
            Some(r) => r,
            None => {
                return RuleEvaluation {
                    rule_id: rule_id.to_string(),
                    passed: false,
                    evaluated_at: Utc::now(),
                    context: context.clone(),
                    message: "Rule not found".to_string(),
                };
            }
        };

        let passed = Self::check_condition(&rule.condition, context);
        let message = if passed {
            format!("Rule '{}' passed", rule.name)
        } else {
            format!(
                "Rule '{}' failed: {} {} {:?}",
                rule.name,
                rule.condition.field,
                rule.condition.operator.as_str(),
                rule.condition.value
            )
        };

        RuleEvaluation {
            rule_id: rule_id.to_string(),
            passed,
            evaluated_at: Utc::now(),
            context: context.clone(),
            message,
        }
    }

    pub fn test_rule(&self, request: &RuleTestRequest) -> RuleTestResult {
        let evaluation = self.evaluate_rule(&request.rule_id, &request.context);
        RuleTestResult {
            rule_id: request.rule_id.clone(),
            passed: evaluation.passed,
            message: evaluation.message,
            evaluated_at: Utc::now(),
        }
    }

    fn check_condition(condition: &RuleCondition, context: &EvaluationContext) -> bool {
        let attr_value = match context.attributes.get(&condition.field) {
            Some(v) => v,
            None => return false,
        };

        match (&condition.operator, &condition.value, attr_value) {
            (ConditionOperator::Equals, ConditionValue::Number(n), ConditionValue::Number(a)) => {
                (a - n).abs() < f64::EPSILON
            }
            (ConditionOperator::Equals, ConditionValue::String(s), ConditionValue::String(a)) => {
                a == s
            }
            (
                ConditionOperator::GreaterThan,
                ConditionValue::Number(n),
                ConditionValue::Number(a),
            ) => a > n,
            (ConditionOperator::LessThan, ConditionValue::Number(n), ConditionValue::Number(a)) => {
                a < n
            }
            (ConditionOperator::Contains, ConditionValue::String(s), ConditionValue::String(a)) => {
                a.contains(s)
            }
            _ => false,
        }
    }

    pub fn get_versions(&self, rule_id: &str) -> Vec<&RuleVersion> {
        self.versions
            .iter()
            .filter(|v| v.rule_id == rule_id)
            .collect()
    }

    pub fn add_template(&mut self, template: RuleTemplate) {
        self.templates.push(template);
    }

    pub fn list_templates(&self, category: Option<RuleCategory>) -> Vec<&RuleTemplate> {
        self.templates
            .iter()
            .filter(|t| category.is_none_or(|c| t.category == c))
            .collect()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RuleError {
    #[error("Rule not found")]
    RuleNotFound,
    #[error("Rule already active")]
    RuleAlreadyActive,
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Template not found")]
    TemplateNotFound,
}

impl From<RuleError> for String {
    fn from(err: RuleError) -> Self {
        err.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_condition_operator_as_str() {
        assert_eq!(ConditionOperator::Equals.as_str(), "eq");
        assert_eq!(ConditionOperator::NotEquals.as_str(), "ne");
        assert_eq!(ConditionOperator::GreaterThan.as_str(), "gt");
        assert_eq!(ConditionOperator::GreaterThanOrEquals.as_str(), "gte");
        assert_eq!(ConditionOperator::LessThan.as_str(), "lt");
        assert_eq!(ConditionOperator::LessThanOrEquals.as_str(), "lte");
        assert_eq!(ConditionOperator::Contains.as_str(), "contains");
        assert_eq!(ConditionOperator::StartsWith.as_str(), "starts_with");
        assert_eq!(ConditionOperator::EndsWith.as_str(), "ends_with");
        assert_eq!(ConditionOperator::Matches.as_str(), "matches");
        assert_eq!(ConditionOperator::In.as_str(), "in");
        assert_eq!(ConditionOperator::NotIn.as_str(), "not_in");
    }

    #[test]
    fn test_rule_severity_as_str() {
        assert_eq!(RuleSeverity::Critical.as_str(), "critical");
        assert_eq!(RuleSeverity::Error.as_str(), "error");
        assert_eq!(RuleSeverity::Warning.as_str(), "warning");
        assert_eq!(RuleSeverity::Info.as_str(), "info");
    }

    #[test]
    fn test_rule_status_as_str() {
        assert_eq!(RuleStatus::Draft.as_str(), "draft");
        assert_eq!(RuleStatus::Active.as_str(), "active");
        assert_eq!(RuleStatus::Suspended.as_str(), "suspended");
        assert_eq!(RuleStatus::Archived.as_str(), "archived");
    }

    #[test]
    fn test_rule_category_as_str() {
        assert_eq!(RuleCategory::GMPCompliance.as_str(), "gmp_compliance");
        assert_eq!(RuleCategory::Temperature.as_str(), "temperature");
        assert_eq!(RuleCategory::Pressure.as_str(), "pressure");
        assert_eq!(RuleCategory::Hygiene.as_str(), "hygiene");
        assert_eq!(RuleCategory::Equipment.as_str(), "equipment");
        assert_eq!(RuleCategory::Quality.as_str(), "quality");
        assert_eq!(RuleCategory::Safety.as_str(), "safety");
        assert_eq!(RuleCategory::Custom.as_str(), "custom");
    }

    #[test]
    fn test_rule_service_new() {
        let service = RuleService::new();
        assert_eq!(service.list_rules(None, None).len(), 0);
    }

    #[test]
    fn test_rule_service_create_rule_empty_name() {
        let mut service = RuleService::new();
        let request = CreateRuleRequest {
            name: "".to_string(),
            description: "test".to_string(),
            condition: RuleCondition {
                expression: "x > 0".to_string(),
                operator: ConditionOperator::GreaterThan,
                value: ConditionValue::Number(0.0),
                field: "x".to_string(),
            },
            severity: RuleSeverity::Warning,
            tags: vec![],
        };
        let result = service.create_rule(request, "user1");
        assert!(result.is_err());
    }

    #[test]
    fn test_rule_service_create_and_get_rule() {
        let mut service = RuleService::new();
        let request = CreateRuleRequest {
            name: "Temperature Check".to_string(),
            description: "Verify temperature is within range".to_string(),
            condition: RuleCondition {
                expression: "temp < 25.0".to_string(),
                operator: ConditionOperator::LessThan,
                value: ConditionValue::Number(25.0),
                field: "temperature".to_string(),
            },
            severity: RuleSeverity::Critical,
            tags: vec!["temperature".to_string()],
        };
        let rule = service.create_rule(request, "user1").unwrap();
        assert_eq!(rule.name, "Temperature Check");
        assert_eq!(rule.status, RuleStatus::Draft);
        assert!(!rule.id.is_empty());

        let found = service.get_rule(&rule.id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Temperature Check");
    }

    #[test]
    fn test_rule_service_evaluate_rule() {
        let mut service = RuleService::new();
        let request = CreateRuleRequest {
            name: "GT Check".to_string(),
            description: "desc".to_string(),
            condition: RuleCondition {
                expression: "x > 10".to_string(),
                operator: ConditionOperator::GreaterThan,
                value: ConditionValue::Number(10.0),
                field: "x".to_string(),
            },
            severity: RuleSeverity::Warning,
            tags: vec![],
        };
        let rule = service.create_rule(request, "user1").unwrap();
        service.activate_rule(&rule.id).unwrap();

        let mut ctx = EvaluationContext {
            entity_type: "sensor".to_string(),
            entity_id: "sensor-001".to_string(),
            attributes: std::collections::HashMap::new(),
        };
        ctx.attributes
            .insert("x".to_string(), ConditionValue::Number(15.0));

        let eval = service.evaluate_rule(&rule.id, &ctx);
        assert!(eval.passed);
        assert_eq!(eval.rule_id, rule.id);
    }

    #[test]
    fn test_rule_error_display() {
        let err = RuleError::RuleNotFound;
        assert_eq!(err.to_string(), "Rule not found");

        let err = RuleError::ValidationError("name required".to_string());
        assert_eq!(err.to_string(), "Validation error: name required");
    }

    #[test]
    fn test_condition_value_untagged() {
        let v_num = ConditionValue::Number(42.0);
        let v_str = ConditionValue::String("test".to_string());
        let v_bool = ConditionValue::Boolean(true);
        let v_arr = ConditionValue::Array(vec![ConditionValue::Number(1.0)]);

        assert!(matches!(v_num, ConditionValue::Number(_)));
        assert!(matches!(v_str, ConditionValue::String(_)));
        assert!(matches!(v_bool, ConditionValue::Boolean(_)));
        assert!(matches!(v_arr, ConditionValue::Array(_)));
    }

    #[test]
    fn test_rule_service_activate_and_suspend_rule() {
        let mut service = RuleService::new();
        let request = CreateRuleRequest {
            name: "Test Rule".to_string(),
            description: "desc".to_string(),
            condition: RuleCondition {
                expression: "x > 0".to_string(),
                operator: ConditionOperator::GreaterThan,
                value: ConditionValue::Number(0.0),
                field: "x".to_string(),
            },
            severity: RuleSeverity::Info,
            tags: vec![],
        };
        let rule = service.create_rule(request, "user1").unwrap();
        let activated = service.activate_rule(&rule.id).unwrap();
        assert_eq!(activated.status, RuleStatus::Active);

        let suspended = service.suspend_rule(&rule.id).unwrap();
        assert_eq!(suspended.status, RuleStatus::Suspended);
    }
}
