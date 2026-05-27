//! 问答 Agent 主循环
//!
//! AI 合规培训助手核心 Agent。
//! 执行流程：查询 → 知识检索 → LLM 推理 → 带引用回答。
//! 支持 mock 模式（无需 LLM）和真实 LLM 模式。

use crate::tutor::knowledge::KnowledgeRetriever;
use crate::tutor::tracker::TrainingTracker;
use crate::tutor::types::{Regulation, TrainingRecord, TutorQuery, TutorResponse};
use chrono::Utc;
use std::time::Instant;

/// 培训 Agent 错误类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TutorAgentError {
    /// 空查询
    EmptyQuery,
    /// 查询过短
    QueryTooShort,
    /// 未找到相关法规
    NoRelevantRegulations,
    /// LLM 调用失败
    LlmFailure,
}

impl std::fmt::Display for TutorAgentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyQuery => write!(f, "查询内容不能为空"),
            Self::QueryTooShort => write!(f, "查询内容过短，请输入更详细的问题"),
            Self::NoRelevantRegulations => write!(f, "未找到与查询相关的 GMP 法规"),
            Self::LlmFailure => write!(f, "LLM 调用失败"),
        }
    }
}

/// LLM 推理客户端 trait
///
/// 用于支持 mock 和真实 LLM 两种模式。
pub trait LlmClient: Send + Sync {
    /// 发送 prompt 到 LLM 并返回回答
    fn chat(&self, prompt: &str, regulations: &[Regulation]) -> Result<String, String>;
}

/// 内置 LLM 推理引擎（规则引擎模式）
///
/// 当没有外部 LLM 时，使用基于模板的推理逻辑。
/// 此模式提供一个基础的规则化回答框架。
pub struct BuiltinLlmEngine;

impl LlmClient for BuiltinLlmEngine {
    fn chat(&self, _prompt: &str, regulations: &[Regulation]) -> Result<String, String> {
        // 内置推理逻辑：基于检索到的法规内容生成回答
        if regulations.is_empty() {
            return Err("No regulations found".to_string());
        }

        let mut answer_parts: Vec<String> = Vec::new();

        // 从最近的法规中提取关键信息
        for reg in regulations.iter().take(3) {
            let summary = extract_key_summary(reg);
            answer_parts.push(summary);
        }

        if answer_parts.is_empty() {
            return Err("Could not generate answer from regulations".to_string());
        }

        // 使用换行连接各部分的回答
        Ok(answer_parts.join("\n\n"))
    }
}

/// 提取法规的关键摘要
fn extract_key_summary(reg: &Regulation) -> String {
    // 从法规内容中提取前 80 个字符作为摘要（使用 chars() 避免 UTF-8 截断）
    let chars: Vec<char> = reg.content.chars().collect();
    let content_preview: String = if chars.len() > 80 {
        chars[..80].iter().collect::<String>() + "..."
    } else {
        reg.content.clone()
    };

    format!("【{}】{}：{}", reg.source, reg.title, content_preview)
}

/// 培训 Agent 配置
#[derive(Debug, Clone)]
pub struct TutorAgentConfig {
    /// LLM 模型名称（用于日志）
    pub model: String,
    /// 检索返回的最大法规数量
    pub top_k: usize,
    /// 最小查询长度
    pub min_query_length: usize,
    /// 是否启用 LLM 推理（false = 仅知识检索）
    pub enable_llm: bool,
}

impl Default for TutorAgentConfig {
    fn default() -> Self {
        Self {
            model: "builtin".to_string(),
            top_k: 5,
            min_query_length: 2,
            enable_llm: true,
        }
    }
}

/// AI 合规培训 Agent
///
/// 核心问答引擎，接收用户查询并返回带引用来源的回答。
pub struct TutorAgent {
    /// 知识检索器
    retriever: KnowledgeRetriever,
    /// 培训记录追踪器
    tracker: TrainingTracker,
    /// LLM 客户端（可选）
    llm_client: Option<Box<dyn LlmClient>>,
    /// Agent 配置
    config: TutorAgentConfig,
}

