# Proposal: tpch-sf1-cross-engine-baseline-via-in-process

## Why

Issue #3423 asks for a TPC-H SF=1.0 cross-engine baseline that
compares sqlrustgo against MariaDB (and ideally SQLite) on a
realistic dataset. The canonical expectation is that the baseline be
runnable end-to-end against an external mysql client (per the
acceptance criteria in the issue body, the 22 queries run via
`cargo test --test tpch_sf01_22_vs_3engines_test --all-features`).

A prerequisite to that baseline is a working external-client
connection to sqlrustgo, and that prerequisite is not currently
satisfied: as of 2026-06-17, mysql-client 8.0.46 (libmysqlclient
8.0.46) either silently drops the result set (the original
symptom), returns `ER_MALFORMED_PACKET` (2027) on a partial
result set, or hangs on the next packet after the OK terminator
(after the partial fix on branch `fix/wire-deprecate-eof-partial`).
The follow-up is tracked in issue #3474 and is not on the
critical path for v3.9.0-rc4.

This change delivers the cross-engine baseline through the
already-working in-process test surface (`MySqlTestClient` via
`start_ephemeral`), which does not depend on a working external
mysql client. The in-process path uses `FileStorage` +
`WalStorage` and is the path that v3.8.0 / v3.9.0
`tpch_sf01_22_vs_3engines_test` already exercises on the SF=0.01
fixture today. We extend it to the SF=1.0 fixture and capture the
row-count and time data in a single Markdown report.

The cross-engine comparison is still real: the SF=1.0 data is the
canonical TPC-H dbgen output at SF=1 (6M lineitem rows, 1.1 GB
on disk), the same data that a real MySQL 8.0 server would load.
The 22 TPC-H queries are the canonical spec. The
`tpch_wire_harness` round-trip counts the rows and compares the
result-set shape against a ground-truth MySQL instance when one
is available on `localhost:3306`; when one is not, the test
records row counts and timing as the baseline.

## What Changes

- Add `tests/tpch_sf1_22_vs_3engines_test.rs` — a 22-query
  TPC-H cross-engine test for the SF=1.0 fixture. Uses
  `MySqlTestClient` (in-process ephemeral server) for sqlrustgo
  and `mysql` / `sqlite3` CLIs for the other two engines, the same
  pattern as the existing SF=0.01 test.
- Add `scripts/tpch_sf1_baseline.sh` — a non-cargo wrapper that
  loads the SF=1.0 fixture into a sqlrustgo ephemeral, runs the 22
  queries, and writes
  `docs/releases/v3.10.0/perf/SF1_BASELINE_REPORT.md` with row
  counts, timings, and the engine comparison.
- Add `docs/releases/v3.10.0/perf/SF1_BASELINE_REPORT.md` (the
  baseline report itself, written by the script).
- Wire the new test into the existing `tpch_wire_harness` so that
  the SF=1.0 fixture and its expected row counts are validated
  once at startup. The expected counts come from
  `scripts/generate_tpch_data.sh --sf 1 --check` and match TPC-H
  spec.

## Capabilities

### New Capabilities

- `tpch-sf1-cross-engine-baseline-via-in-process`: a 22-query
  TPC-H cross-engine baseline for the SF=1.0 fixture, executed
  in-process against the existing wire-protocol test surface.
  Captures row counts and per-query wall-clock timings for
  sqlrustgo, MariaDB (when available on `localhost:3306`), and
  SQLite (`sqlite3` CLI on `/tmp/tpch-sf1/sqlite.db`). Writes
  the result to
  `docs/releases/v3.10.0/perf/SF1_BASELINE_REPORT.md`.

### Modified Capabilities

(none — this change adds one new capability and does not touch
any existing spec).

## Non-Goals

- This change does **not** deliver a working external-client
  (mysql-client 8.0.46) path. That is the subject of issue #3474
  and is out of scope here.
- This change does **not** implement the v3.10.0 TPC-H SF=1.0
  cross-engine baseline through `cargo test --all-features` if
  that requires a working external mysql client. The in-process
  path is sufficient and is what is being delivered.
- This change does **not** alter the SF=0.01 fixture, the
  SF=0.001 fixture, or any of the existing wire-protocol tests.
- This change does **not** introduce a new SQL feature, planner
  change, executor change, or storage change. It is purely test
  surface and reporting.

## Acceptance

1. `bash scripts/tpch_sf1_baseline.sh` runs end-to-end without
   errors and produces
   `docs/releases/v3.10.0/perf/SF1_BASELINE_REPORT.md`.
2. The report contains 22 entries, one per TPC-H query
   (Q1..Q22), with row counts and per-engine wall-clock
   timings.
3. The report explicitly labels the comparison surface as
   in-process via `MySqlTestClient` and notes that the
   external-client follow-up is tracked in issue #3474.
4. `cargo test --test tpch_sf1_22_vs_3engines_test` passes on the
   in-process surface.
5. The SF=1.0 fixture is generated via the official
   `tpch-dbgen` (already installed at
   `/home/openclaw/tpch-dbgen-master/dbgen`) or via the project's
   `scripts/generate_tpch_data.sh --sf 1 --backend dbgen`, never
   the built-in `tpch_data_gen` example (the latter has a
   documented 100x-scale bug per
   `crates/bench/examples/tpch_data_gen.rs`).
