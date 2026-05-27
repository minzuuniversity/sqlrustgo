//! GMP 跨语言合规报告 HTTP 端点 — #1411
//!
//! POST /api/v1/gmp/report/translate

use actix_web::{web, HttpResponse, Responder};
use std::sync::Arc;

use crate::sqlrustgo_gmp_api::ai::report::{
    api::{generate_cross_lang_report, CrossLangReportRequest},
    ollama_client::OllamaTranslateClient,
    translator::CrossLangTranslator,
};

/// GMP Report API Handler
pub async fn translate_gmp_report(
    request: web::Json<CrossLangReportRequest>,
) -> impl Responder {
    let ollama_client = OllamaTranslateClient::new("qwen3:8b-gmp");
    let translator = CrossLangTranslator::new(
        Box::new(ollama_client),
        crate::sqlrustgo_gmp_api::ai::report::translator::TranslateConfig::default(),
    );

    let response = generate_cross_lang_report(&request, Some(&translator)).await;

    if response.success {
        HttpResponse::Ok().json(&response)
    } else {
        HttpResponse::BadRequest().json(&response)
    }
}

/// GMP Report API Handler (本地模式，无需 LLM)
pub async fn translate_gmp_report_local(
    request: web::Json<CrossLangReportRequest>,
) -> impl Responder {
    // 本地模式：不使用 Ollama，仅术语预处理
    let local_request = CrossLangReportRequest {
        source_text: request.source_text.clone(),
        standard: request.standard.clone(),
        batch_id: request.batch_id.clone(),
        local_only: true,
        report_category: request.report_category.clone(),
    };

    let response = generate_cross_lang_report(&local_request, None).await;

    if response.success {
        HttpResponse::Ok().json(&response)
    } else {
        HttpResponse::BadRequest().json(&response)
    }
}

/// Configure GMP Report routes
pub fn configure_gmp_report_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1/gmp")
            .route("/report/translate", web::post().to(translate_gmp_report))
            .route("/report/translate/local", web::post().to(translate_gmp_report_local)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web, App};

    #[actix_web::test]
    async fn test_translate_endpoint_local() {
        let app = test::init_service(
            App::new()
                .configure(configure_gmp_report_routes),
        ).await;

        let req_json = serde_json::json!({
            "source_text": "批次放行报告：生产已完成，原料检验合格，质量部门已批准放行。",
            "standard": "fda_21cfr11",
            "batch_id": "LOT-TEST-001",
            "local_only": true
        });

        let req = test::TestRequest::post()
            .uri("/api/v1/gmp/report/translate/local")
            .set_json(&req_json)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let body = test::read_body(resp).await;
        let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(response["success"], true);
    }
}