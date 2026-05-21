//! CrashScenario defines a single crash scenario to execute

use crate::{CrashCampaignConfig, CrashPoint, CrashSeverity};
use rand::Rng;
use serde::{Deserialize, Serialize};

/// A single crash scenario with configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrashScenario {
    /// The crash point
    pub crash_point: CrashPoint,
    /// The severity/type of crash
    pub severity: CrashSeverity,
    /// Probability of this crash (0.0-1.0), only used in random mode
    pub probability: f64,
    /// Whether to use random probability or deterministic
    pub is_random: bool,
    /// Optional seed for deterministic behavior
    pub seed: Option<u64>,
    /// Number of operations before crash (for deterministic mode)
    pub ops_before_crash: Option<u64>,
}

impl CrashScenario {
    /// Create a deterministic crash scenario at a specific point
    pub fn at(point: CrashPoint) -> Self {
        Self {
            crash_point: point,
            severity: CrashSeverity::HardKill,
            probability: 1.0,
            is_random: false,
            seed: None,
            ops_before_crash: None,
        }
    }

    /// Create a random crash scenario
    pub fn random() -> Self {
        Self {
            crash_point: CrashPoint::WalWrite,
            severity: CrashSeverity::HardKill,
            probability: 0.05,
            is_random: true,
            seed: None,
            ops_before_crash: None,
        }
    }

    /// Create from config
    pub fn from_config(config: &CrashCampaignConfig) -> Vec<CrashScenario> {
        let mut scenarios = Vec::new();

        for point in CrashPoint::all() {
            for _ in 0..config.iterations_per_point {
                scenarios.push(CrashScenario {
                    crash_point: *point,
                    severity: CrashSeverity::HardKill,
                    probability: config.random_probability,
                    is_random: true,
                    seed: None,
                    ops_before_crash: None,
                });
            }
        }

        scenarios
    }

    /// Generate a random scenario
    pub fn generate_random() -> Self {
        let mut rng = rand::thread_rng();
        let points = CrashPoint::all();
        let point = points[rng.gen_range(0..points.len())];
        let severities = [
            CrashSeverity::HardKill,
            CrashSeverity::PowerLoss,
            CrashSeverity::WriteFault,
        ];
        let severity = severities[rng.gen_range(0..severities.len())];

        Self {
            crash_point: point,
            severity,
            probability: rng.gen_range(0.01..0.2),
            is_random: true,
            seed: Some(rng.gen()),
            ops_before_crash: None,
        }
    }
}

/// Builder for CrashScenario
pub struct CrashScenarioBuilder {
    crash_point: CrashPoint,
    severity: CrashSeverity,
    probability: f64,
    is_random: bool,
    seed: Option<u64>,
    ops_before_crash: Option<u64>,
}

impl CrashScenarioBuilder {
    pub fn new() -> Self {
        Self {
            crash_point: CrashPoint::WalWrite,
            severity: CrashSeverity::HardKill,
            probability: 1.0,
            is_random: false,
            seed: None,
            ops_before_crash: None,
        }
    }

    pub fn inject_at(mut self, point: CrashPoint) -> Self {
        self.crash_point = point;
        self
    }

    pub fn with_severity(mut self, severity: CrashSeverity) -> Self {
        self.severity = severity;
        self
    }

    pub fn probability(mut self, p: f64) -> Self {
        self.probability = p;
        self.is_random = true;
        self
    }

    pub fn deterministic(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self.is_random = false;
        self
    }

    pub fn after_ops(mut self, n: u64) -> Self {
        self.ops_before_crash = Some(n);
        self.is_random = false;
        self
    }

    pub fn build(self) -> CrashScenario {
        CrashScenario {
            crash_point: self.crash_point,
            severity: self.severity,
            probability: self.probability,
            is_random: self.is_random,
            seed: self.seed,
            ops_before_crash: self.ops_before_crash,
        }
    }
}

