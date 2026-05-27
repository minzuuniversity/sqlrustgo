//! GMP Report API Server — 独立 HTTP 服务
//!
//! 端点:
//!   POST /api/v1/gmp/report/translate       (Ollama LLM 翻译模式)
//!   POST /api/v1/gmp/report/translate/local (本地术语预处理模式)
//!   GET  /health                           (健康检查)

use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use std::sync::Arc;

use sqlrustgo_gmp_api::ai::report::{
    api::{generate_cross_lang_report, CrossLangReportRequest},
    ollama_client::OllamaTranslateClient,
    translator::{CrossLangTranslator, TranslateConfig},
};

/// GMP Report Handler — 使用 Ollama LLM 翻译
async fn translate_gmp_report(request: web::Json<CrossLangReportRequest>) -> impl Responder {
    let client = OllamaTranslateClient::new("qwen3:8b-gmp");
    let translator = CrossLangTranslator::new(Box::new(client), TranslateConfig::default());

    let response = generate_cross_lang_report(&request, Some(&translator)).await;

    if response.success {
        HttpResponse::Ok().json(&response)
    } else {
        HttpResponse::BadRequest().json(&response)
    }
}

/// GMP Report Handler — 本地模式（无需 LLM）
async fn translate_gmp_report_local(request: web::Json<CrossLangReportRequest>) -> impl Responder {
    let local_request = CrossLangReportRequest {
        source_text: request.source_text.clone(),
        standard: request.standard.clone(),
        batch_id: request.batch_id.clone(),
        local_only: true,
    };

    let response = generate_cross_lang_report(&local_request, None).await;

    if response.success {
        HttpResponse::Ok().json(&response)
    } else {
        HttpResponse::BadRequest().json(&response)
    }
}

/// Health check
async fn health() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "service": "gmp-report-api",
        "version": "0.1.0"
    }))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║        GMP Report API Server — #1411                    ║");
    println!("╠══════════════════════════════════════════════════════════╣");
    println!("║  Endpoints:                                             ║");
    println!("║    POST /api/v1/gmp/report/translate      (LLM 模式)   ║");
    println!("║    POST /api/v1/gmp/report/translate/local (本地模式)   ║");
    println!("║    GET  /health                                  ║");
    println!("╠══════════════════════════════════════════════════════════╣");
    println!("║  Ollama: 192.168.0.252:11434 (qwen3:8b-gmp)          ║");
    println!("╚══════════════════════════════════════════════════════════╝");

    HttpServer::new(|| {
        App::new().route("/health", web::get().to(health)).service(
            web::scope("/api/v1/gmp")
                .route("/report/translate", web::post().to(translate_gmp_report))
                .route(
                    "/report/translate/local",
                    web::post().to(translate_gmp_report_local),
                ),
        )
    })
    .bind("0.0.0.0:8081")?
    .run()
    .await
}