impl TutorAgent {
    /// 创建新的培训 Agent
    pub fn new(config: TutorAgentConfig) -> Self {
        Self {
            retriever: KnowledgeRetriever::new(),
            tracker: TrainingTracker::new(),
            llm_client: None,
            config,
        }
    }

    /// 使用内置推理引擎创建 Agent
    pub fn new_with_builtin_engine(config: TutorAgentConfig) -> Self {
        Self {
            retriever: KnowledgeRetriever::new(),
            tracker: TrainingTracker::new(),
            llm_client: Some(Box::new(BuiltinLlmEngine)),
            config,
        }
    }

    /// 设置 LLM 客户端
    pub fn with_llm_client(mut self, client: Box<dyn LlmClient>) -> Self {
        self.llm_client = Some(client);
        self
    }

    /// 获取培训记录追踪器引用
    pub fn tracker(&self) -> &TrainingTracker {
        &self.tracker
    }

    /// 获取培训记录追踪器可变引用
    pub fn tracker_mut(&mut self) -> &mut TrainingTracker {
        &mut self.tracker
    }

    /// 获取知识检索器引用
    pub fn retriever(&self) -> &KnowledgeRetriever {
        &self.retriever
    }

    /// 处理用户查询（同步入口）
    ///
    /// 主流程：
    /// 1. 验证查询有效性
    /// 2. 从知识库检索相关法规
    /// 3. (可选) 使用 LLM 推理生成回答
    /// 4. 构建带引用的回答
    /// 5. 记录培训活动
    pub fn answer(&mut self, query: &TutorQuery) -> Result<TutorResponse, TutorAgentError> {
        let start = Instant::now();

        // Step 1: 验证查询
        self.validate_query(query)?;

        // Step 2: 知识检索
        let relevant_regulations = self.retrieve_knowledge(query);

        if relevant_regulations.is_empty() {
            return Err(TutorAgentError::NoRelevantRegulations);
        }

        // Step 3: 生成回答
        let (answer, citations, knowledge_ids, is_grounded, confidence) =
            self.generate_answer(query, &relevant_regulations);

        let processing_time_ms = start.elapsed().as_millis() as u64;

        let response = TutorResponse {
            query_id: query.query_id.clone(),
            answer,
            citations,
            knowledge_ids,
            relevant_regulations: relevant_regulations.clone(),
            is_grounded,
            model: if self.config.enable_llm {
                Some(self.config.model.clone())
            } else {
                None
            },
            confidence,
            processing_time_ms,
            timestamp: Utc::now(),
        };

        // Step 4: 记录培训活动
        self.tracker.record_query(query, &response);

        Ok(response)
    }

    /// 验证查询有效性
    fn validate_query(&self, query: &TutorQuery) -> Result<(), TutorAgentError> {
        let trimmed = query.question.trim();
        if trimmed.is_empty() {
            return Err(TutorAgentError::EmptyQuery);
        }
        if trimmed.len() < self.config.min_query_length {
            return Err(TutorAgentError::QueryTooShort);
        }
        Ok(())
    }

    /// 从知识库检索相关法规
    fn retrieve_knowledge(&self, query: &TutorQuery) -> Vec<Regulation> {
        let search_results = if let Some(ref category) = query.category_filter {
            self.retriever
                .search_with_category(&query.question, category, self.config.top_k)
        } else {
            self.retriever.search(&query.question, self.config.top_k)
        };

        search_results.into_iter().cloned().collect()
    }

