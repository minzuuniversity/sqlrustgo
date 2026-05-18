//! Trust metrics

#[derive(Debug, Clone)]
pub struct TrustMetrics {
    pub metrics: Vec<MetricValue>,
    pub period_start: chrono::DateTime<chrono::Utc>,
    pub period_end: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub enum MetricType {
    ComplianceRate,
    ViolationCount,
    EvidenceCompleteness,
    WorkflowThroughput,
    AuditCoverage,
}

#[derive(Debug, Clone)]
pub struct MetricValue {
    pub metric_type: MetricType,
    pub value: f64,
    pub unit: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl TrustMetrics {
    pub fn new(period_start: chrono::DateTime<chrono::Utc>, period_end: chrono::DateTime<chrono::Utc>) -> Self {
        Self {
            metrics: Vec::new(),
            period_start,
            period_end,
        }
    }

    pub fn add_metric(&mut self, metric: MetricValue) {
        self.metrics.push(metric);
    }
}
