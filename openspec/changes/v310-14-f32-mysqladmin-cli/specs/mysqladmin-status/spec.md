# mysqladmin-status

## ADDED Requirements

### Requirement: sqlrustgo-admin status connects to server and reports status

The `sqlrustgo-admin status` command SHALL connect to a running sqlrustgo MySQL server via TCP and report server status including uptime, thread count, query throughput, and slow query count.

#### Scenario: status command with server running

- **WHEN** the user runs `sqlrustgo-admin status --host localhost --port 3306`
- **THEN** the CLI connects to the server, executes `SHOW GLOBAL STATUS` and `SHOW VARIABLES`, and prints a status line of the form: `Uptime: <seconds>  Threads: <n>  Questions: <n>  Slow queries: <n>`

#### Scenario: status command when server is unreachable

- **WHEN** the user runs `sqlrustgo-admin status` but no server is listening on the target host:port
- **THEN** the CLI prints an error message to stderr and exits with exit code 1

#### Scenario: status command with custom connection parameters

- **WHEN** the user runs `sqlrustgo-admin status --host 127.0.0.1 --port 3307 --user root --password secret`
- **THEN** the CLI uses the provided credentials to connect to port 3307 on 127.0.0.1
