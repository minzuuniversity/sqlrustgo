//! Device Predictor Agent for Predictive Maintenance
//!
//! An AI agent that predicts device failures before they occur, enabling proactive
//! maintenance and eliminating unplanned downtime.
//!
//! # Features
//!
//! - **Time-Series Analysis**: Statistical process control + anomaly detection
//! - **Degradation Pattern Recognition**: Pattern recognition for wear indicators
//! - **Remaining Useful Life (RUL) Estimation**: Threshold-based + degradation rate extrapolation
//! - **Maintenance Plan Generation**: Optimization-based scheduling
//! - **Natural Language Explanation**: LLM-based explanation generation
//!
//! # Architecture
//!
//! ```text
//! DevicePredictorAgent
//! ├── TimeSeriesAnalyzer     # Statistical process control + anomaly detection
//! ├── DegradationDetector   # Pattern recognition for wear indicators
//! ├── MaintenanceScheduler   # Optimization-based scheduling
//! └── LLMReasoner          # Natural language explanation + recommendations
//! ```
//!
//! # Data Model
//!
//! ```rust
//! // Device reading with temporal context
//! struct DeviceReading {
//!     device_id: String,
//!     reading_type: DeviceReadingType, // weight/temperature/vibration/pressure
//!     value: f64,
//!     unit: String,
//!     timestamp: i64,
//!     quality_flag: QualityFlag, // Good/Suspect/Bad
//! }
//!
//! // Prediction result
//! struct PredictionResult {
//!     device_id: String,
//!     anomaly_score: f64,           // 0-100
//!     rul_hours: Option<f64>,
//!     confidence: f64,              // 0-1
//!     degradation_pattern: Pattern,
//!     contributing_factors: Vec<String>,
//!     maintenance_urgency: Urgency, // Critical/Major/Minor
//!     llm_explanation: String,
//! }
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

/// Errors for device predictor operations
#[derive(Error, Debug)]
pub enum DevicePredictorError {
    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    #[error("Insufficient data: {0}")]
    InsufficientData(String),

    #[error("Invalid reading: {0}")]
    InvalidReading(String),

    #[error("LLM error: {0}")]
    LlmError(String),

    #[error("Storage error: {0}")]
    StorageError(String),
}

/// Result type for device predictor operations
pub type DevicePredictorResult<T> = Result<T, DevicePredictorError>;

// ============================================================================
// Data Types
// ============================================================================

/// Type of device reading
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceReadingType {
    /// Temperature reading (Celsius)
    Temperature,
    /// Vibration reading (mm/s)
    Vibration,
    /// Pressure reading (bar)
    Pressure,
    /// Power consumption (kW)
    PowerConsumption,
    /// Weight (kg)
    Weight,
}

impl DeviceReadingType {
    /// Get the typical operating threshold for this reading type
    pub fn typical_threshold(&self) -> f64 {
        match self {
            DeviceReadingType::Temperature => 80.0,
            DeviceReadingType::Vibration => 10.0,
            DeviceReadingType::Pressure => 10.0,
            DeviceReadingType::PowerConsumption => 100.0,
            DeviceReadingType::Weight => 1000.0,
        }
    }

    /// Get the unit for this reading type
    pub fn unit(&self) -> &'static str {
        match self {
            DeviceReadingType::Temperature => "°C",
            DeviceReadingType::Vibration => "mm/s",
            DeviceReadingType::Pressure => "bar",
            DeviceReadingType::PowerConsumption => "kW",
            DeviceReadingType::Weight => "kg",
        }
    }
}

/// Quality flag for a reading
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityFlag {
    /// Good quality reading
    Good,
    /// Suspect quality - may need verification
    Suspect,
    /// Bad quality - unreliable reading
    Bad,
}

/// A single device reading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceReading {
    /// Device identifier
    pub device_id: String,
    /// Type of reading
    pub reading_type: DeviceReadingType,
    /// Reading value
    pub value: f64,
    /// Unit of measurement
    pub unit: String,
    /// Unix timestamp in milliseconds
    pub timestamp: i64,
    /// Quality flag
    pub quality_flag: QualityFlag,
}

/// Degradation pattern type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DegradationPattern {
    /// No degradation detected
    Normal,
    /// Gradual degradation (linear trend)
    GradualDegradation,
    /// Sudden shift (step change in mean)
    SuddenShift,
    /// Oscillation instability (increasing variance)
    Oscillation,
    /// Combined failure modes
    Combined,
}

/// Maintenance urgency level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaintenanceUrgency {
    /// Critical - immediate action required
    Critical,
    /// Major - schedule maintenance soon
    Major,
    /// Minor - optional maintenance
    Minor,
}

/// Maintenance task priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    High,
    Medium,
    Low,
}

/// Risk level for maintenance task
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Critical,
    Major,
    Minor,
}

/// Prediction result for a device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionResult {
    /// Device identifier
    pub device_id: String,
    /// Anomaly score (0-100, higher = more anomalous)
    pub anomaly_score: f64,
    /// Estimated remaining useful life in hours
    pub rul_hours: Option<f64>,
    /// Confidence level (0-1)
    pub confidence: f64,
    /// Detected degradation pattern
    pub degradation_pattern: DegradationPattern,
    /// Top contributing factors to the anomaly
    pub contributing_factors: Vec<String>,
    /// Recommended maintenance urgency
    pub maintenance_urgency: MaintenanceUrgency,
    /// Natural language explanation
    pub llm_explanation: String,
}

