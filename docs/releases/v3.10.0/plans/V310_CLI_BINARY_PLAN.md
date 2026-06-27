# V310 CLI Binary Implementation Plan

> **Status**: Phase 1 ✅ Done | Phase 2 🚧 In Progress | Phase 3 📋 Pending | Phase 4 ✅ Done
> **Date**: 2026-06-27

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

## Phase 1: New `sqlrustgo-cli` Binary Entry Point ✅

**Status**: ✅ Done — commits `ec88ef017` + `d7300c877`

**Deliverable**: `crates/sqlrustgo-cli/src/main.rs`

| Task | Status | Notes |
|------|--------|-------|
| P1-1: `Cargo.toml` | ✅ | 4 deps: clap, tokio, tracing, std only |
| P1-2: `main.rs` with 9 subcommands | ✅ | serve/exec/repl/bench/gmp/diag/backup/restore/cli |
| P1-3: Forward `serve` → `sqlrustgo-mysql-server serve` | ✅ | Thin wrapper via `std::process::Command::new()` |
| P1-4: Forward `backup/restore` → `sqlrustgo-mysql-server backup/restore` | ✅ | Same pattern |

**Binary structure**:
```
sqlrustgo-cli serve       → sqlrustgo-mysql-server serve
sqlrustgo-cli exec <SQL>   → sqlrustgo-mysql-server exec <SQL>  (positional arg)
sqlrustgo-cli repl        → sqlrustgo-mysql-server repl
sqlrustgo-cli bench       → sqlrustgo-mysql-server bench
sqlrustgo-cli gmp         → sqlrustgo-mysql-server gmp
sqlrustgo-cli diag        → sqlrustgo-mysql-server diag
sqlrustgo-cli backup      → sqlrustgo-mysql-server backup
sqlrustgo-cli restore     → sqlrustgo-mysql-server restore
sqlrustgo-cli cli         → Phase 3 skeleton (TCP connect + handshake read)
```

**Verification**:
```bash
$ ./target/debug/sqlrustgo-cli --version   # → sqlrustgo 3.9.0
$ ./target/debug/sqlrustgo-cli exec "SELECT 1"  # → col_0 | Integer(1) (1 rows)
$ cargo build -p sqlrustgo-cli   # ✅ exit 0
$ cargo build --all-features    # ✅ exit 0
```

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
| T3-1 | Define `Command::Cli { query, host, port, user, password }` clap variant | 2h |
| T3-2 | On invoke: try connecting; if "Connection refused", spawn background server with exponential backoff (max 5s) | 4h |
| T3-3 | Run query via `Client::query_rows`, print tab-separated results | 3h |
| T3-4 | Support `--batch`, `--execute N` flags; send COM_QUIT on exit | 2h |

**Acceptance**:
- `sqlrustgo cli -c "SELECT 1+1 AS two"` outputs correct result
- `sqlrustgo cli -h 127.0.0.1 -P 3306 -u tester -p tester -c "SELECT 1"` connects to pre-existing server

---

## Phase 4: QUICK_START.md Correction ✅

**Status**: ✅ Done — commit `d7300c877`

**Deliverable**: Fixed `docs/releases/v3.9.0/QUICK_START.md`

| Before (wrong) | After (correct) |
|---------------|-----------------|
| `sqlrustgo server --port 5432` | `sqlrustgo-cli serve` (port 3306) |
| `psql -h localhost -p 5432` | `mysql -h 127.0.0.1 -P 3306 -u tester -ptester` |
| `sqlrustgo cli -c "SELECT 1+1"` | Phase 3 skeleton + note |
| `sqlrustgo tpch generate` | `cargo run --example tpch_data_gen` |
| PostgreSQL v3 wire | MySQL wire protocol |
| GitHub releases URL | Gitea releases URL |
| No REPL section | Added REPL section |
| No "Next steps" | Restored "Next steps" |

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
| `sqlrustgo-cli` | Canonical CLI (NEW, user-facing) | Delegates to mysql-server |
| `sqlrustgo-mysql-server` | Full-featured server (production) | Embedded in-process |
| `sqlrustgo-admin` | Offline admin (backup/restore/pitr) | File-based, no server |

---

## Verification Checklist

- [x] `cargo build -p sqlrustgo-cli --all-features` compiles without errors
- [x] `cargo build -p sqlrustgo-mysql-server --all-features` unchanged, still compiles
- [x] `cargo build -p sqlrustgo-admin --all-features` unchanged, still compiles
- [x] `./target/debug/sqlrustgo-cli exec "SELECT 1"` → `col_0 | Integer(1) (1 rows)` ✅
- [x] `./target/debug/sqlrustgo-cli --help` shows all 9 subcommands ✅
- [x] `./target/debug/sqlrustgo-cli serve --help` ✅
- [x] `./target/debug/sqlrustgo-cli cli --help` ✅
- [x] Every QUICK_START.md command verified against actual binary
- [ ] `cargo fmt --check` → no formatting regressions (pending fmt in new crate)
