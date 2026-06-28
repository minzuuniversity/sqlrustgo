# SOAK Test Specification — P1-3

## Purpose

Establish a 3-layer soak test framework for SQLRustGo v3.9.0 using third-party standard tools. Detects memory leaks, FD leaks, WAL growth, and latency regressions under sustained load.

## Requirements

### Req 1: Layer 1 — Rust Simulation Harness

The system SHALL provide `tests/soak_test.rs` and `tests/soak_test_harness.rs` that simulate a soak test in-process, exercising harness logic (config parsing, threshold checking, report generation) without external dependencies.

#### Scenario: soak_24h_smoke_60s
- **WHEN** `SoakConfig { duration_seconds: 60, queries_per_second: 5 }` is passed to `run_soak_smoke`
- **THEN** `SoakReport.queries_executed` SHALL be `300` (60 × 5)
- **AND** `SoakReport.p99_latency_ms` SHALL be > 0

#### Scenario: soak_24h_smoke_memory_growth_within_threshold
- **WHEN** the 60s smoke test runs with default memory baseline (100 MB)
- **THEN** `SoakReport.memory_growth_pct` SHALL be ≤ 10.0

#### Scenario: soak_alert_message_when_exceeds_threshold
- **WHEN** `SoakConfig { memory_alert_threshold_pct: 0 }` is used
- **THEN** `SoakReport.alert_triggered` SHALL be `true`
- **AND** `SoakReport.alert_reason` SHALL contain "exceeds threshold"

#### Scenario: soak_memory_baseline_invariant
- **WHEN** `SoakConfig { duration_seconds: 0 }` is used (no queries)
- **THEN** `SoakReport.memory_final_bytes` SHALL equal `SoakConfig.memory_baseline_bytes`

### Req 2: Layer 2 — TPC-H Soak with `mariadb-slap`

The system SHALL provide scripts enabling a user to run a TPC-H soak test using `mariadb-slap` with 16 concurrent connections against SF=0.1 data.

#### Scenario: tpch_sf01_data_preparation
- **WHEN** `scripts/soak/prepare_sf01_data.sh` is executed
- **THEN** it SHALL produce SF=0.1 fixture files in `data/tpch-sf01/`
  - `lineitem_sf01_tbl` with ~600,000 rows
  - `orders_sf01_tbl` with ~150,000 rows
  - `customer_sf01_tbl` with ~15,000 rows
  - `part_sf01_tbl` with ~20,000 rows
  - `partsupp_sf01_tbl` with ~80,000 rows
  - `supplier_sf01_tbl` with ~1,000 rows
  - `nation_sf01_tbl` with 25 rows
  - `region_sf01_tbl` with 5 rows
- **AND** all files use `|` delimiter matching TPC-H `.tbl` format

#### Scenario: tpch_schema_creation
- **WHEN** `scripts/soak/tpch_schema.sql` is applied to sqlrustgo
- **THEN** it SHALL create 8 tables: `region`, `nation`, `supplier`, `customer`, `part`, `partsupp`, `orders`, `lineitem`
- **AND** all tables match TPC-H schema (column names, types, primary keys)

#### Scenario: tpch_data_load
- **WHEN** SF=0.1 fixture files exist and schema is created
- **THEN** `mysql --local-infile` SHALL load all 8 tables successfully
- **AND** row counts SHALL match the expected SF=0.1 counts

#### Scenario: tpch_soak_30min
- **WHEN** `scripts/soak/mysqlslap_soak.sh --level=30m` is executed against a running sqlrustgo server with SF=0.1 data loaded
- **THEN** the soak SHALL run for approximately 30 minutes
- **AND** `--concurrency=16` SHALL be used
- **AND** `--query=scripts/soak/tpch_queries.sql` SHALL be used
- **AND** the soak report SHALL be generated

#### Scenario: tpch_soak_memory_stability
- **WHEN** a 30-minute TPCH soak completes with no errors
- **THEN** `SoakReport.memory_growth_pct` SHALL be < 10%
- **AND** `SoakReport.fd_growth` SHALL be < +5

### Req 3: Layer 3 — `mariadb-slap` Auto-Generate Soak

The system SHALL provide scripts for an end-to-end soak using `mariadb-slap` auto-generate schema (sbtest) with 16 concurrent connections.

#### Scenario: autoschema_soak_30min
- **WHEN** `scripts/soak/mysqlslap_soak.sh --level=30m --auto-generate` is executed
- **THEN** `mariadb-slap` SHALL auto-generate the `sbtest` schema
- **AND** `--concurrency=16` SHALL be used
- **AND** `--auto-generate-sql-load-type=mixed` SHALL be used
- **AND** the soak SHALL run for approximately 30 minutes

