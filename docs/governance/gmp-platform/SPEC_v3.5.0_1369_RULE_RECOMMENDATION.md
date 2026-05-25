# SPEC: Rule Recommendation Engine (#1369)

## 1. Concept & Vision

An AI engine that learns from historical GMP violations and proactively recommends new compliance rules. Instead of waiting for repeated violations, the system identifies patterns and suggests preventive rules before problems recur.

**Design Philosophy**: "Rules should evolve with reality" — continuous learning from operational data, not static compliance theater.

---

## 2. System Architecture

### 2.1 Agent Structure

```
RuleRecommendationEngine
├── ViolationAnalyzer    # Pattern detection in historical violations
├── SimilarityEngine     # Rule clustering and gap analysis
├── TrendDetector        # Emerging risk identification
├── RecommendationGen    # LLM-powered rule drafting
└── ApprovalWorkflow     # Human-in-the-loop review
```

---

## 3. Functionality Specification

### 3.1 Core Features

#### F1: Violation Pattern Analysis
- **Input**: Historical violation records (deviations, CAPAs, audit findings)
- **Processing**:
  - Cluster by violation type, location, time, product
  - Identify recurring themes via NLP topic modeling
  - Calculate violation frequency and severity trends
- **Output**: Violation pattern reports

#### F2: Rule Similarity Analysis
- **Approach**:
  - Existing rules as vectors (semantic embedding)
  - New violation patterns compared against rule embeddings
  - Gap detection: "No rule covers this pattern"
- **Output**: Coverage map of rules vs violation types

#### F3: Trend Detection
- **Emerging Risk Indicators**:
  - New violation type appearing (first time)
  - Spike in specific violation category
  - New equipment/location/product with violations
  - Seasonal patterns (Q4 regulatory stress)
- **Output**: Risk alerts with supporting data

#### F4: Rule Recommendation Generation
- **Input**: Violation pattern + similarity analysis + trend data
- **Output**: Draft rule in structured format:
  ```json
  {
    "title": "Equipment Calibration Frequency Enhancement",
    "scope": "All critical process equipment",
    "requirement": "Monthly calibration verification",
    "rationale": "3 calibration failures in Q1 2026 on fermenters",
    "evidence": ["violation_ids", "pattern_analysis"],
    "proposed_by": "RuleRecommendationEngine",
    "confidence": 0.85
  }
  ```

#### F5: Approval Workflow
- **States**: Draft → UnderReview → Approved/Rejected → Implemented
- **Reviewers**: Compliance officer + relevant SME
- **Audit Trail**: Full history of recommendations and decisions

### 3.2 Data Model

```rust
struct ViolationPattern {
    pattern_id: String,
    cluster_type: String,           // Equipment/Human/Process/Material
    description: String,
    frequency: u32,                 // Occurrences in lookback period
    severity_distribution: HashMap<Severity, u32>,
    related_rules: Vec<String>,     // Existing rules that didn't prevent it
    gap_score: f64,                 // How uncovered this pattern is
}

struct RuleRecommendation {
    recommendation_id: String,
    title: String,
    scope: String,
    requirement: String,
    rationale: String,
    supporting_patterns: Vec<String>,
    supporting_violations: Vec<String>,
    confidence: f64,                // 0-1
    proposed_at: i64,
    status: RecommendationStatus,
    review_history: Vec<ReviewRecord>,
}

struct RuleSimilarityResult {
    existing_rule: String,
    new_pattern: String,
    similarity_score: f64,          // 0-1
    coverage_gap: f64,              // How much gap exists
}
```

### 3.3 API Surface

```rust
trait RuleRecommendationEngine {
    /// Analyze violations and generate recommendations
    async fn analyze_and_recommend(&self, lookback_days: u64) 
        -> Result<Vec<RuleRecommendation>>;
    
    /// Get rule coverage analysis
    async fn coverage_analysis(&self) -> Result<CoverageReport>;
    
    /// Get emerging risk alerts
    async fn emerging_risks(&self) -> Result<Vec<RiskAlert>>;
    
    /// Submit recommendation for review
    async fn submit_for_review(&self, recommendation_id: &str) 
        -> Result<()>;
    
    /// Approve/reject recommendation
    async fn review_decision(&self, recommendation_id: &str, decision: Decision) 
        -> Result<()>;
}
```

---

## 4. Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| AC1 | Recommendation accuracy > 70% | Track approved vs rejected recommendations |
| AC2 | Support rule clustering by semantic similarity | Unit tests with known rule sets |
| AC3 | Provide recommendation reasoning | Manual review of rationale quality |
| AC4 | Integrate with approval workflow | Integration test |
| AC5 | False positive rate < 15% | Track rejected recommendations |
| AC6 | Process 1000 violations in < 30s | Performance benchmark |

---

## 5. LLM Prompt Strategy

```
System: You are a GMP compliance expert. Recommend new rules based on 
        violation patterns. Be specific and actionable.
        Prefer preventive rules over detective ones.

User: Analyze this violation pattern and recommend a rule:
Pattern: {pattern_description}
Frequency: {count} occurrences in {period}
Severity: {severity_distribution}
Related violations: {violation_ids}

Existing rules that partially cover this:
{existing_rules}

Generate a rule recommendation with:
1. Clear title
2. Specific scope
3. Precise requirement (not vague)
4. Rationale citing the data
5. Confidence level (high/medium/low)
```

---

## 6. Out of Scope (v1.0)

- Automatic rule implementation
- Integration with QMS systems
- Machine learning model training
- Cross-plant benchmarking
- Regulatory intelligence (automatic rule updates from regulatory bodies)
