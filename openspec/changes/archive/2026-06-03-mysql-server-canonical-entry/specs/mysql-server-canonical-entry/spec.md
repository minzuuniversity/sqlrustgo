# mysql-server-canonical-entry

## Purpose

`sqlrustgo-mysql-server` is the single canonical execution entry
point for SQLRustGo v3.8.0+. Every SQL execution path
(serve / exec / repl / bench / gmp / diag) lives behind one of
its subcommands. The five legacy binaries (`sqlrustgo`,
`sqlrustgo-sql-cli`, `sqlrustgo-bench`, `sqlrustgo-bench-cli`,
`sqlrustgo-tools`) are deprecation stubs that point to the
canonical subcommand.

## ADDED Requirements

### Requirement: One binary, six subcommands

`sqlrustgo-mysql-server` SHALL expose a clap `Subcommand` enum
with the following variants:

- `serve` (default when no subcommand is given) — start the
  MySQL wire-protocol server on a configurable host and port
- `exec "<sql>"` — execute a single SQL statement via the
  in-process `ExecutionEngine` and print the resulting rows
- `repl` — read SQL statements from stdin until EOF or
  `.exit` / `.quit`; print result rows for each
- `bench` — placeholder for the benchmark runner (full feature
  parity migrates in a follow-up)
- `gmp` — placeholder for the GMP (AI Native) workflow
- `diag` — placeholder for diagnostics / catalog dump

The `serve` subcommand SHALL be the default when no subcommand
is given so existing invocations of
`sqlrustgo-mysql-server --host … --port …` keep working.

#### Scenario: Default invocation starts the server

- GIVEN the user runs `sqlrustgo-mysql-server --host 127.0.0.1 --port 3306`
- WHEN the binary parses its CLI
- THEN it MUST dispatch to the `serve` subcommand and start the
  MySQL wire-protocol server on `127.0.0.1:3306`

#### Scenario: Help text lists every subcommand

- GIVEN the user runs `sqlrustgo-mysql-server --help`
- WHEN the binary prints usage
- THEN the output MUST list `serve`, `exec`, `repl`, `bench`,
  `gmp`, `diag`, and `help` as available commands

### Requirement: Retired legacy binaries

The five legacy binaries MUST each print a deprecation notice
on stderr that names the canonical subcommand to use instead,
and MUST exit with status 0 so existing scripts and CI that
invoke them keep working during the migration window. The
five binaries are: `sqlrustgo`, `sqlrustgo-sql-cli`,
`sqlrustgo-bench`, `sqlrustgo-bench-cli`, `sqlrustgo-tools`.

#### Scenario: Running a legacy binary prints a deprecation notice

- GIVEN a user runs `sqlrustgo` (or any of the other four
  legacy binaries)
- WHEN the binary executes
- THEN it MUST print a one-line deprecation message on stderr
  that names the canonical `sqlrustgo-mysql-server` subcommand
  to use, and exit with status 0
