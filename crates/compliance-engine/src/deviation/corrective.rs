//! 纠正措施推荐模块
//!
//! 基于偏差调查结果，生成纠正措施（CAPA）建议。
//! 包含立即纠正、预防措施和长期改进三类建议。

use crate::deviation::types::{
    CorrectiveAction, CorrectiveActionStatus, CorrectiveActionType, CorrectivePriority,
    DeviationEvent, DeviationSeverity, Investigation,
};
use std::collections::HashMap;

/// 纠正措施推荐器
pub struct CorrectiveActionRecommender {
    /// 历史纠正措施模式库（类型 -> 典型措施模板）
    action_templates: HashMap<String, Vec<TemplateAction>>,
}

/// 措施模板
struct TemplateAction {
    description: String,
    action_type: CorrectiveActionType,
    priority: CorrectivePriority,
    effectiveness_check: Option<String>,
    expected_outcome: Option<String>,
}

impl CorrectiveActionRecommender {
    /// 创建新的纠正措施推荐器
    pub fn new() -> Self {
        let mut templates: HashMap<String, Vec<TemplateAction>> = HashMap::new();

        // 温度偏差模板
        templates.insert(
            "temperature".to_string(),
            vec![
                TemplateAction {
                    description: "立即调整温度控制参数至规定范围，并确认所有受影响的物料状态"
                        .to_string(),
                    action_type: CorrectiveActionType::Immediate,
                    priority: CorrectivePriority::Critical,
                    effectiveness_check: Some(
                        "连续监控温度 2 小时，确认稳定在 ±1°C 范围内".to_string(),
                    ),
                    expected_outcome: Some("温度恢复至标准操作范围".to_string()),
                },
                TemplateAction {
                    description: "对受温度影响的批次的进行稳定性考察和评估".to_string(),
                    action_type: CorrectiveActionType::Preventive,
                    priority: CorrectivePriority::High,
                    effectiveness_check: Some("完成稳定性考察报告".to_string()),
                    expected_outcome: Some("确认受影响批次的质量状况".to_string()),
                },
                TemplateAction {
                    description: "升级温度监控系统，增加超温预警和自动调节功能".to_string(),
                    action_type: CorrectiveActionType::LongTerm,
                    priority: CorrectivePriority::Medium,
                    effectiveness_check: Some("新系统在 3 个月内无温度偏差事件".to_string()),
                    expected_outcome: Some("防止温度偏差再次发生".to_string()),
                },
            ],
        );

        // 时间偏差模板
        templates.insert(
            "time".to_string(),
            vec![
                TemplateAction {
                    description: "评估时间偏差对中间体/产品质量的影响，必要时进行复检".to_string(),
                    action_type: CorrectiveActionType::Immediate,
                    priority: CorrectivePriority::High,
                    effectiveness_check: Some("完成质量影响评估报告".to_string()),
                    expected_outcome: Some("确认产品是否可放行".to_string()),
                },
                TemplateAction {
                    description: "优化生产排程和操作 SOP，明确各步骤的时间窗口".to_string(),
                    action_type: CorrectiveActionType::Preventive,
                    priority: CorrectivePriority::Medium,
                    effectiveness_check: Some("更新后的 SOP 已培训并执行".to_string()),
                    expected_outcome: Some("减少时间偏差发生率".to_string()),
                },
                TemplateAction {
                    description: "实施 MES 系统自动计时和提醒功能".to_string(),
                    action_type: CorrectiveActionType::LongTerm,
                    priority: CorrectivePriority::Low,
                    effectiveness_check: Some("系统自动提醒功能上线并运行 1 个月".to_string()),
                    expected_outcome: Some("消除人为计时错误".to_string()),
                },
            ],
        );

        // 操作步骤偏差模板
        templates.insert(
            "procedure".to_string(),
            vec![
                TemplateAction {
                    description: "立即暂停涉及偏差步骤的操作，进行偏差评估".to_string(),
                    action_type: CorrectiveActionType::Immediate,
                    priority: CorrectivePriority::Critical,
                    effectiveness_check: Some("偏差评估报告已审批".to_string()),
                    expected_outcome: Some("防止偏差扩大".to_string()),
                },
                TemplateAction {
                    description: "对相关操作人员进行 SOP 再培训".to_string(),
                    action_type: CorrectiveActionType::Preventive,
                    priority: CorrectivePriority::High,
                    effectiveness_check: Some("培训记录和考核结果已归档".to_string()),
                    expected_outcome: Some("操作人员熟练掌握 SOP".to_string()),
                },
                TemplateAction {
                    description: "审查并优化相关 SOP，增加操作关键步骤的图示和警示".to_string(),
                    action_type: CorrectiveActionType::LongTerm,
                    priority: CorrectivePriority::Medium,
                    effectiveness_check: Some("SOP 更新版本已发布".to_string()),
                    expected_outcome: Some("提高 SOP 的可执行性".to_string()),
                },
            ],
        );

        // 设备偏差模板
        templates.insert(
            "equipment".to_string(),
            vec![
                TemplateAction {
                    description: "立即停用故障设备，启用备用设备或调整生产计划".to_string(),
                    action_type: CorrectiveActionType::Immediate,
                    priority: CorrectivePriority::Critical,
                    effectiveness_check: Some("备用设备运行正常，生产计划已调整".to_string()),
                    expected_outcome: Some("生产不中断".to_string()),
                },
                TemplateAction {
                    description: "对设备进行彻底检查和维修，确定根本原因".to_string(),
                    action_type: CorrectiveActionType::Preventive,
                    priority: CorrectivePriority::High,
                    effectiveness_check: Some("设备维修报告已完成，设备恢复正常".to_string()),
                    expected_outcome: Some("设备恢复至可运行状态".to_string()),
                },
                TemplateAction {
                    description: "制定或更新设备预防性维护计划，增加关键部件更换频率".to_string(),
                    action_type: CorrectiveActionType::LongTerm,
                    priority: CorrectivePriority::Medium,
                    effectiveness_check: Some("设备在下一个维护周期内无故障".to_string()),
                    expected_outcome: Some("降低设备故障率".to_string()),
                },
            ],
        );

        Self {
            action_templates: templates,
        }
    }

