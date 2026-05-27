//! Confidence Scoring
//!
//! Calculates confidence scores for compliance judgments based on logprob analysis.

use serde::{Deserialize, Serialize};

/// Confidence score with explanation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceScore {
    /// Score between 0.0 and 1.0
    pub score: f64,
    /// Human-readable explanation
    pub explanation: String,
    /// Log probability if available
    pub logprob: Option<f64>,
}

/// Threshold for NEEDS_REVIEW status
pub const CONFIDENCE_THRESHOLD: f64 = 0.7;

/// Confidence calculator using logprob analysis
pub struct ConfidenceCalculator;

impl ConfidenceCalculator {
    /// Calculate confidence score from log probability
    pub fn from_logprob(logprob: f64) -> ConfidenceScore {
        // Convert logprob to probability
        let probability = logprob.exp();

        // Scale to 0.0-1.0 range with calibration
        let score = probability.max(0.0).min(1.0);

        let explanation = match score {
            s if s >= 0.9 => "High confidence in judgment".to_string(),
            s if s >= 0.7 => "Medium-high confidence in judgment".to_string(),
            s if s >= 0.5 => "Medium confidence - manual review recommended".to_string(),
            _ => "Low confidence - manual review required".to_string(),
        };

        ConfidenceScore {
            score,
            explanation,
            logprob: Some(logprob),
        }
    }

    /// Create confidence score without logprob (from text analysis)
    pub fn from_text_analysis(analysis: &str) -> ConfidenceScore {
        let score = Self::analyze_text_score(analysis);
        let explanation = Self::generate_explanation(analysis, score);

        ConfidenceScore {
            score,
            explanation,
            logprob: None,
        }
    }

    fn analyze_text_score(analysis: &str) -> f64 {
        let analysis_lower = analysis.to_lowercase();

        // Positive indicators
        let positive_count = ["certain", "definitely", "clearly", "obviously", "certainly"]
            .iter()
            .filter(|w| analysis_lower.contains(*w))
            .count() as f64;

        // Negative indicators
        let negative_count = ["uncertain", "maybe", "perhaps", "possibly", "might", "unclear"]
            .iter()
            .filter(|w| analysis_lower.contains(*w))
            .count() as f64;

        // Neutral indicators
        let neutral_count = ["could", "would", "should"]
            .iter()
            .filter(|w| analysis_lower.contains(*w))
            .count() as f64;

        // Calculate base score
        let base = 0.5 + (positive_count * 0.1) - (negative_count * 0.15) - (neutral_count * 0.05);

        base.max(0.0).min(1.0)
    }

    fn generate_explanation(analysis: &str, score: f64) -> String {
        match score {
            s if s >= 0.9 => "Very high confidence based on clear evidence".to_string(),
            s if s >= 0.7 => "High confidence in judgment".to_string(),
            s if s >= 0.5 => format!("Medium confidence. Analysis: {}", &analysis[..analysis.len().min(100)]),
            _ => format!("Low confidence - manual review required. Analysis: {}", &analysis[..analysis.len().min(100)]),
        }
    }

    /// Check if confidence is below threshold
    pub fn is_below_threshold(score: &ConfidenceScore) -> bool {
        score.score < CONFIDENCE_THRESHOLD
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_logprob() {
        let score = ConfidenceCalculator::from_logprob(-0.1);
        assert!(score.score > 0.9);
        assert!(score.logprob.is_some());
    }

    #[test]
    fn test_from_logprob_low() {
        let score = ConfidenceCalculator::from_logprob(-1.0);
        assert!(score.score < 0.5);
    }

    #[test]
    fn test_threshold() {
        let low_score = ConfidenceCalculator::from_logprob(-1.0);
        assert!(ConfidenceCalculator::is_below_threshold(&low_score));

        let high_score = ConfidenceCalculator::from_logprob(-0.1);
        assert!(!ConfidenceCalculator::is_below_threshold(&high_score));
    }

    #[test]
    fn test_text_analysis_positive() {
        let score = ConfidenceCalculator::from_text_analysis("This is definitely compliant");
        assert!(score.score > 0.5);
    }

    #[test]
    fn test_text_analysis_uncertain() {
        let score = ConfidenceCalculator::from_text_analysis("This might be compliant perhaps");
        assert!(score.score < 0.7);
    }
}
