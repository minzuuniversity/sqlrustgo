use sqlrustgo_wal_verification::{
    PropertySeverity, WALInvariant, WALInvariantResult, WALLog, WALOperation, WALProperty,
    WALRecord, WALState,
};

#[test]
fn test_wal_operation_is_dml() {
    assert!(WALOperation::WriteRow.is_dml());
    assert!(WALOperation::DeleteRow.is_dml());
    assert!(WALOperation::UpdateRow.is_dml());
    assert!(!WALOperation::BeginTxn.is_dml());
    assert!(!WALOperation::CommitTxn.is_dml());
}

#[test]
fn test_wal_operation_is_boundary() {
    assert!(WALOperation::BeginTxn.is_boundary());
    assert!(WALOperation::CommitTxn.is_boundary());
    assert!(WALOperation::AbortTxn.is_boundary());
    assert!(!WALOperation::WriteRow.is_boundary());
}

#[test]
fn test_wal_state_default() {
    let state = WALState::default();
    assert_eq!(state, WALState::Idle);
}

#[test]
fn test_wal_property_all_count() {
    let props = WALProperty::all();
    assert_eq!(props.len(), 8);
}

#[test]
fn test_wal_property_critical_count() {
    let critical = WALProperty::critical();
    assert!(critical.len() >= 7);
}

#[test]
fn test_wal_property_severity() {
    assert_eq!(
        WALProperty::IdempotentReplay.severity(),
        PropertySeverity::Critical
    );
    assert_eq!(
        WALProperty::AtomicWrites.severity(),
        PropertySeverity::Critical
    );
    assert_eq!(
        WALProperty::LSNMonotonicity.severity(),
        PropertySeverity::Critical
    );
    assert_eq!(
        WALProperty::AuditChainContinuity.severity(),
        PropertySeverity::Important
    );
}

#[test]
fn test_wal_record_valid() {
    let record = WALRecord {
        lsn: 100,
        txn_id: 1,
        operation: WALOperation::WriteRow,
        page_id: Some(1),
        row_key: Some(vec![1, 2, 3]),
        before_value: None,
        after_value: Some(vec![4, 5, 6]),
        checksum: 0xABCD,
    };
    assert!(record.is_valid());
}

#[test]
fn test_wal_record_invalid() {
    let record = WALRecord {
        lsn: 0,
        txn_id: 1,
        operation: WALOperation::WriteRow,
        page_id: None,
        row_key: None,
        before_value: None,
        after_value: None,
        checksum: 0,
    };
    assert!(!record.is_valid());
}

#[test]
fn test_wal_log_new() {
    let log = WALLog::new();
    assert!(log.is_empty());
    assert_eq!(log.len(), 0);
}

#[test]
fn test_wal_log_lsn_range() {
    let mut log = WALLog::new();
    log.records.push(WALRecord {
        lsn: 100,
        txn_id: 1,
        operation: WALOperation::WriteRow,
        page_id: None,
        row_key: None,
        before_value: None,
        after_value: None,
        checksum: 0,
    });
    log.records.push(WALRecord {
        lsn: 200,
        txn_id: 1,
        operation: WALOperation::CommitTxn,
        page_id: None,
        row_key: None,
        before_value: None,
        after_value: None,
        checksum: 0,
    });

    assert_eq!(log.lsn_range(), Some((100, 200)));
}

#[test]
fn test_wal_log_check_lsn_monotonicity() {
    let mut log = WALLog::new();
    log.records.push(WALRecord {
        lsn: 100,
        txn_id: 1,
        operation: WALOperation::WriteRow,
        page_id: None,
        row_key: None,
        before_value: None,
        after_value: None,
        checksum: 0,
    });
    log.records.push(WALRecord {
        lsn: 200,
        txn_id: 1,
        operation: WALOperation::CommitTxn,
        page_id: None,
        row_key: None,
        before_value: None,
        after_value: None,
        checksum: 0,
    });

    assert!(log.check_lsn_monotonicity());
}

#[test]
fn test_wal_log_check_lsn_monotonicity_failure() {
    let mut log = WALLog::new();
    log.records.push(WALRecord {
        lsn: 200,
        txn_id: 1,
        operation: WALOperation::WriteRow,
        page_id: None,
        row_key: None,
        before_value: None,
        after_value: None,
        checksum: 0,
    });
    log.records.push(WALRecord {
        lsn: 100,
        txn_id: 1,
        operation: WALOperation::CommitTxn,
        page_id: None,
        row_key: None,
        before_value: None,
        after_value: None,
        checksum: 0,
    });

    assert!(!log.check_lsn_monotonicity());
}

#[test]
fn test_wal_invariant_no_duplicate_lsn() {
    let mut log = WALLog::new();
    log.records.push(WALRecord {
        lsn: 100,
        txn_id: 1,
        operation: WALOperation::WriteRow,
        page_id: None,
        row_key: None,
        before_value: None,
        after_value: None,
        checksum: 0,
    });
    log.records.push(WALRecord {
        lsn: 100,
        txn_id: 1,
        operation: WALOperation::CommitTxn,
        page_id: None,
        row_key: None,
        before_value: None,
        after_value: None,
        checksum: 0,
    });

    let result = WALInvariant::NoDuplicateLSN.check(&log);
    assert!(!result.holds);
    assert!(result.failed_at.is_some());
}
