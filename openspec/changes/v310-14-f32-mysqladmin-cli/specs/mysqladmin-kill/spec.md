# mysqladmin-kill

## ADDED Requirements

### Requirement: sqlrustgo-admin kill terminates a connection by ID

The `sqlrustgo-admin kill <id>` command SHALL connect to the running sqlrustgo MySQL server and terminate the connection with the given numeric ID.

#### Scenario: kill terminates an existing connection

- **WHEN** the user runs `sqlrustgo-admin kill <id>` where `<id>` is an active connection
- **THEN** the CLI executes `KILL <id>` on the server, prints `Killed connection <id>` to stdout, and exits with exit code 0

#### Scenario: kill fails for non-existent connection ID

- **WHEN** the user runs `sqlrustgo-admin kill 99999` where no such connection exists
- **THEN** the CLI prints an error containing `not found` to stderr and exits with exit code 1

#### Scenario: kill requires a connection ID argument

- **WHEN** the user runs `sqlrustgo-admin kill` without an ID argument
- **THEN** the CLI prints usage information to stderr and exits with exit code 1
