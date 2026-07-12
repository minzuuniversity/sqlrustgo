# mysqladmin-reload

## ADDED Requirements

### Requirement: sqlrustgo-admin reload is a no-op success

The `sqlrustgo-admin reload` command SHALL return success without error. sqlrustgo does not maintain grant tables or configuration files that require reloading.

#### Scenario: reload command always succeeds

- **WHEN** the user runs `sqlrustgo-admin reload`
- **THEN** the CLI prints `Reload complete` to stdout and exits with exit code 0
