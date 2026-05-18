//! CrashScenario defines a single crash scenario to execute

use crate::{CrashPoint, CrashSeverity, CrashCampaignConfig};
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
