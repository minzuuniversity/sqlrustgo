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
    pub fn new(
        period_start: chrono::DateTime<chrono::Utc>,
        period_end: chrono::DateTime<chrono::Utc>,
    ) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_timestamp() -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc)
    }

    #[test]
    fn test_trust_metrics_new() {
        let start = test_timestamp();
        let end = test_timestamp();
        let metrics = TrustMetrics::new(start, end);
        assert!(metrics.metrics.is_empty());
        assert_eq!(metrics.period_start, start);
        assert_eq!(metrics.period_end, end);
    }

    #[test]
    fn test_trust_metrics_add_metric() {
        let start = test_timestamp();
        let end = test_timestamp();
        let mut trust_metrics = TrustMetrics::new(start, end);
        let metric = MetricValue {
            metric_type: MetricType::ComplianceRate,
            value: 95.5,
            unit: "%".to_string(),
            timestamp: test_timestamp(),
        };
        trust_metrics.add_metric(metric.clone());
        assert_eq!(trust_metrics.metrics.len(), 1);
        assert_eq!(trust_metrics.metrics[0].value, 95.5);
    }

    #[test]
    fn test_metric_type_variants() {
        let types = vec![
            MetricType::ComplianceRate,
            MetricType::ViolationCount,
            MetricType::EvidenceCompleteness,
            MetricType::WorkflowThroughput,
            MetricType::AuditCoverage,
        ];
        assert_eq!(types.len(), 5);
    }

    #[test]
    fn test_metric_value_creation() {
        let metric = MetricValue {
            metric_type: MetricType::ViolationCount,
            value: 3.0,
            unit: "count".to_string(),
            timestamp: test_timestamp(),
        };
        assert!(matches!(metric.metric_type, MetricType::ViolationCount));
        assert_eq!(metric.value, 3.0);
        assert_eq!(metric.unit, "count");
    }
}
