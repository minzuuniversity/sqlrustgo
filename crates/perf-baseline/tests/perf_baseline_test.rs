use chrono::Utc;
use sqlrustgo_perf_baseline::{BenchmarkResult, MetricType, RegressionResult};

#[test]
fn test_benchmark_result_serialization() {
    let result = BenchmarkResult {
        commit: "abc123".to_string(),
        timestamp: Utc::now(),
        system_info: "test".to_string(),
        qps_update: 10000.0,
        qps_delete: 5000.0,
        qps_insert: 8000.0,
        qps_select: 30000.0,
        qps_complex_where: 5000.0,
        qps_join: 4000.0,
        latency_p50: 0.5,
        latency_p99: 1.5,
        memory_mb: 256.0,
    };

    let json = serde_json::to_string(&result).unwrap();
    let deserialized: BenchmarkResult = serde_json::from_str(&json).unwrap();

    assert_eq!(result.commit, deserialized.commit);
    assert_eq!(result.qps_update, deserialized.qps_update);
    assert_eq!(result.qps_select, deserialized.qps_select);
}

#[test]
fn test_metric_type_from_str() {
    assert_eq!(
        MetricType::from_str("qps_update"),
        Some(MetricType::QpsUpdate)
    );
    assert_eq!(
        MetricType::from_str("qps_delete"),
        Some(MetricType::QpsDelete)
    );
    assert_eq!(
        MetricType::from_str("qps_select"),
        Some(MetricType::QpsSelect)
    );
    assert_eq!(
        MetricType::from_str("latency_p50"),
        Some(MetricType::LatencyP50)
    );
    assert_eq!(MetricType::from_str("invalid"), None);
}

#[test]
fn test_metric_type_as_str() {
    assert_eq!(MetricType::QpsUpdate.as_str(), "qps_update");
    assert_eq!(MetricType::QpsSelect.as_str(), "qps_select");
    assert_eq!(MetricType::LatencyP99.as_str(), "latency_p99");
}

#[test]
fn test_regression_result() {
    let result = RegressionResult {
        metric: "qps_update".to_string(),
        baseline: 10000.0,
        current: 9000.0,
        change_pct: -10.0,
        threshold: 10.0,
        is_regression: true,
    };

    assert!(result.is_regression);
    assert_eq!(result.change_pct, -10.0);
}

#[test]
fn test_regression_no_regression() {
    let result = RegressionResult {
        metric: "qps_update".to_string(),
        baseline: 10000.0,
        current: 9500.0,
        change_pct: -5.0,
        threshold: 10.0,
        is_regression: false,
    };

    assert!(!result.is_regression);
}
