## ADDED Requirements

### Requirement: Immutable audit trail
The system SHALL record all compliance judgments to an immutable audit log with WAL (Write-Ahead Log) persistence.

#### Scenario: Record judgment to audit log
- **WHEN** a compliance judgment is completed
- **THEN** the system SHALL append an AuditLogEntry to the WAL containing: timestamp, operation_id, rule_id, judgment_result, llm_model, latency_ms

#### Scenario: Audit log persistence
- **WHEN** the system recovers from crash
- **THEN** all committed audit log entries SHALL be recoverable

### Requirement: Audit log query
The system SHALL support querying audit logs by operation_id, time range, and judgment status.

#### Scenario: Query by operation
- **WHEN** audit log is queried by operation_id
- **THEN** the system SHALL return all audit entries for that operation in chronological order

#### Scenario: Query by time range
- **WHEN** audit log is queried by time range
- **THEN** the system SHALL return all audit entries within that time range

### Requirement: Audit log integrity
The system SHALL verify audit log integrity using cryptographic hash chain.

#### Scenario: Detect tampering
- **WHEN** an audit log entry has been modified
- **THEN** the hash verification SHALL fail and log a SecurityEvent
