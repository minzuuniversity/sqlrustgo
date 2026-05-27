//! Compliance rule parser
//!
//! 解析合规规则，支持多语言 (zh/en/ja) YAML/JSON 格式。
//! 支持批量加载规则文件、按语言/分类检索。

use crate::types::{ComplianceRule, RuleCondition};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Parser errors
#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Invalid YAML format: {0}")]
    InvalidYaml(String),
    #[error("Invalid JSON format: {0}")]
    InvalidJson(String),
    #[error("Missing required field: {0}")]
    MissingField(String),
    #[error("Invalid field value: {0}")]
    InvalidValue(String),
    #[error("IO error: {0}")]
    Io(String),
    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(String),
}

/// Validation errors
#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Rule incomplete: {0}")]
    Incomplete(String),
    #[error("Rule inconsistent: {0}")]
    Inconsistent(String),
}

/// 单个解析后的合规规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedRule {
    /// 规则唯一 ID
    pub id: String,
    /// 规则文本描述
    pub text: String,
    /// 严重级别: critical / major / minor / advisory
    pub severity: String,
    /// 语言代码: zh / en / ja
    pub language: String,
    /// 规则分类（如 batch_record, equipment, deviation）
    pub category: String,
}

/// 多语言版本的同一条规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiLangRule {
    /// 规则唯一 ID
    pub id: String,
    /// 各语言版本的规则文本
    pub translations: HashMap<String, String>,
    /// 严重级别
    pub severity: String,
    /// 规则分类
    pub category: String,
    /// 适用的操作类型列表
    pub applies_to: Vec<String>,
    /// 条件表达式（可选，用于规则引擎自动匹配）
    pub condition: Option<String>,
}

/// 多语言规则集合
#[derive(Debug, Clone, Default)]
pub struct RuleSet {
    /// 所有规则（按 ID 索引）
    pub rules: HashMap<String, MultiLangRule>,
    /// 按语言索引的规则 ID 列表
    pub by_language: HashMap<String, Vec<String>>,
    /// 按分类索引的规则 ID 列表
    pub by_category: HashMap<String, Vec<String>>,
}

impl RuleSet {
    /// 创建新的空规则集
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加规则
    pub fn add_rule(&mut self, rule: MultiLangRule) {
        let id = rule.id.clone();

        // 按语言索引
        for lang in rule.translations.keys() {
            self.by_language
                .entry(lang.clone())
                .or_default()
                .push(id.clone());
        }

        // 按分类索引
        self.by_category
            .entry(rule.category.clone())
            .or_default()
            .push(id.clone());

        self.rules.insert(id, rule);
    }

    /// 获取指定语言的规则文本
    pub fn get_text(&self, rule_id: &str, language: &str) -> Option<&str> {
        self.rules
            .get(rule_id)
            .and_then(|r| r.translations.get(language))
            .map(|s| s.as_str())
    }

    /// 获取指定分类的所有规则
    pub fn get_rules_by_category(&self, category: &str) -> Vec<&MultiLangRule> {
        self.by_category
            .get(category)
            .map(|ids| ids.iter().filter_map(|id| self.rules.get(id)).collect())
            .unwrap_or_default()
    }

    /// 获取指定语言的所有规则
    pub fn get_rules_by_language(&self, language: &str) -> Vec<&MultiLangRule> {
        self.by_language
            .get(language)
            .map(|ids| ids.iter().filter_map(|id| self.rules.get(id)).collect())
            .unwrap_or_default()
    }

    /// 获取指定语言的规则文本列表（用于构建 LLM prompt）
    pub fn get_rule_texts(&self, rule_ids: &[String], language: &str) -> Vec<(String, String)> {
        rule_ids
            .iter()
            .filter_map(|id| {
                let rule = self.rules.get(id)?;
                let text = rule
                    .translations
                    .get(language)
                    .or_else(|| rule.translations.get("en"))
                    .or_else(|| rule.translations.values().next())?;
                Some((id.clone(), text.clone()))
            })
            .collect()
    }

    /// 规则总数
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
}

