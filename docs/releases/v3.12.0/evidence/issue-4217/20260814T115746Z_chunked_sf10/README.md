# Issue #4217 — chunked bulk-load evidence

**Run id:** `20260814T115746Z_chunked_sf10`
**Captured at:** 2026-08-14T11:57:50Z
**Server binary:** `target/debug/sqlrustgo-mysql-server` (post PR #4214 baseline `9170661f46`)
**Client:** `mysql Ver 8.0.46-0ubuntu0.24.04.3` (system MySQL CLI)
**Data dir:** `/tmp/tpch-evidence` (synthetic TPC-H fixture, 51,630 rows total)
**DB:** `tpch_sf10`
**Knob under test:** `--bulk-insert-rows-per-flush 10000` (new, replaces legacy hard-coded `PERIODIC_FLUSH_ROWS = 100`)

## What this proves

The new `--bulk-insert-rows-per-flush` knob is wired end-to-end:

1. **CLI surface** — `cargo run --bin sqlrustgo-mysql-server -- serve --help` exposes the flag with `default_value_t = 10_000`.
2. **EphemeralConfig propagation** — `EphemeralConfig::bulk_insert_rows_per_flush` defaults to 10_000 and is read by `handle_load_local_infile` (`crates/mysql-server/src/lib.rs:4189`).
3. **LOAD DATA pipeline** — every LOAD DATA LOCAL INFILE call with synthetic TPC-H fixture data lands **all** rows with `parity=match` (src_lines == loaded_rows for every table).
4. **Engine-side helper** — `ExecutionEngine::bulk_insert_chunked` is exercised by `src/execution_engine_tests.rs::test_executor_bulk_insert_chunked_multi_chunk_v312_26` (50K rows / 10K chunks) and three sibling tests (single-chunk fall-through, zero-chunk disable, uneven remainder).

## Per-table row-count parity

| Table      | src_lines | loaded_rows | elapsed_sec | rows/sec | parity |
|------------|-----------|-------------|-------------|----------|--------|
| region     |         5 |           5 |       0.049 |      102 | match  |
| nation     |        25 |          25 |       0.044 |      568 | match  |
| supplier   |       100 |         100 |       0.053 |     1887 | match  |
| customer   |    25,000 |      25,000 |       1.757 |   14,229 | match  |
| part       |       200 |         200 |       0.081 |     2469 | match  |
| partsupp   |       800 |         800 |       0.075 |   10,667 | match  |
| orders     |    25,000 |      25,000 |       2.021 |   12,370 | match  |
| lineitem   |       500 |         500 |       0.113 |     4425 | match  |
| **total**  |**51,630** |   **51,630** |   **4.193** |          |        |

`tables_loaded_ok = 8`, `tables_failed = 0`, `tables_parity_mismatch = 0`.

## How to reproduce

```bash
# Fixture data already lives at /tmp/tpch-evidence
DATA_DIR=/tmp/tpch-evidence bash scripts/tpch/bulk_load_chunked_sf10.sh
```

## Wire-protocol observability

`server.log` contains the full INFO-level trace for every LOAD DATA query:

- Connection accept / TLS upgrade / handshake for each `mysql` CLI invocation.
- `Query [ip:port]: LOAD DATA LOCAL INFILE '<path>' INTO TABLE <tbl> ...` (the exact SQL sent).
- For customer / orders (25K rows): no `parse line error` warnings — every line accepted.
- Final `SELECT COUNT(*)` for each table issued from the script, returns `send_result_set: 1 cols, 1 rows`.

## Files in this evidence directory

| File                       | Purpose                                                  |
|----------------------------|----------------------------------------------------------|
| `README.md`                | This file                                                |
| `server.log`               | Full server stderr/stdout (INFO+ level)                 |
| `bulk_load_summary.json`   | Aggregated per-table parity / throughput                  |
| `bulk_load_summary.jsonl`  | One JSON object per table, in load order                 |
| `bulk_load_log.txt`        | Human-readable log of the script run                     |
| `metadata.json`            | Server version, mysql version, config knobs              |
| `<tbl>_load.log`           | mysql CLI stdout/stderr for the LOAD DATA per table      |
| `<tbl>_create.log`         | mysql CLI stdout/stderr for the CREATE TABLE per table   |
| `data/`                    | sqlrustgo file-storage backend directory                 |
| `server.pid`               | PID of the server process during the run                  |

## Baseline vs. this run

`scripts/tpch/bulk_load_sf10.sh` (Issue #4020 baseline) is the locked-baseline runner
that still uses the historical `PERIODIC_FLUSH_ROWS = 100` default. **Do not edit**
that script — the new runner `bulk_load_chunked_sf10.sh` is the post-#4217 replacement.

## Earlier failed run (20260814T115628Z_chunked_sf10)

The first run of this script loaded all 8 tables but reported
`parity=mismatch` for the 6 small tables (region / nation / supplier / part /
partsupp / lineitem). Investigation showed the `*.tbl` files in `/tmp/tpch-evidence`
were **3-line Git-LFS pointer stubs** (`version https://git-lfs.github.com/spec/v1`,
`oid sha256:...`, `size N`), not real TPC-H data — the server correctly rejected
every line with `WARN parse line error: line has 1 fields, expected at least N`.

This run (20260814T115746Z_chunked_sf10) regenerated all 6 small-table fixtures
with proper TPC-H-shaped content (canonical region.tbl / nation.tbl per the TPC-H
spec; synthetic but schema-conforming data for the others) and re-ran. All 8
tables now reach `parity=match`.