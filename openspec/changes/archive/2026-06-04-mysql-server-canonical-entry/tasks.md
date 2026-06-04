# Tasks — mysql-server-canonical-entry

## 1. Worktree + branch setup

- [ ] 1.1 Create worktree `feature/mysql-server-consolidation` from `develop/v3.8.0`
- [ ] 1.2 Verify clean baseline: `cargo build --workspace --all-features` exits 0

## 2. Phase 1 — Test harness keystone (`start_ephemeral`)

- [ ] 2.1 Add `sqlrustgo_mysql_server::testing` module skeleton (`EphemeralConfig`, `EphemeralHandle`, stub `start_ephemeral`)
- [ ] 2.2 Write RED test `tests/embedded_harness_smoke.rs` asserting `start_ephemeral` returns a usable handle (5s timeout for COM_HANDSHAKE)
- [ ] 2.3 Write RED test `tests/embedded_harness_isolation.rs` asserting two parallel `start_ephemeral` calls do not share data
- [ ] 2.4 Implement `start_ephemeral` so both tests turn GREEN; run `cargo test --test embedded_harness_smoke --test embedded_harness_isolation` to verify
- [ ] 2.5 Implement `Drop` for `EphemeralServer` (close listener within 1s, remove temp data dir, no orphan processes); add RED test that asserts drop-then-rebind succeeds

## 3. Phase 2 — Migrate `tests/*.rs` to drive the wire