impl ValidationResult {
    pub fn valid() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
        }
    }

    pub fn invalid(errors: Vec<String>) -> Self {
        Self {
            valid: false,
            errors,
        }
    }
}

/// 规则解析器，支持 YAML/JSON 格式和多语言规则文件
pub struct RuleParser {
    strict_validation: bool,
}

impl Default for RuleParser {
    fn default() -> Self {
        Self {
            strict_validation: true,
        }
    }
}

impl RuleParser {
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse a rule from YAML string
    pub fn parse_yaml(&self, yaml_str: &str) -> Result<ParsedRule, ParseError> {
        serde_yaml::from_str(yaml_str).map_err(|e| ParseError::InvalidYaml(e.to_string()))
    }

    /// Parse a rule from JSON string
    pub fn parse_json(&self, json_str: &str) -> Result<ParsedRule, ParseError> {
        serde_json::from_str(json_str).map_err(|e| ParseError::InvalidJson(e.to_string()))
    }

    /// Validate a parsed rule
    pub fn validate(&self, rule: &ParsedRule) -> ValidationResult {
        let mut errors = Vec::new();

        if rule.id.is_empty() {
            errors.push("Rule ID cannot be empty".to_string());
        }

        if rule.text.is_empty() {
            errors.push("Rule text cannot be empty".to_string());
        }

        if rule.severity.is_empty() {
            errors.push("Severity cannot be empty".to_string());
        } else if !["critical", "major", "minor", "advisory"].contains(&rule.severity.as_str()) {
            errors.push(format!("Invalid severity: {}", rule.severity));
        }

        if rule.language.is_empty() {
            errors.push("Language cannot be empty".to_string());
        } else if !["zh", "en", "ja"].contains(&rule.language.as_str()) {
            errors.push(format!("Unsupported language: {}", rule.language));
        }

        if rule.category.is_empty() {
            errors.push("Category cannot be empty".to_string());
        }

        if self.strict_validation && errors.is_empty() && rule.text.len() < 10 {
            errors.push("Rule text too short to be meaningful".to_string());
        }

        if errors.is_empty() {
            ValidationResult::valid()
        } else {
            ValidationResult::invalid(errors)
        }
    }

    /// Parse multi-language rule from YAML
    pub fn parse_multi_lang_yaml(&self, yaml_str: &str) -> Result<MultiLangRule, ParseError> {
        serde_yaml::from_str(yaml_str).map_err(|e| ParseError::InvalidYaml(e.to_string()))
    }

    /// Parse multi-language rule from JSON
    pub fn parse_multi_lang_json(&self, json_str: &str) -> Result<MultiLangRule, ParseError> {
        serde_json::from_str(json_str).map_err(|e| ParseError::InvalidJson(e.to_string()))
    }

    /// 批量解析多语言规则（YAML 数组）
    pub fn parse_rules_yaml(&self, yaml_str: &str) -> Result<Vec<MultiLangRule>, ParseError> {
        serde_yaml::from_str(yaml_str).map_err(|e| ParseError::InvalidYaml(e.to_string()))
    }

    /// 批量解析多语言规则（JSON 数组）
    pub fn parse_rules_json(&self, json_str: &str) -> Result<Vec<MultiLangRule>, ParseError> {
        serde_json::from_str(json_str).map_err(|e| ParseError::InvalidJson(e.to_string()))
    }

    /// 从 YAML 文件加载规则到 RuleSet
    pub fn load_rules_into_set(&self, yaml_str: &str, set: &mut RuleSet) -> Result<(), ParseError> {
        let rules = self.parse_rules_yaml(yaml_str)?;
        for rule in rules {
            set.add_rule(rule);
        }
        Ok(())
    }

    /// 从 JSON 文件加载规则到 RuleSet
    pub fn load_rules_json_into_set(
        &self,
        json_str: &str,
        set: &mut RuleSet,
    ) -> Result<(), ParseError> {
        let rules = self.parse_rules_json(json_str)?;
        for rule in rules {
            set.add_rule(rule);
        }
        Ok(())
    }

