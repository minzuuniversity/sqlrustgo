# Design — mysql-server-canonical-entry

## Context

SQLRustGo v3.8.0 currently ships 11 distinct binary entry points and ~30 test files that bypass the wire protocol by calling `ExecutionEngine::execute(...)` directly. The fractured execution surface is the single largest obstacle to GA:

- Every new feature must be wired into 2-3 different binaries (mysql-server, sql-cli, gmp-cli) before it can be considered "exercised end-to-end". This is the root cause of the recurring "this path is broken in the binary but the in-process test passes" bug class.
- The CI A2 gate (`tests/ci/ci_test.rs`) drives a stub: it builds but does not actually exercise the wire protocol.
- Performance tests (`qps_benchmark_test`, `tpch_gate_test`, `buffer_pool_benchmark_test`) hit in-process `ExecutionEngine` only, so the wire protocol's own CPU/IO profile is never measured.
- The TPC-H benchmark (`crates/bench-cli`) and the REPL (`crates/sql-cli`) are separate code paths that share no state with the wire protocol, so a passing TPC-H number says nothing about MySQL-client experience.

We collapse all of this into one canonical entry: `sqlrustgo-mysql-server`. This is the binary we ship, the binary every test drives, and the binary every benchmark measures. The proposal and three specs (`mysql-server-canonical-entry`, `wire-protocol-execution`, `server-embedded-test-harness`) are the contract; this design is the blueprint.

## Goals / Non-Goals

**Goals:**
- One binary, five subcommands, one production execution surface.
- Every in-tree test (e2e, integration, recovery, perf) drives the binary over the MySQL wire protocol via `sqlrustgo_mysql_server::testing::start_ephemeral`.
- Every advanced subsystem (vector, graph, rag, gmp, qmd-bridge, telemetry, spill, network, catalog, information-schema) is reachable as a SQL surface (virtual tables or virtual functions) on the wire.
- Wire-protocol tests run in the A1/A2/A5/A9 gates; the existing A2 (`tests/ci/ci_test.rs`) is rewritten to spawn the server, run a smoke SQL, and shut it down.

**Non-Goals:**
- Replacing the MySQL wire protocol with anything else. It stays MySQL 8.0 wire compatible.
- Touching the gRPC `dtc` interface in `crates/network`. It is consumed in-process by the wire dispatcher (via `SELECT * FROM dtc.session`) and is not itself a top-level entry point.
- Performance tuning of the wire protocol. We aim for parity, not improvement. Real optimization work is a separate change.
- Backporting the consolidation to v3.7.0. v3.7.0 is GA; this is a v3.8.0-only refactor.

## Decisions

### D1. Subcommand dispatch via `clap` derive, not a hand-rolled parser
**Why:** The crate already pulls `clap` in for `sqlrustgo-bench-cli`. Reusing it costs zero and gives us auto-generated `--help`, shell completions, and consistent error formatting. Each subcommand is its own struct with a `#[derive(Subcommand)]`. The `serve` subcommand is the default when no positional is given — implemented by `Cli::parse()` falling through to a `Serve` default in the `Parser` derive's default-value machinery.

**Alternatives considered:**
- *Hand-rolled `match args[0]`*: rejected — error-prone, no `--help`, no completions.
- *Separate `clap` command-tree per subcommand crate*: rejected — splits the binary's source across 5 files for no real benefit when the dispatch is one line.

### D2. Five subcommands, not five binaries inside the same process
**Why:** The user-facing mental model is "one tool with five modes", not "one tool that secretly contains five tools." This is also what `clap` Subcommand naturally produces. Internally, each subcommand is a `fn handle(cli: ServeConfig) -> Result<ExitCode>` so they can be unit-tested in isolation.

**Subcommand inventory:**
| subcommand | replaces | persists state? | notes |
|---|---|---|---|
| `serve` (default) | the current `sqlrustgo-mysql-server` binary | yes (long-running) | default port 3306 |
| `repl` | `sqlrustgo-sql-cli` | no (transient) | uses `rustyline` like the old REPL did |
| `bench` | `sqlrustgo-bench-cli` and `sqlrustgo-bench` | no (CLI run) | dispatches to existing `bench-cli` commands |
| `gmp` | `sqlrustgo-gmp-cli` | no (CLI run) | thin wrapper over the `gmp` library |
| `diag` | `sqlrustgo-tools` | no (CLI run) | wraps the `tools` library |

### D3. Feature flag `advanced` controls the heavy crates
**Why:** A MySQL server binary that links 11 crates, several of which are gigabytes when built (arrow, parquet, lz4), is not a thing we want to ship as the default. The default build must be a "real but lean" server: it speaks MySQL wire protocol, has WAL, has recovery, has a working REPL mode. The `advanced` feature pulls in `vector`, `graph`, `rag`, `gmp`, `qmd-bridge`, `spill`. Both `cargo build -p sqlrustgo-mysql-server` (default) and `cargo build -p sqlrustgo-mysql-server --features advanced` MUST succeed.