    /// 根据偏差事件和调查发现推荐纠正措施
    pub fn recommend(
        &self,
        event: &DeviationEvent,
        investigation: &Investigation,
    ) -> Vec<CorrectiveAction> {
        let mut actions = Vec::new();
        let event_key = event.event_type.as_str().to_lowercase();

        // 从模板获取对应类型的措施
        if let Some(templates) = self.action_templates.get(&event_key) {
            for (i, tmpl) in templates.iter().enumerate() {
                let action = self.template_to_action(tmpl, event, investigation, i);
                actions.push(action);
            }
        } else {
            // 通用措施（用于未匹配的类型）
            actions.push(self.make_generic_immediate_action(event, investigation));
            actions.push(self.make_generic_preventive_action(event, investigation));
            actions.push(self.make_generic_longterm_action(event, investigation));
        }

        // 根据严重级别调整优先级
        actions = self.adjust_priorities(actions, event.severity);

        actions
    }

    /// 将模板转换为具体纠正措施
    fn template_to_action(
        &self,
        tmpl: &TemplateAction,
        event: &DeviationEvent,
        _investigation: &Investigation,
        index: usize,
    ) -> CorrectiveAction {
        let severity_prefix = match event.severity {
            DeviationSeverity::Critical => "CRI",
            DeviationSeverity::Major => "MAJ",
            DeviationSeverity::Minor => "MIN",
            DeviationSeverity::Advisory => "ADV",
        };

        CorrectiveAction {
            id: format!("CA-{}-{}-{}", severity_prefix, event.event_id, index + 1),
            description: tmpl.description.clone(),
            action_type: tmpl.action_type,
            priority: tmpl.priority,
            owner: None,
            deadline: None,
            status: CorrectiveActionStatus::Proposed,
            effectiveness_check: tmpl.effectiveness_check.clone(),
            expected_outcome: tmpl.expected_outcome.clone(),
        }
    }

    /// 生成通用立即纠正措施
    fn make_generic_immediate_action(
        &self,
        event: &DeviationEvent,
        _investigation: &Investigation,
    ) -> CorrectiveAction {
        CorrectiveAction {
            id: format!("CA-GEN-IMM-{}", event.event_id),
            description: format!("立即评估偏差影响，确认是否需要隔离受影响的产品/批次",),
            action_type: CorrectiveActionType::Immediate,
            priority: CorrectivePriority::High,
            owner: None,
            deadline: None,
            status: CorrectiveActionStatus::Proposed,
            effectiveness_check: Some("受影响物料已隔离并标识".to_string()),
            expected_outcome: Some("防止不合格产品流入下一环节".to_string()),
        }
    }

    /// 生成通用预防措施
    fn make_generic_preventive_action(
        &self,
        event: &DeviationEvent,
        _investigation: &Investigation,
    ) -> CorrectiveAction {
        CorrectiveAction {
            id: format!("CA-GEN-PRV-{}", event.event_id),
            description: format!("分析偏差根本原因，更新相关 SOP 和培训计划",),
            action_type: CorrectiveActionType::Preventive,
            priority: CorrectivePriority::Medium,
            owner: None,
            deadline: None,
            status: CorrectiveActionStatus::Proposed,
            effectiveness_check: Some("SOP 已完成更新，人员已培训".to_string()),
            expected_outcome: Some("降低类似偏差重复发生率".to_string()),
        }
    }

