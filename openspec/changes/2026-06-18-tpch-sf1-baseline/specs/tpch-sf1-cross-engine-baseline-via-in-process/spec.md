# Capability: tpch-sf1-cross-engine-baseline-via-in-process

## Purpose

Establish a reproducible TPC-H SF=1.0 baseline for sqlrustgo on the
in-process wire-protocol surface (`MySqlTestClient` + `start_ephemeral`).
The baseline records, for each of the 22 canonical TPC-H queries, the
row count and per-query wall-clock time, and emits a single Markdown
report at `docs/releases/v3.10.0/perf/SF1_BASELINE_REPORT.md`.

This change deliberately does NOT cover a cross-engine comparison
(sqlrustgo vs MariaDB vs SQLite). That comparison depends on a
working external mysql client, which is tracked in issue #3474 and
is out of scope here.

## Scope

In scope:
- Loading the SF=1.0 fixture (8 dbgen `.tbl` files) into the
  in-process sqlrustgo ephemeral via LOAD DATA LOCAL INFILE.
- Running the 22 canonical TPC-H queries (Q1..Q22) on the loaded
  fixture.
- Recording per-query row count and wall-clock time.
- Writing the report to
  `docs/releases/v3.10.0/perf/SF1_BASELINE_REPORT.md`.
- A non-cargo shell wrapper `scripts/tpch_sf1_baseline.sh` that
  drives the same flow on a developer workstation.

Out of scope:
- External mysql client (mysql-client 8.0.46) path — issue #3474.
- MariaDB / SQLite cross-engine comparison — depends on #3474.
- Cell-level value comparison across engines.
- Generating the SF=1.0 fixture. The fixture is generated upstream
  via `dbgen -s 1 -f` and committed to the test data directory as
  LFS-tracked files. Generation is the operator's responsibility;
  the test skips cleanly when the fixture is absent.

## ADDED Requirements

### Requirement: SF=1.0 fixture at `/tmp/tpch-sf1`

The repository MUST provide a TPC-H SF=1.0 fixture at the absolute
path `/tmp/tpch-sf1` containing the 8 dbgen-produced `.tbl` files
(`region.tbl`, `nation.tbl`, `supplier.tbl`, `customer.tbl`,
`part.tbl`, `partsupp.tbl`, `orders.tbl`, `lineitem.tbl`) with row
counts matching the TPC-H spec at scale factor 1.0.

#### Scenario: Fixture present and row counts match TPC-H spec at SF=1.0

- **GIVEN** an operator has run `dbgen -s 1 -f` and moved the
  resulting `.tbl` files to `/tmp/tpch-sf1/`
- **THEN** the following row counts hold:
  - `region.tbl`: 5 rows
  - `nation.tbl`: 25 rows
  - `supplier.tbl`: 10,000 rows
  - `customer.tbl`: 150,000 rows
  - `part.tbl`: 200,000 rows
  - `partsupp.tbl`: 800,000 rows
  - `orders.tbl`: 1,500,000 rows
  - `lineitem.tbl`: 6,001,215 rows
- **AND** the total disk footprint of the 8 files is approximately
  1.1 GB.

#### Scenario: Fixture absent, test skips

- **GIVEN** no SF=1.0 fixture is present at `/tmp/tpch-sf1/`
- **WHEN** the test harness is invoked
- **THEN** the test is marked `#[ignore]`
- **AND** the test prints an actionable skip message naming the
  generator command and target directory.

### Requirement: In-process cross-engine test executes 22/22 TPC-H queries at SF=1.0

The test file `tests/tpch_sf1_22_vs_3engines_test.rs` MUST execute
all 22 canonical TPC-H queries (Q1..Q22) against the in-process
sqlrustgo server via `MySqlTestClient` + `start_ephemeral` on the
SF=1.0 fixture, and MUST record a row count and wall-clock time
per query.

#### Scenario: Test executes all 22 queries without error on the in-process surface

- **GIVEN** the SF=1.0 fixture is present at `/tmp/tpch-sf1/`
- **AND** the fixture has been materialized into the data dir
  (i.e., the per-table `.json` files exist with valid row counts)
- **WHEN** `cargo test --test tpch_sf1_22_vs_3engines_test -- --ignored --nocapture`
  is invoked
- **THEN** the test runs end-to-end with no panics or test failures
- **AND** it records 22 entries (Q1..Q22) with non-empty row counts
- **AND** every query completes within the `LOADER_TIMEOUT_S` budget
  of 1800 seconds.

#### Scenario: Test degrades gracefully when the fixture is absent

- **GIVEN** no SF=1.0 fixture is present at `/tmp/tpch-sf1/`
- **WHEN** `cargo test --test tpch_sf1_22_vs_3engines_test` is
  invoked (without `--ignored`)
