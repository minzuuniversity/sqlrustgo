# SQLRustGo CLI — User Manual

> **Version**: v3.9.0
> **Binary**: `sqlrustgo-cli`
> **Updated**: 2026-06-28
> **Branch**: `feat/sqlrustgo-cli-soak-repl`

## 1. 概述

`sqlrustgo-cli` is a lightweight MySQL wire-protocol client for SQLRustGo. It connects to a running `sqlrustgo-mysql-server` (or any MySQL-compatible server) and provides three operating modes:

| Mode | Command | Purpose |
|------|---------|---------|
| One-shot query | `exec` | Execute a single SQL statement, print results |
| Interactive shell | `repl` | Readline-based REPL with history |
| SOAK test runner | `soak` | Continuous SQL workload with latency/throughput metrics |

### Architecture

```
┌──────────────────┐     MySQL Wire Protocol      ┌──────────────────────┐
│  sqlrustgo-cli   │ ◄─────────────────────────►  │ sqlrustgo-mysql-server│
│  (exec/repl/soak)│     (TCP, native_password)   │   (port 3306/3396)   │
└──────────────────┘                               └──────────────────────┘
```

The client speaks the MySQL wire protocol directly (raw TCP, no external client library). Authentication uses `mysql_native_password`.

## 2. Build

```bash
# Release build
cargo build --release --bin sqlrustgo-cli

# Binary location
./target/release/sqlrustgo-cli --help
```

Dependencies are pulled automatically via Cargo workspace. The binary size is approximately 5.5 MB (debug) / 2.1 MB (release, stripped).

## 3. Global Options

All subcommands accept connection parameters:

| Option | Default | Description |
|--------|---------|-------------|
| `--host` | `127.0.0.1` | Server hostname or IP |
| `--port` | `3306` | Server TCP port |
| `--user` | `root` | MySQL username |
| `--password` | `""` | MySQL password (default empty for no-auth mode) |

## 4. `exec` — One-Shot Query Execution

### Usage

```bash
sqlrustgo-cli exec [OPTIONS] <SQL>
```

### Examples

```bash
# Simple query
sqlrustgo-cli exec "SELECT 1 + 1 AS result"

# With server options
sqlrustgo-cli exec --host 192.168.1.100 --port 3396 --user admin "SELECT COUNT(*) FROM orders"

# JSON output (machine-readable)
sqlrustgo-cli exec --json "SELECT * FROM lineitem LIMIT 5"
```

### Output

**Text mode** (default): column headers, separator line, row data, row count + timing:

```
col_1 | col_2 | col_3
------+-------+-------
    1 | hello | 3.14
    2 | world | 2.71
(2 rows in 0.5ms)
```

**JSON mode** (`--json`):

```json
{
  "columns": ["col_1", "col_2"],
  "rows": [
    {"col_1": "1", "col_2": "hello"},
    {"col_1": "2", "col_2": "world"}
  ],
  "row_count": 2,
  "duration_ms": 0.5
}
```

### Error Handling

- Server returns ERR packet → error message printed to stderr, exit code 1.
- Connection refused → immediate failure with "Connection refused" message.
- Malformed SQL → server-dependent error message.

## 5. `repl` — Interactive REPL

### Usage

```bash
sqlrustgo-cli repl [OPTIONS]
```

### Features

- Readline support with command history (saved to `.sqlrustgo_history` in working directory)
- Multi-line SQL entry (semicolons not required — each line is sent as a complete statement)
- Built-in commands:
  - `exit` or `quit` — exit REPL
  - `Ctrl-D` — exit REPL
- Query results printed in tabular format with timing

### Example Session

```
$ sqlrustgo-cli repl --port 3306
SQLRustGo CLI REPL — connected to 127.0.0.1:3306
Enter SQL statements. Type `exit` or Ctrl-D to quit.

sqlrustgo> SELECT COUNT(*) FROM region
col_1
-----
5
(1 row in 0.3ms)

sqlrustgo> SELECT n_name, COUNT(*) FROM nation, customer
         WHERE n_nationkey = c_nationkey GROUP BY n_name
n_name | COUNT(*)
-------+---------
CHINA  | 3
INDIA  | 2
...
(5 rows in 0.8ms)

sqlrustgo> exit
Bye.
```