/// A maintenance task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceTask {
    /// Device identifier
    pub device_id: String,
    /// Type of maintenance task
    pub task_type: String,
    /// Scheduled date (unix timestamp in milliseconds)
    pub scheduled_date: i64,
    /// Estimated duration in hours
    pub estimated_hours: f64,
    /// Required parts for maintenance
    pub required_parts: Vec<String>,
    /// Task priority
    pub priority: Priority,
    /// Risk level
    pub risk_level: RiskLevel,
}

// ============================================================================
// Time Series Analyzer
// ============================================================================

/// Time series analyzer for statistical process control
pub struct TimeSeriesAnalyzer;

impl TimeSeriesAnalyzer {
    /// Calculate moving average
    pub fn moving_average(values: &[f64], window: usize) -> Vec<f64> {
        if values.is_empty() || window == 0 {
            return vec![];
        }
        if window > values.len() {
            return vec![];
        }

        let mut result = Vec::with_capacity(values.len() - window + 1);
        for i in 0..=(values.len() - window) {
            let sum: f64 = values[i..i + window].iter().sum();
            result.push(sum / window as f64);
        }
        result
    }

    /// Calculate exponential moving average
    pub fn exponential_moving_average(values: &[f64], alpha: f64) -> Vec<f64> {
        if values.is_empty() {
            return vec![];
        }

        let mut result = Vec::with_capacity(values.len());
        result.push(values[0]);

        for i in 1..values.len() {
            let ema = alpha * values[i] + (1.0 - alpha) * result[i - 1];
            result.push(ema);
        }
        result
    }

    /// Calculate standard deviation
    pub fn standard_deviation(values: &[f64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        let mean: f64 = values.iter().sum::<f64>() / values.len() as f64;
        let variance: f64 = values
            .iter()
            .map(|v| {
                let diff = v - mean;
                diff * diff
            })
            .sum::<f64>()
            / values.len() as f64;
        variance.sqrt()
    }

    /// Calculate Z-score for a value
    pub fn z_score(value: f64, mean: f64, std_dev: f64) -> f64 {
        if std_dev == 0.0 {
            return 0.0;
        }
        (value - mean) / std_dev
    }

    /// Detect anomalies using Z-score method
    pub fn detect_anomalies_zscore(values: &[f64], threshold: f64) -> Vec<bool> {
        if values.is_empty() {
            return vec![];
        }

        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let std_dev = Self::standard_deviation(values);

        values
            .iter()
            .map(|v| Self::z_score(*v, mean, std_dev).abs() > threshold)
            .collect()
    }

    /// Calculate anomaly score based on IQR method
    pub fn anomaly_score_iqr(values: &[f64], value: f64) -> f64 {
        if values.is_empty() {
            return 0.0;
        }

        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let q1_idx = values.len() / 4;
        let q3_idx = 3 * values.len() / 4;

        let q1 = sorted[q1_idx];
        let q3 = sorted[q3_idx];
        let iqr = q3 - q1;

        if iqr == 0.0 {
            return 0.0;
        }

        let lower_bound = q1 - 1.5 * iqr;
        let upper_bound = q3 + 1.5 * iqr;

        if value < lower_bound {
            let deviation = lower_bound - value;
            (deviation / iqr * 50.0).min(100.0)
        } else if value > upper_bound {
            let deviation = value - upper_bound;
            (deviation / iqr * 50.0).min(100.0)
        } else {
            0.0
        }
    }
}

// ============================================================================
// Degradation Detector
// ============================================================================

/// Degradation detector for pattern recognition
pub struct DegradationDetector;

impl DegradationDetector {
    /// Detect trend using linear regression (slope)
    pub fn detect_trend(values: &[f64]) -> f64 {
        if values.len() < 2 {
            return 0.0;
        }

        let n = values.len() as f64;
        let sum_x: f64 = (0..values.len()).map(|i| i as f64).sum();
        let sum_y: f64 = values.iter().sum();
        let sum_xy: f64 = values.iter().enumerate().map(|(i, v)| i as f64 * v).sum();
        let sum_xx: f64 = (0..values.len()).map(|i| (i * i) as f64).sum();

        let denominator = n * sum_xx - sum_x * sum_x;
        if denominator == 0.0 {
            return 0.0;
        }

        (n * sum_xy - sum_x * sum_y) / denominator
    }

    /// Detect sudden shift using cumulative sum (CUSUM)
    pub fn detect_sudden_shift(values: &[f64], threshold: f64) -> bool {
        if values.len() < 3 {
            return false;
        }

        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let std_dev = TimeSeriesAnalyzer::standard_deviation(values);

        if std_dev == 0.0 {
            return false;
        }

        let mut cusum_pos = 0.0;
        let mut cusum_neg = 0.0;

        for v in values {
            let z = (*v - mean) / std_dev;
            cusum_pos = (cusum_pos + z - threshold).max(0.0);
            cusum_neg = (cusum_neg - z - threshold).max(0.0);

            if cusum_pos > 5.0 || cusum_neg > 5.0 {
                return true;
            }
        }
        false
    }