**Alternatives considered:**
- *Always link everything, mark advanced-only with cfg*: rejected — CI image size and cold-start time suffer.
- *One binary per feature set*: rejected — violates the "one canonical entry" goal.
- *Compile-time `gated-sql` macros that the wire dispatcher expands*: rejected — adds a build step with no real benefit over a Cargo feature.

### D4. `start_ephemeral` is in-process, not a child process
**Why:** A test-harness that forks a real OS process is slow, OS-dependent (PATH, signal handling, port collisions on shared CI), and brittle in cross-platform CI. An in-process server bound to `port = 0` is portable, fast (sub-second to ready), and lets tests assert on internal state via the public API. The `EphemeralHandle` carries the bound `port` so the test can dial it with any MySQL client.

**Alternatives considered:**
- *Real `tokio::process::Command::spawn("sqlrustgo-mysql-server")`*: rejected — adds 200-500ms per test, requires the binary to be on PATH, flakier on shared CI.
- *Reuse one global server for the whole test binary*: rejected — fails the "data dir isolation" scenario in the spec, and parallel-test interference makes triage painful.

### D5. COM_QUERY dispatch goes through `ExecutionEngine::execute`, not a new path
**Why:** The wire protocol is the thinnest possible client — it parses the SQL, builds a `Statement`, and calls `ExecutionEngine::execute(&statement)`. There is no second "wire-shaped" execution path. This is what makes "one execution surface" a statement of fact, not an aspiration. The `ExecutionEngine` is also where WAL, MVCC, and the executor's recovery hooks already live; we do not duplicate them.

**Edge case:** The advanced subsystems (`vector`, `graph`, …) need to expose SQL-callable surfaces. We do this with a single hook point in the wire dispatcher: before `ExecutionEngine::execute`, if the statement looks like a virtual-table reference (e.g. `SELECT * FROM cypher_match(...)` or `SELECT * FROM server_health`), the dispatcher routes to a registered `VirtualTableProvider`. The provider is a trait object, so the `crates/vector` crate can register one from inside `sqlrustgo-mysql-server::main`. This is the ONLY way subsystems are reachable from the wire.

**Alternatives considered:**
- *Each subsystem implements its own COM_QUERY handler*: rejected — would create 10 parallel paths inside the wire server, defeating the consolidation.
- *Use Postgres-style FDW*: rejected — overengineered for v3.8.0.

### D6. Retired binaries deleted from `Cargo.toml` and `src/main.rs`
**Why:** Leaving a retired binary in the workspace invites re-introduction. The `crates/sql-cli` library stays (its command parser is useful inside `repl` mode), but its `src/main.rs` is deleted and its `[[bin]]` block is removed from `crates/sql-cli/Cargo.toml`. The same is done for `gmp`, `tools`, `bench`, `bench-cli`. The root `src/main.rs` is replaced with a one-line `eprintln!` that says "use `cargo run -p sqlrustgo-mysql-server`" and exits 1.

**Migration:** Any caller that invoked the retired binaries must change their invocation. In-tree, the only such callers are tests and gates, which are rewritten in this same change.

### D7. CI gates consume `start_ephemeral`, not a long-running server
**Why:** A single long-running server would have state leakage between gates. Each A-step that needs the server spawns a fresh ephemeral one. The A-step also runs the smoke SQL (e.g. `SELECT version()`) and asserts the result before declaring success. This mirrors the in-process "fresh state per test" hygiene.

**Alternatives considered:**
- *One server, started at CI bootstrap, killed at the end*: rejected — flaky on shared CI, no isolation between A-steps, blocks parallel A-step execution.

## Risks / Trade-offs

- **Risk:** Test runtime increases ~30-50% because every test now pays the wire-protocol handshake cost.
  → **Mitigation:** `start_ephemeral` reuses a single tokio runtime across the test binary via `OnceCell`. We budget the extra ~5-15s into the A2 gate (raise the timeout from 300s to 360s). Per-test time stays in milliseconds; the total is dominated by `cargo test` setup, not the per-test cost.

- **Risk:** Some subsystem features may not be reachable as pure SQL — e.g. graph traversal with deep nested queries. Pure-SQL is more constrained than the in-process API.
  → **Mitigation:** Each advanced subsystem gets a `VirtualTableProvider` that returns 100% of the in-process feature set as virtual tables/functions. Subsystems that cannot fit the SQL model (none today) get a documented "advanced only via direct library call" caveat. We do NOT regress the in-process API; we only require that the wire surface be a superset of what any previous binary offered.

