//! OpenAPI Specification for SQLRustGo Network API
//!
//! This module provides OpenAPI 3.0.3 specification for the HTTP endpoints
//! exposed by the SQLRustGo server.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Health status enum
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Simple health check response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    /// Current health status
    pub status: HealthStatus,
    /// Server version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Readiness check response with component details
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReadinessResponse {
    /// Current health status
    pub status: HealthStatus,
    /// Server version
    pub version: String,
    /// Timestamp of the health check
    pub timestamp: u64,
    /// Individual component health statuses
    pub components: Vec<ComponentHealth>,
}

/// Individual component health information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ComponentHealth {
    /// Component name
    pub name: String,
    /// Component status
    pub status: HealthStatus,
    /// Optional status message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Optional latency in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
}

/// SQL Query request body
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QueryRequest {
    /// SQL query string to execute
    pub sql: String,
    /// Optional transaction ID for explicit transaction control
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_id: Option<String>,
    /// Optional query timeout in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u64>,
}

/// SQL Query response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QueryResponse {
    /// Whether the query was successful
    pub success: bool,
    /// Number of rows affected (for INSERT/UPDATE/DELETE)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub affected_rows: Option<u64>,
    /// Query results (for SELECT queries)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub results: Option<Vec<Vec<serde_json::Value>>>,
    /// Error message if the query failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Execution time in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_time_ms: Option<u64>,
}

/// Prometheus-compatible metrics response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MetricsResponse {
    /// Raw Prometheus metrics text
    pub metrics: String,
}

/// Error response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    /// Error code
    pub error: String,
    /// Detailed error message
    pub message: String,
}

/// GET /health - Health check endpoint
///
/// Returns the current health status of the service.
/// This endpoint is suitable for Kubernetes liveness probes.
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    )
)]
pub async fn health_check() -> HealthResponse {
    HealthResponse {
        status: HealthStatus::Healthy,
        version: None,
    }
}

/// GET /health/live - Liveness probe endpoint
///
/// Returns a simple healthy status for Kubernetes liveness probes.
#[utoipa::path(
    get,
    path = "/health/live",
    responses(
        (status = 200, description = "Service is alive", body = HealthResponse)
    )
)]
pub async fn liveness_probe() -> HealthResponse {
    HealthResponse {
        status: HealthStatus::Healthy,
        version: None,
    }
}

/// GET /health/ready - Readiness probe endpoint
///
/// Returns detailed readiness information including version and component status.
/// This endpoint is suitable for Kubernetes readiness probes.
#[utoipa::path(
    get,
    path = "/health/ready",
    responses(
        (status = 200, description = "Service is ready", body = ReadinessResponse),
        (status = 503, description = "Service is not ready", body = ErrorResponse)
    )
)]
pub async fn readiness_probe() -> ReadinessResponse {
    ReadinessResponse {
        status: HealthStatus::Healthy,
        version: "1.4.0".to_string(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
        components: vec![],
    }
}

/// GET /ready - Alternative readiness path
///
/// Returns detailed readiness information.
/// Alias for /health/ready.
#[utoipa::path(
    get,
    path = "/ready",
    responses(
        (status = 200, description = "Service is ready", body = ReadinessResponse),
        (status = 503, description = "Service is not ready", body = ErrorResponse)
    )
)]
pub async fn ready_alias() -> ReadinessResponse {
    readiness_probe().await
}

/// POST /query - SQL Query submission endpoint
///
/// Submits a SQL query for execution.
/// Supports SELECT, INSERT, UPDATE, DELETE and other SQL statements.
#[utoipa::path(
    post,
    path = "/query",
    request_body = QueryRequest,
    responses(
        (status = 200, description = "Query executed successfully", body = QueryResponse),
        (status = 400, description = "Invalid query or request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn query_endpoint(_body: QueryRequest) -> QueryResponse {
    QueryResponse {
        success: true,
        affected_rows: None,
        results: None,
        error: None,
        execution_time_ms: None,
    }
}

/// GET /metrics - Prometheus metrics endpoint
///
/// Returns Prometheus-compatible metrics in text format.
/// Suitable for scraping by Prometheus or other monitoring systems.
#[utoipa::path(
    get,
    path = "/metrics",
    responses(
        (status = 200, description = "Metrics in Prometheus format", body = MetricsResponse)
    )
)]
pub async fn metrics_endpoint() -> MetricsResponse {
    MetricsResponse {
        metrics: "# HELP sqlrustgo_uptime_seconds Time since service start\n# TYPE sqlrustgo_uptime_seconds gauge\nsqlrustgo_uptime_seconds 0".to_string(),
    }
}

/// Returns the OpenAPI specification as a JSON string
pub fn openapi_json() -> String {
    include_str!("../../../docs/api/openapi.yaml").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: HealthStatus::Healthy,
            version: Some("1.4.0".to_string()),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("Healthy"));
        assert!(json.contains("1.4.0"));
    }

    #[test]
    fn test_query_request_serialization() {
        let request = QueryRequest {
            sql: "SELECT * FROM users".to_string(),
            transaction_id: None,
            timeout_seconds: Some(30),
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("SELECT * FROM users"));
        assert!(json.contains("30"));
    }

    #[test]
    fn test_query_response_serialization() {
        let response = QueryResponse {
            success: true,
            affected_rows: Some(5),
            results: None,
            error: None,
            execution_time_ms: Some(42),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("true"));
        assert!(json.contains("5"));
        assert!(json.contains("42"));
    }

    #[test]
    fn test_openapi_json_loads() {
        let json_str = openapi_json();
        assert!(json_str.contains("openapi"));
        assert!(json_str.contains("3.0.3"));
        assert!(json_str.contains("/health"));
        assert!(json_str.contains("/query"));
        assert!(json_str.contains("/metrics"));
    }
}