    /// 从目录加载所有规则文件到 RuleSet
    pub fn load_rules_from_dir(
        &self,
        dir_path: &str,
        set: &mut RuleSet,
    ) -> Result<usize, ParseError> {
        let mut count = 0;
        let walker = walkdir::WalkDir::new(dir_path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file());

        for entry in walker {
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

            let content = std::fs::read_to_string(path)
                .map_err(|e| ParseError::Io(format!("Failed to read {:?}: {}", path, e)))?;

            match ext {
                "yaml" | "yml" => {
                    self.load_rules_into_set(&content, set)?;
                    count += 1;
                }
                "json" => {
                    self.load_rules_json_into_set(&content, set)?;
                    count += 1;
                }
                _ => {} // 跳过非规则文件
            }
        }

        Ok(count)
    }

    /// 加载内置的 GMP 合规规则
    pub fn load_builtin_rules(&self) -> RuleSet {
        let mut set = RuleSet::new();

        // GMP 批记录规则 — 中英双语
        let batch_record_rule = MultiLangRule {
            id: "gmp-batch-001".to_string(),
            translations: vec![
                (
                    "zh".to_string(),
                    "所有批记录必须由合格人员审核和签署，签署人须具有 QA 资质。".to_string(),
                ),
                (
                    "en".to_string(),
                    "All batch records must be reviewed and signed by qualified personnel with QA qualification.".to_string(),
                ),
                (
                    "ja".to_string(),
                    "すべてのバッチ記録は、有資格者（QA資格を有する者）による審査と署名が必要です。".to_string(),
                ),
            ]
            .into_iter()
            .collect(),
            severity: "major".to_string(),
            category: "batch_record".to_string(),
            applies_to: vec!["batch_record".to_string()],
            condition: None,
        };
        set.add_rule(batch_record_rule);

        // GMP 双签名规则
        let dual_sig_rule = MultiLangRule {
            id: "gmp-batch-002".to_string(),
            translations: vec![
                (
                    "zh".to_string(),
                    "关键批次的批记录必须由两名合格人员（生产+QA）双签确认。".to_string(),
                ),
                (
                    "en".to_string(),
                    "Critical batch records must be double-signed by two qualified personnel (Production and QA).".to_string(),
                ),
                (
                    "ja".to_string(),
                    "重要なバッチ記録は、2名の有資格者（製造担当者とQA担当者）によるダブルサインが必要です。".to_string(),
                ),
            ]
            .into_iter()
            .collect(),
            severity: "critical".to_string(),
            category: "batch_record".to_string(),
            applies_to: vec!["batch_record".to_string()],
            condition: None,
        };
        set.add_rule(dual_sig_rule);

        // 设备校准规则
        let equipment_rule = MultiLangRule {
            id: "gmp-equip-001".to_string(),
            translations: vec![
                (
                    "zh".to_string(),
                    "所有生产设备必须按规定周期进行校准，校准记录需保存至少两年。".to_string(),
                ),
                (
                    "en".to_string(),
                    "All production equipment must be calibrated at specified intervals; calibration records must be retained for at least two years.".to_string(),
                ),
                (
                    "ja".to_string(),
                    "すべての製造設備は規定の周期で校正を実施し、校正記録は少なくとも2年間保存しなければなりません。".to_string(),
                ),
            ]
            .into_iter()
            .collect(),
            severity: "major".to_string(),
            category: "equipment".to_string(),
            applies_to: vec!["equipment".to_string()],
            condition: None,
        };
        set.add_rule(equipment_rule);

        // 偏差管理规则
        let deviation_rule = MultiLangRule {
            id: "gmp-deviation-001".to_string(),
            translations: vec![
                (
                    "zh".to_string(),
                    "所有偏差必须记录、评估并采取纠正和预防措施（CAPA）。".to_string(),
                ),
                (
                    "en".to_string(),
                    "All deviations must be recorded, evaluated, and followed by Corrective and Preventive Actions (CAPA).".to_string(),
                ),
                (
                    "ja".to_string(),
                    "すべての逸脱は記録、評価され、是正措置および予防措置（CAPA）を講じなければなりません。".to_string(),
                ),
            ]
            .into_iter()
            .collect(),
            severity: "critical".to_string(),
            category: "deviation".to_string(),
            applies_to: vec!["deviation".to_string()],
            condition: None,
        };
        set.add_rule(deviation_rule);

        // 培训合规规则
        let training_rule = MultiLangRule {
            id: "gmp-training-001".to_string(),
            translations: vec![
                (
                    "zh".to_string(),
                    "所有 GMP 相关岗位人员必须完成上岗培训并定期进行再培训。".to_string(),
                ),
                (
                    "en".to_string(),
                    "All GMP-related personnel must complete initial training and receive periodic refresher training.".to_string(),
                ),
                (
                    "ja".to_string(),
                    "すべてのGMP関連業務従事者は、初期研修を修了し、定期的に再研修を受けなければなりません。".to_string(),
                ),
            ]
            .into_iter()
            .collect(),
            severity: "major".to_string(),
            category: "training".to_string(),
            applies_to: vec!["training".to_string()],
            condition: None,
        };
        set.add_rule(training_rule);

        // 环境监测规则
        let env_monitor_rule = MultiLangRule {
            id: "gmp-env-001".to_string(),
            translations: vec![
                (
                    "zh".to_string(),
                    "洁净区必须定期进行悬浮粒子和微生物监测，监测结果需记录。".to_string(),
                ),
                (
                    "en".to_string(),
                    "Clean areas must be regularly monitored for particulate and microbial levels; monitoring results must be documented.".to_string(),
                ),
                (
                    "ja".to_string(),
                    "清浄区域は定期的に浮遊微粒子および微生物のモニタリングを実施し、結果を記録しなければなりません。".to_string(),
                ),
            ]
            .into_iter()
            .collect(),
            severity: "major".to_string(),
            category: "environment".to_string(),
            applies_to: vec!["environment".to_string()],
            condition: None,
        };
        set.add_rule(env_monitor_rule);

        set
    }

