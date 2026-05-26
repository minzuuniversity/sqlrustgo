//! Confidence scoring for LLM judgments
//!
//! Calculates confidence scores based on LLM response characteristics.

use super::types::JudgmentStatus;

/// Confidence score threshold below which human review is required
pub const CONFIDENCE_THRESHOLD: f32 = 0.7;

/// Calculator for judgment confidence scores
pub struct ConfidenceCalculator {
    /// Base confidence for high logprob responses
    high_logprob_threshold: f32,
    /// Base confidence for ambiguous responses
    ambiguous_penalty: f32,
}

impl Default for ConfidenceCalculator {
    fn default() -> Self {
        Self {
            high_logprob_threshold: -1.0,
            ambiguous_penalty: 0.3,
        }
    }
}

impl ConfidenceCalculator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate confidence score from LLM response
    ///
    /// Factors:
    /// - logprob: Higher logprob = more confident
    /// - response_clarity: Unambiguous responses get higher scores
    /// - rule_citation: Responses citing specific rules get higher scores
    pub fn calculate(
        &self,
        logprob: Option<f32>,
        response_clarity: ResponseClarity,
        rule_citations: usize,
    ) -> f32 {
        let mut score = 0.8; // Base score

        // Adjust for logprob
        if let Some(lp) = logprob {
            if lp > self.high_logprob_threshold {
                score += 0.15;
            } else if lp > -2.0 {
                score += 0.05;
            } else {
                score -= 0.2;
            }
        }

        // Adjust for response clarity
        match response_clarity {
            ResponseClarity::Clear => score += 0.05,
            ResponseClarity::Ambiguous => score -= self.ambiguous_penalty,
            ResponseClarity::Contradictory => score -= 0.4,
        }

        // Adjust for rule citations
        if rule_citations > 0 {
            score += 0.05_f32.min(rule_citations as f32 * 0.02);
        }

        // Clamp to [0.0, 1.0]
        score.clamp(0.0, 1.0)
    }

    /// Determine judgment status based on confidence score
    pub fn determine_status(
        &self,
        confidence_score: f32,
        response_clarity: ResponseClarity,
    ) -> JudgmentStatus {
        if confidence_score < CONFIDENCE_THRESHOLD
            || response_clarity == ResponseClarity::Contradictory
        {
            JudgmentStatus::NeedsReview
        } else {
            JudgmentStatus::Compliant
        }
    }
}

/// Response clarity classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseClarity {
    /// Response is clear and unambiguous
    Clear,
    /// Response contains ambiguity
    Ambiguous,
    /// Response contains contradictions
    Contradictory,
}

impl ResponseClarity {
    /// Classify response based on keywords
    pub fn from_response(response: &str) -> Self {
        let response_lower = response.to_lowercase();

        // Check for contradictory language
        let contradictory_markers = ["however", "but", "although", "despite", "contradicts"];
        if contradictory_markers
            .iter()
            .any(|m| response_lower.contains(m))
        {
            // If contradictory markers appear with definitive language, may be contradictory
            if response_lower.contains("must") || response_lower.contains("required") {
                return ResponseClarity::Contradictory;
            }
        }

        // Check for ambiguous language
        let ambiguous_markers = [
            "might",
            "could",
            "perhaps",
            "possibly",
            "unclear",
            "uncertain",
        ];
        if ambiguous_markers.iter().any(|m| response_lower.contains(m)) {
            return ResponseClarity::Ambiguous;
        }

        ResponseClarity::Clear
    }
}

/// Confidence score with explanation
#[derive(Debug, Clone)]
pub struct ConfidenceScore {
    /// Raw confidence score (0.0-1.0)
    pub score: f32,
    /// Human-readable explanation
    pub explanation: String,
    /// Contributing factors
    pub factors: Vec<ConfidenceFactor>,
}

/// Factors contributing to confidence score
#[derive(Debug, Clone)]
pub enum ConfidenceFactor {
    /// Based on LLM logprob
    Logprob { value: f32, contribution: f32 },
    /// Based on response clarity
    Clarity {
        clarity: ResponseClarity,
        contribution: f32,
    },
    /// Based on rule citations
    RuleCitations { count: usize, contribution: f32 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confidence_high_logprob_clear() {
        let calc = ConfidenceCalculator::new();
        let score = calc.calculate(Some(-0.5), ResponseClarity::Clear, 2);
        assert!(score > 0.9);
    }

    #[test]
    fn test_confidence_low_logprob_ambiguous() {
        let calc = ConfidenceCalculator::new();
        let score = calc.calculate(Some(-3.0), ResponseClarity::Ambiguous, 0);
        assert!(score < 0.5);
    }

    #[test]
    fn test_confidence_threshold() {
        let calc = ConfidenceCalculator::new();
        let status = calc.determine_status(0.8, ResponseClarity::Clear);
        assert_eq!(status, JudgmentStatus::Compliant);

        let status = calc.determine_status(0.6, ResponseClarity::Clear);
        assert_eq!(status, JudgmentStatus::NeedsReview);
    }

    #[test]
    fn test_response_clarity_classification() {
        assert_eq!(
            ResponseClarity::from_response("This is compliant. Must follow rules."),
            ResponseClarity::Clear
        );
        assert_eq!(
            ResponseClarity::from_response("This might be compliant."),
            ResponseClarity::Ambiguous
        );
        assert_eq!(
            ResponseClarity::from_response("Must comply, however may not be required."),
            ResponseClarity::Contradictory
        );
    }
}
