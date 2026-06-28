## ADDED Requirements

### Requirement: Mixed workload driver runs TPC-H queries and CRUD concurrently

The system SHALL provide a SOAK driver that executes TPC-H Q1-Q22 queries intermixed with INSERT/UPDATE/DELETE CRUD operations on 5 TPC-H tables (orders, lineitem, customer, partsupp, part) within a single wall-clock run.

#### Scenario: Driver accepts and parses mixed workload flags
- **WHEN** user invokes `python3 scripts/soak/tpch_mixed_soak_driver.py --duration=1800 --mix-ratio=80 --concurrency-min=4 --concurrency-max=32`
- **THEN** the driver parses all flags without error and starts server connection probe

#### Scenario: Driver performs CRUD on all 5 tables
- **WHEN** driver runs against TPC-H SF=0.1 dataset
- **THEN** the SoakReport.json `crud.insert_count`, `crud.update_count`, `crud.delete_count` are all > 0

### Requirement: Read/write ratio is enforced at 80:20

The driver SHALL execute approximately 80% queries and 20% CRUD operations over the run duration, with each worker choosing query or CRUD independently per iteration.

#### Scenario: Ratio within tolerance over 30-minute run
- **WHEN** driver runs for 1800 seconds against TPC-H SF=0.1
- **THEN** `crud_ratio_actual` is within [18%, 22%] of total operations

### Requirement: Dynamic concurrency between 4 and 32 workers

The driver SHALL start with a random concurrency in [4, 32] and adjust target concurrency every 30-120 seconds to a new random value in the same range, spawning or retiring workers to match.

#### Scenario: Concurrency adapts over time
- **WHEN** driver runs for 1800 seconds
- **THEN** at least 3 distinct concurrency values are observed during the run

#### Scenario: Workers join cleanly on retirement
- **WHEN** controller reduces concurrency from 16 to 8
- **THEN** no orphaned threads remain (`threading.enumerate()` count returns to baseline)

### Requirement: Data replenishment prevents depletion

A dedicated monitor thread SHALL check row counts every 60 seconds and bulk-INSERT new rows when any of the 5 tables drops below its configured minimum threshold.

#### Scenario: Replenishment restores row count after DELETE pressure
- **WHEN** 1000 rows are deleted from `customer` within 60 seconds
- **THEN** within 90 seconds the `customer` row count returns above `MIN_ROWS["customer"]` (default 1000)

### Requirement: Metrics include CRUD QPS and lock wait time

The driver SHALL report `crud.insert_qps`, `crud.update_qps`, `crud.delete_qps`, `lock_wait_time_avg_ms`, and `read_write_conflicts` in the SoakReport.json output.

#### Scenario: SoakReport contains all required fields
- **WHEN** driver completes a run
- **THEN** SoakReport.json parses successfully and contains the keys: `crud.insert_qps`, `crud.update_qps`, `crud.delete_qps`, `lock_wait_time_avg_ms`, `read_write_conflicts`

### Requirement: Gate check verifies mixed driver importability

The P13 SOAK gate check (`scripts/gate/check_p13_soak_test.sh`) SHALL verify that `scripts/soak/tpch_mixed_soak_driver.py` and `scripts/soak/crud_templates.py` exist and pass Python syntax validation (`python3 -m py_compile`).

#### Scenario: Gate check passes with new files
- **WHEN** `bash scripts/gate/check_p13_soak_test.sh` is invoked
- **THEN** check 10 (mixed driver exists) and check 11 (templates module compiles) both output `✅ PASS`