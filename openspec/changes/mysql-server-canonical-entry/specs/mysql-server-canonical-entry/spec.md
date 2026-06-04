# Spec: mysql-server-canonical-entry

## ADDED Requirements

### Requirement: One binary, five subcommands
The workspace SHALL expose a single production binary `sqlrustgo-mysql-server`. The binary MUST dispatch on the first positional argument (`serve` | `repl` | `bench` | `gmp` | `diag`) and reject unknown subcommands with a non-zero exit code. When invoked with no subcommand, the binary MUST default to `serve`.

#### Scenario: Default invocation
- **WHEN** a user runs `sqlrustgo-mysql-server --port 3306 --data-dir /var/lib/sqlrustgo`
- **THEN** the process starts the MySQL wire-protocol TCP listener on port 3306 backed by a `FileStorage` rooted at `/var/lib/sqlrustgo`, with exit code 0 while the listener is healthy and non-zero on bind/initialization failure

#### Scenario: Subcommand dispatch
- **WHEN** a user runs `sqlrustgo-mysql-server repl`, `sqlrustgo-mysql-server bench tpc-h`, `sqlrustgo-mysql-server gmp retrieve`, or `sqlrustgo-mysql-server diag doctor`
- **THEN** the corresponding handler runs and the other four subcommands are NOT executed

#### Scenario: Unknown subcommand rejected
- **WHEN** a user runs `sqlrustgo-mysql-server bogus`
- **THEN** the process exits with code 64 (EX_USAGE) and prints a one-line usage hint

### Requirement: Full crate stack wired
The `sqlrustgo-mysql-server` binary MUST be a direct dependency of every functional crate in the workspace (`vector`, `graph`, `rag`, `gmp`, `qmd-bridge`, `telemetry`, `spill`, `network`, `catalog`, `information-schema`) so that a single `cargo build -p sqlrustgo-mysql-server --features advanced` succeeds without further feature flags. The non-advanced build (default features) MUST compile without the advanced-store feature set.

#### Scenario: Default build wires core crates
- **WHEN** `cargo build -p sqlrustgo-mysql-server` is run with no features
- **THEN** the build succeeds and the resulting binary can answer a `SELECT version()` over the wire

#### Scenario: Advanced build wires feature crates
- **WHEN** `cargo build -p sqlrustgo-mysql-server --features advanced` is run
- **THEN** the build succeeds and the resulting binary can answer a `SELECT * FROM vector_index_status` and a `SELECT * FROM gmp_collection` over the wire

### Requirement: Retired binaries removed
The workspace `Cargo.toml` MUST NOT define a `[[bin]]` for any of: `sqlrustgo` (root stub), `sqlrustgo-sql-cli`, `sqlrustgo-gmp-cli`, `sqlrustgo-tools`, `sqlrustgo-bench`, `sqlrustgo-bench-cli`. The corresponding `src/main.rs` files MUST be deleted. The underlying library crates (used internally by mysql-server) MUST be retained.

#### Scenario: cargo build --bins list is single
- **WHEN** `cargo build --workspace --bins` is run
- **THEN** the build plan contains exactly one bin target: `sqlrustgo-mysql-server`

#### Scenario: Retired crates still compile as libs
- **WHEN** `cargo build -p sqlrustgo-gmp --lib -p sqlrustgo-tools --lib` is run
- **THEN** both library crates build successfully (they are consumed by `sqlrustgo-mysql-server`)

#### Scenario: Documentation reflects single binary
- **WHEN** a user reads `docs/releases/v3.8.0/README.md`
- **THEN** the "Getting Started" section shows exactly one command to run the server (`cargo run -p sqlrustgo-mysql-server -- serve`)

### Requirement: Backward-compatible default port and config
The default invocation of `sqlrustgo-mysql-server` MUST listen on TCP 3306 (overridable with `--port`) and MUST accept the same on-disk layout that previous binaries used (a `t.json` per table under `--data-dir`).

#### Scenario: Port override
- **WHEN** `sqlrustgo-mysql-server --port 33307` is run
- **THEN** the process listens on 33307 and not on 3306

#### Scenario: Data dir persistence
- **WHEN** an engine writes 1000 rows through one process, the process exits, and a second process is started with the same `--data-dir`
- **THEN** the second process serves the 1000 rows on the next `SELECT`
