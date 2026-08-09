## Why

V312-14 verifies SQLRustGo crash recovery and upgrade/downgrade paths:
- kill -9 recovery
- WAL replay
- Dirty page recovery
- Backup/restore checksum
- v3.10/v3.11 fixture upgrade to v3.12
- Rollback verification

## What Changes

### Crash Recovery Verification
- Run kill -9 tests
- Verify WAL replay works
- Verify dirty page recovery

### Backup/Restore Verification
- Verify checksum integrity
- Test upgrade path from v3.10/v3.11 fixtures

### Rollback Verification
- Test rollback from v3.12 to v3.11/v3.10

## Capabilities

### New Capabilities
- `crash-recovery-verification`: Documented crash recovery status
- `upgrade-path-verification`: Documented upgrade/downgrade status

## Impact

### Affected Modules
- `crates/storage` - WAL, recovery
- `crates/admin` - Backup/restore