## 6. `soak` — Real SQL SOAK Test Runner

This is the primary feature for stability testing. Unlike the simulated soak harness (`tests/soak_test_harness.rs`), the CLI soak runner executes **real SQL queries** over the MySQL wire protocol against a live server.

### Usage

```bash
sqlrustgo-cli soak [OPTIONS]
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--duration` | `60` | Wall-clock duration in seconds |
| `--rate` | `5.0` | Target queries per second (0 = max speed) |
| `--query-file` | (built-in) | Path to SQL query file (one statement per line) |
| `--report-interval` | `10` | Progress report interval in seconds |
| `--json` | (false) | Output report as JSON |

### Built-in Query Pool

When no `--query-file` is specified, the soak runner cycles through 12 verified queries that work on any MySQL-compatible server:

```sql
SELECT 1
SELECT 1 + 1 AS two
SELECT 2 * 3 AS six
SELECT 64 / 8 AS eight
SELECT 42 AS answer
SELECT 3.14 AS pi
SELECT 'hello' AS greeting
SELECT 'sqlrustgo' AS name
SELECT LENGTH('sqlrustgo') AS len
SELECT 100 + 200 AS sum
SELECT 1000 - 1 AS minus
SELECT 7 * 8 AS product
```

These queries are deliberately simple and universally supported — they test basic arithmetic, string, and expression evaluation paths.

### Custom Query File

For realistic SOAK testing, provide TPC-H queries against a loaded dataset:

```bash
# Create query file (one SQL per line, skip comments with -- or #)
cat > /tmp/tpch_workload.sql << 'EOF'
-- Aggregations
SELECT COUNT(*) FROM lineitem
SELECT COUNT(*) FROM orders
-- Joins
SELECT COUNT(*) FROM customer, orders WHERE c_custkey = o_custkey
-- Group by
SELECT o_orderpriority, COUNT(*) FROM orders GROUP BY o_orderpriority
-- Multi-table
SELECT n_name, COUNT(*) FROM nation, customer
  WHERE n_nationkey = c_nationkey GROUP BY n_name
EOF

# Run soak with custom workload
sqlrustgo-cli soak --port 3397 --duration 3600 --rate 5 \
  --query-file /tmp/tpch_workload.sql
```

### SOAK Test Lifecycle

1. **Connect** — authenticate to server with MySQL native password
2. **Warm-up** — run each query once to establish baseline
3. **Main loop** — cycle through query pool at specified rate, collecting per-query latency
4. **Rate limiting** — if ahead of schedule, sleep to maintain target QPS
5. **Progress reporting** — every `--report-interval` seconds, print cumulative stats to stderr
6. **Report** — on completion, print latency percentiles and throughput

### Report Format (Text Mode)

```
═ SOAK Test Report ═══════════════════════════════
  Duration:              60s
  Queries executed:      300
  Errors:                0
  Actual QPS:            5.0
  Latency:
    P50:                 0.55ms
    P90:                 0.65ms
    P99:                 0.74ms
    Avg:                 0.53ms
    Max:                 2.90ms
═══════════════════════════════════════════════════

  ✅ SOAK PASSED
```

### Report Format (JSON Mode)

```bash
sqlrustgo-cli soak --duration 60 --rate 5 --json
```

Output:

```json
{
  "duration_secs": 60,
  "queries_executed": 300,
  "errors": 0,
  "p50_latency_ms": 0.55,
  "p90_latency_ms": 0.65,
  "p99_latency_ms": 0.74,
  "avg_latency_ms": 0.53,
  "max_latency_ms": 2.90,
  "actual_qps": 5.0
}
```

### PASS/FAIL Criteria

| Condition | Result |
|-----------|--------|
| 0 errors | PASS |
| P99 < 5000ms | PASS |
| > 0 queries executed | PASS |
| Any error > 0 | WARNING (still PASS) |
| P99 >= 5000ms | FAIL |
| 0 queries executed | FAIL |

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | SOAK PASSED (all criteria met) |
| 1 | SOAK FAILED (errors > 0 or zero queries) or internal error |

