# V310 CL I Binary Implementation Plan

> **Issue**: QUICK_START.md documents a `sqlrustgo` binary that does not exist
> **Date**: 2026-06-27
> **Status**: Planning

---

## Problem Statement

v3.9.0 QUICK_START.md describes a `sqlrustgo` CLI binary that **never existed**:

| Documented | Actual | Works? |
|-----------|--------|--------|
| `sqlrustgo server --port 5432` | `sqlrustgo-mysql-server serve --port 3306` | Partial |
| `sqlrustgo cli -c "SELECT 1+1"` | **NO `cli` subcommand** | ❌ FAILS |
| `sqlrustgo tpch generate` | **DOES NOT EXIST** | ❌ FAILS |
| PostgreSQL v3 wire protocol | MySQL wire protocol | ❌ WRONG |
| Port 5432 | Port 3306 | ❌ WRONG |
| Pre-built binary from GitHub | Binary on Gitea | ❌ WRONG URL |

**Critical**: Users following QUICK_START cannot execute a single SQL query.

---

## Existing Assets

- **MySqlTestClient** (`tests/common/mod.rs:422-812`): 287 lines of complete MySQL wire protocol client — TCP handshake, auth, query, result parsing
- **sqlrustgo-mysql-server**: Full server implementation with `serve`, `exec`, `repl` subcommands
- **sqlrustgo-admin**: Backup/restore/verify/pitr

---

## Phase 1: New `sqlrustgo` Binary Entry Point

**Deliverable**: `crates/sqlrustgo-cli/src/main.rs`

| Task | Description | Effort |
|------|-------------|--------|
| P1-1 | Create `crates/sqlrustgo-cli/Cargo.toml` as workspace member | 1h |
| P1-2 | Create `main.rs` with 9 subcommands: serve/exec/repl/bench/gmp/diag/backup/restore/cli | 5h |
| P1-3 | Forward `serve` → `run_server_v2`, `backup/restore` → `sqlrustgo-tools` | 2h |
| P1-4 | Forward `exec/repl` → embedded engine (no TCP needed) | 2h |

**Acceptance**:
- `cargo run --bin sqlrustgo -- serve` starts server on port 3306
- `cargo run --bin sqlrustgo -- --help` shows all 9 subcommands
- `sqlrustgo --version` matches `sqlrustgo-mysql-server --version`

---

## Phase 2: MySqlClient Library Extraction

**Deliverable**: `crates/mysql-client/src/lib.rs`

| Task | Description | Effort |
|------|-------------|--------|
| P2-1 | Extract `MySqlTestClient` from `tests/common/mod.rs:422-812` → rename `MySqlClient` | 8h |
| P2-2 | Publish `wire_err::Error` → `sqlrustgo_mysql_client::Error`, remove `EphemeralHandle` | 2h |
| P2-3 | Add `Client::connect_url("mysql://user:pass@host:port/db")` URL parser | 3h |
| P2-4 | Add async API via tokio wrapper | 3h |
| P2-5 | Update `tests/common/mod.rs` to `pub use sqlrustgo_mysql_client::*` | 1h |

**Acceptance**:
- `Client::connect_at(("127.0.0.1", 3306), "tester", "tester")` returns connected client
- `client.query_rows("SELECT 1+1 AS two")` returns `[["2"]]`
- `Client::connect_url("mysql://tester:tester@127.0.0.1:3306/")` parses and connects

---

## Phase 3: `cli` Subcommand Implementation

**Deliverable**: `cli` subcommand in `crates/sqlrustgo-cli`

| Task | Description | Effort |
|------|-------------|--------|
| P3-1 | Define `Command::Cli { query, host, port, user, password }` clap variant | 2h |
| P3-2 | On invoke: try connecting; if "Connection refused", spawn background server with exponential backoff (max 5s) | 4h |
| P3-3 | Run query via `Client::query_rows`, print tab-separated results | 3h |
| P3-4 | Support `--batch`, `--execute N` flags; send COM_QUIT on exit | 2h |

**Acceptance**:
- `sqlrustgo cli -c "SELECT 1+1 AS two"` outputs correct result
- `sqlrustgo cli -h 127.0.0.1 -P 3306 -u tester -p tester -c "SELECT 1"` connects to pre-existing server

---

## Phase 4: QUICK_START.md Correction

**Deliverable**: Fixed `docs/releases/v3.9.0/QUICK_START.md`

| Wrong | Correct |
|-------|---------|
| `sqlrustgo server --port 5432` | `sqlrustgo serve --port 3306` |
| `sqlrustgo cli -c "SELECT 1+1"` | `sqlrustgo cli -c "SELECT 1+1 AS two"` |
| `psql -h localhost -p 5432` | `mysql -h localhost -P 3306 -u tester -ptester` |
| `sqlrustgo tpch generate` | `cargo run --example tpch_data_gen -- --sf 0.001` |
| "PostgreSQL v3 wire protocol" | "MySQL wire protocol (mysql_native_password auth)" |
| Pre-built binary from GitHub | Gitea release artifact URL |
| "MySQL: ❌" | "MySQL: ✅ Full client support" |

**Acceptance**: Every `sqlrustgo` command in doc verified against actual binary.

---

## Phase 5: ARCH-2 MergeExecutor Remediation (Orthogonal, Non-Blocking)

**Deliverable**: `crates/executor/src/merge.rs` — no SQL string construction

**Problem**: `MergeExecutor` calls `build_update_sql()` + `engine.execute(sql_string)` — re-parses SQL on every MERGE operation.

| Stage | Task | Description | Effort |
|-------|------|-------------|--------|
| C7-1 (Stage 2) | Extend `crates::ExecutionEngine` trait | Add `execute_insert/update/delete`; implement in `LocalExecutor` delegating to root | 10h |
| C7-2 (Stage 3) | Replace MergeExecutor calls | Delete `build_update_sql()` / `build_insert_sql()`; call trait DML methods directly; fix PK filter offset bug | 8h |

**Acceptance**: `MergeExecutor` makes zero SQL string construction calls.

**Note**: CLI Phases 1-4 are fully implementable without ARCH-2. ARCH-2 is tracked separately.

---

## Binary Roles

| Binary | Role | Engine |
|--------|------|--------|
| `sqlrustgo` | Canonical CLI (NEW, user-facing) | Embedded in-process |
| `sqlrustgo-mysql-server` | Full-featured server (production) | Embedded in-process |
| `sqlrustgo-admin` | Offline admin (backup/restore/pitr) | File-based, no server |

---

## Verification Checklist

- [ ] `cargo build --bin sqlrustgo --all-features` compiles without errors
- [ ] `cargo build --bin sqlrustgo-mysql-server --all-features` unchanged, still compiles
- [ ] `cargo build --bin sqlrustgo-admin --all-features` unchanged, still compiles
- [ ] `cargo run --bin sqlrustgo -- serve` starts server on port 3306
- [ ] `cargo run --bin sqlrustgo -- cli -c "SELECT 1+1 AS two"` outputs correct result
- [ ] `cargo run --bin sqlrustgo -- --help` shows all 9 subcommands
- [ ] Every QUICK_START.md command verified against actual binary
- [ ] `cargo clippy --all-features -- -D warnings` → 0 warnings in new crates
- [ ] `cargo fmt --check` → no formatting regressions
