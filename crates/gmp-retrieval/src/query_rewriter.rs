//! Query Rewriter for GMP Retrieval
//!
//! Converts natural language queries into structured retrieval queries.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::time::timeout;

/// Query rewrite timeout (500ms as per requirements)
const REWRITE_TIMEOUT_MS: u64 = 500;

/// Query intent types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum QueryIntent {
    /// Search for documents
    #[default]
    Search,
    /// Filter documents by criteria
    Filter,
    /// Compare documents
    Comparison,
    /// Aggregate/summarize documents
    Aggregation,
}


/// Filter condition for structured queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterCondition {
    /// Field name to filter on
    pub field: String,
    /// Operator (eq, ne, gt, gte, lt, lte, contains)
    pub op: String,
    /// Value to compare against
    pub value: serde_json::Value,
}

/// Rewritten query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewrittenQuery {
    /// Original query text (filled by from_original or parse_response)
    #[serde(default)]
    pub original: String,
    /// Extracted keywords
    pub keywords: Vec<String>,
    /// Filter conditions
    pub filters: Vec<FilterCondition>,
    /// Detected intent
    pub intent: QueryIntent,
}

impl RewrittenQuery {
    /// Create a rewritten query from the original (fallback when LLM fails)
    pub fn from_original(original: &str) -> Self {
        Self {
            original: original.to_string(),
            keywords: vec![original.to_string()],
            filters: vec![],
            intent: QueryIntent::Search,
        }
    }
}

/// Errors that can occur during query rewriting
#[derive(Error, Debug)]
pub enum RewriterError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Timeout: rewrite exceeded {0}ms")]
    Timeout(u64),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("LLM unavailable: {0}")]
    Unavailable(String),
}

/// Result type for rewriter operations
pub type RewriterResult<T> = Result<T, RewriterError>;

/// Ollama chat client trait for query rewriting
#[async_trait]
pub trait OllamaChatClient: Send + Sync {
    /// Send a chat request
    async fn chat(&self, prompt: &str, model: &str) -> Result<String, RewriterError>;

    /// Check if the client is available
    fn is_available(&self) -> bool;
}

/// Query Rewriter Agent
pub struct QueryRewriter {
    client: Box<dyn OllamaChatClient>,
    model: String,
    prompt_template: String,
}

impl QueryRewriter {
    /// Create a new QueryRewriter
    ///
    /// # Arguments
    /// * `client` - Ollama chat client
    /// * `model` - Model name for chat (e.g., "qwen2.5:14b")
    pub fn new(client: Box<dyn OllamaChatClient>, model: &str) -> Self {
        Self {
            client,
            model: model.to_string(),
            prompt_template: Self::default_prompt_template(),
        }
    }

    /// Default prompt template for Chinese query rewriting
    fn default_prompt_template() -> String {
        r#"你是一个 GMP 文档检索查询改写助手。将用户的自然语言查询转换为结构化的检索查询。

## 任务
分析用户输入，提取：
1. 关键词（用于全文搜索）
2. 过滤条件（精确匹配）
3. 查询意图

## 支持的操作符
- eq: 等于
- ne: 不等于
- gt: 大于
- gte: 大于等于
- lt: 小于
- lte: 小于等于
- contains: 包含

## 支持的意图类型
- search: 搜索文档
- filter: 按条件过滤
- comparison: 比较文档
- aggregation: 聚合/统计

## 输出格式
请输出 JSON 格式：
```json
{
  "keywords": ["关键词1", "关键词2"],
  "filters": [
    {"field": "字段名", "op": "操作符", "value": "值"}
  ],
  "intent": "search|filter|comparison|aggregation"
}
```

## 示例
输入：找一下最近一周批准的偏差报告
输出：
{
  "keywords": ["偏差", "报告", "批准"],
  "filters": [
    {"field": "status", "op": "eq", "value": "approved"},
    {"field": "date", "op": "gte", "value": "2026-05-19"}
  ],
  "intent": "search"
}

## 注意
- 只输出 JSON，不要有其他内容
- 如果无法解析，保持简单关键词搜索
- 日期相关查询请转换为相对日期或绝对日期"#
            .to_string()
    }

    /// Build prompt from query
    fn build_prompt(&self, query: &str) -> String {
        format!("{}\n\n## 用户输入\n{}", self.prompt_template, query)
    }

