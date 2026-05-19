use sqlrustgo_crash_sim::{
    CrashPoint, CrashScenario, CrashSeverity,
    CrashTestResult, CrashCampaignConfig,
    scenario::CrashScenarioBuilder,
};

#[test]
fn test_crash_point_all_count() {
    let points = CrashPoint::all();
    assert_eq!(points.len(), 11);
}

#[test]
fn test_crash_point_name() {
    assert_eq!(CrashPoint::WalWrite.name(), "wal_write");
    assert_eq!(CrashPoint::WalFlush.name(), "wal_flush");
    assert_eq!(CrashPoint::TransactionCommit.name(), "transaction_commit");
    assert_eq!(CrashPoint::Checkpoint.name(), "checkpoint");
}

#[test]
fn test_crash_point_description() {
    assert!(CrashPoint::WalWrite.description().contains("WAL"));
    assert!(CrashPoint::TransactionCommit.description().contains("commit"));
}

#[test]
fn test_crash_scenario_at() {
    let scenario = CrashScenario::at(CrashPoint::WalFlush);
    assert_eq!(scenario.crash_point, CrashPoint::WalFlush);
    assert!(!scenario.is_random);
}

#[test]
fn test_crash_scenario_builder() {
    let scenario = CrashScenarioBuilder::new()
        .inject_at(CrashPoint::Checkpoint)
        .with_severity(CrashSeverity::PowerLoss)
        .deterministic(42)
        .build();

    assert_eq!(scenario.crash_point, CrashPoint::Checkpoint);
    assert_eq!(scenario.severity, CrashSeverity::PowerLoss);
    assert!(!scenario.is_random);
}

#[test]
fn test_crash_severity_as_str() {
    assert_eq!(CrashSeverity::HardKill.as_str(), "hard_kill");
    assert_eq!(CrashSeverity::PowerLoss.as_str(), "power_loss");
    assert_eq!(CrashSeverity::WriteFault.as_str(), "write_fault");
}

#[test]
fn test_crash_test_result_success() {
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
    assert!(result.summary().contains("recovered=true"));
}

#[test]
fn test_crash_test_result_failure() {
    let result = CrashTestResult {
        scenario: CrashScenario::at(CrashPoint::TransactionCommit),
        recovered: true,
        data_integrity_ok: true,
        wal_replay_ok: false,
        audit_chain_intact: true,
        signature_valid: true,
        phantom_commits: 0,
        lost_writes: 0,
        recovery_time_ms: 100,
        error_message: Some("WAL replay failed".to_string()),
    };

    assert!(!result.is_success());
}

#[test]
fn test_crash_campaign_config_default() {
    let config = CrashCampaignConfig::default();
    assert_eq!(config.iterations_per_point, 100);
    assert!(config.verify_audit_chain);
    assert!(config.verify_signatures);
}
