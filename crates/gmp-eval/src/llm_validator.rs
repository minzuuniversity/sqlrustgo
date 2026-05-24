//! LLM-assisted validation for 408-scenario evaluation
//!
//! Replaces keyword-only matching with semantic validation:
//! Given a query and retrieved documents, the LLM determines
//! whether each document truly answers the query.

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

// ============ Data Structures ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmValidationRequest {
    pub query: String,
    pub documents: Vec<DocSnippet>,
    #[serde(default)]
    pub expected_doc_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocSnippet {
    pub doc_id: String,
    pub title: String,
    pub content: String, // truncated for prompt efficiency
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmValidationResult {
    pub verdict: ValidationVerdict,
    pub relevant_doc_ids: Vec<String>,
    pub reason: String,
    #[serde(default)]
    pub missing_info: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum ValidationVerdict {
    PASS,
    PARTIAL,
    FAIL,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OllamaGenerateResponse {
    response: String,
}

// ============ LLM Validator ============

#[derive(Clone)]
pub struct LlmValidator {
    ollama_url: String,
    model: String,
    client: Client,
}

impl LlmValidator {
    pub fn new(ollama_url: String, model: String) -> Self {
        Self {
            ollama_url,
            model,
            client: Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .expect("reqwest Client::new should not fail"),
        }
    }

    /// Validate whether retrieved documents answer the query
    pub async fn validate(&self, req: &LlmValidationRequest) -> Result<LlmValidationResult> {
        let prompt = self.build_prompt(req);
        let resp_text = self.call_llm(&prompt).await?;

        // Parse JSON from LLM response — strip markdown code blocks if present
        let cleaned = resp_text
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        let result: LlmValidationResult =
            serde_json::from_str(cleaned).context(format!("Failed to parse LLM response: {}", cleaned))?;

        Ok(result)
    }

    /// Build the LLM prompt for document relevance evaluation
    fn build_prompt(&self, req: &LlmValidationRequest) -> String {
        let docs_text = req
            .documents
            .iter()
            .take(10) // Limit to top 10 docs to stay within context
            .enumerate()
            .map(|(i, d)| {
                let snippet = &d.content[..d.content.len().min(800)];
                format!(
                    "[文档 {}] ID: {}\n标题: {}\n内容摘要:\n{}\n",
                    i + 1,
                    d.doc_id,
                    d.title,
                    snippet
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n---\n\n");

        format!(
            r#"## 查询
{}

## 候选文档（按相关性排序）
{}

## 判断要求
你是一个GMP药品生产质量管理系统文档专家。请判断:
1. 上述候选文档中，哪些文档能够回答上述查询？
2. 如果有，指出最相关的1-3篇文档及其原因。
3. 如果没有，说明缺失的核心信息是什么。

## 输出格式（严格JSON）:
{{
  "verdict": "PASS" | "PARTIAL" | "FAIL",
  "relevant_doc_ids": ["doc_id1", "doc_id2"],
  "reason": "判断原因的简要说明",
  "missing_info": "缺失的核心信息（可选）"
}}

注意:
- verdict="PASS" 表示至少1篇文档能完整回答查询
- verdict="PARTIAL" 表示有部分相关信息但不足以完整回答
- verdict="FAIL" 表示没有任何文档能回答该查询
- relevant_doc_ids 只填写你判断为相关的文档ID"#,
            req.query,
            docs_text
        )
    }

    /// Call Ollama /api/generate endpoint
    async fn call_llm(&self, prompt: &str) -> Result<String> {
        let resp = self
            .client
            .post(format!("{}/api/generate", self.ollama_url))
            .json(&serde_json::json!({
                "model": self.model,
                "prompt": prompt,
                "stream": false,
                "options": {
                    "temperature": 0.1, // Low temperature for deterministic output
                    "num_predict": 512  // Limit response length
                }
            }))
            .send()
            .await
            .context("Ollama request failed")?;

        if !resp.status().is_success() {
            anyhow::bail!("Ollama API error: {}", resp.status());
        }

        let ollama_resp: OllamaGenerateResponse =
            resp.json().await.context("Failed to parse Ollama response")?;

        Ok(ollama_resp.response)
    }

    /// Check if Ollama service is available
    pub async fn is_available(&self) -> bool {
        match self
            .client
            .get(format!("{}/api/tags", self.ollama_url))
            .timeout(Duration::from_secs(3))
            .send()
            .await
        {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }
}

// ============ Integration Helpers ============

use super::TestScenario;
use gmp_storage::SearchResult;

/// Integrate LLM validation into evaluation result
pub async fn evaluate_with_llm(
    scenario: &TestScenario,
    search_results: &[SearchResult],
    validator: &LlmValidator,
    top_k: usize,
) -> LlmEvalResult {
    // Step 1: Original keyword matching (fallback)
    let keyword_pass = keyword_match(scenario, search_results);

    // Step 2: LLM semantic validation
    let llm_result = if validator.is_available().await {
        let docs: Vec<DocSnippet> = search_results
            .iter()
            .take(10)
            .map(|r| DocSnippet {
                doc_id: r.doc_id.clone(),
                title: r.title.clone(),
                content: r.content.clone(),
            })
            .collect();

        let req = LlmValidationRequest {
            query: scenario.query.clone(),
            documents: docs,
            expected_doc_ids: scenario.expected_doc_ids.clone(),
        };

        validator.validate(&req).await.ok()
    } else {
        None
    };

    // Step 3: Combined verdict
    let verdict = match (&keyword_pass, &llm_result) {
        (true, _) => EvalVerdict::PASS,
        (false, Some(r)) if r.verdict == ValidationVerdict::PASS => EvalVerdict::PASS,
        (false, Some(r)) if r.verdict == ValidationVerdict::PARTIAL => EvalVerdict::PARTIAL,
        _ => EvalVerdict::FAIL,
    };

    LlmEvalResult {
        keyword_pass,
        llm_result,
        verdict,
    }
}

/// Original keyword-based matching
fn keyword_match(scenario: &TestScenario, results: &[SearchResult]) -> bool {
    if scenario.expected_doc_ids.is_empty() || results.is_empty() {
        return false;
    }

    let hit_count = scenario
        .expected_doc_ids
        .iter()
        .filter(|kw| {
            let kw_lower = kw.to_lowercase();
            results.iter().any(|r| {
                r.doc_id.to_lowercase().contains(&kw_lower)
                    || r.title.to_lowercase().contains(&kw_lower)
                    || r.content.to_lowercase().contains(&kw_lower)
            })
        })
        .count();

    hit_count > 0
}

// ============ Types for main.rs integration ============

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum EvalVerdict {
    PASS,
    PARTIAL,
    FAIL,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmEvalResult {
    pub keyword_pass: bool,
    pub llm_result: Option<LlmValidationResult>,
    pub verdict: EvalVerdict,
}
