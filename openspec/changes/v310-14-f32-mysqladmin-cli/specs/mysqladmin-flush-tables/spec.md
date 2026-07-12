# mysqladmin-flush-tables

## ADDED Requirements

### Requirement: sqlrustgo-admin flush-tables is a no-op success

The `sqlrustgo-admin flush-tables` command SHALL return success without error. sqlrustgo manages its own buffer pool and does not expose a flush-tables operation.

#### Scenario: flush-tables command always succeeds

- **WHEN** the user runs `sqlrustgo-admin flush-tables`
- **THEN** the CLI prints `Flushing tables successful` to stdout and exits with exit code 0