- **Risk:** Deleting the REPL `sqlrustgo-sql-cli` binary breaks any local workflow that depends on it.
  → **Mitigation:** The REPL is preserved as `sqlrustgo-mysql-server repl` with identical UX (rustyline, history, exit on EOF). The CHANGELOG entry is explicit.

- **Risk:** The `start_ephemeral` API becomes a long-term commitment; if we change the wire protocol, all tests using `start_ephemeral` must change.
  → **Mitigation:** `start_ephemeral` is a thin wrapper over `tokio::net::TcpListener::bind("127.0.0.1:0")` plus the existing server loop. It has no protocol-level knowledge; it does not break when the protocol evolves.

- **Risk:** Benchmarks that go through the wire are noisier (TCP syscall overhead per query) than in-process benchmarks, so the "official" TPC-H number may go down.
  → **Mitigation:** We publish both the in-process and wire numbers, label the wire number as "official", and document the overhead. The expected drop is 5-15% (consistent with MySQL client wire overhead benchmarks). We do NOT game the in-process number to inflate the headline.

## Migration Plan

1. **Cut `feature/mysql-server-consolidation` from `develop/v3.8.0`.** All work happens on this branch via `git worktree`.
2. **Phase 1 — Land the test harness (RED → GREEN).** Add `start_ephemeral`, write one failing test that asserts on it, then implement. This is the keystone; everything else depends on it.
3. **Phase 2 — Migrate tests one suite at a time, in alphabetical order of `tests/*.rs`:** `boundary_test`, `cbo_integration_test`, `concurrency_stress_test`, `distinct_test`, `expression_operators_test`, `in_value_list_test`, `long_run_stability_test`, `long_run_stability_72h_test`, `mvcc_transaction_test`, `qps_benchmark_test`, `show_tables_test`, `stored_procedure_parser_test`, `stored_proc_catalog_test`, `tpch_gate_test`, `wal_integration_test`, `wal_tx_contract_test`, `tx_wal_contract_tests`, `exp_g_wal_contracts_verified`, `parser_token_test`, `aggregate_functions_test`, `limit_clause_test`, `binary_format_test`, `e2e/*`, `ci/*`. Each file's migration is one PR-sized commit.
4. **Phase 3 — Add the five subcommands.** One PR per subcommand: `repl`, `bench`, `gmp`, `diag`, `serve` (the last is "promote the existing binary to the default").
5. **Phase 4 — Wire the advanced subsystems.** Per subsystem: add `VirtualTableProvider` impl, register in `main`, add a smoke test under `tests/`.
6. **Phase 5 — Retire the binaries.** Delete the `[[bin]]` blocks and `src/main.rs` files. Update `Cargo.toml` at the root. Run `cargo build --workspace --bins` and assert the output is exactly `sqlrustgo-mysql-server`.
7. **Phase 6 — Update gates and docs.** `scripts/gate/*` spawns the server. `docs/releases/v3.8.0/*` describe the single entry point. `CHANGELOG.md` records the breaking internal change.
8. **Phase 7 — L3 acceptance evidence.** Run `mysql --protocol=TCP -h 127.0.0.1 -P <port>` from a system-level `mysql` CLI against `start_ephemeral` and capture the handshake + a smoke `SELECT 1`. This is the formal-verification evidence per `docs/governance/FORMAL_VERIFICATION_E2E.md`.
9. **Phase 8 — PR to `develop/v3.8.0`.** All CI gates green, all `start_ephemeral`-driven tests pass, manual smoke test from system `mysql` CLI succeeds. `Issue #2778` (the v3.8.0 rectification meta-issue) gets a child task checked off; this change is its own first-class task.

**Rollback:** Each phase is one PR, and each PR is independently revertable. The keystone (`start_ephemeral`) is additive (it does not modify any existing test), so its PR is strictly additive and trivially safe to keep even if later phases are rolled back.

## Open Questions

- **Q1:** Does the `start_ephemeral` API need to be `pub` (public) or `pub(crate)` (workspace-internal)? We resolve this at PR-2 by exporting it from `sqlrustgo-mysql-server` and using it from at least one downstream test crate (e.g. `crates/bench-cli`'s integration test). If a downstream crate does NOT use it, we keep it `pub(crate)` and document that consumers must build the crate from source.
- **Q2:** Should the `repl` subcommand share the `crates/sql-cli/commands.rs` command parser, or implement a fresh one? We resolve this by reusing the existing parser via `pub use` in `sqlrustgo-sql-cli`. If the import is awkward, we extract the parser into a sub-module `sqlrustgo-sql-cli::command_parser` and import that.
- **Q3:** Does the `serve` subcommand need a `--socket /tmp/sqlrustgo.sock` option for local non-TCP use? We resolve this by NOT including it in v3.8.0; if a v3.8.0+1 user requests it, we add it as an additive flag.