    /// 生成回答
    ///
    /// 如果配置了 LLM 客户端，使用 LLM 生成自然语言回答；
    /// 否则基于知识检索结果构建结构化回答。
    fn generate_answer(
        &self,
        query: &TutorQuery,
        regulations: &[Regulation],
    ) -> (String, Vec<String>, Vec<String>, bool, f64) {
        let citations: Vec<String> = regulations
            .iter()
            .map(|r| format!("{} — {}", r.source, r.title))
            .collect();

        let knowledge_ids: Vec<String> = regulations.iter().map(|r| r.id.clone()).collect();

        let (answer, is_grounded) = if self.config.enable_llm {
            match &self.llm_client {
                Some(client) => {
                    let prompt = build_prompt(query, regulations);
                    match client.chat(&prompt, regulations) {
                        Ok(response) => {
                            // LLM 回答 + 引用
                            let full_answer = if !citations.is_empty() {
                                format!(
                                    "{}\n\n参考来源：\n{}",
                                    response,
                                    citations
                                        .iter()
                                        .enumerate()
                                        .map(|(i, c)| format!("{}. {}", i + 1, c))
                                        .collect::<Vec<_>>()
                                        .join("\n")
                                )
                            } else {
                                response
                            };
                            (full_answer, true)
                        }
                        Err(e) => {
                            // LLM 失败，回退到知识检索模式
                            let fallback = build_retrieval_answer(regulations, &citations);
                            (format!("{} (注: LLM 推理失败 — {})", fallback, e), false)
                        }
                    }
                }
                None => {
                    // 无 LLM 客户端，使用纯检索模式
                    (build_retrieval_answer(regulations, &citations), false)
                }
            }
        } else {
            // LLM 被禁用，使用纯检索模式
            (build_retrieval_answer(regulations, &citations), false)
        };

        // 置信度 = grounded 回答的匹配程度
        let confidence = if is_grounded {
            (regulations.len() as f64 / self.config.top_k as f64).min(1.0)
        } else {
            0.5
        };

        (answer, citations, knowledge_ids, is_grounded, confidence)
    }

    /// 创建一个培训课程（会话）
    pub fn create_training_session(&mut self, trainee_id: &str, topic: &str) -> TrainingRecord {
        self.tracker.create_record(trainee_id, topic)
    }

    /// 获取学员的培训记录列表
    pub fn get_trainee_records(&self, trainee_id: &str) -> Vec<&TrainingRecord> {
        self.tracker.get_trainee_records(trainee_id)
    }
}

/// 构建 LLM prompt
fn build_prompt(query: &TutorQuery, regulations: &[Regulation]) -> String {
    let reg_context: String = regulations
        .iter()
        .enumerate()
        .map(|(i, r)| {
            format!(
                "[{i}] {title} ({source})\n{rcontent}\n",
                i = i + 1,
                title = r.title,
                source = r.source,
                rcontent = r.content
            )
        })
        .collect();

    format!(
        r#"你是 GMP 合规培训助手。请根据以下 GMP 法规知识回答员工的问题。

## 相关法规知识
{reg_context}

## 学员问题
{question}

## 回答要求
1. 用中文清晰回答学员的问题
2. 回答必须基于上述法规知识，不要编造信息
3. 如果问题超出法规范围，请说明"根据现有知识库无法回答"
4. 引用法规时使用 [法规编号] 格式
5. 如适用，提供实际生产中的示例

## 语言
{language_instruction}"#,
        question = query.question,
        language_instruction = match query.language.as_str() {
            "en" => "Please respond in English.",
            _ => "请用中文回答。",
        },
    )
}