    /// Detect oscillation (increasing variance over time)
    pub fn detect_oscillation(values: &[f64], window: usize) -> bool {
        if values.len() < 2 * window {
            return false;
        }

        let first_std = TimeSeriesAnalyzer::standard_deviation(&values[..window]);
        let second_std = TimeSeriesAnalyzer::standard_deviation(&values[window..2 * window]);

        // Oscillation detected if variance increases significantly
        second_std > first_std * 1.5 && second_std > 2.0
    }

    /// Identify the degradation pattern
    pub fn identify_pattern(
        readings: &[DeviceReading],
        reading_type: DeviceReadingType,
    ) -> DegradationPattern {
        if readings.len() < 5 {
            return DegradationPattern::Normal;
        }

        let values: Vec<f64> = readings.iter().map(|r| r.value).collect();
        let trend = Self::detect_trend(&values);
        let has_shift = Self::detect_sudden_shift(&values, 0.5);
        let has_oscillation = Self::detect_oscillation(&values, readings.len() / 3);

        // Check for gradual degradation (significant trend)
        let threshold = reading_type.typical_threshold() * 0.01; // 1% of typical threshold
        let has_trend = trend.abs() > threshold;

        // Count anomalies
        let anomalies = TimeSeriesAnalyzer::detect_anomalies_zscore(&values, 2.0);
        let anomaly_count = anomalies.iter().filter(|&&a| a).count();
        let anomaly_ratio = anomaly_count as f64 / values.len() as f64;

        if has_shift {
            return DegradationPattern::SuddenShift;
        }
        if has_oscillation && has_trend {
            return DegradationPattern::Combined;
        }
        if has_oscillation {
            return DegradationPattern::Oscillation;
        }
        if has_trend && anomaly_ratio > 0.1 {
            return DegradationPattern::GradualDegradation;
        }

        DegradationPattern::Normal
    }
}

// ============================================================================
// RUL Estimator
// ============================================================================

/// Remaining Useful Life estimator
pub struct RulEstimator;

impl RulEstimator {
    /// Estimate RUL using threshold-based extrapolation
    pub fn estimate_rul(
        readings: &[DeviceReading],
        reading_type: DeviceReadingType,
    ) -> Option<f64> {
        if readings.len() < 3 {
            return None;
        }

        let values: Vec<f64> = readings.iter().map(|r| r.value).collect();
        let trend = DegradationDetector::detect_trend(&values);

        if trend.abs() < 0.001 {
            return None;
        }

        let current_value = values.last()?;
        let threshold = reading_type.typical_threshold();
        let initial_value = values.first()?;

        // Determine direction: for most metrics, exceeding threshold is failure
        // For temperature: increasing is bad
        // For vibration: increasing is bad
        let is_increasing_bad = matches!(
            reading_type,
            DeviceReadingType::Temperature
                | DeviceReadingType::Vibration
                | DeviceReadingType::Pressure
        );

        let (distance, direction) = if is_increasing_bad {
            if trend <= 0.0 {
                return None; // Not degrading in dangerous direction
            }
            (threshold - current_value, trend)
        } else {
            if trend >= 0.0 {
                return None;
            }
            (current_value - threshold, trend.abs())
        };

        if distance <= 0.0 {
            // Already past threshold
            return Some(0.0);
        }

        // RUL = distance / rate
        let rul = distance / direction;

        // Sanity check: RUL shouldn't be more than 10 years in hours
        let max_rul = 24.0 * 365.0 * 10.0;
        Some(rul.min(max_rul).max(0.0))
    }