    /// 获取规则适用的操作类型映射
    pub fn rule_category_hint(rule_ids: &[String], set: &RuleSet) -> Vec<String> {
        rule_ids
            .iter()
            .filter_map(|id| set.rules.get(id))
            .map(|r| format!("[{}] {}: {}", r.category, r.id, r.severity))
            .collect()
    }

    /// 将规则集转换为 LLM prompt 格式
    pub fn format_rules_for_prompt(rule_ids: &[String], set: &RuleSet, language: &str) -> String {
        let rule_texts = set.get_rule_texts(rule_ids, language);
        if rule_texts.is_empty() {
            return "(无适用规则)".to_string();
        }

        rule_texts
            .iter()
            .map(|(id, text)| format!("- [{}] {}", id, text))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Convert ParsedRule to internal ComplianceRule
    pub fn to_compliance_rule(&self, parsed: &ParsedRule) -> ComplianceRule {
        ComplianceRule {
            name: parsed.id.clone(),
            description: parsed.text.clone(),
            applies_to: vec![parsed.category.clone()],
            condition: RuleCondition::Presence {
                field: "operation".to_string(),
                required: true,
            },
            violation: crate::types::ViolationSpec {
                severity: parsed.severity.clone(),
                message: parsed.text.clone(),
                remediation: None,
            },
            enabled: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_YAML_RULE: &str = r#"
id: "rule-001"
text: "All batch records must be reviewed and signed by a qualified person"
severity: "major"
language: "en"
category: "batch_record"
"#;

    const VALID_JSON_RULE: &str = r#"{
        "id": "rule-001",
        "text": "所有批记录必须由合格人员审核和签署",
        "severity": "major",
        "language": "zh",
        "category": "batch_record"
    }"#;

    // 多语言规则 YAML（单条）
    const MULTI_LANG_YAML: &str = r#"
id: "gmp-batch-001"
translations:
  zh: "所有批记录必须由合格人员审核和签署"
  en: "All batch records must be reviewed and signed by qualified personnel"
  ja: "すべてのバッチ記録は有資格者による審査と署名が必要です"
severity: "major"
category: "batch_record"
applies_to:
  - "batch_record"
"#;

    // 多语言规则 JSON（批量）
    const MULTI_LANG_JSON_BATCH: &str = r#"[
  {
    "id": "gmp-batch-001",
    "translations": {
      "zh": "所有批记录必须由合格人员审核和签署",
      "en": "All batch records must be reviewed and signed"
    },
    "severity": "major",
    "category": "batch_record",
    "applies_to": ["batch_record"]
  },
  {
    "id": "gmp-equip-001",
    "translations": {
      "zh": "所有生产设备必须按规定周期进行校准",
      "en": "All production equipment must be calibrated"
    },
    "severity": "major",
    "category": "equipment",
    "applies_to": ["equipment"]
  }
]"#;

    #[test]
    fn test_parse_yaml_valid() {
        let parser = RuleParser::new();
        let result = parser.parse_yaml(VALID_YAML_RULE).unwrap();
        assert_eq!(result.id, "rule-001");
        assert!(result.text.contains("reviewed"));
        assert_eq!(result.severity, "major");
        assert_eq!(result.language, "en");
    }

    #[test]
    fn test_parse_json_valid() {
        let parser = RuleParser::new();
        let result = parser.parse_json(VALID_JSON_RULE).unwrap();
        assert_eq!(result.id, "rule-001");
        assert!(result.text.contains("批记录"));
        assert_eq!(result.language, "zh");
    }

    #[test]
    fn test_validate_valid() {
        let parser = RuleParser::new();
        let rule = parser.parse_yaml(VALID_YAML_RULE).unwrap();
        let result = parser.validate(&rule);
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validate_empty_id() {
        let parser = RuleParser::new();
        let rule = ParsedRule {
            id: "".to_string(),
            text: "Valid text".to_string(),
            severity: "major".to_string(),
            language: "en".to_string(),
            category: "test".to_string(),
        };
        let result = parser.validate(&rule);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("ID")));
    }

