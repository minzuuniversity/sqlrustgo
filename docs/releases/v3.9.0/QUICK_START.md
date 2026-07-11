<!-- 2026-07-11 文档同步: v3.9.0 GA CUT 状态更新 — 168h SOAK ✅ PASS (2026-07-12) -->

---

# Quick Start — SQLRustGo v3.9.0

> 5-minute hands-on guide to the v3.9.0 release. v3.9.0 ships with
> full 22-query TPC-H coverage, in-process and wire-protocol parity,
> and three-way cell-level comparison against SQLite + PostgreSQL.

## What is SQLRustGo?

SQLRustGo is a from-scratch SQL database engine implemented in pure
Rust. It is designed as a learning + research platform for SQL
internals: parser, planner, optimizer, executor, transaction
manager, MVCC, and WAL are all written from scratch (no SQLite
fork, no PostgreSQL reuse). v3.9.0 is the **22-query TPC-H
benchmark PASS** milestone.

## Highlights in v3.9.0

- 22/22 TPC-H queries pass cell-level comparison against SQLite
  (SF=0.001, in-process) and 22/22 over the wire protocol
- 21/22 match against SQLite at SF=0.01 (Q22 is a known
  multi-COUNT expression divergence)
- Q8/Q9/Q17/Q21 perf: all < 2s (Q8=200ms, Q21=1.7s)
- Q13 subquery fix: NOT IN (subquery_with_LIKE) now correctly
  excludes zero customers (previously returned 0 rows)
- Q7/Q8/Q9 SQLite baseline fix: ORDER BY stripping + EXTRACT(YEAR
  FROM) regex rewrite
- SGL-001 cargo fmt + B3 clippy clean (0 errors, 0 hard fails)
- Plan integrity gate: IR plan + legacy path cross-validation
- 4-engine wired test (engine / SQLite / MariaDB / PostgreSQL)
- DuckDB baseline added for additional TPC-H coverage
- INT-2 cross-version upgrade (synthetic v3.8.0 file format)
- MySQL server restart persistence (WAL replay on startup)

## Install

### Pre-built binary (recommended for evaluation)

```bash
# Download from Gitea releases (not GitHub)
curl -L https://github.com/openclaw/sqlrustgo/releases/download/v3.9.0/sqlrustgo-v3.9.0-linux-x86_64.tar.gz | tar xz
# If using the MySQL-server binary directly:
./sqlrustgo-mysql-server --version
# → sqlrustgo-mysql-server v3.9.0
```

### From source

```bash
git clone http://192.168.0.252:3000/openclaw/sqlrustgo.git
cd sqlrustgo
git checkout v3.9.0
cargo build --release -p sqlrustgo-cli
# Binary at: ./target/release/sqlrustgo-cli
cargo build --release -p sqlrustgo-mysql-server
# Binary at: ./target/release/sqlrustgo-mysql-server
```

Requirements: Rust 1.75+, a C linker (gcc/cc/clang).

## First server

```bash
# Using sqlrustgo-cli (canonical entry point, delegates to sqlrustgo-mysql-server):
./target/release/sqlrustgo-cli serve
# Server starts on 127.0.0.1:3306 speaking MySQL wire protocol

# With persistence:
mkdir -p /tmp/sqlrustgo/{data,wal}
./target/release/sqlrustgo-cli serve --data-dir /tmp/sqlrustgo/data
```

> **Note**: The server speaks **MySQL wire protocol** (mysql_native_password auth),
> not PostgreSQL. Use a MySQL client to connect.

## First query

```bash
# Execute a single query via the exec subcommand (no server needed):
./target/release/sqlrustgo-cli exec "SELECT 1 + 1 AS two"
# → col_0 | Integer(2) (1 rows)

# Or connect with the mysql CLI client to a running server:
mysql -h 127.0.0.1 -P 3306 -u tester -ptester -e "SELECT 1 + 1 AS two"
```

## Interactive REPL

```bash
# Start an interactive REPL session:
./target/release/sqlrustgo-cli repl
# SQLRustGo REPL v3.8.0 — type `.help` for commands, `.exit` to quit
# sqlrustgo> CREATE TABLE t (id INT, name TEXT);
# sqlrustgo> INSERT INTO t VALUES (1, 'Alice'), (2, 'Bob');
# sqlrustgo> SELECT * FROM t;
```

> **Note**: The `sqlrustgo cli` subcommand (auto-spawn + query) is planned for v3.10.0 (Phase 3 of CLI plan).

## Run TPC-H 22-query suite

```bash
# Generate TPC-H data using the bench example:
cargo run --example tpch_data_gen -- --sf 0.001 --output tests/data/tpch-sf001

# Run in-process TPC-H tests:
cargo test --test tpch_full_22_test -- --nocapture
# Expected: test result: ok. 22 passed; 0 failed

# Run wire-protocol TPC-H tests:
cargo test --test tpch_22_queries_wire_test -- --nocapture
# Expected: 22/22 wire PASS
```

## Troubleshooting

### Server won't start
```bash
ss -tlnp | grep :3306
# If port 3306 is in use:
./target/release/sqlrustgo-cli serve --port 3307
```

### Tests fail with "fixture not found"
```bash
cargo run --example tpch_data_gen -- --sf 0.001 --output tests/data/tpch-sf001
```

### Build is slow
```bash
cargo install mold
export RUSTFLAGS="-C link-arg=-fuse-ld=mold"
```

### Tests OOM
Reduce fixture scale: `cargo run --example tpch_data_gen -- --sf 0.0001`.

## Architecture at a glance

- **Parser** (`crates/parser/`): recursive-descent, ~30 KLOC
- **Planner + IR** (`src/planner.rs`, `crates/ir/`):
  IR-based plans with cross-validation against legacy path
- **Optimizer** (`src/cbo_estimator.rs`): cost-based with histogram
  stats
- **Executor** (`src/execution_engine.rs`): pull-based iterator model
- **Storage** (`crates/storage/`): MVCC + heap pages
- **WAL** (`crates/wal/`): write-ahead log with group commit
- **Wire** (`src/wire_protocol.rs`): PostgreSQL v3 protocol

## Five-Engine Baseline

v3.9.0 ships reference cell-level baselines for five engines:

| Engine | Status | Reference |
|---|---|---|
| SQLRustGo v3.9.0 | 22/22 PASS | This release |
| SQLite 3.45+ | baseline | Reference |
| MariaDB 10.11+ | baseline | Wire test (optional) |
| PostgreSQL 15+ | baseline | Wire test (optional) |
| DuckDB 0.10+ | baseline | Reference (v3.9.0+) |

## Wire protocol support

| Protocol | Status |
|---|---|
| PostgreSQL v3 | ✅ Simple + extended query |
| TLS (PostgreSQL) | ✅ sslmode=require |
| SCRAM-SHA-256 auth | ✅ |
| MySQL | ❌ Out of scope |

## Storage engine

| Feature | Status |
|---|---|
| Heap pages (8 KB) | ✅ |
| MVCC | ✅ Snapshot isolation |
| WAL | ✅ Group commit |
| Checkpointing | ✅ Fuzzy |
| Crash recovery | ✅ REDO only (no UNDO) |

## SQL coverage

Full TPC-H 22/22 PASS. SQL92 core (DML/joins/aggregates) is
production-quality. Window functions and recursive CTEs are not yet
implemented (see FEATURE_MATRIX.md for the complete list).
