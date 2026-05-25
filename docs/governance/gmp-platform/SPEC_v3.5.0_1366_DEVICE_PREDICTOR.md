# SPEC: Device Predictor Agent (#1366)

## 1. Concept & Vision

An AI agent that predicts device failures before they occur, enabling proactive maintenance and eliminating unplanned downtime. Built as an autonomous LLM agent that analyzes time-series sensor data, identifies degradation patterns, and generates actionable maintenance plans.

**Design Philosophy**: "Predictive, not reactive" — shift from break-fix to prevent-fix maintenance culture.

---

## 2. System Architecture

### 2.1 Agent Structure

```
DevicePredictorAgent
├── TimeSeriesAnalyzer     # Statistical process control + anomaly detection
├── DegradationDetector   # Pattern recognition for wear indicators  
├── MaintenanceScheduler   # Optimization-based scheduling
└── LLMReasoner          # Natural language explanation + recommendations
```

### 2.2 Integration Points

| Component | Protocol | Purpose |
|-----------|----------|---------|
| GMP-Platform Storage | SQLite/SQLRustGo | Historical device readings |
| Audit Chain | GMP Audit Store | Maintenance history |
| LLM | Ollama (qwen2.5:14b) | Natural language output |
| Notification | GMP Workflow Events | Alert dispatch |

---

## 3. Functionality Specification

### 3.1 Core Features

#### F1: Time-Series Analysis
- **Input**: Device sensor data (temperature, vibration, pressure, power consumption)
- **Processing**: 
  - Statistical process control (SPC) charts
  - Moving average / exponential smoothing
  - Trend detection (linear regression + curvature)
  - Anomaly scoring (Z-score, IQR-based)
- **Output**: Anomaly score per reading, trend classification

#### F2: Degradation Pattern Recognition
- **Patterns**:
  - Gradual degradation (linear trend)
  - Sudden shift (step change in mean)
  - Oscillation instability (increasing variance)
  - Combined failure modes
- **ML Approach**: Rule-based + lightweight ML (no heavy training)

#### F3: Remaining Useful Life (RUL) Estimation
- **Method**: Threshold-based + degradation rate extrapolation
- **Formula**: `RUL = (threshold - current_value) / degradation_rate`
- **Output**: RUL in hours/days, confidence interval

#### F4: Maintenance Plan Generation
- **Input**: Device criticality, RUL, current maintenance backlog
- **Output**: Prioritized maintenance task list with:
  - Suggested date
  - Required parts/tools
  - Estimated duration
  - Risk level (critical/major/minor)

#### F5: Natural Language Explanation
- **LLM Prompt Template**:
  ```
  Based on the following device telemetry analysis:
  - Device: {device_id}
  - Current status: {status}
  - Anomaly score: {score}/100
  - RUL estimate: {rul} hours
  - Top 3 contributing factors: {factors}
  
  Explain in plain language:
  1. What is likely happening with this device?
  2. How urgent is maintenance?
  3. What should maintenance crew do first?
  ```

### 3.2 Data Model

```rust
// Device reading with temporal context
struct DeviceReading {
    device_id: String,
    reading_type: DeviceReadingType, // weight/temperature/vibration/pressure
    value: f64,
    unit: String,
    timestamp: i64,
    quality_flag: QualityFlag, // Good/Suspect/Bad
}

// Prediction result
struct PredictionResult {
    device_id: String,
    anomaly_score: f64,           // 0-100
    rul_hours: Option<f64>,
    confidence: f64,              // 0-1
    degradation_pattern: Pattern,
    contributing_factors: Vec<String>,
    maintenance_urgency: Urgency, // Critical/Major/Minor
    llm_explanation: String,
}

// Maintenance task
struct MaintenanceTask {
    device_id: String,
    task_type: String,
    scheduled_date: i64,
    estimated_hours: f64,
    required_parts: Vec<String>,
    priority: Priority,
    risk_level: RiskLevel,
}
```

### 3.3 API Surface

```rust
// GMP LLM Agent Trait (for integration)
trait DevicePredictorAgent {
    /// Analyze device and return prediction
    async fn predict(&self, device_id: &str, lookback_hours: u64) 
        -> Result<PredictionResult>;
    
    /// Generate maintenance plan for a device
    async fn generate_plan(&self, device_id: &str) 
        -> Result<Vec<MaintenanceTask>>;
    
    /// Batch predict for all monitored devices
    async fn batch_predict(&self) 
        -> Result<Vec<PredictionResult>>;
}
```

---

## 4. Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| AC1 | Prediction accuracy > 80% | Backtest on 3 months historical data |
| AC2 | Support 5+ device types | Unit tested with mock data per type |
| AC3 | Generate actionable maintenance suggestions | Integration test with LLM output |
| AC4 | RUL estimation within ±20% | Compare against actual failure data |
| AC5 | Response time < 5s per device | Benchmark with 100 devices |
| AC6 | Integration with GMP Audit Chain | Write audit events for predictions |

---

## 5. Technical Constraints

- **No heavy ML training**: Use rule-based + statistical methods
- **Embedded deployment**: Must run on existing GMP-Platform infrastructure
- **Offline-first**: Core logic works without cloud connectivity
- **Audit compliance**: All predictions logged to audit chain with full reasoning

---

## 6. Out of Scope (v1.0)

- Deep learning models
- Real-time streaming analytics
- Mobile app integration
- Parts inventory optimization