    /// Parse JSON response with fallback
    fn parse_response(&self, text: &str, original: &str) -> RewrittenQuery {
        // Try to extract JSON from response
        if let Some(ref json_str) = Self::extract_json(text) {
            if let Ok(query) = serde_json::from_str::<RewrittenQuery>(json_str) {
                return query;
            }
        }

        // Fallback: return original query as keyword
        RewrittenQuery::from_original(original)
    }

    /// Extract JSON from text (handle markdown code blocks)
    fn extract_json(text: &str) -> Option<String> {
        // Try direct parse first
        let text_trimmed = text.trim();
        if let Ok(query) = serde_json::from_str::<RewrittenQuery>(text_trimmed) {
            return serde_json::to_string(&query).ok();
        }

        // Try to find JSON in code block - search for ```json pattern
        if let Some(start) = text_trimmed.find("```json") {
            let after_marker = &text_trimmed[start + 7..];
            if let Some(end) = after_marker.find("```") {
                let json_str = &after_marker[..end].trim();
                if let Ok(query) = serde_json::from_str::<RewrittenQuery>(json_str) {
                    return serde_json::to_string(&query).ok();
                }
            }
        }

        // Try to find JSON in generic code block
        if let Some(start) = text_trimmed.find("```") {
            // Skip the first ``` and find the next one
            let after_first = &text_trimmed[start + 3..];
            if let Some(end) = after_first.find("```") {
                let json_str = &after_first[..end].trim();
                if let Ok(query) = serde_json::from_str::<RewrittenQuery>(json_str) {
                    return serde_json::to_string(&query).ok();
                }
            }
        }

        None
    }

    /// Rewrite a query with timeout
    pub async fn rewrite(&self, query: &str) -> RewrittenQuery {
        let start = Instant::now();

        let result = timeout(
            Duration::from_millis(REWRITE_TIMEOUT_MS),
            self.execute_rewrite(query),
        )
        .await;

        let rewritten = match result {
            Ok(Ok(q)) => q,
            Ok(Err(e)) => {
                eprintln!("Rewriter error: {}", e);
                RewrittenQuery::from_original(query)
            }
            Err(_) => {
                eprintln!("Rewriter timeout for query: {}", query);
                RewrittenQuery::from_original(query)
            }
        };

        let elapsed = start.elapsed().as_millis() as u64;
        if elapsed > REWRITE_TIMEOUT_MS {
            eprintln!(
                "Warning: rewrite took {}ms (target: {}ms)",
                elapsed, REWRITE_TIMEOUT_MS
            );
        }

        rewritten
    }

    /// Execute the actual rewrite
    async fn execute_rewrite(&self, query: &str) -> RewriterResult<RewrittenQuery> {
        let prompt = self.build_prompt(query);

        let response = self.client.chat(&prompt, &self.model).await?;

        Ok(self.parse_response(&response, query))
    }
}

/// Mock chat client for testing
#[cfg(test)]
pub struct MockChatClient {
    pub response: String,
    pub should_fail: bool,
}

#[cfg(test)]
impl MockChatClient {
    pub fn new(response: &str) -> Self {
        Self {
            response: response.to_string(),
            should_fail: false,
        }
    }

    pub fn with_failure(self) -> Self {
        Self {
            should_fail: true,
            ..self
        }
    }
}

#[cfg(test)]
#[async_trait]
impl OllamaChatClient for MockChatClient {
    async fn chat(&self, _prompt: &str, _model: &str) -> Result<String, RewriterError> {
        if self.should_fail {
            return Err(RewriterError::Unavailable("Mock failure".to_string()));
        }
        Ok(self.response.clone())
    }

    fn is_available(&self) -> bool {
        !self.should_fail
    }
}

/// Ollama-hosted chat client
pub struct OllamaChatClientImpl {
    base_url: String,
    model: String,
    client: Client,
}

impl OllamaChatClientImpl {
    /// Create a new OllamaChatClientImpl
    pub fn new(base_url: &str, model: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            model: model.to_string(),
            client: Client::new(),
        }
    }

    /// Check if Ollama server is available
    fn check_availability(&self) -> bool {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .ok();
        if let Some(rt) = rt {
            rt.block_on(async {
                reqwest::get(&format!("{}/api/tags", self.base_url))
                    .await
                    .is_ok()
            })
        } else {
            false
        }
    }
}

#[async_trait]
impl OllamaChatClient for OllamaChatClientImpl {
    async fn chat(&self, prompt: &str, _model: &str) -> Result<String, RewriterError> {
        use serde::Serialize;

        #[derive(Serialize)]
        struct ChatRequest<'a> {
            model: &'a str,
            prompt: &'a str,
            stream: bool,
        }

