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
curl -L https://github.com/openclaw/sqlrustgo/releases/download/v3.9.0/sqlrustgo-v3.9.0-linux-x86_64.tar.gz | tar xz
sudo mv sqlrustgo /usr/local/bin/
sqlrustgo --version
# → sqlrustgo v3.9.0
```

### From source

```bash
git clone https://github.com/openclaw/sqlrustgo.git
cd sqlrustgo
git checkout v3.9.0
cargo build --release
./target/release/sqlrustgo --version
```

Requirements: Rust 1.75+, a C linker (gcc/cc/clang).

## First server

```bash
# In-memory mode (no persistence)
sqlrustgo server --port 5432

# With persistence
mkdir -p /var/lib/sqlrustgo/{data,wal,snapshots}
sqlrustgo server --port 5432 --data-dir /var/lib/sqlrustgo
```

The server speaks PostgreSQL v3 wire protocol.

## First query

```bash
sqlrustgo cli -c "SELECT 1 + 1 AS two;"
psql -h localhost -p 5432 -c "SELECT 1 + 1 AS two;"
```

## Run TPC-H 22-query suite

```bash
sqlrustgo tpch generate --sf 0.001 --output tests/data/tpch-sf001
cargo test --test tpch_full_22_test -- --nocapture
# Expected: test result: ok. 22 passed; 0 failed
cargo test --test tpch_22_queries_wire_test -- --nocapture
# Expected: 22/22 wire PASS
```

## Troubleshooting

### Server won't start
```bash
ss -tlnp | grep :5432
sqlrustgo server --port 5433
```

### Tests fail with "fixture not found"
```bash
sqlrustgo tpch generate --sf 0.001 --output tests/data/tpch-sf001
```

### Build is slow
```bash
cargo install mold
export RUSTFLAGS="-C link-arg=-fuse-ld=mold"
```

### Tests OOM
Reduce fixture scale: `sqlrustgo tpch generate --sf 0.0001`.

## Next steps

- [FEATURE_MATRIX.md](FEATURE_MATRIX.md) — full feature list
- [RELEASE_NOTES.md](RELEASE_NOTES.md) — what changed since v3.8.0
- [DEPLOYMENT_GUIDE.md](DEPLOYMENT_GUIDE.md) — production deployment
- [MIGRATION_GUIDE.md](MIGRATION_GUIDE.md) — upgrade from v3.8.0
- [INSTALL.md](INSTALL.md) — full build options
- [CHANGELOG.md](CHANGELOG.md) — per-commit history
- [EVALUATION_REPORT.md](EVALUATION_REPORT.md) — TPC-H results


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
