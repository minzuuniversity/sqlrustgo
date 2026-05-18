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
