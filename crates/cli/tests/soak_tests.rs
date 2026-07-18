//! CLI soak module tests
//!
//! Tests: SoakConfig, SoakReport, print_report

use sqlrustgo_soak::soak::{print_report, SoakConfig, SoakReport};

// ============ SoakConfig tests ============

#[test]
fn test_soak_config_default() {
    let cfg = SoakConfig {
        host: "127.0.0.1".to_string(),
        port: 3306,
        user: "root".to_string(),
        password: "".to_string(),
        duration_secs: 60,
        target_qps: 0.0,
        query_file: None,
        report_interval: 10,
    };
    assert_eq!(cfg.host, "127.0.0.1");
    assert_eq!(cfg.port, 3306);
    assert_eq!(cfg.duration_secs, 60);
    assert_eq!(cfg.target_qps, 0.0);
    assert!(cfg.query_file.is_none());
    assert_eq!(cfg.report_interval, 10);
}

#[test]
fn test_soak_config_with_query_file() {
    let cfg = SoakConfig {
        host: "localhost".to_string(),
        port: 3307,
        user: "admin".to_string(),
        password: "secret".to_string(),
        duration_secs: 300,
        target_qps: 100.0,
        query_file: Some("/tmp/queries.txt".to_string()),
        report_interval: 30,
    };
    assert_eq!(cfg.host, "localhost");
    assert_eq!(cfg.port, 3307);
    assert_eq!(cfg.user, "admin");
    assert_eq!(cfg.password, "secret");
    assert_eq!(cfg.target_qps, 100.0);
    assert_eq!(cfg.query_file.as_deref(), Some("/tmp/queries.txt"));
}

#[test]
fn test_soak_config_clone() {
    let cfg = SoakConfig {
        host: "127.0.0.1".to_string(),
        port: 3306,
        user: "root".to_string(),
        password: "".to_string(),
        duration_secs: 60,
        target_qps: 50.0,
        query_file: None,
        report_interval: 10,
    };
    let cfg2 = cfg.clone();
    assert_eq!(cfg.host, cfg2.host);
    assert_eq!(cfg.port, cfg2.port);
    assert_eq!(cfg.target_qps, cfg2.target_qps);
}

#[test]
fn test_soak_config_debug() {
    let cfg = SoakConfig {
        host: "127.0.0.1".to_string(),
        port: 3306,
        user: "root".to_string(),
        password: "".to_string(),
        duration_secs: 60,
        target_qps: 0.0,
        query_file: None,
        report_interval: 10,
    };
    let debug = format!("{:?}", cfg);
    assert!(debug.contains("SoakConfig"));
    assert!(debug.contains("127.0.0.1"));
}

// ============ SoakReport tests ============

#[test]
fn test_soak_report_fields() {
    let report = SoakReport {
        duration_secs: 300,
        queries_executed: 15000,
        errors: 5,
        p50_latency_ms: 1.2,
        p90_latency_ms: 3.4,
        p99_latency_ms: 8.9,
        avg_latency_ms: 1.8,
        max_latency_ms: 15.0,
        actual_qps: 50.0,
    };
    assert_eq!(report.duration_secs, 300);
    assert_eq!(report.queries_executed, 15000);
    assert_eq!(report.errors, 5);
    assert_eq!(report.p50_latency_ms, 1.2);
    assert_eq!(report.p90_latency_ms, 3.4);
    assert_eq!(report.p99_latency_ms, 8.9);
    assert_eq!(report.avg_latency_ms, 1.8);
    assert_eq!(report.max_latency_ms, 15.0);
    assert_eq!(report.actual_qps, 50.0);
}

#[test]
fn test_soak_report_zero_errors() {
    let report = SoakReport {
        duration_secs: 60,
        queries_executed: 6000,
        errors: 0,
        p50_latency_ms: 0.8,
        p90_latency_ms: 1.5,
        p99_latency_ms: 3.0,
        avg_latency_ms: 0.9,
        max_latency_ms: 5.0,
        actual_qps: 100.0,
    };
    assert_eq!(report.errors, 0);
    assert_eq!(report.queries_executed, 6000);
}

#[test]
fn test_soak_report_debug() {
    let report = SoakReport {
        duration_secs: 60,
        queries_executed: 1000,
        errors: 2,
        p50_latency_ms: 1.0,
        p90_latency_ms: 2.0,
        p99_latency_ms: 5.0,
        avg_latency_ms: 1.1,
        max_latency_ms: 10.0,
        actual_qps: 16.7,
    };
    let debug = format!("{:?}", report);
    assert!(debug.contains("SoakReport"));
    assert!(debug.contains("1000"));
}

#[test]
fn test_soak_report_clone() {
    let report = SoakReport {
        duration_secs: 120,
        queries_executed: 6000,
        errors: 1,
        p50_latency_ms: 2.0,
        p90_latency_ms: 4.0,
        p99_latency_ms: 10.0,
        avg_latency_ms: 2.5,
        max_latency_ms: 20.0,
        actual_qps: 50.0,
    };
    let r2 = report.clone();
    assert_eq!(report.queries_executed, r2.queries_executed);
    assert_eq!(report.errors, r2.errors);
}

// ============ print_report tests ============

#[test]
fn test_print_report_zero_errors() {
    let report = SoakReport {
        duration_secs: 60,
        queries_executed: 6000,
        errors: 0,
        p50_latency_ms: 1.0,
        p90_latency_ms: 2.0,
        p99_latency_ms: 5.0,
        avg_latency_ms: 1.1,
        max_latency_ms: 10.0,
        actual_qps: 100.0,
    };
    print_report(&report);
}

#[test]
fn test_print_report_with_errors() {
    let report = SoakReport {
        duration_secs: 300,
        queries_executed: 15000,
        errors: 42,
        p50_latency_ms: 1.2,
        p90_latency_ms: 3.4,
        p99_latency_ms: 8.9,
        avg_latency_ms: 1.8,
        max_latency_ms: 15.0,
        actual_qps: 50.0,
    };
    print_report(&report);
}

#[test]
fn test_print_report_high_qps() {
    let report = SoakReport {
        duration_secs: 10,
        queries_executed: 100000,
        errors: 0,
        p50_latency_ms: 0.5,
        p90_latency_ms: 1.0,
        p99_latency_ms: 2.0,
        avg_latency_ms: 0.6,
        max_latency_ms: 3.0,
        actual_qps: 10000.0,
    };
    print_report(&report);
}