- **THEN** the test is skipped via `#[ignore]`
- **AND** the skip message names the generator command and target
  directory.

#### Scenario: Test degrades gracefully when the fixture is present but the data dir is empty

- **GIVEN** the SF=1.0 `.tbl` files are present at `/tmp/tpch-sf1/`
- **AND** no per-table `.json` files exist in the data dir
- **WHEN** the test runs
- **THEN** the test runs LOAD DATA LOCAL INFILE to materialize the
  fixture before executing the 22 queries
- **AND** subsequent runs reuse the materialized `.json` files and
  skip the LOAD DATA step.

### Requirement: Baseline report at `docs/releases/v3.10.0/perf/SF1_BASELINE_REPORT.md`

The test harness MUST write a Markdown report to
`docs/releases/v3.10.0/perf/SF1_BASELINE_REPORT.md` after the
22-query run. The report MUST contain:

- Header metadata: test date, scale factor, fixture path,
  sqlrustgo version, branch name, commit hash.
- Setup section: fixture path, generation tool, and the
  per-table row counts.
- Per-query results table: query number, row count, elapsed
  milliseconds, free-form notes column.
- Summary: total rows, total elapsed time, slowest query.
- Limitations section: explicit mention of the in-process surface,
  and pointer to issue #3474 for external-client follow-up.

#### Scenario: Report is written after a successful run

- **GIVEN** the test has executed 22/22 queries
- **WHEN** the test function returns
- **THEN** the report file exists at
  `docs/releases/v3.10.0/perf/SF1_BASELINE_REPORT.md`
- **AND** the file contains 22 rows in the per-query table
- **AND** the limitations section names issue #3474.

#### Scenario: Report metadata identifies the in-process surface

- **WHEN** the report is read
- **THEN** the report header explicitly labels the comparison
  surface as in-process via `MySqlTestClient`
- **AND** the report does NOT claim a cross-engine comparison.

### Requirement: Non-cargo wrapper `scripts/tpch_sf1_baseline.sh`

A shell script at `scripts/tpch_sf1_baseline.sh` MUST be
executable (mode 0755) and MUST drive the same SF=1.0 baseline
flow as a non-cargo developer convenience: validate the fixture,
boot the sqlrustgo ephemeral, run the 22 queries, and write the
report.

#### Scenario: Script is executable and parses arguments

- **WHEN** the operator runs `bash scripts/tpch_sf1_baseline.sh --dry-run`
- **THEN** the script prints the plan (fixture path, binary path,
  per-engine timeouts, report path) without starting the server
- **AND** exits with status 0.

#### Scenario: Script runs the baseline end-to-end

- **GIVEN** the SF=1.0 fixture is present at `/tmp/tpch-sf1/`
- **AND** the `sqlrustgo-mysql-server` binary is built at
  `target/release/sqlrustgo-mysql-server`
- **WHEN** the operator runs `bash scripts/tpch_sf1_baseline.sh`
- **THEN** the script boots the server, loads the fixture if
  needed, runs Q1..Q22, and writes the report
- **AND** exits with status 0 if all 22 queries returned at least
  one row, or status 2 if any query failed.

#### Scenario: Script is in-process, not external mysql CLI

- **WHEN** the script's source is read
- **THEN** it does NOT shell out to the external `mysql` CLI for
  query execution
- **AND** it uses the in-process `MySqlTestClient` flow
  (i.e., either it boots a sqlrustgo-mysql-server and drives it
  through an in-process client wrapper, or it defers query
  execution to the cargo test above).

### Requirement: Per-query timeout accommodates Q9 6-way join

The test harness MUST use a per-query wall-clock budget of at
least 1800 seconds (30 minutes) so that the Q9 6-way join at
SF=1.0 can complete.

#### Scenario: Q9 completes within the 1800s budget

- **GIVEN** the SF=1.0 fixture is loaded
- **WHEN** the test runs Q9
- **THEN** Q9 returns at least one row
- **AND** Q9 completes within 1800 seconds.

### Requirement: External-client follow-up is out of scope

This change MUST NOT attempt to deliver an external mysql client
(mysql-client 8.0.46) path. The external-client follow-up is
tracked in issue #3474.

#### Scenario: No external mysql CLI shell-outs

- **WHEN** the test source and the wrapper script are inspected
- **THEN** neither contains a call to the external `mysql` CLI for
  query execution against sqlrustgo
- **AND** the report does not claim a cross-engine comparison
  result.

## Non-Goals

- External mysql client (mysql-client 8.0.46) — issue #3474.
- MariaDB / SQLite cross-engine comparison — depends on #3474.
- Cell-level value comparison across engines.
- Generating the SF=1.0 fixture as part of this change; the
  fixture is generated by the operator and committed to the test
  data directory.
- Any SQL feature, planner, executor, or storage change. This
  change is purely test surface and reporting.