    /// 生成通用长期改进措施
    fn make_generic_longterm_action(
        &self,
        event: &DeviationEvent,
        _investigation: &Investigation,
    ) -> CorrectiveAction {
        CorrectiveAction {
            id: format!("CA-GEN-LTP-{}", event.event_id),
            description: format!("建立偏差趋势分析机制，定期审查偏差数据并驱动持续改进",),
            action_type: CorrectiveActionType::LongTerm,
            priority: CorrectivePriority::Low,
            owner: None,
            deadline: None,
            status: CorrectiveActionStatus::Proposed,
            effectiveness_check: Some("偏差趋势分析报告已生成并纳入管理评审".to_string()),
            expected_outcome: Some("系统性降低偏差发生率".to_string()),
        }
    }

    /// 根据事件严重级别调整措施优先级
    fn adjust_priorities(
        &self,
        actions: Vec<CorrectiveAction>,
        severity: DeviationSeverity,
    ) -> Vec<CorrectiveAction> {
        let bump = match severity {
            DeviationSeverity::Critical => 2, // 所有措施升级两级
            DeviationSeverity::Major => 1,    // 所有措施升级一级
            _ => 0,
        };

        actions
            .into_iter()
            .map(|action| {
                let mut adjusted = action;
                if bump > 0 {
                    adjusted.priority = self.bump_priority(adjusted.priority, bump);
                }
                adjusted
            })
            .collect()
    }

    /// 优先级升级
    fn bump_priority(&self, priority: CorrectivePriority, levels: u8) -> CorrectivePriority {
        match priority {
            CorrectivePriority::Low if levels >= 1 => CorrectivePriority::Medium,
            CorrectivePriority::Low => CorrectivePriority::Low,
            CorrectivePriority::Medium if levels >= 2 => CorrectivePriority::Critical,
            CorrectivePriority::Medium if levels >= 1 => CorrectivePriority::High,
            CorrectivePriority::Medium => CorrectivePriority::Medium,
            CorrectivePriority::High if levels >= 1 => CorrectivePriority::Critical,
            CorrectivePriority::High => CorrectivePriority::High,
            CorrectivePriority::Critical => CorrectivePriority::Critical,
        }
    }

    /// 根据根因分析结果优化措施描述
    pub fn refine_actions_with_root_cause(
        &self,
        actions: Vec<CorrectiveAction>,
        root_cause: &str,
    ) -> Vec<CorrectiveAction> {
        actions
            .into_iter()
            .map(|action| {
                let mut refined = action;
                if refined.action_type == CorrectiveActionType::Preventive
                    || refined.action_type == CorrectiveActionType::LongTerm
                {
                    refined.description =
                        format!("{} （针对根因：{}）", refined.description, root_cause);
                }
                refined
            })
            .collect()
    }
}

impl Default for CorrectiveActionRecommender {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deviation::types::{
        DeviationEvent, DeviationEventType, DeviationSeverity, Investigation,
    };

    fn make_test_event() -> DeviationEvent {
        DeviationEvent::new(
            "DEV-TEST-001",
            DeviationEventType::Temperature,
            "灭菌温度超出设定范围，设定值 121°C，实际值 118.5°C",
            DeviationSeverity::Major,
            "BMS 系统",
        )
        .with_batch("注射用水", "WFI-20240501")
    }

    fn make_test_investigation(event: &DeviationEvent) -> Investigation {
        Investigation::new("INV-TEST-001", event, "QA 调查员")
    }

    #[test]
    fn test_recommender_new() {
        let recommender = CorrectiveActionRecommender::new();
        assert!(!recommender.action_templates.is_empty());
        assert!(recommender.action_templates.contains_key("temperature"));
        assert!(recommender.action_templates.contains_key("time"));
        assert!(recommender.action_templates.contains_key("procedure"));
        assert!(recommender.action_templates.contains_key("equipment"));
    }

    #[test]
    fn test_recommend_temperature_deviation() {
        let event = make_test_event();
        let investigation = make_test_investigation(&event);
        let recommender = CorrectiveActionRecommender::new();

        let actions = recommender.recommend(&event, &investigation);
        assert!(!actions.is_empty());
        assert!(
            actions.len() >= 3,
            "应该有至少 3 个纠正措施（立即/预防/长期）"
        );

        // 检查三种类型都存在
        let types: Vec<CorrectiveActionType> = actions.iter().map(|a| a.action_type).collect();
        assert!(types.contains(&CorrectiveActionType::Immediate));
        assert!(types.contains(&CorrectiveActionType::Preventive));
        assert!(types.contains(&CorrectiveActionType::LongTerm));
    }

