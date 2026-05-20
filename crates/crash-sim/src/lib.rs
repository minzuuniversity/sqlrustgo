//! Crash Simulation Framework for SQLRustGo
//!
//! Provides deterministic and randomized crash injection for testing
//! database recovery correctness.

pub mod crash_point;
pub mod runner;
pub mod scenario;
pub mod verifier;

pub use crash_point::CrashPoint;
pub use runner::CrashTestRunner;
pub use scenario::CrashScenario;
pub use verifier::RecoveryVerifier;

/// Severity level of crash simulation
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CrashSeverity {
    HardKill,
    WriteFault,
    PowerLoss,
    DiskFull,
    NetworkPartition,
}

impl CrashSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::HardKill => "hard_kill",
            Self::WriteFault => "write_fault",
            Self::PowerLoss => "power_loss",
            Self::DiskFull => "disk_full",
            Self::NetworkPartition => "network_partition",
        }
    }
}

/// Result of a crash simulation test
#[derive(Debug, Clone)]
pub struct CrashTestResult {
    pub scenario: CrashScenario,
    pub recovered: bool,
    pub data_integrity_ok: bool,
    pub wal_replay_ok: bool,
    pub audit_chain_intact: bool,
    pub signature_valid: bool,
    pub phantom_commits: u32,
    pub lost_writes: u32,
    pub recovery_time_ms: u64,
    pub error_message: Option<String>,
}

impl CrashTestResult {
    pub fn is_success(&self) -> bool {
        self.recovered
            && self.data_integrity_ok
            && self.wal_replay_ok
            && self.audit_chain_intact
            && self.phantom_commits == 0
            && self.lost_writes == 0
    }

    pub fn summary(&self) -> String {
        if self.is_success() {
            format!(
                "recovered={}, data_ok={}, wal_ok={}, audit_ok={}, phantom={}, lost={}, time={}ms",
                self.recovered,
                self.data_integrity_ok,
                self.wal_replay_ok,
                self.audit_chain_intact,
                self.phantom_commits,
                self.lost_writes,
                self.recovery_time_ms
            )
        } else {
            format!(
                "recovered={}, error={:?}",
                self.recovered,
                self.error_message.as_ref().map(|s| &s[..])
            )
        }
    }
}

/// Configuration for crash campaign
#[derive(Debug, Clone)]
pub struct CrashCampaignConfig {
    pub iterations_per_point: u32,
    pub random_probability: f64,
    pub recovery_timeout_ms: u64,
    pub verify_audit_chain: bool,
    pub verify_signatures: bool,
}

