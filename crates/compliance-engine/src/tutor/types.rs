//! 培训助手数据类型
//!
//! 定义 AI 合规培训助手核心数据类型：
//! - Regulation: GMP 法规/条款
//! - KnowledgeEntry: 知识库条目
//! - TutorQuery: 用户查询
//! - TutorResponse: 系统回答
//! - TrainingRecord: 培训记录

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// GMP 法规/条款
///
/// 表示一条具体的 GMP 法规条款，包含来源、内容和分类信息。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Regulation {
    /// 法规唯一标识（如 "cfr-211-110"）
    pub id: String,
    /// 法规标题
    pub title: String,
    /// 来源（如 "21 CFR Part 211", "EU GMP Annex 11"）
    pub source: String,
    /// 法规正文
    pub content: String,
    /// 分类标签（如 "生产", "质量", "设备", "验证"）
    pub category: String,
    /// 关键词（用于检索匹配）
    pub keywords: Vec<String>,
    /// 法规版本/生效年份
    pub version: String,
}

impl Regulation {
    /// 创建新的法规条目
    pub fn new(
        id: &str,
        title: &str,
        source: &str,
        content: &str,
        category: &str,
        keywords: Vec<&str>,
        version: &str,
    ) -> Self {
        Self {
            id: id.to_string(),
            title: title.to_string(),
            source: source.to_string(),
            content: content.to_string(),
            category: category.to_string(),
            keywords: keywords.into_iter().map(|k| k.to_string()).collect(),
            version: version.to_string(),
        }
    }

    /// 检查查询文本是否匹配此法规（基于关键词和内容）
    pub fn matches(&self, query: &str) -> bool {
        if query.trim().is_empty() {
            return false;
        }
        let query_lower = query.to_lowercase();
        self.keywords
            .iter()
            .any(|k| query_lower.contains(&k.to_lowercase()))
            || self.content.to_lowercase().contains(&query_lower)
            || self.title.to_lowercase().contains(&query_lower)
    }

    /// 计算与查询的相关性得分（简化版的 TF 匹配）
    pub fn relevance_score(&self, query: &str) -> f64 {
        let query_lower = query.to_lowercase();
        let mut score = 0.0_f64;

        // 关键词匹配加分
        for kw in &self.keywords {
            if query_lower.contains(&kw.to_lowercase()) {
                score += 2.0;
            }
        }

        // 标题匹配加分
        if self.title.to_lowercase().contains(&query_lower) {
            score += 3.0;
        }

        // 内容匹配加分
        let content_lower = self.content.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        for word in &query_words {
            if word.len() > 2 && content_lower.contains(word) {
                score += 0.5;
            }
        }

        score
    }
}

/// 知识库条目
///
/// 包含法规知识及其解释，用于培训问答。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    /// 知识条目 ID
    pub id: String,
    /// 引用的法规 ID
    pub regulation_id: String,
    /// 知识点标题
    pub title: String,
    /// 知识点详细解释
    pub explanation: String,
    /// 适用场景/示例
    pub examples: Vec<String>,
    /// 常见问题（FAQ 模式）
    pub faq_questions: Vec<String>,
    /// 分类标签
    pub category: String,
    /// 难度级别（1-5）
    pub difficulty: u8,
}

impl KnowledgeEntry {
    /// 创建新知识条目
    pub fn new(
        id: &str,
        regulation_id: &str,
        title: &str,
        explanation: &str,
        category: &str,
        difficulty: u8,
    ) -> Self {
        Self {
            id: id.to_string(),
            regulation_id: regulation_id.to_string(),
            title: title.to_string(),
            explanation: explanation.to_string(),
            examples: Vec::new(),
            faq_questions: Vec::new(),
            category: category.to_string(),
            difficulty,
        }
    }

    /// 添加示例
    pub fn with_examples(mut self, examples: Vec<&str>) -> Self {
        self.examples = examples.into_iter().map(|s| s.to_string()).collect();
        self
    }

    /// 添加 FAQ 问题
    pub fn with_faq(mut self, questions: Vec<&str>) -> Self {
        self.faq_questions = questions.into_iter().map(|s| s.to_string()).collect();
        self
    }
}

