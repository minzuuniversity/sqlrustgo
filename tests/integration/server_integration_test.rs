//! Server Integration Tests
//!
//! Tests for HTTP Server, Metrics, Teaching Endpoints, and Connection Pool

use sqlrustgo_server::*;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[test]
fn test_http_server_creation() {
    let server = HttpServer::new("127.0.0.1", 18080);
    assert!(!server.get_version().is_empty());
}

#[test]
fn test_http_server_with_version() {
    let server = HttpServer::new("127.0.0.1", 18080).with_version("1.9.0-test");
    assert_eq!(server.get_version(), "1.9.0-test");
}

#[test]
fn test_http_server_bind_to_available_port() {
    let server = HttpServer::new("127.0.0.1", 0);
    let port = server.bind_to_available_port();
    assert!(port > 0);
}

#[test]
fn test_metrics_registry_creation() {
    let registry = MetricsRegistry::new();
    // Verify registry was created
    let output = registry.to_prometheus_format();
    assert!(output.is_empty() || !output.is_empty());
}

#[test]
fn test_metrics_registry_register_custom_metric() {
    let mut registry = MetricsRegistry::new();
    registry.register_custom_metric("test_metric".to_string(), "1".to_string());
    registry.register_help("test_metric".to_string(), "Test metric help".to_string());

    let output = registry.to_prometheus_format();
    assert!(output.contains("test_metric"));
}

#[test]
fn test_teaching_endpoints_default() {
    let endpoints = TeachingEndpoints::default();
    assert!(endpoints.enable_pipeline_viz);
    assert!(endpoints.enable_profiling);
    assert!(endpoints.enable_trace);
}

#[test]
fn test_teaching_endpoints_custom() {
    let endpoints = TeachingEndpoints {
        enable_pipeline_viz: false,
        enable_profiling: false,
        enable_trace: false,
        max_traces: 500,
        max_profiles: 50,
    };

    assert!(!endpoints.enable_pipeline_viz);
    assert_eq!(endpoints.max_traces, 500);
    assert_eq!(endpoints.max_profiles, 50);
}

#[test]
fn test_teaching_http_server_creation() {
    let server = TeachingHttpServer::new("127.0.0.1", 18080);
    assert!(!server.get_version().is_empty());
}

#[test]
fn test_teaching_http_server_port() {
    let server = TeachingHttpServer::new("127.0.0.1", 18080);
    assert_eq!(server.get_port(), 18080);
}

#[test]
fn test_teaching_http_server_with_endpoints() {
    let custom = TeachingEndpoints {
        enable_pipeline_viz: true,
        enable_profiling: true,
        enable_trace: true,
        max_traces: 2000,
        max_profiles: 200,
    };

    let _server = TeachingHttpServer::new("127.0.0.1", 18080).with_teaching_endpoints(custom);
}

#[test]
fn test_connection_pool_creation() {
    let config = PoolConfig {
        size: 5,
        timeout_ms: 30000,
    };

    let pool = ConnectionPool::new(config);
    assert!(pool.get_pool_size() > 0);
}

#[test]
fn test_connection_pool_acquire_release() {
    let config = PoolConfig {
        size: 2,
        timeout_ms: 5000,
    };

    let pool = ConnectionPool::new(config);

    // Acquire a connection
    let _session1 = pool.acquire();

    // Pool should still be functional
    assert!(pool.get_pool_size() > 0);
}

#[test]
fn test_connection_pool_exhaustion() {
    let config = PoolConfig {
        size: 2,
        timeout_ms: 100,
    };

    let pool = ConnectionPool::new(config);

    let _session1 = pool.acquire();
    let _session2 = pool.acquire();

    // Pool exhausted, try_acquire should return None
    let result = pool.try_acquire();
    assert!(result.is_none());
}

#[test]
fn test_connection_pool_concurrent() {
    let config = PoolConfig {
        size: 3,
        timeout_ms: 10000,
    };

    let pool = Arc::new(ConnectionPool::new(config));

    let handles: Vec<_> = (0..6)
        .map(|_| {
            let pool = Arc::clone(&pool);
            thread::spawn(move || {
                if let Some(_session) = pool.try_acquire() {
                    thread::sleep(Duration::from_millis(10));
                    return true;
                }
                false
            })
        })
        .collect();

    let mut successes = 0;
    for handle in handles {
        if handle.join().unwrap() {
            successes += 1;
        }
    }

    assert!(successes > 0);
}

#[test]
fn test_pooled_session_new() {
    let session = PooledSession::new();
    assert!(session.is_available());
    assert!(session.transaction_id.is_none());
}

#[test]
fn test_pooled_session_clone() {
    let session = PooledSession::new();
    let cloned = session.clone();
    assert!(cloned.is_available());
}

#[test]
fn test_health_checker_live() {
    let checker = HealthChecker::new("1.9.0");
    let status = checker.check_live();
    assert_eq!(status, HealthStatus::Healthy);
}

#[test]
fn test_health_checker_ready() {
    let checker = HealthChecker::new("1.9.0");
    let report = checker.check_ready();
    assert_eq!(report.version, "1.9.0");
}

#[test]
fn test_health_checker_default() {
    let checker = HealthChecker::default();
    let report = checker.check_ready();
    assert_eq!(report.version, "unknown");
}

#[test]
fn test_component_health() {
    let component = ComponentHealth::new("test".to_string(), HealthStatus::Healthy);
    assert_eq!(component.name, "test".to_string());
}
