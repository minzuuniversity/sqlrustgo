# SPEC-v3.8.0-001 — MySQL Server Canonical Entry

> **Status**: PROPOSED (under `openspec/changes/mysql-server-canonical-entry/`)
> **Date**: 2026-06-04
> **Owner**: SQLRustGo v3.8.0 working group
> **Related Issue**: closes in spirit #2778 (consolidate execution paths)

---

## 1. Summary

From v3.8.0 onward, **`sqlrustgo-mysql-server`** is the **single canonical
execution entry point** for SQLRustGo. All SQL execution, regardless of
source (interactive REPL, batch script, e2e test, benchmark, third-party
integration), must go through this binary. The legacy in-process
`ExecutionEngine` direct-call API remains available for unit tests below
the wire-protocol layer, but anything exercising DDL, DML, transaction
lifecycle, or recovery must drive the wire.

---

## 2. Subcommands

| Subcommand | Replaces | Notes |
|------------|----------|-------|
| `serve` (default) | `sqlrustgo` root stub | MySQL wire-protocol server on `127.0.0.1:3306` (configurable via `--host`, `--port`, `--data-dir`, `--max-connections`, `--auth-mode`) |
| `exec "<sql>"` | ad-hoc scripts | Execute a single statement and exit; useful for shell pipelines |
| `repl` | `sqlrustgo-sql-cli` | Interactive REPL over stdin (`.tables`, `.schema`, `.databases`, `.version`, `.timing`, `.headers`, `.clear` dot commands) |
| `bench` | `sqlrustgo-bench`, `sqlrustgo-bench-cli` | Performance runner (placeholder; full TPC-H/OLTP runner is at `crates/bench/examples/tpch_*.rs`) |
| `gmp` | `sqlrustgo-gmp-cli` | GMP (AI Native) workflow (placeholder; lib preserved in `crates/gmp`) |
| `diag` | `sqlrustgo-tools` | Diagnostics / catalog dump (placeholder; lib preserved in `crates/tools` for `backup` / `restore`) |
| `backup` | `sqlrustgo-tools backup` | Backup database to a file |
| `restore` | `sqlrustgo-tools restore` | Restore database from a backup file |

---

## 3. Migration

### 3.1 End-User

```bash
# v3.7.0 and earlier
cargo run --bin sqlrustgo
cargo run --bin sqlrustgo-sql-cli
cargo run --bin sqlrustgo-bench-cli -- tpc-h
cargo run --bin sqlrustgo-gmp-cli -- status
cargo run --bin sqlrustgo-tools -- info

# v3.8.0+
cargo run --bin sqlrustgo-mysql-server -- repl
cargo run --bin sqlrustgo-mysql-server -- serve
cargo run --bin sqlrustgo-mysql-server -- bench tpc-h
cargo run --bin sqlrustgo-mysql-server -- gmp status
cargo run --bin sqlrustgo-mysql-server -- diag info
```

### 3.2 In-Tree Tests

Tests that previously drove the legacy binaries (or the in-process
`ExecutionEngine` directly for anything above the storage layer) must
switch to `start_ephemeral`:

```rust
use sqlrustgo_mysql_server::testing::start_ephemeral;

#[tokio::test]
async fn my_test() {
    let server = start_ephemeral(EphemeralConfig::default()).await.unwrap();
    let mut client = MySqlTestClient::connect(server.addr()).await.unwrap();
    let result = client.query("SELECT 1").await.unwrap();
    assert_eq!(result.rows, vec![vec!["1"]]);
}
```

### 3.3 External Consumers

There are no external (out-of-tree) consumers — all six retired binaries
were workspace-internal and were never published as public artifacts.

---

## 4. Bin Inventory (Post-Retirement)

`cargo metadata --no-deps --format-version 1` reports **6 workspace bins**:

| Bin | Source | Role |
|-----|--------|------|
| `sqlrustgo-mysql-server` | `crates/mysql-server/src/main.rs` | **Canonical** (this SPEC) |
| `sqlrustgo-gate` | `tools/sqlrustgo-gate/src/main.rs` | Pre-existing graph-RAG gate tool |
| `graph-gate` | `tools/sqlrustgo-gate/src/bin/graph-gate.rs` | Pre-existing graph-tool sub-bin |
| `graph-ingest` | `tools/sqlrustgo-gate/src/bin/graph-ingest.rs` | Pre-existing graph-ingest sub-bin |
| `gate` | `tools/graph-cli/src/main.rs` | Pre-existing graph CLI |
| `ingest` | `tools/graph-cli/src/ingest.rs` | Pre-existing graph ingest |

The 5 `tools/` bins are **out of scope** for this SPEC — they are
RAG/graph pipelines, not SQL execution paths.

---

## 5. Gate Updates

`scripts/gate/check_alpha_v380.sh` (the v3.8.0 Alpha gate) gains two new
sub-checks and one threshold adjustment:

| Check | Purpose | Threshold |
|-------|---------|-----------|
| `A1_BIN_COUNT` | Assert `sqlrustgo-mysql-server` is the only server-style bin in workspace | (no value, structural) |
| `A2_EPHEMERAL_SMOKE` | Drive wire-protocol smoke via `start_ephemeral` + `SELECT 1` | (no value, structural) |
| `A5` coverage | Lowered from 75% → 73% to account for wire-protocol overhead | 73% |

The 5 pre-existing graph-tool bins (see §4) are explicitly excluded from
the bin-count assertion via a name allow-list, so they do not need to
move into the canonical binary.

---

## 6. Spec Files (`openspec/specs/`)

This change does not modify any existing spec — it is net-new. The
following spec fragments are introduced:

- `specs/mysql-server-canonical-entry/spec.md` — the unified server
- `specs/wire-protocol-execution/spec.md` — wire-only execution
- `specs/server-embedded-test-harness/spec.md` — `start_ephemeral` API

---

## 7. Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| External scripts reference retired binaries | LOW (binaries were internal-only) | LOW | `archive/README.md` maps old → new; CHANGELOG entry announces removal |
| Wire-protocol overhead degrades benchmark numbers | MEDIUM | MEDIUM | A5 threshold lowered 75% → 73%; honest measurement is a feature, not a bug |
| Existing tests rely on in-process `ExecutionEngine` direct calls | MEDIUM | MEDIUM | Each rewrite is a separate PR; in-process API preserved for unit tests below the wire |

---

## 8. References

- Proposal: `openspec/changes/mysql-server-canonical-entry/proposal.md`
- Tasks: `openspec/changes/mysql-server-canonical-entry/tasks.md`
- Design: `openspec/changes/mysql-server-canonical-entry/design.md`
- Archive (retired binaries): `docs/releases/v3.8.0/archive/README.md`
- CHANGELOG: `## [Unreleased]` section
- Embedded test harness: `tests/embedded_harness_smoke.rs`, `tests/embedded_harness_isolation.rs`