## 7. SOAK Testing Methodology

### 7.1 Compressed-Time vs Real-Time

The CLI soak runner supports both approaches:

| Approach | Duration | Purpose |
|----------|----------|---------|
| **Compressed-time smoke** | 60-420s (5 qps) | CI regression, harness validation |
| **Real-time soak** | 24h / 72h / 168h | GA gate, production readiness |

The compressed-time approach assumes that if resource patterns (alloc/dealloc) are stable at 5 qps for 60s, they will remain stable at the same rate for 24h. This is validated by the existing `tests/soak_test.rs` harness for memory/FD/lock invariants.

The CLI soak runner adds real query execution to this model — it validates that the **SQL engine itself** (not just the harness) remains stable under continuous load.

### 7.2 Recommended SOAK Levels

| Level | Duration | Rate | Queries | Equivalent real | Purpose |
|-------|----------|------|---------|-----------------|---------|
| Smoke | 60s | 5 qps | 300 | ~24h at same rate | CI / dev testing |
| Short | 3600s (1h) | 5 qps | 18,000 | — | Pre-merge validation |
| Medium | 8h | 5 qps | 144,000 | — | Nightly gate |
| Full 24h | 86,400s | 5 qps | 432,000 | 24h | GA gate |
| Full 72h | 259,200s | 1 qps | 259,200 | 72h | RC gate |

### 7.3 Workload Types

| Workload | Query File | Data Required | Coverage |
|----------|-----------|---------------|----------|
| Built-in (generic) | (none) | None | Arithmetic, expressions |
| TPC-H | Custom `.sql` | TPC-H SF>=0.001 | Joins, aggregations, group by |
| Custom | `--query-file` | User-defined | Application-specific |

### 7.4 Running a Full 24h SOAK

**Method A — Using `run_24h_soak_v2.sh` with `USE_CLI_SOAK=1`:**

```bash
USE_CLI_SOAK=1 CLI_SOAK_RATE=5 HOURS=24 \
  ./scripts/stability/run_24h_soak_v2.sh
```

This script:
1. Starts `sqlrustgo-mysql-server` on port 3396
2. Builds `sqlrustgo-cli` if not found
3. Launches `sqlrustgo-cli soak --duration 86400 --rate 5 ...`
4. Collects OS-level metrics (RSS, FD, CPU, WAL) every 60s
5. Generates `STABILITY_REPORT.md` with pass/fail verdict

**Method B — Manual:**

```bash
# 1. Start server
./target/release/sqlrustgo-mysql-server serve \
  --port 3396 --data-dir /tmp/soak-data --auth-mode none &

# 2. Wait for ready
sleep 3

# 3. Run SOAK
./target/release/sqlrustgo-cli soak \
  --host 127.0.0.1 --port 3396 \
  --duration 86400 --rate 5 \
  --report-interval 600 \
  --json > soak_report.json

# 4. Check results
cat soak_report.json

# 5. Stop server
kill %1
```

### 7.5 TPC-H SOAK (with real data)

Load TPC-H SF=0.01 dataset and run targeted queries:

```bash
# Start server with pre-loaded data dir
./target/release/sqlrustgo-mysql-server serve \
  --port 3397 --data-dir tests/data/tpch-sf01 \
  --auth-mode none &

# Run SOAK with TPC-H query workload
sqlrustgo-cli soak --port 3397 --duration 3600 --rate 3 \
  --query-file /tmp/tpch_queries.sql

# Verify data is accessible
sqlrustgo-cli exec --port 3397 "SELECT COUNT(*) FROM lineitem"
```

The TPC-H SF=0.01 dataset contains approximately:
- `lineitem`: 6,100 rows
- `orders`: 1,500 rows
- `customer`: 150 rows
- `part`: 200 rows
- `partsupp`: 800 rows
- `supplier`: 10 rows
- `nation`: 25 rows
- `region`: 5 rows

SF=0.001 is 1/10th scale (approximately 501 lineitem rows).

## 8. Integration with Existing Soak Scripts

