## Why

SQLRustGo v3.8.0 currently exposes **eleven** different execution entry points (root `sqlrustgo` stub, `sqlrustgo-sql-cli` REPL, `sqlrustgo-mysql-server` MySQL wire protocol, `sqlrustgo-gmp-cli` retrieval CLI, `sqlrustgo-tools` diagnostic CLI, `sqlrustgo-bench`/`sqlrustgo-bench-cli` performance harnesses, `sqlrustgo-server` dual HTTP server, `sqlrustgo-gate`/`graph-cli`/`graph-ingest` tool binaries, plus an in-process `ExecutionEngine` API used by ~30 test files). Every additional entry point is a separate surface that must be regression-tested, benchmarked, fuzzed, documented, and kept ABI/feature-compatible — but only one of them (mysql-server) actually models a real client/server boundary. This fractured execution model is the root cause of the recurring "the test passed in-process but the network path is broken" class of bugs and is the single largest obstacle to GA. We collapse all of them into one canonical, fully-integrated entry point: **`sqlrustgo-mysql-server`**, and rewire every test, benchmark, and CI gate to drive through it.

## What Changes

- **Promote `sqlrustgo-mysql-server` to the single canonical execution entry point.** It becomes the only binary on the `v3.8.0` production path.
- **Add new subcommands to `sqlrustgo-mysql-server`** to subsume functionality that lived in retired binaries:
  - `sqlrustgo-mysql-server repl [--data-dir <path>]` — interactive shell mode (replaces `sqlrustgo-sql-cli`)
  - `sqlrustgo-mysql-server bench tpc-h|oltp|custom [--config <file>]` — performance runs (replaces `sqlrustgo-bench-cli`)
  - `sqlrustgo-mysql-server gmp retrieve|index|status` — retrieval engine (replaces `sqlrustgo-gmp-cli`)
  - `sqlrustgo-mysql-server diag doctor|info|stats` — diagnostic tooling (replaces `sqlrustgo-tools`)
  - `sqlrustgo-mysql-server serve [--port N] [--data-dir <path>]` (default) — MySQL wire protocol TCP server
- **Wire the full crate stack into `sqlrustgo-mysql-server`** that is currently only reachable in-process:
  - `vector` (ANN index)
  - `graph` (Cypher)
  - `rag` (retrieval)
  - `gmp` (GMP retrieval v3)
  - `qmd-bridge` (QMD adapter)
  - `telemetry` (metrics/health)
  - `spill` (spill-to-disk)
  - `network` (DTC gRPC, exposed as management SQL via `SELECT * FROM dtc.*`)
  - `catalog` (information schema)
  - `information-schema` (server introspection)