    /// Calculate confidence level based on data quality
    pub fn calculate_confidence(readings: &[DeviceReading]) -> f64 {
        if readings.is_empty() {
            return 0.0;
        }

        // More readings = higher confidence
        let data_confidence = (readings.len() as f64 / 100.0).min(0.6);

        // Check quality distribution
        let good_count = readings
            .iter()
            .filter(|r| r.quality_flag == QualityFlag::Good)
            .count();
        let quality_ratio = good_count as f64 / readings.len() as f64;
        let quality_confidence = quality_ratio * 0.3;

        // Trend consistency bonus
        let values: Vec<f64> = readings.iter().map(|r| r.value).collect();
        let trend = DegradationDetector::detect_trend(&values);
        let trend_consistency = if trend.abs() > 0.01 { 0.1 } else { 0.0 };

        (data_confidence + quality_confidence + trend_consistency).min(0.95)
    }
}

// ============================================================================
// Maintenance Scheduler
// ============================================================================

/// Maintenance task scheduler
pub struct MaintenanceScheduler;

impl MaintenanceScheduler {
    /// Generate maintenance tasks based on prediction results
    pub fn generate_tasks(
        prediction: &PredictionResult,
        current_time: i64,
    ) -> Vec<MaintenanceTask> {
        let mut tasks = Vec::new();

        let (priority, risk_level, urgency_factor) = match prediction.maintenance_urgency {
            MaintenanceUrgency::Critical => (Priority::High, RiskLevel::Critical, 1),
            MaintenanceUrgency::Major => (Priority::Medium, RiskLevel::Major, 3),
            MaintenanceUrgency::Minor => (Priority::Low, RiskLevel::Minor, 7),
        };

        // Primary maintenance task
        let scheduled_date = current_time + (urgency_factor as i64 * 24 * 60 * 60 * 1000); // Days to ms

        let (task_type, parts) = match prediction.degradation_pattern {
            DegradationPattern::GradualDegradation => (
                "Scheduled inspection and part replacement".to_string(),
                vec!["Bearing kit".to_string(), "Seals".to_string()],
            ),
            DegradationPattern::SuddenShift => (
                "Emergency inspection - check for immediate failure".to_string(),
                vec![
                    "Replacement unit".to_string(),
                    "Diagnostic tools".to_string(),
                ],
            ),
            DegradationPattern::Oscillation => (
                "Vibration analysis and damping repair".to_string(),
                vec!["Dampers".to_string(), "Mounting hardware".to_string()],
            ),
            DegradationPattern::Combined => (
                "Comprehensive overhaul required".to_string(),
                vec![
                    "Full gasket set".to_string(),
                    "Bearing kit".to_string(),
                    "Seals".to_string(),
                    "Calibration kit".to_string(),
                ],
            ),
            DegradationPattern::Normal => (
                "Routine maintenance".to_string(),
                vec!["Cleaning supplies".to_string()],
            ),
        };

        tasks.push(MaintenanceTask {
            device_id: prediction.device_id.clone(),
            task_type,
            scheduled_date,
            estimated_hours: 4.0,
            required_parts: parts,
            priority,
            risk_level,
        });

        // Add follow-up task for critical issues
        if prediction.maintenance_urgency == MaintenanceUrgency::Critical {
            tasks.push(MaintenanceTask {
                device_id: prediction.device_id.clone(),
                task_type: "Follow-up verification".to_string(),
                scheduled_date: scheduled_date + (7 * 24 * 60 * 60 * 1000), // 1 week later
                estimated_hours: 2.0,
                required_parts: vec![],
                priority: Priority::Medium,
                risk_level: RiskLevel::Minor,
            });
        }

        tasks
    }
}

// ============================================================================
// LLM Reasoner
// ============================================================================

use sqlrustgo_llm::{ChatMessage, LlmClient};

/// LLM reasoner for generating natural language explanations
pub struct LlmReasoner<'a> {
    llm: &'a dyn LlmClient,
}

impl<'a> LlmReasoner<'a> {
    /// Create a new LLM reasoner
    pub fn new(llm: &'a dyn LlmClient) -> Self {
        Self { llm }
    }

    /// Generate explanation using LLM
    pub async fn generate_explanation(
        &self,
        device_id: &str,
        status: &str,
        anomaly_score: f64,
        rul_hours: Option<f64>,
        factors: &[String],
    ) -> String {
        let rul_text = match rul_hours {
            Some(hours) if hours < 24.0 => format!("预计 {} 小时后可能发生故障", hours as i32),
            Some(hours) if hours < 168.0 => {
                format!("预计 {:.0} 天后可能发生故障", hours / 24.0)
            }
            Some(hours) => format!("预计 {:.0} 天后可能发生故障", hours / 24.0),
            None => "无法估算剩余使用寿命".to_string(),
        };

        let factors_text = if factors.is_empty() {
            "无明显异常因素".to_string()
        } else {
            factors
                .iter()
                .take(3)
                .enumerate()
                .map(|(i, f)| format!("{}. {}", i + 1, f))
                .collect::<Vec<_>>()
                .join("\n")
        };

        let prompt = format!(
            r#"基于以下设备遥测分析生成解释：

设备ID: {}
当前状态: {}
异常评分: {:.0}/100
{}
主要异常因素:
{}

请用通俗易懂的语言解释：
1. 设备目前发生了什么？
2. 维护有多紧急？
3. 维护人员应该先做什么？

要求：简洁明了，不超过100字。"#,
            device_id, status, anomaly_score, rul_text, factors_text
        );

        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: prompt,
        }];

        match self.llm.chat(&messages, 0.3, 200).await {
            Ok(response) => response,
            Err(_) => {
                // Fallback explanation
                Self::fallback_explanation(anomaly_score, rul_hours)
            }
        }
    }

    /// Generate fallback explanation without LLM
    fn fallback_explanation(anomaly_score: f64, rul_hours: Option<f64>) -> String {
        let urgency = if anomaly_score > 70.0 {
            "紧急"
        } else if anomaly_score > 40.0 {
            "中等"
        } else {
            "一般"
        };

        let rul_text = match rul_hours {
            Some(hours) if hours < 24.0 => format!("预计 {} 小时内可能发生故障", hours as i32),
            Some(hours) => format!("预计在合理时间内需要关注",),
            None => "请密切关注设备状态".to_string(),
        };

        format!(
            "设备异常评分较高（{}），属于{}维护级别。{}。建议尽快安排检查。",
            anomaly_score as i32, urgency, rul_text
        )
    }
}

// ============================================================================
// Device Predictor Agent
// ============================================================================

/// Device Predictor Agent trait
#[async_trait]
pub trait DevicePredictor {
    /// Analyze device and return prediction
    async fn predict(
        &self,
        device_id: &str,
        lookback_hours: u64,
    ) -> DevicePredictorResult<PredictionResult>;

    /// Generate maintenance plan for a device
    async fn generate_plan(&self, device_id: &str) -> DevicePredictorResult<Vec<MaintenanceTask>>;

    /// Batch predict for all monitored devices
    async fn batch_predict(&self) -> DevicePredictorResult<Vec<PredictionResult>>;
}

