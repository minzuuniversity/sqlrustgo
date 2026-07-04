# binary-prepared-statement-roundtrip Specification

## Purpose
TBD - created by archiving change fix-sysbench-stmt-prepare-error-2000. Update Purpose after archive.
## Requirements
### Requirement: Wire-format trace logging for COM_STMT_EXECUTE

`crates/mysql-server/src/lib.rs::do_command_loop` MUST log, when
`RUST_LOG=sqlrustgo_mysql_server=info` is set, the SQL after parameter
substitution for each `COM_STMT_EXECUTE` execution so operators can correlate
client-bound parameters to the executed query.

#### Scenario: Trace log shows spliced SQL

- **WHEN** a client sends `COM_STMT_PREPARE` + `COM_STMT_EXECUTE` for
  `SELECT v FROM t WHERE id = ?` bound to `3`
- **THEN** the server `info` log line for that execution MUST contain the
  string `WHERE id = 3` (decimal, no quotes)

