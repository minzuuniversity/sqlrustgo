//! CrashTestRunner executes crash scenarios and collects results

use crate::{CrashCampaignConfig, CrashScenario, CrashTestResult, RecoveryVerifier};
use rand::Rng;
use std::time::{Duration, Instant};

/// Runs crash simulation scenarios
pub struct CrashTestRunner {
    config: CrashCampaignConfig,
    results: Vec<CrashTestResult>,
}

impl CrashTestRunner {
    pub fn new(config: CrashCampaignConfig) -> Self {
        Self {
            config,
            results: Vec::new(),
        }
    }

    /// Run a single crash scenario
    pub fn run_scenario(&mut self, scenario: &CrashScenario) -> CrashTestResult {
        let start = Instant::now();
        let mut result = CrashTestResult {
            scenario: scenario.clone(),
            recovered: false,
            data_integrity_ok: false,
            wal_replay_ok: false,
            audit_chain_intact: false,
            signature_valid: false,
            phantom_commits: 0,
            lost_writes: 0,
            recovery_time_ms: 0,
            error_message: None,
        };

        // Execute crash
        let crashed = self.execute_crash(scenario);
        result.recovered = crashed;

        // If we crashed, try to recover
        if crashed {
            let recovery_start = Instant::now();
            let recovered = self.execute_recovery(scenario);
            result.recovery_time_ms = recovery_start.elapsed().as_millis() as u64;

            if recovered {
                // Verify data integrity
                let verifier = RecoveryVerifier::new();
                verifier.verify(&mut result);
            } else {
                result.error_message = Some("Recovery failed".to_string());
            }
        } else {
            result.error_message = Some("Crash injection did not trigger".to_string());
        }

        result
    }

    /// Run a full campaign (all crash points x iterations)
    pub fn run_campaign(&mut self) -> Vec<CrashTestResult> {
        let scenarios = CrashScenario::from_config(&self.config);

        for scenario in scenarios {
            let result = self.run_scenario(&scenario);
            self.results.push(result);
        }

        self.results.clone()
    }

    /// Run N random scenarios
    pub fn run_random(&mut self, count: u32) -> Vec<CrashTestResult> {
        for _ in 0..count {
            let scenario = CrashScenario::generate_random();
            let result = self.run_scenario(&scenario);
            self.results.push(result);
        }

        self.results.clone()
    }

    /// Execute the actual crash injection
    fn execute_crash(&self, scenario: &CrashScenario) -> bool {
        // This is a framework - actual implementation would:
        // 1. For testing: use mock/fake storage that tracks crash points
        // 2. For integration testing: use fault injection
        // 3. For chaos testing: actually kill the process

        let mut rng = rand::thread_rng();

        if scenario.is_random {
            rng.gen_bool(scenario.probability)
        } else {
            true // Deterministic - always crash
        }
    }

    /// Execute recovery
    fn execute_recovery(&self, scenario: &CrashScenario) -> bool {
        // This is a framework - actual implementation would:
        // 1. Attempt to open the database
        // 2. Run WAL replay
        // 3. Verify integrity

        // For framework testing, always succeed
        true
    }

    /// Get summary statistics
    pub fn summary(&self) -> CrashCampaignSummary {
        let total = self.results.len();
        let successes = self.results.iter().filter(|r| r.is_success()).count();
        let failures = total - successes;

        CrashCampaignSummary {
            total_scenarios: total,
            successes,
            failures,
            success_rate: if total > 0 {
                successes as f64 / total as f64
            } else {
                0.0
            },
        }
    }

    /// Print report to stdout
    pub fn print_report(&self) {
        let summary = self.summary();

        println!("=== Crash Simulation Campaign Report ===");
        println!("Total scenarios: {}", summary.total_scenarios);
        println!("Successes: {}", summary.successes);
        println!("Failures: {}", summary.failures);
        println!("Success rate: {:.1}%", summary.success_rate * 100.0);
        println!();

        if summary.failures > 0 {
            println!("=== Failed Scenarios ===");
            for (i, result) in self.results.iter().enumerate() {
                if !result.is_success() {
                    println!("  {}. {:?}", i + 1, result.scenario);
                    println!("     {}", result.summary());
                }
            }
        }
    }
}

impl Default for CrashTestRunner {
    fn default() -> Self {
        Self::new(CrashCampaignConfig::default())
    }
}

/// Summary of a crash campaign
#[derive(Debug, Clone)]
pub struct CrashCampaignSummary {
    pub total_scenarios: usize,
    pub successes: usize,
    pub failures: usize,
    pub success_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CrashPoint;

    #[test]
    fn test_crash_test_runner_new() {
        let config = CrashCampaignConfig::default();
        let runner = CrashTestRunner::new(config);
        assert!(runner.results.is_empty());
    }

    #[test]
    fn test_crash_test_runner_run_scenario() {
        let config = CrashCampaignConfig::default();
        let mut runner = CrashTestRunner::new(config);
        let scenario = CrashScenario::at(CrashPoint::WalWrite);
        let result = runner.run_scenario(&scenario);
        // Framework always succeeds in execute_crash/execute_recovery
        assert!(result.recovered);
    }

    #[test]
    fn test_crash_test_runner_run_random() {
        let config = CrashCampaignConfig::default();
        let mut runner = CrashTestRunner::new(config);
        let results = runner.run_random(5);
        assert_eq!(results.len(), 5);
        assert_eq!(runner.results.len(), 5);
    }

    #[test]
    fn test_crash_test_runner_summary_empty() {
        let config = CrashCampaignConfig::default();
        let runner = CrashTestRunner::new(config);
        let summary = runner.summary();
        assert_eq!(summary.total_scenarios, 0);
        assert_eq!(summary.successes, 0);
        assert_eq!(summary.failures, 0);
        assert_eq!(summary.success_rate, 0.0);
    }

    #[test]
    fn test_crash_test_runner_summary_with_results() {
        let config = CrashCampaignConfig {
            iterations_per_point: 1,
            random_probability: 0.05,
            recovery_timeout_ms: 30_000,
            verify_audit_chain: true,
            verify_signatures: true,
        };
        let mut runner = CrashTestRunner::new(config);
        // run_campaign populates self.results
        let results = runner.run_campaign();
        let summary = runner.summary();
        // 11 crash points * 1 iteration = 11 scenarios
        assert_eq!(results.len(), 11);
        assert_eq!(summary.total_scenarios, 11);
        // successes + failures should equal total
        assert_eq!(summary.successes + summary.failures, 11);
        // Success rate should be between 0 and 1
        assert!(summary.success_rate >= 0.0 && summary.success_rate <= 1.0);
    }

    #[test]
    fn test_crash_campaign_summary_debug() {
        let summary = CrashCampaignSummary {
            total_scenarios: 10,
            successes: 8,
            failures: 2,
            success_rate: 0.8,
        };
        let debug_str = format!("{:?}", summary);
        assert!(debug_str.contains("10"));
        assert!(debug_str.contains("8"));
    }

    #[test]
    fn test_crash_test_runner_default() {
        let runner = CrashTestRunner::default();
        assert!(runner.results.is_empty());
    }
}
