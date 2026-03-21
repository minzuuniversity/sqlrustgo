//! Profiling Endpoint Module
//!
//! Provides HTTP endpoints for query profiling and tracing data.
//! This is a teaching-enhanced feature that helps understand query performance.

use actix_web::{web, HttpResponse, Responder};
use sqlrustgo_executor::{
    pipeline_trace::GLOBAL_TRACE_COLLECTOR,
    operator_profile::{self, GLOBAL_PROFILER},
};

/// Configuration for profiling endpoints
#[derive(Clone)]
pub struct ProfilingConfig {
    /// Enable detailed tracing
    pub tracing_enabled: bool,
    /// Enable profiling
    pub profiling_enabled: bool,
    /// Maximum traces to keep
    pub max_traces: usize,
}

impl Default for ProfilingConfig {
    fn default() -> Self {
        Self {
            tracing_enabled: true,
            profiling_enabled: true,
            max_traces: 1000,
        }
    }
}

/// Profiling state shared across endpoints
#[derive(Clone)]
pub struct ProfilingState {
    pub config: ProfilingConfig,
}

impl ProfilingState {
    pub fn new() -> Self {
        Self {
            config: ProfilingConfig::default(),
        }
    }
}

impl Default for ProfilingState {
    fn default() -> Self {
        Self::new()
    }
}

/// Handler for GET /trace - returns all traces as JSON
pub async fn get_all_traces() -> impl Responder {
    let traces = GLOBAL_TRACE_COLLECTOR.get_all();
    
    let response = serde_json::json!({
        "success": true,
        "count": traces.len(),
        "traces": traces,
    });
    
    HttpResponse::Ok()
        .content_type("application/json")
        .json(response)
}

/// Handler for GET /trace/summary - returns trace summary statistics
pub async fn get_trace_summary() -> impl Responder {
    let summary = GLOBAL_TRACE_COLLECTOR.summary();
    
    HttpResponse::Ok()
        .content_type("application/json")
        .json(summary)
}

/// Handler for POST /trace - clears all traces
pub async fn clear_traces() -> impl Responder {
    GLOBAL_TRACE_COLLECTOR.clear();
    
    HttpResponse::Ok()
        .content_type("application/json")
        .json(serde_json::json!({
            "success": true,
            "message": "All traces cleared",
        }))
}

/// Handler for GET /profile - returns all profiles as JSON
pub async fn get_all_profiles() -> impl Responder {
    let profiles = GLOBAL_PROFILER.get_all();
    
    let response = serde_json::json!({
        "success": true,
        "count": profiles.len(),
        "profiles": profiles,
    });
    
    HttpResponse::Ok()
        .content_type("application/json")
        .json(response)
}

/// Handler for GET /profile/summary - returns profile summary statistics
pub async fn get_profile_summary() -> impl Responder {
    let summary = GLOBAL_PROFILER.summary();
    
    HttpResponse::Ok()
        .content_type("application/json")
        .json(summary)
}

/// Handler for GET /profile/slowest - returns slowest queries
pub async fn get_slowest_queries() -> impl Responder {
    let profiles = GLOBAL_PROFILER.get_slowest(10);
    
    HttpResponse::Ok()
        .content_type("application/json")
        .json(serde_json::json!({
            "success": true,
            "count": profiles.len(),
            "profiles": profiles,
        }))
}

/// Handler for POST /profile - clears all profiles
pub async fn clear_profiles() -> impl Responder {
    GLOBAL_PROFILER.clear();
    
    HttpResponse::Ok()
        .content_type("application/json")
        .json(serde_json::json!({
            "success": true,
            "message": "All profiles cleared",
        }))
}

/// Handler for GET /profile/report - returns detailed profiling report
pub async fn get_profiling_report() -> impl Responder {
    let report = operator_profile::get_profiling_report(&GLOBAL_PROFILER);
    
    HttpResponse::Ok()
        .content_type("application/json")
        .body(report)
}