impl Default for CrashCampaignConfig {
    fn default() -> Self {
        Self {
            iterations_per_point: 100,
            random_probability: 0.05,
            recovery_timeout_ms: 30_000,
            verify_audit_chain: true,
            verify_signatures: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crash_severity_all_variants() {
        assert_eq!(CrashSeverity::HardKill.as_str(), "hard_kill");
        assert_eq!(CrashSeverity::WriteFault.as_str(), "write_fault");
        assert_eq!(CrashSeverity::PowerLoss.as_str(), "power_loss");
        assert_eq!(CrashSeverity::DiskFull.as_str(), "disk_full");
        assert_eq!(
            CrashSeverity::NetworkPartition.as_str(),
            "network_partition"
        );
    }

    #[test]
    fn test_crash_severity_as_str_no_spaces() {
        for severity in &[
            CrashSeverity::HardKill,
            CrashSeverity::WriteFault,
            CrashSeverity::PowerLoss,
            CrashSeverity::DiskFull,
            CrashSeverity::NetworkPartition,
        ] {
            let s = severity.as_str();
            assert!(
                !s.contains(' '),
                "as_str() '{}' should not contain spaces",
                s
            );
            assert!(s.chars().all(|c| c.is_lowercase() || c == '_'));
        }
    }

    #[test]
    fn test_crash_severity_clone() {
        let original = CrashSeverity::HardKill;
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_crash_test_result_is_success_all_true() {
        let result = CrashTestResult {
            scenario: CrashScenario::at(CrashPoint::WalWrite),
            recovered: true,
            data_integrity_ok: true,
            wal_replay_ok: true,
            audit_chain_intact: true,
            signature_valid: true,
            phantom_commits: 0,
            lost_writes: 0,
            recovery_time_ms: 100,
            error_message: None,
        };
        assert!(result.is_success());
    }

    #[test]
    fn test_crash_test_result_is_success_false_on_recovered() {
        let result = CrashTestResult {
            scenario: CrashScenario::at(CrashPoint::WalWrite),
            recovered: false,
            data_integrity_ok: true,
            wal_replay_ok: true,
            audit_chain_intact: true,
            signature_valid: true,
            phantom_commits: 0,
            lost_writes: 0,
            recovery_time_ms: 0,
            error_message: Some("crash".to_string()),
        };
        assert!(!result.is_success());
    }

    #[test]
    fn test_crash_test_result_is_success_false_on_phantom_commits() {
        let result = CrashTestResult {
            scenario: CrashScenario::at(CrashPoint::TransactionCommit),
            recovered: true,
            data_integrity_ok: true,
            wal_replay_ok: true,
            audit_chain_intact: true,
            signature_valid: true,
            phantom_commits: 5,
            lost_writes: 0,
            recovery_time_ms: 100,
            error_message: None,
        };
        assert!(!result.is_success());
    }

    #[test]
    fn test_crash_test_result_is_success_false_on_lost_writes() {
        let result = CrashTestResult {
            scenario: CrashScenario::at(CrashPoint::WalFlush),
            recovered: true,
            data_integrity_ok: true,
            wal_replay_ok: true,
            audit_chain_intact: true,
            signature_valid: true,
            phantom_commits: 0,
            lost_writes: 3,
            recovery_time_ms: 100,
            error_message: None,
        };
        assert!(!result.is_success());
    }

    #[test]
    fn test_crash_test_result_summary_success() {
        let result = CrashTestResult {
            scenario: CrashScenario::at(CrashPoint::Checkpoint),
            recovered: true,
            data_integrity_ok: true,
            wal_replay_ok: true,
            audit_chain_intact: true,
            signature_valid: true,
            phantom_commits: 0,
            lost_writes: 0,
            recovery_time_ms: 50,
            error_message: None,
        };
        let summary = result.summary();
        assert!(summary.contains("recovered=true"));
        assert!(summary.contains("data_ok=true"));
        assert!(summary.contains("wal_ok=true"));
        assert!(summary.contains("phantom=0"));
        assert!(summary.contains("lost=0"));
    }

    #[test]
    fn test_crash_test_result_summary_failure() {
        let result = CrashTestResult {
            scenario: CrashScenario::at(CrashPoint::WalWrite),
            recovered: false,
            data_integrity_ok: false,
            wal_replay_ok: false,
            audit_chain_intact: false,
            signature_valid: false,
            phantom_commits: 0,
            lost_writes: 0,
            recovery_time_ms: 0,
            error_message: Some("disk error".to_string()),
        };
        let summary = result.summary();
        assert!(summary.contains("recovered=false"));
        assert!(summary.contains("error"));
    }

    #[test]
    fn test_crash_test_result_clone() {
        let original = CrashTestResult {
            scenario: CrashScenario::at(CrashPoint::WalWrite),
            recovered: true,
            data_integrity_ok: true,
            wal_replay_ok: true,
            audit_chain_intact: true,
            signature_valid: true,
            phantom_commits: 0,
            lost_writes: 0,
            recovery_time_ms: 100,
            error_message: None,
        };
        let cloned = original.clone();
        assert_eq!(original.recovered, cloned.recovered);
        assert_eq!(original.data_integrity_ok, cloned.data_integrity_ok);
    }

    #[test]
    fn test_crash_campaign_config_default() {
        let config = CrashCampaignConfig::default();
        assert_eq!(config.iterations_per_point, 100);
        assert_eq!(config.random_probability, 0.05);
        assert_eq!(config.recovery_timeout_ms, 30_000);
        assert!(config.verify_audit_chain);
        assert!(config.verify_signatures);
    }

    #[test]
    fn test_crash_campaign_config_clone() {
        let original = CrashCampaignConfig::default();
        let cloned = original.clone();
        assert_eq!(original.iterations_per_point, cloned.iterations_per_point);
        assert_eq!(original.random_probability, cloned.random_probability);
        assert_eq!(original.verify_audit_chain, cloned.verify_audit_chain);
    }
}
