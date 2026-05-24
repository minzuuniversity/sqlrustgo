//! GMP API Server - RESTful API for GMP Compliance Retrieval

use actix_web::{middleware, web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use tracing::info;

// ========== Request/Response Types ==========

#[derive(Debug, Deserialize)]
struct SearchRequest {
    query: String,
    #[serde(default = "default_channel")]
    #[allow(dead_code)]
    channel: String,
    #[serde(default = "default_top_k")]
    top_k: usize,
}

fn default_channel() -> String {
    "fts".to_string()
}
fn default_top_k() -> usize {
    10
}

#[derive(Debug, Serialize)]
struct SearchResponse {
    query: String,
    results: Vec<SearchResultItem>,
    total: usize,
    time_ms: f64,
}

#[derive(Debug, Serialize)]
struct SearchResultItem {
    doc_id: String,
    title: String,
    content: String,
    score: f64,
    doc_type: String,
}

#[derive(Debug, Serialize)]
struct StatsResponse {
    documents: usize,
    chunks: usize,
    ok: bool,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: String,
    version: String,
}

// ========== App State ==========

struct AppState {
    storage: gmp_storage::StorageEngine,
}

// ========== Handlers ==========

/// POST /api/v1/search
async fn search_handler(
    data: web::Data<AppState>,
    req: web::Json<SearchRequest>,
) -> impl Responder {
    let start = std::time::Instant::now();
    let results = match data.storage.search_fts(&req.query, req.top_k) {
        Ok(r) => r,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Search failed: {}", e)
            }));
        }
    };

    let items: Vec<SearchResultItem> = results
        .into_iter()
        .map(|r| SearchResultItem {
            doc_id: r.doc_id,
            title: r.title,
            content: r.content.chars().take(500).collect(),
            score: r.score,
            doc_type: r.doc_type.unwrap_or_default(),
        })
        .collect();

    let total = items.len();
    let time_ms = start.elapsed().as_secs_f64() * 1000.0;

    HttpResponse::Ok().json(SearchResponse {
        query: req.query.clone(),
        results: items,
        total,
        time_ms,
    })
}

/// GET /api/v1/stats
async fn stats_handler(data: web::Data<AppState>) -> impl Responder {
    let documents = match data.storage.get_document_count() {
        Ok(c) => c,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Stats failed: {}", e)
            }));
        }
    };
    HttpResponse::Ok().json(StatsResponse {
        documents: documents as usize,
        chunks: 0, // not available in current API
        ok: true,
    })
}

/// GET /api/v1/health
async fn health_handler() -> impl Responder {
    HttpResponse::Ok().json(HealthResponse {
        status: "ok".to_string(),
        version: "1.0.0".to_string(),
    })
}

// ========== Config ==========

#[derive(Debug, Clone, Deserialize)]
struct ApiConfig {
    #[serde(default = "default_host")]
    host: String,
    #[serde(default = "default_port")]
    port: u16,
    #[serde(default = "default_db_path")]
    db_path: String,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}
fn default_port() -> u16 {
    8080
}
fn default_db_path() -> String {
    "./data/gmp.db".to_string()
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            db_path: default_db_path(),
        }
    }
}

// ========== Main ==========

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new("info"))
        .init();

    let config = ApiConfig::default();

    info!(
        "GMP API Server starting on {}:{} (db: {})",
        config.host, config.port, config.db_path
    );

    let storage = gmp_storage::StorageConfig {
        data_dir: "./data".to_string(),
        db_path: config.db_path.clone(),
        embedding_dim: 768,
        use_sqlrustgo_storage: true,
    };

    let engine = gmp_storage::StorageEngine::new(storage).expect("Failed to initialize storage");

    let app_state = web::Data::new(AppState { storage: engine });

    info!("Starting server at http://{}:{}", config.host, config.port);

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .wrap(middleware::Logger::default())
            .route("/api/v1/health", web::get().to(health_handler))
            .route("/api/v1/stats", web::get().to(stats_handler))
            .route("/api/v1/search", web::post().to(search_handler))
    })
    .bind(format!("{}:{}", config.host, config.port))?
    .run()
    .await
}
