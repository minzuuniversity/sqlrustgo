//! Crash Simulation Framework for SQLRustGo
//!
//! Provides deterministic and randomized crash injection for testing
//! database recovery correctness.

pub mod crash_point;
pub mod scenario;
pub mod verifier;
pub mod runner;

pub use crash_point::CrashPoint;
pub use scenario::CrashScenario;
pub use verifier::RecoveryVerifier;
pub use runner::CrashTestRunner;

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
            format!("recovered={}, error={:?}", self.recovered, self.error_message.as_ref().map(|s| &s[..]))
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