- **Retire the following binaries** by removing their `[[bin]]` entries and any `[[example]]`/`src/main.rs`:
  - `src/main.rs` (root stub that just prints version)
  - `crates/sql-cli/src/main.rs` (`sqlrustgo-sql-cli` REPL)
  - `crates/server/src/...` HTTP endpoints (`sqlrustgo-server` library stays; its routes are exposed through mysql-server's HTTP sidecar)
  - `crates/gmp/src/main.rs` (`sqlrustgo-gmp-cli`)
  - `crates/tools/src/bin/*` (`sqlrustgo-tools`)
  - `crates/bench/src/main.rs` and `crates/bench-cli/src/main.rs` (bench and bench-cli)
- **Rewrite every test under `tests/`** that previously drove one of the retired entry points to instead boot `sqlrustgo-mysql-server` (in-process via `MySqlServer::start_ephemeral()`) and connect with a real MySQL client (`mysql_async`/`tokio-mysql`). The in-process `ExecutionEngine` direct-call tests are kept for unit tests below the wire-protocol layer, but anything exercising DDL/DML/RECOVERY must go through the wire.
- **Make `scripts/gate/*.sh`** spawn `sqlrustgo-mysql-server` once and drive all A1–A9 gate steps against the same server instance.
- **Add a new feature flag `--features gmp-vec-graph-rag`** to `sqlrustgo-mysql-server` that pulls in the advanced-store feature set; without it the binary still works as a pure SQL/MySQL server.
- **BREAKING**: Remove the public `sqlrustgo_sql_cli` and `sqlrustgo_gmp_cli` binaries from the workspace. They were never published and are only used in-tree, so the blast radius is internal. Documented in `CHANGELOG.md`.

## Capabilities

### New Capabilities
- `mysql-server-canonical-entry`: the unified server binary. One binary, five subcommands (`serve`, `repl`, `bench`, `gmp`, `diag`), each a complete superset of the corresponding retired binary. All wired crates (vector, graph, rag, gmp, qmd-bridge, telemetry, spill, network, catalog, information-schema) are reachable from every subcommand, with the feature flag gating the advanced set.
- `wire-protocol-execution`: every SQL execution, including DDL, DML, transaction lifecycle, recovery, and gated feature surface, is reachable through the MySQL wire protocol. No hidden in-process shortcuts. New COM_QUERY dispatcher routes SQL to the appropriate subsystem based on statement shape.
- `server-embedded-test-harness`: a public, stable API (`sqlrustgo_mysql_server::testing::start_ephemeral`) that boots the server in-process on an OS-assigned port and returns a `MySqlServerHandle` for tests. This is the only sanctioned way for in-tree tests to drive SQL.

### Modified Capabilities
*(none — this change introduces net-new capabilities; it does not alter existing spec'd behavior because no `openspec/specs/` exist yet)*

## Impact

- **Workspace structure** (`Cargo.toml`): remove `[[bin]]` blocks for the six retired binaries, keep their underlying libraries (used internally by mysql-server).
- **CI gates** (`scripts/gate/*`): A1 (build) and A2 (test) must spawn `sqlrustgo-mysql-server` exactly once. The existing `tests/ci/ci_test.rs` is rewritten to use `start_ephemeral`.
- **Documentation** (`docs/`): update `ENGINEERING_EVOLUTION_STANDARD.md`, `GATE_CI_CD.md`, `GATE_CONDITIONS.md`, `IMMUTABLE_RELEASE_ARCHITECTURE.md`, and the v3.8.0 README to reflect the single entry point. Archive retired-binary docs (v3.7.0 REPL docs, `gmp-cli.md`, etc.) under `docs/releases/v3.8.0/archive/`.
- **Dependencies**: `sqlrustgo-mysql-server` gains direct dependencies on `vector`, `graph`, `rag`, `gmp`, `qmd-bridge`, `telemetry`, `spill`, `catalog`, `information-schema`, `network` (all already in workspace). No external dep changes.
- **Test surface**: ~30 test files under `tests/` are rewritten. Per the project's `tdd-workflow` and `verification-before-completion` skills, each rewrite must land with the new test in RED state (mysql-server not yet wired → test fails) before the implementation lands.
- **CHANGELOG / SPEC**: a new `SPEC-v3.8.0-001-mysql-server-canonical-entry.md` is added under `docs/releases/v3.8.0/`; `CHANGELOG.md` notes the binary removal as a breaking internal change.
- **Performance**: all future benchmarks (`sqlrustgo-bench`, `qps_benchmark_test`, `buffer_pool_benchmark_test`, `page_io_benchmark_test`, `tpch_gate_test`) drive through the wire protocol. This is more honest but also more CPU-intensive per iteration; we adjust the A5 coverage gate to allow the extra wire-protocol overhead.
- **Issue close evidence**: closing the consolidation requires a `start_ephemeral()`-driven A2 test, a `qps_benchmark_test` over the wire, and a verified `mysql --protocol=TCP -h 127.0.0.1 -P <port>` handshake from a system-level `mysql` CLI. The last item is the L3 acceptance evidence per `docs/governance/FORMAL_VERIFICATION_E2E.md`.