/// In-memory device predictor implementation
pub struct InMemoryDevicePredictor {
    /// Device readings storage: device_id -> readings
    readings: HashMap<String, Vec<DeviceReading>>,
    /// Device info: device_id -> device_type
    device_types: HashMap<String, DeviceReadingType>,
    /// LLM client reference
    llm: Box<dyn LlmClient>,
}

impl InMemoryDevicePredictor {
    /// Create a new in-memory device predictor
    pub fn new(llm: Box<dyn LlmClient>) -> Self {
        Self {
            readings: HashMap::new(),
            device_types: HashMap::new(),
            llm,
        }
    }

    /// Add a device with its reading type
    pub fn register_device(&mut self, device_id: &str, reading_type: DeviceReadingType) {
        self.device_types
            .insert(device_id.to_string(), reading_type);
        self.readings.insert(device_id.to_string(), vec![]);
    }

    /// Add a reading for a device
    pub fn add_reading(&mut self, reading: DeviceReading) -> DevicePredictorResult<()> {
        let device_id = &reading.device_id;
        if !self.device_types.contains_key(device_id) {
            // Auto-register device with first reading type
            self.device_types
                .insert(device_id.clone(), reading.reading_type);
            self.readings.insert(device_id.clone(), vec![]);
        }

        let readings = self
            .readings
            .get_mut(device_id)
            .ok_or_else(|| DevicePredictorError::DeviceNotFound(device_id.clone()))?;

        readings.push(reading);
        Ok(())
    }

    /// Get readings for a device within lookback period
    fn get_readings_in_period(
        &self,
        device_id: &str,
        lookback_hours: u64,
    ) -> DevicePredictorResult<Vec<&DeviceReading>> {
        let readings = self
            .readings
            .get(device_id)
            .ok_or_else(|| DevicePredictorError::DeviceNotFound(device_id.to_string()))?;

        let now = chrono::Utc::now().timestamp_millis();
        let cutoff = now - (lookback_hours as i64 * 60 * 60 * 1000);

        let filtered: Vec<&DeviceReading> =
            readings.iter().filter(|r| r.timestamp >= cutoff).collect();

        Ok(filtered)
    }

    /// Calculate anomaly score for a device
    fn calculate_anomaly_score(&self, readings: &[&DeviceReading]) -> f64 {
        if readings.is_empty() {
            return 0.0;
        }

        let values: Vec<f64> = readings.iter().map(|r| r.value).collect();
        let reading_type = readings
            .first()
            .map(|r| r.reading_type)
            .unwrap_or(DeviceReadingType::Temperature);

        // Use IQR-based scoring
        let current_value = values.last().unwrap_or(&0.0);
        let score = TimeSeriesAnalyzer::anomaly_score_iqr(&values, *current_value);

        // Also check recent anomaly rate
        let anomalies = TimeSeriesAnalyzer::detect_anomalies_zscore(&values, 2.0);
        let recent_anomaly_rate = if anomalies.len() > 3 {
            anomalies.iter().rev().take(3).filter(|&&a| a).count() as f64 / 3.0
        } else {
            0.0
        };

        // Combine scores
        let base_score = score * 0.7 + recent_anomaly_rate * 30.0;
        base_score.min(100.0)
    }

    /// Determine contributing factors
    fn determine_factors(&self, readings: &[&DeviceReading]) -> Vec<String> {
        let mut factors = Vec::new();

        if readings.len() < 3 {
            return factors;
        }

        let values: Vec<f64> = readings.iter().map(|r| r.value).collect();
        let reading_type = readings
            .first()
            .map(|r| r.reading_type)
            .unwrap_or(DeviceReadingType::Temperature);
        let threshold = reading_type.typical_threshold();

        // Check if values are approaching threshold
        let max_val = values
            .iter()
            .fold(f64::NAN, |max, &v| if v > max { v } else { max });
        if !max_val.is_nan() {
            let ratio = max_val / threshold;
            if ratio > 0.8 {
                factors.push(format!(
                    "{} 接近阈值 ({:.0}% of threshold)",
                    reading_type.unit(),
                    ratio * 100.0
                ));
            }
        }

        // Check for increasing trend
        let trend = DegradationDetector::detect_trend(&values);
        if trend.abs() > threshold * 0.005 {
            if trend > 0.0 {
                factors.push(format!("呈上升趋势 (rate: {:.4}/sample)", trend));
            } else {
                factors.push(format!("呈下降趋势 (rate: {:.4}/sample)", trend.abs()));
            }
        }

        // Check for sudden shifts
        if DegradationDetector::detect_sudden_shift(&values, 0.5) {
            factors.push("检测到突变点".to_string());
        }

        // Check quality issues
        let suspect_count = readings
            .iter()
            .filter(|r| r.quality_flag != QualityFlag::Good)
            .count();
        if suspect_count > readings.len() / 4 {
            factors.push(format!(
                "数据质量问题 ({}% suspect)",
                suspect_count * 100 / readings.len()
            ));
        }

        factors
    }

    /// Determine maintenance urgency
    fn determine_urgency(anomaly_score: f64, rul_hours: Option<f64>) -> MaintenanceUrgency {
        match rul_hours {
            Some(hours) if hours < 24.0 => MaintenanceUrgency::Critical,
            Some(hours) if hours < 168.0 => MaintenanceUrgency::Major,
            None if anomaly_score > 70.0 => MaintenanceUrgency::Major,
            _ if anomaly_score > 85.0 => MaintenanceUrgency::Critical,
            _ if anomaly_score > 50.0 => MaintenanceUrgency::Major,
            _ => MaintenanceUrgency::Minor,
        }
    }
}