#### Scenario: autoschema_soak_memory_stability
- **WHEN** a 30-minute auto-generate soak completes with 0 errors
- **THEN** `SoakReport.memory_growth_pct` SHALL be < 10%
- **AND** `SoakReport.fd_growth` SHALL be < +5

### Req 4: SoakReport Format

The system SHALL produce JSON reports conforming to the following schema:

```json
{
  "level": "string",          // "30m" | "4h"
  "duration_seconds": "number",
  "concurrency": "number",    // 16
  "queries_executed": "number",
  "errors": "number",        // MUST be 0
  "memory_baseline_bytes": "number",
  "memory_final_bytes": "number",
  "memory_growth_pct": "number",   // MUST be < 10.0
  "fd_baseline": "number",
  "fd_final": "number",
  "fd_growth": "number",          // MUST be < 5
  "p50_latency_ms": "number",
  "p99_latency_ms": "number",
  "alert_triggered": "boolean"     // MUST be false
}
```

#### Scenario: report_has_all_required_fields
- **WHEN** `extract_soak_report.py` processes valid input
- **THEN** the output JSON SHALL contain all 14 fields listed above
- **AND** no additional fields SHALL be present

### Req 5: G7 Gate

The system SHALL provide `scripts/gate/check_p13_soak_test.sh` that verifies all 9 conditions:

1. `tests/soak_test_harness.rs` exists
2. `tests/soak_test.rs` exists and is registered in `Cargo.toml`
3. 3-level smoke equivalence constants are correct (24h→60s, 72h→180s, 168h→420s)
4. `cargo check --test soak_test` passes
5. `cargo test --test soak_test` runs ≥ 10 tests and all pass
6. Alert-threshold mechanism works (tight threshold triggers alert)
7. Memory baseline invariant holds (no-query run equals baseline)
8. `scripts/soak/mysqlslap_soak.sh` exists
9. `scripts/soak/extract_soak_report.py` exists

## Test Cases

### Layer 1 (Rust, CI)

| ID | Test | Command | Expected |
|----|------|---------|----------|
| L1-1 | soak_24h_smoke | `cargo test --test soak_test test_soak_24h_smoke` | PASS |
| L1-2 | soak_memory_threshold | `cargo test --test soak_test test_soak_24h_smoke_memory_growth` | PASS |
| L1-3 | soak_alert | `cargo test --test soak_test test_soak_alert_message` | PASS |
| L1-4 | soak_baseline_invariant | `cargo test --test soak_test test_soak_memory_baseline_invariant` | PASS |
| L1-5 | soak_p50_p99_ordering | `cargo test --test soak_test test_soak_p50_p99_ordering` | PASS |

### Layer 2 (TPC-H + mysqlslap, Manual)

| ID | Test | Command | Expected |
|----|------|---------|----------|
| L2-1 | Data prep | `bash scripts/soak/prepare_sf01_data.sh` | SF=0.1 files created |
| L2-2 | Schema | `mysql ... < scripts/soak/tpch_schema.sql` | 8 tables created |
| L2-3 | Data load | `mysql --local-infile ... LOAD DATA` | 8 tables loaded |
| L2-4 | Soak 30m | `bash scripts/soak/mysqlslap_soak.sh --level=30m` | ~30 min, PASS |
| L2-5 | Report | `python3 scripts/soak/extract_soak_report.py` | JSON SoakReport |

### Layer 3 (Auto-generate + mysqlslap, Manual)

| ID | Test | Command | Expected |
|----|------|---------|----------|
| L3-1 | Soak 30m | `bash scripts/soak/mysqlslap_soak.sh --level=30m --auto-generate` | ~30 min, PASS |
| L3-2 | Report | `python3 scripts/soak/extract_soak_report.py` | JSON SoakReport |

## Coverage

| Metric | Tool | Threshold |
|--------|------|-----------|
| Memory RSS | procfs | baseline + 10% |
| File descriptors | procfs | baseline + 5 |
| WAL size | procfs | baseline + 5% |
| Query latency P99 | mysqlslap | < 500ms |
| Error count | mysqlslap | = 0 |

## Dependencies

- `mariadb-slap` or `mysqlslap` (MySQL/MariaDB client)
- `mysql` CLI with `--local-infile` support
- `python3` (for extract_soak_report.py)
- SF=1 TPC-H data in `data/tpch-sf01/` (existing)
