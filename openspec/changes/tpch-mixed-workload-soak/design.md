## Context

`scripts/soak/tpch_soak_driver.py` (existing) implements a pure read-only SOAK:
- 22 TPC-H query files in `scripts/soak/tpch_queries/q01.sql`..`q22.sql`
- Worker pool runs queries in batch via `mysql` CLI subprocess
- Metrics: RSS, FD, query latency, queries per second
- No CRUD, no concurrent writers, no MVCC validation

This violates the TPC-H spec's explicit "queries and concurrent data modifications" requirement and misses critical storage layer bugs that only appear under mixed workload.

## Goals / Non-Goals

**Goals:**
- Run TPC-H Q1-Q22 **mixed** with INSERT/UPDATE/DELETE on 5 TPC-H tables
- Maintain 80:20 read:write ratio (close to TPC-H Refresh Function spec ratio)
- Exercise WAL, MVCC, locking layers in addition to query engine
- Auto-replenish rows after DELETE to keep dataset stable across long runs
- Dynamic concurrency 4-32 (random walk) to simulate real load fluctuations
- Report per-CRD QPS + lock wait + read-write conflict counters
- Stay single-binary dependency-free (Python 3.8 stdlib + `mysql` CLI)

**Non-Goals:**
- Replacing the pure-read driver (kept for comparison)
- Implementing TPC-C mixed workload (different scope)
- Multi-server distributed testing (single-server only)
- Replace sysbench; this is a TPC-H-specific mixed driver

## Decisions

### 1. Single new file vs. refactor `tpch_soak_driver.py`

**Decision**: Create a new file `scripts/soak/tpch_mixed_soak_driver.py` alongside the existing driver.

**Rationale**:
- Pure-read driver is gate-checked (`check_p13_soak_test.sh`) and used by other workflows
- Mixed workload has fundamentally different metrics and threading model
- Clean separation makes rollback trivial

### 2. CRUD templates in separate module

**Decision**: Put parameterized CRUD SQL templates in `scripts/soak/crud_templates.py`.

**Rationale**:
- Separates SQL templates from concurrency / threading logic
- Easier to test templates independently
- Reusable for future mixed workloads

### 3. ID pool strategy: pre-allocated ranges per worker

**Decision**: Each worker reserves a 1000-ID block per table. When exhausted, refetch `MAX(id)` from DB.

**Rationale**:
- TPC-H tables have INTEGER primary keys, no AUTO_INCREMENT
- Pre-allocated blocks avoid lock contention on every INSERT
- Block refresh from DB ensures no conflict with existing data
- Per-worker blocks prevent overlap between concurrent workers

### 4. Replenishment on a separate monitoring thread

**Decision**: Background thread checks row counts every 60s; if below threshold, bulk INSERT to restore.

**Rationale**:
- DELETE-only would deplete data within hours
- Monitoring thread keeps main worker threads focused on workload
- Threshold-based replenishment prevents thrashing

### 5. Dynamic concurrency via thread spawn/retire

**Decision**: Controller thread adjusts worker count every 30-120s to a random target in [4, 32].

**Rationale**:
- Production traffic is never constant; static concurrency hides issues
- Worker spawn is cheap in Python (`threading.Thread`)
- Retire stops cleanly via `threading.Event` (workers check stop flag per iteration)

### 6. MySQL CLI subprocess per query

**Decision**: Same as pure-read driver — call `mysql` CLI via `subprocess.run` per query.

**Rationale**:
- Reuses existing `mysql` CLI client behavior
- Avoids writing wire-protocol code in Python
- Consistent with sibling drivers (`long_running_soak.py`, etc.)

### 7. Metrics: sample at 30s interval (matching pure-read)

**Decision**: Same `/proc/$PID/status` sampler; add per-CRD QPS counters updated every 5s.

**Rationale**:
- Compatible with existing `metrics.csv` format (append new columns)
- 5s flush window for QPS counters balances precision vs overhead
- Lock wait time tracked via per-query `time.perf_counter()` diff against lock acquisition

## Risks / Trade-offs

| Risk | Mitigation |
|------|-----------|
| INSERT might fail with FK violation (e.g., lineitem needs valid partkey) | ID pool blocks include only valid existing IDs (validated via SELECT before INSERT) |
| DELETE might cascade and orphan other rows | DELETE templates use `WHERE NOT EXISTS (SELECT ...)` patterns |
| Replenishment bulk-INSERT may spike WAL | Limit batch size to 100 rows per replenishment call |
| Worker churn (frequent spawn/retire) might leak threads | Each retired worker joins with timeout; controller logs thread count |
| 80:20 ratio skews under high write latency | Worker decision uses pre-iteration token bucket (not ratio counter) |
| Mixed driver competes for port 3396 with pure-read | Driver accepts `--port` flag; CI uses 3397 for mixed |

## Migration Plan

1. Land `crud_templates.py` and `tpch_mixed_soak_driver.py` as new files
2. Extend `scripts/gate/check_p13_soak_test.sh` with 2 new checks:
   - mixed driver compiles (Python import + argparse)
   - CRUD templates module imports without error
3. No existing functionality removed or changed
4. Smoke run: 5-min mixed SOAK against SF=0.1 must complete with `errors == 0`
5. Rollback: delete the 2 new files; revert gate check extension

## Open Questions

None — all decisions resolved during brainstorming.