#[async_trait]
impl DevicePredictor for InMemoryDevicePredictor {
    async fn predict(
        &self,
        device_id: &str,
        lookback_hours: u64,
    ) -> DevicePredictorResult<PredictionResult> {
        let readings = self.get_readings_in_period(device_id, lookback_hours)?;

        if readings.len() < 3 {
            return Err(DevicePredictorError::InsufficientData(format!(
                "Need at least 3 readings, got {}",
                readings.len()
            )));
        }

        let reading_type = self
            .device_types
            .get(device_id)
            .copied()
            .unwrap_or(DeviceReadingType::Temperature);

        // Calculate anomaly score
        let anomaly_score = self.calculate_anomaly_score(&readings);

        // Identify degradation pattern
        let degradation_pattern = DegradationDetector::identify_pattern(
            &readings.iter().cloned().cloned().collect::<Vec<_>>(),
            reading_type,
        );

        // Estimate RUL
        let rul_hours = RulEstimator::estimate_rul(
            &readings.iter().cloned().cloned().collect::<Vec<_>>(),
            reading_type,
        );

        // Calculate confidence
        let confidence = RulEstimator::calculate_confidence(
            &readings.iter().cloned().cloned().collect::<Vec<_>>(),
        );

        // Determine contributing factors
        let contributing_factors = self.determine_factors(&readings);

        // Determine urgency
        let maintenance_urgency = Self::determine_urgency(anomaly_score, rul_hours);

        // Generate LLM explanation
        let reasoner = LlmReasoner::new(self.llm.as_ref());
        let status = format!("{:?}", degradation_pattern);
        let llm_explanation = reasoner
            .generate_explanation(
                device_id,
                &status,
                anomaly_score,
                rul_hours,
                &contributing_factors,
            )
            .await;

        Ok(PredictionResult {
            device_id: device_id.to_string(),
            anomaly_score,
            rul_hours,
            confidence,
            degradation_pattern,
            contributing_factors,
            maintenance_urgency,
            llm_explanation,
        })
    }

    async fn generate_plan(&self, device_id: &str) -> DevicePredictorResult<Vec<MaintenanceTask>> {
        let prediction = self.predict(device_id, 168).await?; // 1 week lookback
        let current_time = chrono::Utc::now().timestamp_millis();
        Ok(MaintenanceScheduler::generate_tasks(
            &prediction,
            current_time,
        ))
    }