    #[test]
    fn test_validate_invalid_severity() {
        let parser = RuleParser::new();
        let rule = ParsedRule {
            id: "rule-001".to_string(),
            text: "Valid text".to_string(),
            severity: "invalid".to_string(),
            language: "en".to_string(),
            category: "test".to_string(),
        };
        let result = parser.validate(&rule);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("severity")));
    }

    #[test]
    fn test_validate_unsupported_language() {
        let parser = RuleParser::new();
        let rule = ParsedRule {
            id: "rule-001".to_string(),
            text: "Valid text".to_string(),
            severity: "major".to_string(),
            language: "fr".to_string(),
            category: "test".to_string(),
        };
        let result = parser.validate(&rule);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("language")));
    }

    #[test]
    fn test_to_compliance_rule() {
        let parser = RuleParser::new();
        let parsed = parser.parse_yaml(VALID_YAML_RULE).unwrap();
        let rule = parser.to_compliance_rule(&parsed);
        assert_eq!(rule.name, "rule-001");
        assert_eq!(rule.violation.severity, "major");
    }

    #[test]
    fn test_parse_multi_lang_yaml() {
        let parser = RuleParser::new();
        let rule = parser.parse_multi_lang_yaml(MULTI_LANG_YAML).unwrap();
        assert_eq!(rule.id, "gmp-batch-001");
        assert!(rule.translations.contains_key("zh"));
        assert!(rule.translations.contains_key("en"));
        assert!(rule.translations.contains_key("ja"));
        assert_eq!(rule.severity, "major");
        assert_eq!(rule.category, "batch_record");
    }

    #[test]
    fn test_parse_rules_json_batch() {
        let parser = RuleParser::new();
        let rules = parser.parse_rules_json(MULTI_LANG_JSON_BATCH).unwrap();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].id, "gmp-batch-001");
        assert_eq!(rules[1].id, "gmp-equip-001");
    }

    #[test]
    fn test_ruleset_add_and_query() {
        let parser = RuleParser::new();
        let mut set = RuleSet::new();

        let rule = parser.parse_multi_lang_yaml(MULTI_LANG_YAML).unwrap();
        set.add_rule(rule);

        assert_eq!(set.len(), 1);

        // 查询中文文本
        let text_zh = set.get_text("gmp-batch-001", "zh");
        assert!(text_zh.is_some());
        assert!(text_zh.unwrap().contains("批记录"));

        // 查询英文文本
        let text_en = set.get_text("gmp-batch-001", "en");
        assert!(text_en.is_some());
        assert!(text_en.unwrap().contains("batch"));

        // 查询不存在的语言，应 fallback
        let text_fallback = set.get_text("gmp-batch-001", "fr");
        assert!(text_fallback.is_none());

        // 按分类查询
        let category_rules = set.get_rules_by_category("batch_record");
        assert_eq!(category_rules.len(), 1);

        // 按语言查询
        let zh_rules = set.get_rules_by_language("zh");
        assert_eq!(zh_rules.len(), 1);

        // 查询不存在的分类
        let empty = set.get_rules_by_category("nonexistent");
        assert!(empty.is_empty());
    }

    #[test]
    fn test_ruleset_get_rule_texts() {
        let parser = RuleParser::new();
        let mut set = RuleSet::new();
        let rule = parser.parse_multi_lang_yaml(MULTI_LANG_YAML).unwrap();
        set.add_rule(rule);

        let texts = set.get_rule_texts(&["gmp-batch-001".to_string()], "zh");
        assert_eq!(texts.len(), 1);
        assert_eq!(texts[0].0, "gmp-batch-001");
        assert!(texts[0].1.contains("批记录"));
    }

    #[test]
    fn test_format_rules_for_prompt() {
        let parser = RuleParser::new();
        let mut set = RuleSet::new();
        let rule = parser.parse_multi_lang_yaml(MULTI_LANG_YAML).unwrap();
        set.add_rule(rule);

        let formatted =
            RuleParser::format_rules_for_prompt(&["gmp-batch-001".to_string()], &set, "zh");
        assert!(formatted.contains("gmp-batch-001"));
        assert!(formatted.contains("批记录"));
    }

    #[test]
    fn test_empty_ruleset() {
        let set = RuleSet::new();
        assert!(set.is_empty());
        assert_eq!(set.len(), 0);
    }

    #[test]
    fn test_ruleset_get_rule_texts_nonexistent() {
        let set = RuleSet::new();
        let texts = set.get_rule_texts(&["nonexistent".to_string()], "zh");
        assert!(texts.is_empty());
    }

    #[test]
    fn test_builtin_rules() {
        let parser = RuleParser::new();
        let set = parser.load_builtin_rules();

        assert_eq!(set.len(), 6);
        assert!(set.rules.contains_key("gmp-batch-001"));
        assert!(set.rules.contains_key("gmp-batch-002"));
        assert!(set.rules.contains_key("gmp-equip-001"));
        assert!(set.rules.contains_key("gmp-deviation-001"));
        assert!(set.rules.contains_key("gmp-training-001"));
        assert!(set.rules.contains_key("gmp-env-001"));

        // 所有规则应有三语言版本
        for rule in set.rules.values() {
            assert!(
                rule.translations.contains_key("zh"),
                "Rule {} missing zh translation",
                rule.id
            );
            assert!(
                rule.translations.contains_key("en"),
                "Rule {} missing en translation",
                rule.id
            );
            assert!(
                rule.translations.contains_key("ja"),
                "Rule {} missing ja translation",
                rule.id
            );
        }

        // 按分类查询
        assert_eq!(set.get_rules_by_category("batch_record").len(), 2);
        assert_eq!(set.get_rules_by_category("equipment").len(), 1);
        assert_eq!(set.get_rules_by_category("deviation").len(), 1);
        assert_eq!(set.get_rules_by_category("training").len(), 1);
        assert_eq!(set.get_rules_by_category("environment").len(), 1);
    }

    #[test]
    fn test_rule_category_hint() {
        let parser = RuleParser::new();
        let set = parser.load_builtin_rules();

        let hints = RuleParser::rule_category_hint(&["gmp-batch-001".to_string()], &set);
        assert_eq!(hints.len(), 1);
        assert!(hints[0].contains("batch_record"));
    }
}