/// 培训查询
///
/// 用户向培训助手提出的问题。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TutorQuery {
    /// 查询唯一 ID
    pub query_id: String,
    /// 用户 ID（培训学员）
    pub user_id: String,
    /// 问题文本
    pub question: String,
    /// 可选的分类过滤（如仅查询"生产"类法规）
    pub category_filter: Option<String>,
    /// 查询语言（默认 "zh"）
    pub language: String,
    /// 查询时间
    pub timestamp: DateTime<Utc>,
}

impl TutorQuery {
    /// 创建新的培训查询
    pub fn new(user_id: &str, question: &str) -> Self {
        Self {
            query_id: uuid::Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            question: question.to_string(),
            category_filter: None,
            language: "zh".to_string(),
            timestamp: Utc::now(),
        }
    }

    /// 设置分类过滤
    pub fn with_category(mut self, category: &str) -> Self {
        self.category_filter = Some(category.to_string());
        self
    }

    /// 设置语言
    pub fn with_language(mut self, language: &str) -> Self {
        self.language = language.to_string();
        self
    }
}

/// 培训回答
///
/// 培训助手对用户查询的完整回答，包含引用来源。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TutorResponse {
    /// 对应的查询 ID
    pub query_id: String,
    /// 回答内容（自然语言）
    pub answer: String,
    /// 引用的法规来源列表
    pub citations: Vec<String>,
    /// 引用的知识条目 ID 列表
    pub knowledge_ids: Vec<String>,
    /// 相关法规详细内容
    pub relevant_regulations: Vec<Regulation>,
    /// 回答是否基于知识库（而非猜测）
    pub is_grounded: bool,
    /// LLM 模型名称（如有）
    pub model: Option<String>,
    /// 置信度（0.0-1.0）
    pub confidence: f64,
    /// 回答生成时间（毫秒）
    pub processing_time_ms: u64,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
}

impl TutorResponse {
    /// 创建一个无法回答的兜底响应
    pub fn cannot_answer(query_id: &str) -> Self {
        Self {
            query_id: query_id.to_string(),
            answer: "抱歉，我目前的知识库中没有找到与您问题相关的 GMP 法规信息。".to_string(),
            citations: Vec::new(),
            knowledge_ids: Vec::new(),
            relevant_regulations: Vec::new(),
            is_grounded: false,
            model: None,
            confidence: 0.0,
            processing_time_ms: 0,
            timestamp: Utc::now(),
        }
    }
}

/// 培训记录
///
/// 追踪学员的培训活动和掌握情况。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingRecord {
    /// 记录 ID
    pub record_id: String,
    /// 学员 ID
    pub trainee_id: String,
    /// 培训模块/主题
    pub topic: String,
    /// 提问次数
    pub question_count: u32,
    /// 正确回答数（用于评估掌握程度）
    pub correct_count: u32,
    /// 总培训时长（秒）
    pub duration_seconds: u64,
    /// 掌握率（correct_count / question_count）
    pub mastery_rate: f64,
    /// 涉及的知识条目 ID 列表
    pub knowledge_ids: Vec<String>,
    /// 培训开始时间
    pub started_at: DateTime<Utc>,
    /// 最后活动时间
    pub last_activity_at: DateTime<Utc>,
    /// 培训状态
    pub status: TrainingStatus,
}

/// 培训状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrainingStatus {
    /// 进行中
    InProgress,
    /// 已完成
    Completed,
    /// 未通过（需要重新培训）
    Failed,
}

impl TrainingStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

impl TrainingRecord {
    /// 创建新的培训记录
    pub fn new(trainee_id: &str, topic: &str) -> Self {
        let now = Utc::now();
        Self {
            record_id: uuid::Uuid::new_v4().to_string(),
            trainee_id: trainee_id.to_string(),
            topic: topic.to_string(),
            question_count: 0,
            correct_count: 0,
            duration_seconds: 0,
            mastery_rate: 0.0,
            knowledge_ids: Vec::new(),
            started_at: now,
            last_activity_at: now,
            status: TrainingStatus::InProgress,
        }
    }

    /// 记录一次回答
    pub fn record_answer(&mut self, is_correct: bool, knowledge_id: &str) {
        self.question_count += 1;
        if is_correct {
            self.correct_count += 1;
        }
        if !self.knowledge_ids.contains(&knowledge_id.to_string()) {
            self.knowledge_ids.push(knowledge_id.to_string());
        }
        self.last_activity_at = Utc::now();
        self.recalculate_mastery_rate();
    }

