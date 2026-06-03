# F-32 SPEC: mysqladmin Equivalent

> **Issue**: #2831
> **Version**: v3.8.0
> **Status**: COMPLETED

## 1. Background
v3.0.0 COMPLETE_LEGACY_TRACKING_REPORT.md defined F-32 (mysqladmin equivalent)
as ❌ unimplemented since v2.0.0.

## 2. Scope
8 mysqladmin subcommands: ping, status, version, processlist, kill, reload,
flush-logs, variables. Implemented as in-test `MysqlAdmin` command dispatcher.

## 3. Test Coverage
| Test | Command | Coverage |
|------|---------|----------|
| test_ping | ping | liveness check |
| test_status | status | uptime/threads/queries/slow |
| test_version | version | server version string |
| test_processlist | processlist | active connections list |
| test_kill | kill | terminate connection |
| test_kill_invalid_id | kill | error handling |
| test_reload | reload | grant table reload |
| test_flush_logs | flush-logs | log rotation |
| test_variables | variables | system variables |
| test_dispatch_unknown_command | (dispatch) | error handling |
| test_dispatch_kill_missing_arg | (dispatch) | arg validation |

## 4. Acceptance
- [x] 11 tests (>= 8 required)
- [x] All tests pass (11/11)
- [x] INT5 inventory updated (F-32 CLOSED)
- [ ] `cross_version_debt.sh` shows F-32 CLOSED (after PR merge)

## 5. Limitations
- In-test mock (real CLI binary in v3.9.0)
- No network-level admin protocol

## 6. References
- v3.0.0 COMPLETE_LEGACY_TRACKING_REPORT.md (F-32)
- openspec/changes/f-32-mysqladmin
- INT5_PLUS_DEBT_INVENTORY.md
