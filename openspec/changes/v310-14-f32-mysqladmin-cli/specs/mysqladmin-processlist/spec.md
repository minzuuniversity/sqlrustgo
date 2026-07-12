# mysqladmin-processlist

## ADDED Requirements

### Requirement: sqlrustgo-admin processlist reports active connections

The `sqlrustgo-admin processlist` command SHALL connect to the running sqlrustgo MySQL server and list all active connections with their Id, User, Host, db, Command, Time, State, and Info.

#### Scenario: processlist shows active connections

- **WHEN** the user runs `sqlrustgo-admin processlist` and the server has active connections
- **THEN** the CLI prints a tab-separated table with headers `Id	User	Host	db	Command	Time	State	Info`, one row per connection, and exits with exit code 0

#### Scenario: processlist shows empty list

- **WHEN** the user runs `sqlrustgo-admin processlist` and no connections are active
- **THEN** the CLI prints only the header row and exits with exit code 0

#### Scenario: processlist when server is unreachable

- **WHEN** the user runs `sqlrustgo-admin processlist` but no server is listening
- **THEN** the CLI prints an error to stderr and exits with exit code 1