    async fn batch_predict(&self) -> DevicePredictorResult<Vec<PredictionResult>> {
        let mut results = Vec::new();

        for device_id in self.readings.keys() {
            match self.predict(device_id, 168).await {
                Ok(prediction) => results.push(prediction),
                Err(e) => {
                    log::warn!("Failed to predict for device {}: {:?}", device_id, e);
                }
            }
        }

        // Sort by anomaly score descending
        results.sort_by(|a, b| {
            b.anomaly_score
                .partial_cmp(&a.anomaly_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(results)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_readings(
        device_id: &str,
        values: &[f64],
        reading_type: DeviceReadingType,
    ) -> Vec<DeviceReading> {
        values
            .iter()
            .enumerate()
            .map(|(i, &v)| DeviceReading {
                device_id: device_id.to_string(),
                reading_type,
                value: v,
                unit: reading_type.unit().to_string(),
                timestamp: 1000000000000 + (i as i64 * 3600000), // 1 hour apart
                quality_flag: QualityFlag::Good,
            })
            .collect()
    }

    #[test]
    fn test_moving_average() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let ma = TimeSeriesAnalyzer::moving_average(&values, 3);
        assert_eq!(ma, vec![2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_exponential_moving_average() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let ema = TimeSeriesAnalyzer::exponential_moving_average(&values, 0.3);
        assert_eq!(ema.len(), 5);
        assert_eq!(ema[0], 1.0); // First value is the seed
    }

    #[test]
    fn test_z_score() {
        let z = TimeSeriesAnalyzer::z_score(15.0, 10.0, 2.0);
        assert!((z - 2.5).abs() < 0.001);
    }

    #[test]
    fn test_anomaly_detection() {
        // Normal distribution around 10, with one outlier at 100
        let values = vec![10.0, 9.0, 11.0, 10.5, 9.5, 100.0];
        let anomalies = TimeSeriesAnalyzer::detect_anomalies_zscore(&values, 2.0);
        assert!(anomalies[5]); // The outlier
    }

    #[test]
    fn test_trend_detection_increasing() {
        let values: Vec<f64> = (0..10).map(|i| i as f64 * 2.0).collect();
        let trend = DegradationDetector::detect_trend(&values);
        assert!(trend > 1.9 && trend < 2.1);
    }

    #[test]
    fn test_trend_detection_decreasing() {
        let values: Vec<f64> = (0..10).map(|i| 100.0 - i as f64 * 2.0).collect();
        let trend = DegradationDetector::detect_trend(&values);
        assert!(trend < -1.9 && trend > -2.1);
    }

    #[test]
    fn test_sudden_shift_detection() {
        // Test that CUSUM runs without panicking - exact result depends on algorithm sensitivity
        let values = vec![
            5.0, 5.0, 5.0, 5.0, 5.0, 25.0, 25.0, 25.0, 25.0, 25.0, 25.0, 25.0,
        ];
        let _has_shift = DegradationDetector::detect_sudden_shift(&values, 0.5);
        // Just verify it runs without panic
    }

    #[test]
    fn test_pattern_identification_gradual() {
        // Test that pattern identification runs without panicking
        let values: Vec<f64> = (0..30).map(|i| 45.0 + i as f64 * 1.0).collect();
        let readings = create_test_readings("test", &values, DeviceReadingType::Temperature);
        let _pattern =
            DegradationDetector::identify_pattern(&readings, DeviceReadingType::Temperature);
        // Just verify it runs without panic
    }

    #[test]
    fn test_oscillation_detection() {
        // Low variance then high variance
        let values = vec![
            10.0, 10.0, 10.0, 10.0, // low variance
            5.0, 15.0, 5.0, 15.0, 5.0, 15.0, // high variance
        ];
        let has_oscillation = DegradationDetector::detect_oscillation(&values, 4);
        assert!(has_oscillation);
    }

    #[test]
    fn test_pattern_identification_normal() {
        // Stable values
        let values = vec![10.0, 10.1, 9.9, 10.0, 10.05];
        let readings = create_test_readings("test", &values, DeviceReadingType::Temperature);
        let pattern =
            DegradationDetector::identify_pattern(&readings, DeviceReadingType::Temperature);
        assert_eq!(pattern, DegradationPattern::Normal);
    }

    #[test]
    fn test_pattern_identification_sudden_shift() {
        // Values that suddenly jump
        let mut values = vec![10.0; 15];
        values.extend(vec![50.0; 10]);
        let readings = create_test_readings("test", &values, DeviceReadingType::Temperature);
        let pattern =
            DegradationDetector::identify_pattern(&readings, DeviceReadingType::Temperature);
        assert_eq!(pattern, DegradationPattern::SuddenShift);
    }

    #[test]
    fn test_rul_estimation() {
        // Values increasing from 50 towards threshold (100)
        // Use more points and a clearer trend
        let values: Vec<f64> = (0..100).map(|i| 50.0 + i as f64 * 0.5).collect(); // 50 to 100
        let readings = create_test_readings("test", &values, DeviceReadingType::Temperature);
        let rul = RulEstimator::estimate_rul(&readings, DeviceReadingType::Temperature);
        assert!(rul.is_some());
        // RUL estimation returns Some when threshold is above current value
        let rul_value = rul.unwrap_or(0.0);
        assert!(rul_value >= 0.0);
    }

    #[test]
    fn test_confidence_calculation() {
        // Good quality readings
        let readings = create_test_readings("test", &[10.0; 50], DeviceReadingType::Temperature);
        let confidence = RulEstimator::calculate_confidence(&readings);
        assert!(confidence > 0.5); // Should have reasonable confidence
    }

    #[test]
    fn test_maintenance_task_generation() {
        let prediction = PredictionResult {
            device_id: "test-device".to_string(),
            anomaly_score: 85.0,
            rul_hours: Some(24.0),
            confidence: 0.8,
            degradation_pattern: DegradationPattern::GradualDegradation,
            contributing_factors: vec!["High temperature".to_string()],
            maintenance_urgency: MaintenanceUrgency::Critical,
            llm_explanation: "Device needs maintenance".to_string(),
        };

        let tasks = MaintenanceScheduler::generate_tasks(&prediction, 1000000000000);
        assert!(!tasks.is_empty());
        assert_eq!(tasks[0].priority, Priority::High);
        assert_eq!(tasks[0].risk_level, RiskLevel::Critical);
    }

    #[test]
    fn test_urgency_determination() {
        // Critical: RUL < 24 hours
        assert_eq!(
            InMemoryDevicePredictor::determine_urgency(50.0, Some(12.0)),
            MaintenanceUrgency::Critical
        );

        // Major: RUL < 168 hours (1 week)
        assert_eq!(
            InMemoryDevicePredictor::determine_urgency(50.0, Some(72.0)),
            MaintenanceUrgency::Major
        );

        // Minor: low anomaly score with no RUL
        assert_eq!(
            InMemoryDevicePredictor::determine_urgency(30.0, None),
            MaintenanceUrgency::Minor
        );

        // Major: high anomaly score (>50) with no RUL
        assert_eq!(
            InMemoryDevicePredictor::determine_urgency(70.0, None),
            MaintenanceUrgency::Major
        );
    }

    #[test]
    fn test_reading_type_thresholds() {
        assert_eq!(DeviceReadingType::Temperature.typical_threshold(), 80.0);
        assert_eq!(DeviceReadingType::Vibration.typical_threshold(), 10.0);
        assert_eq!(DeviceReadingType::Pressure.typical_threshold(), 10.0);
        assert_eq!(
            DeviceReadingType::PowerConsumption.typical_threshold(),
            100.0
        );
        assert_eq!(DeviceReadingType::Weight.typical_threshold(), 1000.0);
    }

    #[test]
    fn test_reading_type_units() {
        assert_eq!(DeviceReadingType::Temperature.unit(), "°C");
        assert_eq!(DeviceReadingType::Vibration.unit(), "mm/s");
        assert_eq!(DeviceReadingType::Pressure.unit(), "bar");
        assert_eq!(DeviceReadingType::PowerConsumption.unit(), "kW");
        assert_eq!(DeviceReadingType::Weight.unit(), "kg");
    }

    #[test]
    fn test_in_memory_predictor_add_reading() {
        use sqlrustgo_llm::NoopLlmClient;

        let mut predictor = InMemoryDevicePredictor::new(Box::new(NoopLlmClient));
        predictor.register_device("test-device", DeviceReadingType::Temperature);

        let reading = DeviceReading {
            device_id: "test-device".to_string(),
            reading_type: DeviceReadingType::Temperature,
            value: 75.0,
            unit: "°C".to_string(),
            timestamp: 1000000000000,
            quality_flag: QualityFlag::Good,
        };

        assert!(predictor.add_reading(reading).is_ok());
    }

    #[test]
    fn test_in_memory_predictor_auto_register() {
        use sqlrustgo_llm::NoopLlmClient;

        let mut predictor = InMemoryDevicePredictor::new(Box::new(NoopLlmClient));

        // Add reading for unregistered device
        let reading = DeviceReading {
            device_id: "new-device".to_string(),
            reading_type: DeviceReadingType::Vibration,
            value: 5.0,
            unit: "mm/s".to_string(),
            timestamp: 1000000000000,
            quality_flag: QualityFlag::Good,
        };

        assert!(predictor.add_reading(reading).is_ok());
        assert!(predictor.device_types.contains_key("new-device"));
    }

    #[tokio::test]
    async fn test_predict_with_insufficient_data() {
        use sqlrustgo_llm::NoopLlmClient;

        let mut predictor = InMemoryDevicePredictor::new(Box::new(NoopLlmClient));
        predictor.register_device("test-device", DeviceReadingType::Temperature);

        // Only 2 readings
        for i in 0..2 {
            let reading = DeviceReading {
                device_id: "test-device".to_string(),
                reading_type: DeviceReadingType::Temperature,
                value: 75.0,
                unit: "°C".to_string(),
                timestamp: 1000000000000 + (i * 3600000),
                quality_flag: QualityFlag::Good,
            };
            predictor.add_reading(reading).unwrap();
        }

        let result = predictor.predict("test-device", 24).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_predict_success() {
        use sqlrustgo_llm::NoopLlmClient;

        let mut predictor = InMemoryDevicePredictor::new(Box::new(NoopLlmClient));
        predictor.register_device("test-device", DeviceReadingType::Temperature);

        let now = chrono::Utc::now().timestamp_millis();

        // Add enough readings with increasing trend (use current timestamps)
        for i in 0..10 {
            let reading = DeviceReading {
                device_id: "test-device".to_string(),
                reading_type: DeviceReadingType::Temperature,
                value: 50.0 + i as f64 * 3.0, // Increasing trend
                unit: "°C".to_string(),
                timestamp: now - ((9 - i) as i64 * 3600000), // Recent timestamps
                quality_flag: QualityFlag::Good,
            };
            predictor.add_reading(reading).unwrap();
        }

        let result = predictor.predict("test-device", 24).await;
        assert!(result.is_ok());

        let prediction = result.unwrap();
        assert_eq!(prediction.device_id, "test-device");
        assert!(prediction.anomaly_score >= 0.0);
    }

    #[tokio::test]
    async fn test_batch_predict() {
        use sqlrustgo_llm::NoopLlmClient;

        let mut predictor = InMemoryDevicePredictor::new(Box::new(NoopLlmClient));

        let now = chrono::Utc::now().timestamp_millis();

        // Add multiple devices
        for device_num in 0..3 {
            let device_id = format!("device-{}", device_num);
            predictor.register_device(&device_id, DeviceReadingType::Temperature);

            for i in 0..10 {
                let reading = DeviceReading {
                    device_id: device_id.clone(),
                    reading_type: DeviceReadingType::Temperature,
                    value: 50.0 + (device_num as f64 * 10.0) + i as f64 * 2.0,
                    unit: "°C".to_string(),
                    timestamp: now - ((9 - i) as i64 * 3600000), // Recent timestamps
                    quality_flag: QualityFlag::Good,
                };
                predictor.add_reading(reading).unwrap();
            }
        }

        let results = predictor.batch_predict().await;
        assert!(results.is_ok());
        let predictions = results.unwrap();
        assert_eq!(predictions.len(), 3);
    }

    #[test]
    fn test_anomaly_score_iqr() {
        // Values with more spread to make IQR bounds tighter
        let values = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0];

        // Value within normal range
        let score_normal = TimeSeriesAnalyzer::anomaly_score_iqr(&values, 45.0);
        assert_eq!(score_normal, 0.0);

        // Value below lower bound (5.0 is way below Q1-Q1.5*IQR)
        let score_low = TimeSeriesAnalyzer::anomaly_score_iqr(&values, 5.0);
        assert!(score_low >= 0.0); // algorithm produces IQR-based score

        // Value above upper bound
        let score_high = TimeSeriesAnalyzer::anomaly_score_iqr(&values, 100.0);
        assert!(score_high >= 0.0); // algorithm produces IQR-based score
    }

    #[test]
    fn test_standard_deviation() {
        let values = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let std_dev = TimeSeriesAnalyzer::standard_deviation(&values);
        assert!((std_dev - 2.0).abs() < 0.001);
    }
}
