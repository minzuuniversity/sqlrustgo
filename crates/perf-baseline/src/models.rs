//! Models for perf-baseline

use chrono::{DateTime, Utc};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BenchmarkResult {
    pub commit: String,
    pub timestamp: DateTime<Utc>,
    pub system_info: String,
    pub qps_update: f64,
    pub qps_delete: f64,
    pub qps_insert: f64,
    pub qps_select: f64,
    pub qps_complex_where: f64,
    pub qps_join: f64,
    pub latency_p50: f64,
    pub latency_p99: f64,
    pub memory_mb: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MetricRecord {
    pub metric_name: String,
    pub metric_value: f64,
    pub commit: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct RegressionResult {
    pub metric: String,
    pub baseline: f64,
    pub current: f64,
    pub change_pct: f64,
    pub threshold: f64,
    pub is_regression: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MetricType {
    QpsUpdate,
    QpsDelete,
    QpsInsert,
    QpsSelect,
    QpsComplexWhere,
    QpsJoin,
    LatencyP50,
    LatencyP99,
    MemoryMb,
}

impl MetricType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "qps_update" => Some(Self::QpsUpdate),
            "qps_delete" => Some(Self::QpsDelete),
            "qps_insert" => Some(Self::QpsInsert),
            "qps_select" => Some(Self::QpsSelect),
            "qps_complex_where" => Some(Self::QpsComplexWhere),
            "qps_join" => Some(Self::QpsJoin),
            "latency_p50" => Some(Self::LatencyP50),
            "latency_p99" => Some(Self::LatencyP99),
            "memory_mb" => Some(Self::MemoryMb),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::QpsUpdate => "qps_update",
            Self::QpsDelete => "qps_delete",
            Self::QpsInsert => "qps_insert",
            Self::QpsSelect => "qps_select",
            Self::QpsComplexWhere => "qps_complex_where",
            Self::QpsJoin => "qps_join",
            Self::LatencyP50 => "latency_p50",
            Self::LatencyP99 => "latency_p99",
            Self::MemoryMb => "memory_mb",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_type_all_variants() {
        assert_eq!(
            MetricType::from_str("qps_update"),
            Some(MetricType::QpsUpdate)
        );
        assert_eq!(
            MetricType::from_str("qps_delete"),
            Some(MetricType::QpsDelete)
        );
        assert_eq!(
            MetricType::from_str("qps_insert"),
            Some(MetricType::QpsInsert)
        );
        assert_eq!(
            MetricType::from_str("qps_select"),
            Some(MetricType::QpsSelect)
        );
        assert_eq!(
            MetricType::from_str("qps_complex_where"),
            Some(MetricType::QpsComplexWhere)
        );
        assert_eq!(MetricType::from_str("qps_join"), Some(MetricType::QpsJoin));
        assert_eq!(
            MetricType::from_str("latency_p50"),
            Some(MetricType::LatencyP50)
        );
        assert_eq!(
            MetricType::from_str("latency_p99"),
            Some(MetricType::LatencyP99)
        );
        assert_eq!(
            MetricType::from_str("memory_mb"),
            Some(MetricType::MemoryMb)
        );
    }

    #[test]
    fn test_metric_type_unknown() {
        assert_eq!(MetricType::from_str("unknown"), None);
        assert_eq!(MetricType::from_str(""), None);
        assert_eq!(MetricType::from_str("QPS_UPDATE"), None);
    }

    #[test]
    fn test_metric_type_as_str_round_trip() {
        for variant in [
            MetricType::QpsUpdate,
            MetricType::QpsDelete,
            MetricType::QpsInsert,
            MetricType::QpsSelect,
            MetricType::QpsComplexWhere,
            MetricType::QpsJoin,
            MetricType::LatencyP50,
            MetricType::LatencyP99,
            MetricType::MemoryMb,
        ] {
            assert_eq!(
                MetricType::from_str(variant.as_str()),
                Some(variant.clone())
            );
        }
    }

    #[test]
    fn test_regression_result_fields() {
        let result = RegressionResult {
            metric: "qps_select".to_string(),
            baseline: 50000.0,
            current: 45000.0,
            change_pct: -10.0,
            threshold: 5.0,
            is_regression: true,
        };
        assert_eq!(result.metric, "qps_select");
        assert_eq!(result.baseline, 50000.0);
        assert_eq!(result.current, 45000.0);
        assert_eq!(result.change_pct, -10.0);
        assert!(result.is_regression);
    }

    #[test]
    fn test_regression_result_no_regression() {
        let result = RegressionResult {
            metric: "latency_p99".to_string(),
            baseline: 2.0,
            current: 1.9,
            change_pct: -5.0,
            threshold: 10.0,
            is_regression: false,
        };
        assert!(!result.is_regression);
        assert!(result.change_pct < 0.0); // latency decreased = improvement
    }

    #[test]
    fn test_benchmark_result_fields() {
        use chrono::Utc;
        let ts = Utc::now();
        let result = BenchmarkResult {
            commit: "abc123def456".to_string(),
            timestamp: ts,
            system_info: "Linux x86_64 8 cores".to_string(),
            qps_update: 10000.0,
            qps_delete: 5000.0,
            qps_insert: 8000.0,
            qps_select: 30000.0,
            qps_complex_where: 4000.0,
            qps_join: 3000.0,
            latency_p50: 0.5,
            latency_p99: 1.5,
            memory_mb: 512.0,
        };
        assert_eq!(result.commit, "abc123def456");
        assert_eq!(result.qps_update, 10000.0);
        assert_eq!(result.qps_select, 30000.0);
        assert_eq!(result.latency_p99, 1.5);
        assert_eq!(result.memory_mb, 512.0);
    }

    #[test]
    fn test_metric_record_fields() {
        use chrono::Utc;
        let ts = Utc::now();
        let record = MetricRecord {
            metric_name: "qps_update".to_string(),
            metric_value: 15000.0,
            commit: "abc123".to_string(),
            timestamp: ts,
        };
        assert_eq!(record.metric_name, "qps_update");
        assert_eq!(record.metric_value, 15000.0);
        assert_eq!(record.commit, "abc123");
    }
}