/// Configure profiling endpoints
pub fn configure_profiling_scope(cfg: &mut web::ServiceConfig) {
    // Trace endpoints
    cfg.service(
        web::resource("/trace")
            .route(web::get().to(get_all_traces))
            .route(web::post().to(clear_traces))
    );
    
    cfg.service(
        web::resource("/trace/summary")
            .route(web::get().to(get_trace_summary))
    );
    
    // Profile endpoints
    cfg.service(
        web::resource("/profile")
            .route(web::get().to(get_all_profiles))
            .route(web::post().to(clear_profiles))
    );
    
    cfg.service(
        web::resource("/profile/summary")
            .route(web::get().to(get_profile_summary))
    );
    
    cfg.service(
        web::resource("/profile/slowest")
            .route(web::get().to(get_slowest_queries))
    );
    
    cfg.service(
        web::resource("/profile/report")
            .route(web::get().to(get_profiling_report))
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlrustgo_executor::pipeline_trace;
    use sqlrustgo_executor::operator_profile;

    #[test]
    fn test_profiling_state_default() {
        let state = ProfilingState::default();
        assert!(state.config.tracing_enabled);
        assert!(state.config.profiling_enabled);
    }

    #[test]
    fn test_profiling_config_default() {
        let config = ProfilingConfig::default();
        assert_eq!(config.max_traces, 1000);
    }

    #[test]
    fn test_trace_summary_serialization() {
        let summary = pipeline_trace::TraceSummary {
            total_queries: 10,
            successful_queries: 9,
            failed_queries: 1,
            avg_duration_ns: 1000,
            total_rows: 100,
        };
        
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("total_queries"));
        assert!(json.contains("10"));
    }

    #[test]
    fn test_profiler_summary_serialization() {
        let summary = operator_profile::ProfilerSummary {
            total_queries: 10,
            successful_queries: 9,
            failed_queries: 1,
            avg_duration_ns: 1000,
            min_duration_ns: 100,
            max_duration_ns: 5000,
            total_cpu_time_ns: 8000,
            total_rows: 100,
        };
        
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("total_queries"));
        assert!(json.contains("10"));
    }

    #[actix_web::test]
    async fn test_get_all_traces_empty() {
        GLOBAL_TRACE_COLLECTOR.clear();
        
        let app = actix_web::test::init_service(
            actix_web::App::new()
                .configure(configure_profiling_scope),
        )
        .await;

        let req = actix_web::test::TestRequest::get()
            .uri("/trace")
            .to_request();

        let resp = actix_web::test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    #[actix_web::test]
    async fn test_get_trace_summary() {
        GLOBAL_TRACE_COLLECTOR.clear();
        
        let app = actix_web::test::init_service(
            actix_web::App::new()
                .configure(configure_profiling_scope),
        )
        .await;

        let req = actix_web::test::TestRequest::get()
            .uri("/trace/summary")
            .to_request();

        let resp = actix_web::test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        
        let body = actix_web::test::read_body(resp).await;
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("total_queries"));
    }

    #[actix_web::test]
    async fn test_get_profile_summary() {
        GLOBAL_PROFILER.clear();
        
        let app = actix_web::test::init_service(
            actix_web::App::new()
                .configure(configure_profiling_scope),
        )
        .await;

        let req = actix_web::test::TestRequest::get()
            .uri("/profile/summary")
            .to_request();

        let resp = actix_web::test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        
        let body = actix_web::test::read_body(resp).await;
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("total_queries"));
    }

    #[actix_web::test]
    async fn test_clear_traces() {
        GLOBAL_TRACE_COLLECTOR.clear();
        
        let app = actix_web::test::init_service(
            actix_web::App::new()
                .configure(configure_profiling_scope),
        )
        .await;

        let req = actix_web::test::TestRequest::post()
            .uri("/trace")
            .to_request();

        let resp = actix_web::test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }
}