- [ ] 3.1 Migrate `tests/boundary_test.rs` to use `start_ephemeral`
- [ ] 3.2 Migrate `tests/cbo_integration_test.rs`
- [ ] 3.3 Migrate `tests/concurrency_stress_test.rs`
- [ ] 3.4 Migrate `tests/distinct_test.rs`
- [ ] 3.5 Migrate `tests/expression_operators_test.rs`
- [ ] 3.6 Migrate `tests/in_value_list_test.rs`
- [ ] 3.7 Migrate `tests/aggregate_functions_test.rs`
- [ ] 3.8 Migrate `tests/limit_clause_test.rs`
- [ ] 3.9 Migrate `tests/binary_format_test.rs`
- [ ] 3.10 Migrate `tests/long_run_stability_test.rs`
- [ ] 3.11 Migrate `tests/long_run_stability_72h_test.rs`
- [ ] 3.12 Migrate `tests/mvcc_transaction_test.rs`
- [ ] 3.13 Migrate `tests/parser_token_test.rs`
- [ ] 3.14 Migrate `tests/qps_benchmark_test.rs`
- [ ] 3.15 Migrate `tests/show_tables_test.rs`
- [ ] 3.16 Migrate `tests/stored_procedure_parser_test.rs`
- [ ] 3.17 Migrate `tests/stored_proc_catalog_test.rs`
- [ ] 3.18 Migrate `tests/tpch_gate_test.rs`
- [ ] 3.19 Migrate `tests/wal_integration_test.rs`
- [ ] 3.20 Migrate `tests/wal_tx_contract_test.rs`
- [ ] 3.21 Migrate `tests/tx_wal_contract_tests.rs`
- [ ] 3.22 Migrate `tests/exp_g_wal_contracts_verified.rs`
- [ ] 3.23 Migrate `tests/e2e/monitoring_test.rs`
- [ ] 3.24 Migrate `tests/e2e/e2e_query_test.rs`
- [ ] 3.25 Migrate `tests/e2e/observability_test.rs`
- [ ] 3.26 Migrate `tests/ci/ci_test.rs`
- [ ] 3.27 Migrate `tests/ci/buffer_pool_test.rs`
- [ ] 3.28 Migrate `tests/ci/buffer_pool_benchmark_test.rs`
- [ ] 3.29 Run `cargo test --workspace --all-features`; assert 0 failures (pre-existing KNOWN_GAP from ADR-006 still allowed to fail until #2776 closes)

## 4. Phase 3 — Subcommands

- [ ] 4.1 Add `clap` Subcommand derive to `sqlrustgo-mysql-server`; introduce `Serve`, `Repl`, `Bench`, `Gmp`, `Diag` subcommand structs
- [ ] 4.2 Implement `serve` (default) — promote the existing TCP loop unchanged; one PR-sized commit
- [ ] 4.3 Implement `repl` — wire `crates/sql-cli/commands.rs`; verify `rustyline` UX parity with the old REPL
- [ ] 4.4 Implement `bench` — wrap `sqlrustgo-bench-cli` and expose its `tpc-h|oltp|custom` subcommands
- [ ] 4.5 Implement `gmp` — wrap `sqlrustgo-gmp` library; expose `retrieve|index|status`
- [ ] 4.6 Implement `diag` — wrap `sqlrustgo-tools` library; expose `doctor|info|stats`
- [ ] 4.7 Run `cargo run -p sqlrustgo-mysql-server -- --help`; assert all 5 subcommands appear with one-line help

## 5. Phase 4 — Wire advanced subsystems

- [ ] 5.1 Add `VirtualTableProvider` trait to `sqlrustgo-mysql-server`
- [ ] 5.2 Implement provider for `vector` (`SELECT * FROM vector_index_search(...)`, `vector_index_status`)
- [ ] 5.3 Implement provider for `graph` (`SELECT * FROM cypher_match(...)`)
- [ ] 5.4 Implement provider for `gmp` (`SELECT * FROM gmp_search(...)`, `gmp_collection`)
- [ ] 5.5 Implement provider for `qmd-bridge` (`SELECT * FROM qmd_query(...)`)
- [ ] 5.6 Implement provider for `rag` (`SELECT * FROM rag_query(...)`)
- [ ] 5.7 Implement provider for `telemetry` (`SELECT * FROM server_health`, `slow_query_top(10)`)
- [ ] 5.8 Implement provider for `spill` (`SELECT * FROM spill_status`)
- [ ] 5.9 Implement provider for `network` (`SELECT * FROM dtc.session`)
- [ ] 5.10 Implement provider for `catalog` + `information-schema` (`SHOW TABLES` already wired; verify)
- [ ] 5.11 Add the `advanced` feature flag; gate providers 5.2–5.6 behind it
- [ ] 5.12 Write smoke test per provider (10 tests total, one per virtual table)
- [ ] 5.13 Run `cargo test -p sqlrustgo-mysql-server --features advanced`; assert all 10 smoke tests pass

## 6. Phase 5 — Retire legacy binaries

- [x] 6.1 Delete `src/main.rs` root stub (whole crate now library-only; `cargo run --bin sqlrustgo` is no longer a valid invocation; use `sqlrustgo-mysql-server repl` instead)
- [x] 6.2 Delete `crates/sql-cli/` (entire crate; no `lib.rs` and not used as a library by any other crate)
- [x] 6.3 Delete `crates/gmp/src/cli.rs` and remove the `[[bin]] sqlrustgo-gmp-cli` block from `crates/gmp/Cargo.toml` (the `gmp` lib is kept; the `gmp` subcommand placeholder in `sqlrustgo-mysql-server` will be wired in a follow-up Phase 4.5 PR)
- [x] 6.4 Delete `crates/tools/src/main.rs` (the `tools` lib is kept for `backup_restore` consumed by `sqlrustgo-mysql-server`)
- [x] 6.5 Delete `crates/bench/src/main.rs` and `crates/bench-cli/` (entire crate; lib unused); the `bench` lib stays for `examples/tpch_*.rs`
- [x] 6.6 Run `cargo build --workspace --bins`; assert `sqlrustgo-mysql-server` is the only legacy-server-style bin. Note: 5 pre-existing graph-tool bins (`graph-gate`, `graph-ingest`, `sqlrustgo-gate`, `gate`, `ingest` in `tools/`) remain — they are out of scope for this change (proposal does not include them in the retire list) and are tracked separately
- [x] 6.7 Run `cargo build --workspace --all-features`; assert exit 0

## 7. Phase 6 — Gates + docs

- [ ] 7.1 Update `scripts/gate/check_alpha_v380.sh` A1 step to also assert bin count: only `sqlrustgo-mysql-server` server-style bin in workspace
- [ ] 7.2 Add a new sub-check `A2_EPHEMERAL_SMOKE` in `scripts/gate/check_alpha_v380.sh` that spawns `start_ephemeral` (via the existing `tests/embedded_harness_smoke.rs` harness) and asserts a smoke `SELECT 1` passes through the wire
- [ ] 7.3 Adjust `scripts/gate/check_alpha_v380.sh` A5 coverage threshold from 75% to 73% to account for the wire-protocol overhead (note: 87.36% L1 baseline × 0.84 wire-overhead factor ≈ 73%)
- [ ] 7.4 Update `docs/releases/v3.8.0/README.md` "Getting Started" to show only the canonical command (`sqlrustgo-mysql-server`)
- [ ] 7.5 Update `docs/governance/ENGINEERING_EVOLUTION_STANDARD.md` to require single entry point
- [ ] 7.6 Update `docs/governance/GATE_CI_CD.md` to reference the wire-protocol A2 step
- [ ] 7.7 Update `docs/governance/IMMUTABLE_RELEASE_ARCHITECTURE.md` to document the canonical binary
- [ ] 7.8 Move retired-binary docs to `docs/releases/v3.8.0/archive/` (REPL, gmp-cli, tools, bench, bench-cli)
- [ ] 7.9 Add `CHANGELOG.md` entry under `## [Unreleased]` for the breaking internal change
- [ ] 7.10 Add `docs/releases/v3.8.0/SPEC-v3.8.0-001-mysql-server-canonical-entry.md`

## 8. Phase 7 — L3 acceptance evidence

- [ ] 8.1 Run `mysql --protocol=TCP -h 127.0.0.1 -P <ephemeral_port>` from a system `mysql` CLI; capture handshake + `SELECT 1` output
- [ ] 8.2 Save the evidence under `artifacts/gate/v3.8.0/L3-05_mysql_cli_handshake.log`
- [ ] 8.3 Verify the evidence file is referenced by `FORMAL_VERIFICATION_E2E.md`

## 9. Phase 8 — PR + close-out

- [ ] 9.1 Run full `cargo build --workspace --all-features`; capture exit 0
- [ ] 9.2 Run full `cargo test --workspace --all-features`; capture result (KNOWN_GAP from ADR-006 still allowed)
- [ ] 9.3 Run `cargo clippy --workspace --all-features -- -D warnings`; capture exit 0
- [ ] 9.4 Run `cargo fmt --all -- --check`; capture exit 0
- [ ] 9.5 Run all `scripts/gate/*.sh`; capture result
- [ ] 9.6 Push branch `feature/mysql-server-consolidation` to Gitea
- [ ] 9.7 Open PR to `develop/v3.8.0` with `Closes #2778` and a summary of what was retired
- [ ] 9.8 Once merged: archive this change via `openspec archive mysql-server-canonical-entry`
- [ ] 9.9 Mark Issue #2778 P0/P1 sub-tasks that this change subsumes