### `run_24h_soak_v2.sh`

The existing 24h real wall-clock soak script supports `sqlrustgo-cli soak` as an alternative load generator via environment variables:

```bash
# Default: uses sysbench
HOURS=24 ./scripts/stability/run_24h_soak_v2.sh

# With CLI soak runner (replaces sysbench)
USE_CLI_SOAK=1 HOURS=24 ./scripts/stability/run_24h_soak_v2.sh

# With custom query file and rate
USE_CLI_SOAK=1 CLI_SOAK_RATE=10 CLI_SOAK_QUERIES="/tmp/tpch.sql" \
  HOURS=24 ./scripts/stability/run_24h_soak_v2.sh
```

When `USE_CLI_SOAK=1`:
- Sysbench dependency check is skipped
- `sqlrustgo-cli` is built from source
- The CLI soak runner replaces sysbench as the load generator
- Metrics collection (RSS/FD/CPU/WAL) still uses the same monitoring loop
- QPS is read from `cli_soak.log` instead of `sysbench.log`
- The report template adapts to the load generator

### `run_72h_soak.sh`, `run_168h_soak.sh`

These scripts follow the same pattern as `run_24h_soak_v2.sh`. The `USE_CLI_SOAK=1` option works identically.

## 9. Troubleshooting

### Connection Refused

```bash
sqlrustgo-cli exec --port 3306 "SELECT 1"
# Error: connect 127.0.0.1:3306: Connection refused
```

Causes:
- Server not running. Start it: `sqlrustgo-mysql-server serve --port 3306`
- Wrong port. Verify with `lsof -i :3306`
- Firewall. Ensure port is accessible.

### Auth Failed

```bash
sqlrustgo-cli exec --port 3306 --user root "SELECT 1"
# Error: auth failed: server ERR: Access denied
```

Solutions:
- Use `--auth-mode none` when starting the server for testing
- Check username/password match the server configuration
- Default: `root` with empty password

### Slow Queries / Timeouts

The client has a 30-second read timeout. If a query takes longer:

```bash
# If you hit the timeout, the query may still be running on the server.
# The connection will be broken. Start a new one.
```

For long-running SOAK tests, ensure the server has adequate resources:
- `--max-connections` at least 10 (default depends on server build)
- Sufficient data directory space for WAL growth

### Error During Result Set

```bash
Error: ERR during result set: <message>
```

The server returned an error mid-query (e.g., division by zero, constraint violation). The soak runner counts this as an error and continues.

## 10. Comparison: Simulated vs Real SOAK

| Aspect | `tests/soak_test.rs` (simulated) | `sqlrustgo-cli soak` (real) |
|--------|----------------------------------|----------------------------|
| SQL execution | None (tight CPU loop) | Real MySQL wire protocol |
| Latency measurement | Simulated (fixed delay) | Real timing |
| Query types | N/A | Any SQL supported by server |
| Connection | None | Real TCP connection |
| Duration | 60-420s (compressed) | Any (seconds to weeks) |
| Report granularity | P50/P99/Max | Same |
| Error detection | Simulated | Real server errors |
| Data required | None | Depends on workload |
| CI suitability | ✅ (fast) | ✅ (with `--duration 60`) |
| GA gate validity | Partial | ✅ Full |

## 11. Roadmap

| Feature | Status | Target |
|---------|--------|--------|
| MySQL wire protocol client | ✅ | v3.9.0 |
| `exec` subcommand | ✅ | v3.9.0 |
| `repl` subcommand | ✅ | v3.9.0 |
| `soak` subcommand (built-in queries) | ✅ | v3.9.0 |
| `soak` subcommand (custom query file) | ✅ | v3.9.0 |
| JSON output | ✅ | v3.9.0 |
| TPC-H query file for SOAK | ✅ | v3.9.0 |
| PostgreSQL wire protocol support | ❌ | v3.10+ |
| TLS/SSL connection | ❌ | v3.10+ |
| Prepared statement support | ❌ | v3.10+ |
| Prometheus metrics export during SOAK | ❌ | v3.10+ |
| Distributed SOAK (multi-node) | ❌ | v4.0+ |