    /// 更新培训时长
    pub fn update_duration(&mut self, seconds: u64) {
        self.duration_seconds = seconds;
    }

    /// 重新计算掌握率
    fn recalculate_mastery_rate(&mut self) {
        if self.question_count > 0 {
            self.mastery_rate = self.correct_count as f64 / self.question_count as f64;
        }
    }

    /// 完成培训
    pub fn complete(&mut self, passing_threshold: f64) {
        self.recalculate_mastery_rate();
        self.status = if self.mastery_rate >= passing_threshold {
            TrainingStatus::Completed
        } else {
            TrainingStatus::Failed
        };
        self.last_activity_at = Utc::now();
    }

    /// 检查是否需要更多培训（掌握率低于阈值且问题数足够）
    pub fn needs_remediation(&self, min_questions: u32, passing_threshold: f64) -> bool {
        self.question_count >= min_questions && self.mastery_rate < passing_threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Regulation tests ──

    #[test]
    fn test_regulation_creation() {
        let reg = Regulation::new(
            "cfr-211-110",
            "21 CFR 211.110 生产和过程控制",
            "21 CFR Part 211",
            "生产过程应进行控制以确保产品质量。",
            "生产",
            vec!["生产", "过程控制", "质量"],
            "2024",
        );
        assert_eq!(reg.id, "cfr-211-110");
        assert_eq!(reg.source, "21 CFR Part 211");
        assert_eq!(reg.category, "生产");
        assert_eq!(reg.keywords.len(), 3);
    }

    #[test]
    fn test_regulation_matches_by_keyword() {
        let reg = Regulation::new(
            "eu-annex11-1",
            "EU GMP Annex 11 计算机化系统",
            "EU GMP Annex 11",
            "计算机化系统应经过验证。",
            "验证",
            vec!["计算机化", "验证", "系统"],
            "2023",
        );
        assert!(reg.matches("计算机化系统验证"));
        assert!(reg.matches("验证要求"));
        assert!(!reg.matches("清洁生产工艺"));
    }

    #[test]
    fn test_regulation_matches_by_content() {
        let reg = Regulation::new(
            "cfr-211-113",
            "21 CFR 211.113 微生物污染控制",
            "21 CFR Part 211",
            "应建立并执行微生物污染控制程序。",
            "质量",
            vec!["微生物", "污染控制"],
            "2024",
        );
        assert!(reg.matches("微生物污染控制程序"));
    }

    #[test]
    fn test_regulation_relevance_score() {
        let reg = Regulation::new(
            "cfr-211-100",
            "21 CFR 211.100 书面程序",
            "21 CFR Part 211",
            "应有书面的生产和过程控制程序。",
            "生产",
            vec!["书面程序", "SOP"],
            "2024",
        );
        let score = reg.relevance_score("书面程序要求 SOP");
        assert!(score > 0.0);
    }

    #[test]
    fn test_regulation_clone() {
        let reg = Regulation::new(
            "test-001",
            "测试法规",
            "Test Source",
            "测试内容",
            "测试",
            vec!["测试"],
            "1.0",
        );
        let cloned = reg.clone();
        assert_eq!(cloned.id, reg.id);
        assert_eq!(cloned.source, reg.source);
    }

    // ── KnowledgeEntry tests ──

    #[test]
    fn test_knowledge_entry_creation() {
        let entry = KnowledgeEntry::new(
            "k-001",
            "cfr-211-110",
            "生产过程控制要求",
            "生产过程中必须建立并执行书面控制程序...",
            "生产",
            3,
        );
        assert_eq!(entry.id, "k-001");
        assert_eq!(entry.regulation_id, "cfr-211-110");
        assert_eq!(entry.difficulty, 3);
        assert!(entry.examples.is_empty());
    }

    #[test]
    fn test_knowledge_entry_with_examples() {
        let entry = KnowledgeEntry::new("k-002", "cfr-211-113", "微生物控制", "内容", "质量", 2)
            .with_examples(vec!["无菌操作", "环境监控"]);
        assert_eq!(entry.examples.len(), 2);
        assert_eq!(entry.examples[0], "无菌操作");
    }

    #[test]
    fn test_knowledge_entry_with_faq() {
        let entry = KnowledgeEntry::new(
            "k-003",
            "eu-annex11-1",
            "计算机化系统验证",
            "内容",
            "验证",
            4,
        )
        .with_faq(vec!["什么系统需要验证？", "验证频率是多少？"]);
        assert_eq!(entry.faq_questions.len(), 2);
    }

    // ── TutorQuery tests ──

    #[test]
    fn test_tutor_query_creation() {
        let query = TutorQuery::new("user-001", "生产过程控制有哪些要求？");
        assert!(!query.query_id.is_empty());
        assert_eq!(query.user_id, "user-001");
        assert_eq!(query.question, "生产过程控制有哪些要求？");
        assert_eq!(query.language, "zh");
    }

    #[test]
    fn test_tutor_query_with_category() {
        let query = TutorQuery::new("user-002", "验证要求").with_category("验证");
        assert_eq!(query.category_filter, Some("验证".to_string()));
    }

    #[test]
    fn test_tutor_query_with_language() {
        let query = TutorQuery::new("user-003", "validation requirements").with_language("en");
        assert_eq!(query.language, "en");
    }

    // ── TutorResponse tests ──

    #[test]
    fn test_tutor_response_cannot_answer() {
        let resp = TutorResponse::cannot_answer("query-001");
        assert_eq!(resp.query_id, "query-001");
        assert!(!resp.is_grounded);
        assert_eq!(resp.confidence, 0.0);
        assert!(resp.citations.is_empty());
    }

    // ── TrainingRecord tests ──

    #[test]
    fn test_training_record_creation() {
        let record = TrainingRecord::new("trainee-001", "GMP 基础知识");
        assert_eq!(record.trainee_id, "trainee-001");
        assert_eq!(record.topic, "GMP 基础知识");
        assert_eq!(record.question_count, 0);
        assert_eq!(record.status, TrainingStatus::InProgress);
    }

    #[test]
    fn test_training_record_record_answer() {
        let mut record = TrainingRecord::new("trainee-001", "GMP 基础知识");
        record.record_answer(true, "k-001");
        assert_eq!(record.question_count, 1);
        assert_eq!(record.correct_count, 1);
        assert!((record.mastery_rate - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_training_record_record_answer_wrong() {
        let mut record = TrainingRecord::new("trainee-001", "GMP 基础知识");
        record.record_answer(false, "k-001");
        assert_eq!(record.question_count, 1);
        assert_eq!(record.correct_count, 0);
        assert!((record.mastery_rate - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_training_record_complete_pass() {
        let mut record = TrainingRecord::new("trainee-001", "GMP 基础知识");
        record.record_answer(true, "k-001");
        record.record_answer(true, "k-002");
        record.record_answer(true, "k-003");
        record.complete(0.8);
        assert_eq!(record.status, TrainingStatus::Completed);
    }

    #[test]
    fn test_training_record_complete_fail() {
        let mut record = TrainingRecord::new("trainee-001", "GMP 基础知识");
        record.record_answer(true, "k-001");
        record.record_answer(false, "k-002");
        record.record_answer(false, "k-003");
        record.complete(0.8);
        assert_eq!(record.status, TrainingStatus::Failed);
    }

    #[test]
    fn test_training_record_needs_remediation() {
        let mut record = TrainingRecord::new("trainee-001", "GMP 基础知识");
        record.record_answer(false, "k-001");
        record.record_answer(false, "k-002");
        record.record_answer(false, "k-003");
        assert!(record.needs_remediation(3, 0.8));
    }

    #[test]
    fn test_training_record_no_remediation_if_few_questions() {
        let mut record = TrainingRecord::new("trainee-001", "GMP 基础知识");
        record.record_answer(false, "k-001");
        assert!(!record.needs_remediation(3, 0.8));
    }

    #[test]
    fn test_training_status_as_str() {
        assert_eq!(TrainingStatus::InProgress.as_str(), "in_progress");
        assert_eq!(TrainingStatus::Completed.as_str(), "completed");
        assert_eq!(TrainingStatus::Failed.as_str(), "failed");
    }

    #[test]
    fn test_training_record_update_duration() {
        let mut record = TrainingRecord::new("trainee-001", "GMP 基础知识");
        record.update_duration(3600);
        assert_eq!(record.duration_seconds, 3600);
    }
}