        #[derive(Deserialize)]
        struct ChatResponse {
            response: String,
        }

        let resp = self
            .client
            .post(format!("{}/api/generate", self.base_url))
            .json(&ChatRequest {
                model: &self.model,
                prompt,
                stream: false,
            })
            .send()
            .await
            .map_err(|e| RewriterError::Unavailable(e.to_string()))?;

        if resp.status() == 404 {
            return Err(RewriterError::Unavailable("Model not found".to_string()));
        }

        let result: ChatResponse = resp
            .json()
            .await
            .map_err(|e| RewriterError::InvalidResponse(e.to_string()))?;

        Ok(result.response)
    }

    fn is_available(&self) -> bool {
        self.check_availability()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_intent_default() {
        assert_eq!(QueryIntent::default(), QueryIntent::Search);
    }

    #[test]
    fn test_rewritten_query_from_original() {
        let query = RewrittenQuery::from_original("测试查询");
        assert_eq!(query.original, "测试查询");
        assert_eq!(query.keywords, vec!["测试查询"]);
        assert!(query.filters.is_empty());
        assert_eq!(query.intent, QueryIntent::Search);
    }

    #[test]
    fn test_extract_json_direct() {
        let json = r#"{"keywords":["测试"],"filters":[],"intent":"search"}"#;
        let result = QueryRewriter::extract_json(json);
        assert!(result.is_some());
    }

    #[test]
    fn test_extract_json_with_code_block() {
        let text = r#"这是一些解释文字
```json
{"keywords":["测试"],"filters":[],"intent":"search"}
```
更多文字"#;
        let result = QueryRewriter::extract_json(text);
        assert!(result.is_some());
    }

    #[test]
    fn test_extract_json_invalid() {
        let text = "这不是有效的JSON";
        let result = QueryRewriter::extract_json(text);
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_rewrite_with_mock_success() {
        let mock = MockChatClient::new(
            r#"{"keywords":["偏差","报告"],"filters":[{"field":"status","op":"eq","value":"approved"}],"intent":"search"}"#,
        );
        let rewriter = QueryRewriter::new(Box::new(mock), "qwen2.5:14b");

        let result = rewriter.rewrite("找偏差报告").await;
        assert_eq!(result.keywords, vec!["偏差", "报告"]);
        assert_eq!(result.filters.len(), 1);
        assert_eq!(result.intent, QueryIntent::Search);
    }

    #[tokio::test]
    async fn test_rewrite_fallback_on_invalid_json() {
        let mock = MockChatClient::new("这不是有效的JSON响应");
        let rewriter = QueryRewriter::new(Box::new(mock), "qwen2.5:14b");

        let result = rewriter.rewrite("测试查询").await;
        // Should fallback to original
        assert_eq!(result.original, "测试查询");
        assert_eq!(result.keywords, vec!["测试查询"]);
    }

    #[tokio::test]
    async fn test_rewrite_fallback_on_failure() {
        let mock = MockChatClient::new("response").with_failure();
        let rewriter = QueryRewriter::new(Box::new(mock), "qwen2.5:14b");

        let result = rewriter.rewrite("测试查询").await;
        // Should fallback to original
        assert_eq!(result.original, "测试查询");
    }

    #[tokio::test]
    async fn test_rewrite_timeout() {
        // Create a mock that delays response
        struct DelayedClient;
        #[async_trait]
        impl OllamaChatClient for DelayedClient {
            async fn chat(&self, _prompt: &str, _model: &str) -> Result<String, RewriterError> {
                tokio::time::sleep(Duration::from_secs(2)).await;
                Ok(r#"{"keywords":["test"],"filters":[],"intent":"search"}"#.to_string())
            }

            fn is_available(&self) -> bool {
                true
            }
        }

        let client = Box::new(DelayedClient);
        let rewriter = QueryRewriter::new(client, "qwen2.5:14b");

        let result = rewriter.rewrite("测试查询").await;
        // Should fallback to original on timeout
        assert_eq!(result.original, "测试查询");
    }

    #[test]
    fn test_filter_condition_serialization() {
        let filter = FilterCondition {
            field: "status".to_string(),
            op: "eq".to_string(),
            value: serde_json::json!("approved"),
        };
        let json = serde_json::to_string(&filter).unwrap();
        assert!(json.contains("status"));
        assert!(json.contains("eq"));
        assert!(json.contains("approved"));
    }
}
