## Why

Current TPC-H SOAK test (`scripts/soak/tpch_soak_driver.py`) is **pure read-only** — it runs only TPC-H Q1-Q22 queries against static data. This fails to surface critical production issues:

- **WAL unbounded growth** — only writes generate WAL entries; checkpoint truncation behavior is untested
- **MVCC visibility bugs** — read-write conflicts during concurrent transactions are untested
- **Lock contention / deadlocks** — write locks vs read locks never collide
- **ACID guarantees** — only the "R" (read) part is verified; CUD is never tested

According to **TPC-H V3.0 specification**, the benchmark is defined as "a suite of business oriented ad-hoc queries **and concurrent data modifications**". The current SOAK implementation violates this fundamental design intent.

**Goal**: Implement a realistic TPC-H SOAK test that combines CRUD operations (INSERT/UPDATE/DELETE) with Q1-Q22 queries, exercising the database the way real production DSS systems are used.

## What Changes

- **Add** `scripts/soak/tpch_mixed_soak_driver.py` — new driver that mixes queries with CRUD
- **Add** `scripts/soak/crud_templates.py` — parameterized CRUD operations for 5 TPC-H tables
- **Extend** metrics collection:
  - Per-CRD QPS (insert_qps, update_qps, delete_qps)
  - read-write conflict counter
  - lock wait time average
- **Add** data replenishment thread that maintains stable row counts after DELETE
- **Add** dynamic concurrency controller (4-32 workers randomly)
- **Keep** existing `tpch_soak_driver.py` (pure-read) unchanged for comparison
- **Add** new gate check entry in `scripts/gate/check_p13_soak_test.sh` for mixed workload

## Capabilities

### New Capabilities

- `tpch-mixed-soak`: Mixed-workload TPC-H SOAK test combining Q1-Q22 queries with INSERT/UPDATE/DELETE CRUD operations on 5 TPC-H tables, with dynamic concurrency (4-32) and automatic data replenishment

### Modified Capabilities

_None — existing specs unchanged._

## Impact

**Files added:**
- `scripts/soak/tpch_mixed_soak_driver.py`
- `scripts/soak/crud_templates.py`

**Files modified:**
- `scripts/gate/check_p13_soak_test.sh` (add 2 new checks for mixed driver)

**Dependencies:**
- `mysql` CLI client (already required)
- Python 3.8+ stdlib only (no new deps)

**Performance impact on CI:**
- Mixed driver runs in 30-min smoke test (vs 30-min pure-read)
- Optional 4h/24h stages disabled by default

**Test impact:**
- New gate check ensures mixed driver compiles and imports correctly
- 30-min smoke verifies queries + CRUD execute without error and metrics are reported