/// 构建纯检索模式的回答（无需 LLM）
fn build_retrieval_answer(regulations: &[Regulation], citations: &[String]) -> String {
    if regulations.is_empty() {
        return "抱歉，未找到相关的 GMP 法规信息。请尝试使用不同的关键词提问。".to_string();
    }

    let mut parts = Vec::new();

    // 回答概要
    parts.push("根据 GMP 法规知识库，为您找到以下相关信息：".to_string());

    // 列出每条法规的概要
    for (i, reg) in regulations.iter().enumerate() {
        let chars: Vec<char> = reg.content.chars().collect();
        let content_preview: String = if chars.len() > 100 {
            chars[..100].iter().collect::<String>() + "..."
        } else {
            reg.content.clone()
        };
        parts.push(format!(
            "\n{}.\n来源：{} ({}）\n内容：{}\n关键词：{}",
            i + 1,
            reg.title,
            reg.source,
            content_preview,
            reg.keywords.join("、")
        ));
    }

    // 引用来源
    parts.push("\n参考来源：".to_string());
    for (i, citation) in citations.iter().enumerate() {
        parts.push(format!("{}. {}", i + 1, citation));
    }

    parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Mock LLM client for testing ──

    struct MockLlmClient {
        response: String,
    }

    impl MockLlmClient {
        fn new(response: &str) -> Self {
            Self {
                response: response.to_string(),
            }
        }
    }

    impl LlmClient for MockLlmClient {
        fn chat(&self, _prompt: &str, _regulations: &[Regulation]) -> Result<String, String> {
            Ok(self.response.clone())
        }
    }

    struct FailingMockLlmClient;

    impl LlmClient for FailingMockLlmClient {
        fn chat(&self, _prompt: &str, _regulations: &[Regulation]) -> Result<String, String> {
            Err("LLM service unavailable".to_string())
        }
    }

    // ── TutorAgentError tests ──

    #[test]
    fn test_tutor_agent_error_display() {
        assert_eq!(TutorAgentError::EmptyQuery.to_string(), "查询内容不能为空");
        assert_eq!(
            TutorAgentError::QueryTooShort.to_string(),
            "查询内容过短，请输入更详细的问题"
        );
        assert_eq!(
            TutorAgentError::NoRelevantRegulations.to_string(),
            "未找到与查询相关的 GMP 法规"
        );
        assert_eq!(TutorAgentError::LlmFailure.to_string(), "LLM 调用失败");
    }

    // ── TutorAgent creation tests ──

    #[test]
    fn test_tutor_agent_creation() {
        let agent = TutorAgent::new(TutorAgentConfig::default());
        assert!(agent.llm_client.is_none());
        assert_eq!(agent.config.enable_llm, true);
    }

    #[test]
    fn test_tutor_agent_with_builtin_engine() {
        let agent = TutorAgent::new_with_builtin_engine(TutorAgentConfig::default());
        assert!(agent.llm_client.is_some());
    }

    #[test]
    fn test_tutor_agent_with_llm_client() {
        let client = Box::new(MockLlmClient::new("测试回答"));
        let agent = TutorAgent::new(TutorAgentConfig::default()).with_llm_client(client);
        assert!(agent.llm_client.is_some());
    }

    // ── Query validation tests ──

    #[test]
    fn test_validate_empty_query() {
        let agent = TutorAgent::new(TutorAgentConfig::default());
        let query = TutorQuery::new("user-001", "");
        assert_eq!(
            agent.validate_query(&query),
            Err(TutorAgentError::EmptyQuery)
        );
    }

    #[test]
    fn test_validate_whitespace_query() {
        let agent = TutorAgent::new(TutorAgentConfig::default());
        let query = TutorQuery::new("user-001", "   ");
        assert_eq!(
            agent.validate_query(&query),
            Err(TutorAgentError::EmptyQuery)
        );
    }

    #[test]
    fn test_validate_short_query() {
        let agent = TutorAgent::new(TutorAgentConfig::default());
        let query = TutorQuery::new("user-001", "x");
        assert_eq!(
            agent.validate_query(&query),
            Err(TutorAgentError::QueryTooShort)
        );
    }

    #[test]
    fn test_validate_valid_query() {
        let agent = TutorAgent::new(TutorAgentConfig::default());
        let query = TutorQuery::new("user-001", "生产过程控制要求");
        assert!(agent.validate_query(&query).is_ok());
    }

    // ── Knowledge retrieval tests ──

    #[test]
    fn test_retrieve_knowledge_found() {
        let agent = TutorAgent::new(TutorAgentConfig::default());
        let query = TutorQuery::new("user-001", "计算机化系统验证");
        let results = agent.retrieve_knowledge(&query);
        assert!(!results.is_empty(), "应找到相关法规");
    }

    #[test]
    fn test_retrieve_knowledge_with_category() {
        let agent = TutorAgent::new(TutorAgentConfig::default());
        let query = TutorQuery::new("user-001", "验证").with_category("验证");
        let results = agent.retrieve_knowledge(&query);
        assert!(!results.is_empty());
        for reg in &results {
            assert_eq!(reg.category, "验证", "所有结果应为验证类");
        }
    }

    #[test]
    fn test_retrieve_knowledge_no_match() {
        let agent = TutorAgent::new(TutorAgentConfig::default());
        let query = TutorQuery::new("user-001", "zzz_nonexistent_term");
        let results = agent.retrieve_knowledge(&query);
        assert!(results.is_empty());
    }

    // ── Answer tests (builtin engine) ──

    #[test]
    fn test_answer_with_builtin_engine() {
        let mut agent = TutorAgent::new_with_builtin_engine(TutorAgentConfig::default());
        let query = TutorQuery::new("user-001", "生产过程控制要求有哪些？");
        let result = agent.answer(&query);
        assert!(result.is_ok(), "应成功生成回答: {:?}", result.err());
        let response = result.unwrap();
        assert!(!response.answer.is_empty(), "回答不应为空");
        assert!(response.is_grounded, "基于知识库的回答应标记为 grounded");
        assert!(!response.citations.is_empty(), "应包含引用来源");
        assert!(response.confidence > 0.0);
    }

    #[test]
    fn test_answer_with_llm_client() {
        let client = Box::new(MockLlmClient::new(
            "生产过程需要遵循21 CFR 211.110等相关法规的要求。[cfr-211-110]",
        ));
        let mut agent = TutorAgent::new(TutorAgentConfig::default()).with_llm_client(client);
        let query = TutorQuery::new("user-001", "生产过程控制要求？");
        let result = agent.answer(&query);
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.answer.contains("cfr-211-110"));
        assert!(response.is_grounded);
    }

    #[test]
    fn test_answer_with_failing_llm() {
        let client = Box::new(FailingMockLlmClient);
        let mut agent = TutorAgent::new(TutorAgentConfig::default()).with_llm_client(client);
        let query = TutorQuery::new("user-001", "生产控制");
        let result = agent.answer(&query);
        assert!(result.is_ok(), "LLM 失败应回退，不应导致整体失败");
        let response = result.unwrap();
        assert!(!response.answer.is_empty());
    }

    #[test]
    fn test_answer_empty_query() {
        let mut agent = TutorAgent::new_with_builtin_engine(TutorAgentConfig::default());
        let query = TutorQuery::new("user-001", "");
        let result = agent.answer(&query);
        assert_eq!(result, Err(TutorAgentError::EmptyQuery));
    }

    #[test]
    fn test_answer_no_match() {
        let mut agent = TutorAgent::new_with_builtin_engine(TutorAgentConfig::default());
        let query = TutorQuery::new("user-001", "zzz_nonexistent_gmp_term");
        let result = agent.answer(&query);
        assert_eq!(result, Err(TutorAgentError::NoRelevantRegulations));
    }

    #[test]
    fn test_answer_with_disabled_llm() {
        let mut config = TutorAgentConfig::default();
        config.enable_llm = false;
        let mut agent = TutorAgent::new(config);
        let query = TutorQuery::new("user-001", "数据完整性要求");
        let result = agent.answer(&query);
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.is_grounded, "LLM 禁用时不标记 grounded");
        assert_eq!(response.model, None);
    }

    // ── Training session tests ──

    #[test]
    fn test_create_training_session() {
        let mut agent = TutorAgent::new_with_builtin_engine(TutorAgentConfig::default());
        let record = agent.create_training_session("trainee-001", "GMP 基础知识");
        assert_eq!(record.trainee_id, "trainee-001");
        assert_eq!(record.topic, "GMP 基础知识");
    }

    #[test]
    fn test_get_trainee_records() {
        let mut agent = TutorAgent::new_with_builtin_engine(TutorAgentConfig::default());
        agent.create_training_session("trainee-001", "GMP 基础知识");
        agent.create_training_session("trainee-001", "验证专题");
        let records = agent.get_trainee_records("trainee-001");
        assert_eq!(records.len(), 2);
    }

    #[test]
    fn test_tracker_integration() {
        let mut agent = TutorAgent::new_with_builtin_engine(TutorAgentConfig::default());
        agent.create_training_session("trainee-001", "GMP 基础知识");

        // Answer a query, which should auto-record
        let query = TutorQuery::new("trainee-001", "生产过程控制");
        let _ = agent.answer(&query);

        let records = agent.get_trainee_records("trainee-001");
        assert!(!records.is_empty());
    }

    // ── BuiltinLlmEngine tests ──

    #[test]
    fn test_builtin_llm_engine_chat_success() {
        let engine = BuiltinLlmEngine;
        let regulations = vec![Regulation::new(
            "test-001",
            "测试法规",
            "Test Source",
            "测试内容",
            "测试",
            vec!["测试"],
            "1.0",
        )];
        let result = engine.chat("测试问题", &regulations);
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.contains("测试法规"));
    }

    #[test]
    fn test_builtin_llm_engine_chat_empty() {
        let engine = BuiltinLlmEngine;
        let result = engine.chat("测试问题", &[]);
        assert!(result.is_err());
    }

    // ── Build prompt test ──

    #[test]
    fn test_build_prompt() {
        let query = TutorQuery::new("user-001", "生产控制要求？");
        let regulations = vec![Regulation::new(
            "cfr-211-110",
            "生产和过程控制",
            "21 CFR Part 211",
            "生产控制要求内容",
            "生产",
            vec!["生产"],
            "2024",
        )];
        let prompt = build_prompt(&query, &regulations);
        assert!(prompt.contains("生产控制要求？"));
        assert!(prompt.contains("生产和过程控制"));
        assert!(prompt.contains("21 CFR Part 211"));
    }

    // ── Build retrieval answer test ──

    #[test]
    fn test_build_retrieval_answer_with_results() {
        let regulations = vec![Regulation::new(
            "cfr-211-110",
            "生产和过程控制",
            "21 CFR Part 211",
            "生产控制要求内容",
            "生产",
            vec!["生产"],
            "2024",
        )];
        let citations = vec!["21 CFR Part 211 — 生产和过程控制".to_string()];
        let answer = build_retrieval_answer(&regulations, &citations);
        assert!(answer.contains("生产和过程控制"));
        assert!(answer.contains("21 CFR Part 211"));
    }

    #[test]
    fn test_build_retrieval_answer_empty() {
        let answer = build_retrieval_answer(&[], &[]);
        assert!(answer.contains("未找到"));
    }

    // ── Extract key summary test ──

    #[test]
    fn test_extract_key_summary() {
        let reg = Regulation::new(
            "test-001",
            "测试法规",
            "Test Source",
            "这是一条测试法规内容，用于验证提取摘要的功能是否正常工作。",
            "测试",
            vec!["测试"],
            "1.0",
        );
        let summary = extract_key_summary(&reg);
        assert!(summary.contains("测试法规"));
        assert!(summary.contains("Test Source"));
    }

    // ── Config default test ──

    #[test]
    fn test_tutor_agent_config_default() {
        let config = TutorAgentConfig::default();
        assert_eq!(config.model, "builtin");
        assert_eq!(config.top_k, 5);
        assert_eq!(config.min_query_length, 2);
        assert!(config.enable_llm);
    }
}
