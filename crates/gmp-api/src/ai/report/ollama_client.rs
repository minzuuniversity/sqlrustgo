//! 生产级 Ollama LLM 客户端 — 连接 192.168.0.252:11434
//!
//! 使用 qwen3:8b-gmp 模型

use async_trait::async_trait;
use reqwest::Client;
use std::time::Duration;

use crate::ai::report::translator::TranslateOllamaClient;
use sqlrustgo_compliance_engine::judge::agent::{OllamaError, OllamaResponse};

/// 连接 192.168.0.252 Ollama 的生产客户端
pub struct OllamaTranslateClient {
    client: Client,
    base_url: String,
    model: String,
}

impl OllamaTranslateClient {
    pub fn new(model: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("reqwest Client");

        Self {
            client,
            base_url: "http://192.168.0.252:11434".to_string(),
            model: model.to_string(),
        }
    }

    pub fn with_url(base_url: &str, model: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("reqwest Client");

        Self {
            client,
            base_url: base_url.to_string(),
            model: model.to_string(),
        }
    }
}

#[async_trait]
impl TranslateOllamaClient for OllamaTranslateClient {
    async fn chat(&self, prompt: &str, _model: &str) -> Result<OllamaResponse, OllamaError> {
        let request_body = serde_json::json!({
            "model": self.model,
            "prompt": prompt,
            "stream": false,
            "options": {
                "temperature": 0.3,
                "num_predict": 2048
            }
        });

        let response = self
            .client
            .post(format!("{}/api/generate", self.base_url))
            .json(&request_body)
            .send()
            .await
            .map_err(|e| OllamaError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(OllamaError::Network(format!(
                "Ollama returned status {}",
                response.status()
            )));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| OllamaError::Network(e.to_string()))?;

        let content = json["response"].as_str().unwrap_or("").to_string();

        Ok(OllamaResponse {
            content,
            logprob: None,
            model: self.model.clone(),
        })
    }
}

#[tokio::test]
#[ignore] // 需要 Ollama 运行在 192.168.0.252:11434
async fn test_real_ollama_fda_translation() {
    use crate::ai::report::{
        api::{generate_cross_lang_report, CrossLangReportRequest},
        translator::{CrossLangTranslator, TranslateConfig},
    };

    let client = OllamaTranslateClient::new("qwen3:8b-gmp");
    let translator = CrossLangTranslator::new(Box::new(client), TranslateConfig::default());

    let request = CrossLangReportRequest {
        source_text: "批次放行报告：生产已完成，原料检验合格，质量部门已批准放行。".to_string(),
        standard: "fda_21cfr11".to_string(),
        batch_id: Some("LOT-TEST-001".to_string()),
        local_only: false, // 使用真实 Ollama
    };

    let response = generate_cross_lang_report(&request, Some(&translator)).await;

    assert!(
        response.success,
        "Ollama translation failed: {:?}",
        response.error
    );
    let report = response.report.expect("Report must exist");
    assert!(!report.content.is_empty());
    // 验证包含 GMP 术语
    let en_upper = report.content.to_uppercase();
    assert!(en_upper.contains("BATCH") || en_upper.contains("QUALITY"));

    println!("FDA Translation:\n{}", report.content);
}

#[tokio::test]
#[ignore]
async fn test_real_ollama_ema_translation() {
    use crate::ai::report::{
        api::{generate_cross_lang_report, CrossLangReportRequest},
        translator::{CrossLangTranslator, TranslateConfig},
    };

    let client = OllamaTranslateClient::new("qwen3:8b-gmp");
    let translator = CrossLangTranslator::new(Box::new(client), TranslateConfig::default());

    let request = CrossLangReportRequest {
        source_text: "计算机化系统验证报告：审计跟踪已审查，电子签名已签署，数据完整性确认。"
            .to_string(),
        standard: "ema_annex11".to_string(),
        batch_id: Some("CSV-TEST-001".to_string()),
        local_only: false,
    };

    let response = generate_cross_lang_report(&request, Some(&translator)).await;

    assert!(
        response.success,
        "Ollama EMA translation failed: {:?}",
        response.error
    );
    let report = response.report.expect("EMA report must exist");
    assert!(!report.content.is_empty());

    println!("EMA Translation:\n{}", report.content);
}