    #[test]
    fn test_recommend_procedure_deviation() {
        let event = DeviationEvent::new(
            "DEV-TEST-002",
            DeviationEventType::Procedure,
            "操作步骤顺序错误",
            DeviationSeverity::Minor,
            "操作员B",
        );
        let investigation = make_test_investigation(&event);
        let recommender = CorrectiveActionRecommender::new();

        let actions = recommender.recommend(&event, &investigation);
        assert!(!actions.is_empty());
        let has_procedure_action = actions.iter().any(|a| a.description.contains("SOP"));
        assert!(has_procedure_action);
    }

    #[test]
    fn test_recommend_generic_unknown_type() {
        let event = DeviationEvent::new(
            "DEV-TEST-003",
            DeviationEventType::Other("自定义偏差".to_string()),
            "自定义偏差类型",
            DeviationSeverity::Advisory,
            "测试",
        );
        let investigation = make_test_investigation(&event);
        let recommender = CorrectiveActionRecommender::new();

        let actions = recommender.recommend(&event, &investigation);
        assert_eq!(actions.len(), 3);
        assert!(actions[0].id.contains("GEN-IMM"));
        assert!(actions[1].id.contains("GEN-PRV"));
        assert!(actions[2].id.contains("GEN-LTP"));
    }

    #[test]
    fn test_priority_adjustment_critical() {
        let event = DeviationEvent::new(
            "DEV-TEST-004",
            DeviationEventType::Temperature,
            "关键温度偏差",
            DeviationSeverity::Critical,
            "系统",
        );
        let investigation = make_test_investigation(&event);
        let recommender = CorrectiveActionRecommender::new();

        let actions = recommender.recommend(&event, &investigation);
        // Critical 事件的措施优先级应该都较高
        let priorities: Vec<CorrectivePriority> = actions.iter().map(|a| a.priority).collect();
        assert!(priorities
            .iter()
            .any(|p| *p == CorrectivePriority::Critical || *p == CorrectivePriority::High));
    }

    #[test]
    fn test_refine_with_root_cause() {
        let recommender = CorrectiveActionRecommender::new();
        let event = make_test_event();
        let investigation = make_test_investigation(&event);
        let actions = recommender.recommend(&event, &investigation);

        let refined = recommender.refine_actions_with_root_cause(actions, "温度传感器校准过期");
        for action in &refined {
            if action.action_type == CorrectiveActionType::Preventive
                || action.action_type == CorrectiveActionType::LongTerm
            {
                assert!(
                    action.description.contains("温度传感器校准过期"),
                    "预防/长期措施应包含根因信息: {}",
                    action.description
                );
            }
        }
    }

    #[test]
    fn test_priority_bump_logic() {
        let recommender = CorrectiveActionRecommender::new();

        assert_eq!(
            recommender.bump_priority(CorrectivePriority::Low, 1),
            CorrectivePriority::Medium
        );
        assert_eq!(
            recommender.bump_priority(CorrectivePriority::Low, 2),
            CorrectivePriority::Medium
        );
        assert_eq!(
            recommender.bump_priority(CorrectivePriority::Medium, 1),
            CorrectivePriority::High
        );
        assert_eq!(
            recommender.bump_priority(CorrectivePriority::Medium, 2),
            CorrectivePriority::Critical
        );
        assert_eq!(
            recommender.bump_priority(CorrectivePriority::High, 1),
            CorrectivePriority::Critical
        );
        assert_eq!(
            recommender.bump_priority(CorrectivePriority::Critical, 1),
            CorrectivePriority::Critical
        );
        assert_eq!(
            recommender.bump_priority(CorrectivePriority::Low, 0),
            CorrectivePriority::Low
        );
    }

    #[test]
    fn test_action_id_format() {
        let event = make_test_event();
        let investigation = make_test_investigation(&event);
        let recommender = CorrectiveActionRecommender::new();

        let actions = recommender.recommend(&event, &investigation);
        for action in &actions {
            assert!(
                action.id.starts_with("CA-"),
                "Action ID 应以 CA- 开头: {}",
                action.id
            );
            assert!(
                action.id.contains("DEV-TEST-001"),
                "Action ID 应包含事件 ID: {}",
                action.id
            );
        }
    }

    #[test]
    fn test_default_impl() {
        let recommender: CorrectiveActionRecommender = Default::default();
        assert!(!recommender.action_templates.is_empty());
    }
}