impl Default for CrashScenarioBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for CrashScenario {
    fn default() -> Self {
        Self::random()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crash_scenario_at_is_deterministic() {
        let scenario = CrashScenario::at(CrashPoint::WalFlush);
        assert_eq!(scenario.crash_point, CrashPoint::WalFlush);
        assert_eq!(scenario.severity, CrashSeverity::HardKill);
        assert!(!scenario.is_random);
        assert_eq!(scenario.probability, 1.0);
    }

    #[test]
    fn test_crash_scenario_random_is_random() {
        let scenario = CrashScenario::random();
        assert!(scenario.is_random);
        assert!(scenario.probability > 0.0 && scenario.probability <= 1.0);
    }

    #[test]
    fn test_crash_scenario_from_config() {
        let config = CrashCampaignConfig {
            iterations_per_point: 2,
            random_probability: 0.1,
            recovery_timeout_ms: 5000,
            verify_audit_chain: true,
            verify_signatures: true,
        };
        let scenarios = CrashScenario::from_config(&config);
        // 11 crash points * 2 iterations = 22 scenarios
        assert_eq!(scenarios.len(), 22);
    }

    #[test]
    fn test_crash_scenario_generate_random() {
        let scenario = CrashScenario::generate_random();
        assert!(scenario.is_random);
        assert!(scenario.seed.is_some());
    }

    #[test]
    fn test_crash_scenario_builder_new() {
        let builder = CrashScenarioBuilder::new();
        assert_eq!(builder.crash_point, CrashPoint::WalWrite);
        assert_eq!(builder.severity, CrashSeverity::HardKill);
        assert!(!builder.is_random);
    }

    #[test]
    fn test_crash_scenario_builder_inject_at() {
        let scenario = CrashScenarioBuilder::new()
            .inject_at(CrashPoint::TransactionCommit)
            .build();
        assert_eq!(scenario.crash_point, CrashPoint::TransactionCommit);
    }

    #[test]
    fn test_crash_scenario_builder_with_severity() {
        let scenario = CrashScenarioBuilder::new()
            .with_severity(CrashSeverity::DiskFull)
            .build();
        assert_eq!(scenario.severity, CrashSeverity::DiskFull);
    }

    #[test]
    fn test_crash_scenario_builder_probability() {
        let scenario = CrashScenarioBuilder::new().probability(0.15).build();
        assert_eq!(scenario.probability, 0.15);
        assert!(scenario.is_random);
    }

    #[test]
    fn test_crash_scenario_builder_deterministic() {
        let scenario = CrashScenarioBuilder::new().deterministic(12345).build();
        assert_eq!(scenario.seed, Some(12345));
        assert!(!scenario.is_random);
    }

    #[test]
    fn test_crash_scenario_builder_after_ops() {
        let scenario = CrashScenarioBuilder::new().after_ops(100).build();
        assert_eq!(scenario.ops_before_crash, Some(100));
        assert!(!scenario.is_random);
    }

    #[test]
    fn test_crash_scenario_builder_chaining() {
        let scenario = CrashScenarioBuilder::new()
            .inject_at(CrashPoint::PageSplit)
            .with_severity(CrashSeverity::NetworkPartition)
            .probability(0.25)
            .deterministic(999)
            .after_ops(50)
            .build();

        assert_eq!(scenario.crash_point, CrashPoint::PageSplit);
        assert_eq!(scenario.severity, CrashSeverity::NetworkPartition);
        assert_eq!(scenario.probability, 0.25);
        assert_eq!(scenario.seed, Some(999));
        assert_eq!(scenario.ops_before_crash, Some(50));
    }

    #[test]
    fn test_crash_scenario_clone() {
        let original = CrashScenario::at(CrashPoint::Checkpoint);
        let cloned = original.clone();
        assert_eq!(original.crash_point, cloned.crash_point);
        assert_eq!(original.severity, cloned.severity);
    }

    #[test]
    fn test_crash_scenario_serialize() {
        let scenario = CrashScenario::at(CrashPoint::WalWrite);
        let json = serde_json::to_string(&scenario).unwrap();
        // JSON contains field "crash_point" with value like "WalWrite"
        assert!(json.contains("crash_point"));
    }
}
