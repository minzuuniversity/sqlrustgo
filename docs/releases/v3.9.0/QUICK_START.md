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
- Three-engine baseline: engine / SQLite / PostgreSQL side-by-side
- Q13 subquery fix: NOT IN (subquery_with_LIKE) now correctly
  excludes zero customers (previously returned 0 rows)
- Q7/Q8/Q9 SQLite baseline fix: ORDER BY stripping + EXTRACT(YEAR
  FROM) regex rewrite so the SQLite reference matches engine output
- SGL-001 cargo fmt + B3 clippy clean (0 errors, 0 hard fails)
- Plan integrity gate: IR plan + legacy path cross-validation
- 4-engine wired test (engine / SQLite / MariaDB / PostgreSQL) for
  Sprint 3 Operator Regression Suite
- DuckDB baseline added for additional TPC-H coverage
- INT-3 mixed-scenario DML tests added

## Install

### Pre-built binary (recommended for evaluation)

```bash
# Linux x86_64
curl -L https://github.com/openclaw/sqlrustgo/releases/download/v3.9.0/sqlrustgo-v3.9.0-linux-x86_64.tar.gz | tar xz
sudo mv sqlrustgo /usr/local/bin/

# Verify
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

Requirements:
- Rust 1.75+ (`rustup install stable`)
- A C linker (`gcc` / `cc` / `clang`)
- 4 GB RAM for a release build (in-process tests want 8 GB)

## First server

```bash
# In-memory mode (no persistence — fastest start)
sqlrustgo server --port 5432

# With persistence
mkdir -p /var/lib/sqlrustgo
sqlrustgo server --port 5432 --data-dir /var/lib/sqlrustgo

# Foreground verbose
RUST_LOG=info sqlrustgo server --port 5432 --data-dir /var/lib/sqlrustgo
```

The server speaks the PostgreSQL wire protocol (v3), so any
PostgreSQL client works.

## First query

```bash
# CLI REPL
sqlrustgo cli
sqlrustgo> SELECT 1 + 1 AS two;
 two
-----
   2
(1 row)

# Single command
sqlrustgo cli -c "SELECT 1 + 1 AS two;"

# psql
psql -h localhost -p 5432 -U sqlrustgo -c "SELECT 1 + 1 AS two;"

# Python (psycopg2)
psql -h localhost -p 5432 -c "SELECT version();" | head -3
```

## Run the TPC-H 22-query suite

```bash
# Generate SF=0.001 fixture (6 MB, 22 queries run in ~2.3s)
sqlrustgo tpch generate --sf 0.001 --output tests/data/tpch-sf001

# Run all 22 queries in-process
cargo test --test tpch_full_22_test -- --nocapture
# Expected: test result: ok. 22 passed; 0 failed

# Run via wire protocol
cargo test --test tpch_22_queries_wire_test -- --nocapture
# Expected: test result: ok. 22 passed; 0 failed

# Three-way comparison: engine vs SQLite vs (optionally) PostgreSQL
cargo test --test tpch_sf01_22_vs_3engines -- --nocapture
# Expected: 21/22 PASS, 1 known divergence (Q22)
```

## Architecture at a glance

- **Parser** (`crates/parser/`): recursive-descent, ~30 KLOC
- **Planner + IR** (`src/planner.rs`, `crates/ir/`):
  IR-based plans with cross-validation against legacy
- **Optimizer** (`src/cbo_estimator.rs`): cost-based with histogram
  stats
- **Executor** (`src/execution_engine.rs`): pull-based iterator
  model
- **Storage** (`crates/storage/`): MVCC + heap pages
- **WAL** (`crates/wal/`): write-ahead log with group commit
- **Wire** (`src/wire_protocol.rs`): PostgreSQL v3 protocol

## Troubleshooting

### Server won't start: "address already in use"

```bash
# Find what's using the port
ss -tlnp | grep :5432
# Or change the port
sqlrustgo server --port 5433
```

### Tests fail with "fixture not found"

```bash
# Re-generate the fixture
sqlrustgo tpch generate --sf 0.001 --output tests/data/tpch-sf001
# Or set TPCH_FIXTURE env var
export TPCH_FIXTURE=tests/data/tpch-sf001
```

### Build is slow

```bash
# Use mold linker (Linux) for 3-5x faster link
cargo install mold
export RUSTFLAGS="-C link-arg=-fuse-ld=mold"

# Or use sccache for incremental builds
cargo install sccache
export RUSTC_WRAPPER=sccache
```

### Tests panic with "out of memory"

TPC-H 22-query suite needs ~4 GB RAM at SF=0.01. Reduce the
fixture scale:

```bash
sqlrustgo tpch generate --sf 0.0001 --output tests/data/tpch-sf0001
export TPCH_FIXTURE=tests/data/tpch-sf0001
```

## Next steps

- [`FEATURE_MATRIX.md`](FEATURE_MATRIX.md) — full feature list and
  SQL92 coverage
- [`RELEASE_NOTES.md`](RELEASE_NOTES.md) — what changed since v3.8.0
- [`DEPLOYMENT_GUIDE.md`](DEPLOYMENT_GUIDE.md) — production
  deployment with systemd + TLS
- [`MIGRATION_GUIDE.md`](MIGRATION_GUIDE.md) — upgrade from v3.8.0
- [`EVALUATION_REPORT.md`](EVALUATION_REPORT.md) — TPC-H results
  vs SQLite + PostgreSQL
- [`INSTALL.md`](INSTALL.md) — full build options and platform notes
- [`CHANGELOG.md`](CHANGELOG.md) — per-commit history
