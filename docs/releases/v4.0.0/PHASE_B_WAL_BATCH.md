# V400-WAL-BATCH: WAL sync batching reduces fsync overhead

**Date**: 2026-09-16
**Branch**: `develop/v4.0.0`
**Status**: ✅ CLI opt-in shipped, **but A/B smoke shows no significant improvement** at v4.0.0 throughput scale

## Background

After Phase B server-read-perf landed (single-process ceiling ~4000 OPS, 7× via
multi-process POC), the 1h SOAK validation surfaced a new bottleneck: **single-core
fsync overhead**. Every transaction calls `WAL::sync()` → `File::sync_data()` →
`fsync(2)` syscall. On macOS SSD, each fsync takes ~1-10ms. With one executor
thread, this serializes all writes.

The hypothesis: `--wal-sync batch:N` (already supported in
`crates/mysql-server/src/lib.rs::parse_wal_sync_mode`) would reduce fsync calls
100× by buffering N transactions in the WAL and only flushing once.

## A/B comparison (clean runs, identical environment)

Both runs use `scripts/soak/v400_1h_soak.sh 5 <port>` with default settings:
- 10K rows sbtest1
- 16 client threads, 80% reads / 20% writes
- 5-minute duration
- File storage
- `--server-threads 16`

| Metric | `WAL_SYNC=every` (default) | `WAL_SYNC=batch:100` | Δ |
|---|---:|---:|---|
| Queries completed | 8190 | 8016 | **−2.1%** |
| QPS | 27.3 | 26.7 | **−2.2%** |
| Latency p50 | 385.0 ms | 397.1 ms | **+3.1%** |
| Latency p90 | 1263.6 ms | 1226.5 ms | **−2.9%** |
| Latency p99 | 2601.7 ms | 2583.2 ms | **−0.7%** |
| Latency max | 3997.9 ms | 3398.0 ms | **−15.0%** |
| Latency mean | 587.9 ms | 600.2 ms | **+2.1%** |
| Errors | 0 | 0 | — |
| Panics | 0 | 0 | — |
| High-CPU episodes | 0 | 0 | — |

## Findings

### 1. fsync was not the dominant bottleneck at this scale

With ~27 QPS and ~600ms mean latency, the executor spends the majority of
each transaction in **storage engine operations** (file I/O for table reads,
row serialization, MVCC bookkeeping) — not in fsync. fsync is ~1ms per call,
and even at the every-sync baseline the executor logs show:
```
SERVER: eng.execute(sql=...)
SERVER: storage.commit → WAL.append_entry → WAL.sync → fsync
```
The fsync portion is small relative to the read-scan + serialize cost for
sbtest1 row materialization.

### 2. Why my first analysis was wrong

The earlier estimate (batch:100 → p50 −43%) was based on comparing the
*partial* 1h every-sync run at 25min (RSS ≈ 1656 MB, buffer pool hot, WAL
buffer accumulating) vs the *partial* 5min batch:100 run at 5min (RSS ≈ 1000 MB,
buffer pool warming up). The performance delta was dominated by **WAL buffer
warm-up time**, not by fsync frequency.

The clean A/B (both fresh 5min, identical data dir setup) shows batch:100
provides **no measurable improvement** at this workload size. To see fsync
benefit, we'd need:
- A workload with much higher write rate (closer to 100% INSERT/UPDATE)
- Many concurrent writers contending on a single fsync

### 3. The single-core CPU bottleneck is real but not fsync

CPU stays at 95-110% of one core throughout both runs. The bottleneck is
more likely the **storage engine's serialized write path** (single mutex on
the row/page store) than fsync. The multi-process POC from
`PHASE_B_HORIZONTAL_SCALING_POC.md` already showed that scaling to 4 shards
gives 9.78× — that's the proven path to higher throughput, not batched fsync.

## What changed in code

### `scripts/soak/v400_1h_soak.sh`

Added `WAL_SYNC` env var with default `every` (no behaviour change for
existing callers):

```sh
WAL_SYNC="${WAL_SYNC:-every}"

"$SQLRUSTGO_BIN" serve \
  --port "$PORT" \
  --data-dir "$DATA_DIR" \
  --storage file \
  --log-level warn \
  --server-threads "$SERVER_THREADS" \
  --wal-sync "$WAL_SYNC" \
  ...
```

A `case "$WAL_SYNC" in off)` block prints a warning when durability is
disabled (test-only mode).

### No changes to server code

The `--wal-sync` flag was already supported in
`parse_wal_sync_mode` (since v3.12.0). The script change just exposes the
knob through the SOAK harness.

## Usage

Default (preserves durable behaviour, every-tx fsync):
```sh
bash scripts/soak/v400_1h_soak.sh 60
```

Opt-in batch mode (fsync every 100 tx, ~99 tx potential loss on crash):
```sh
WAL_SYNC=batch:100 bash scripts/soak/v400_1h_soak.sh 60
```

Test-only no-fsync mode (max throughput, no durability):
```sh
WAL_SYNC=off bash scripts/soak/v400_5min_soak.sh 5  # NB: do not run for SOAK
```

## Recommendations

1. **Keep `WAL_SYNC=every` as the SOAK default**. The clean A/B shows
   no measurable benefit at v4.0.0 scale, so the durability cost isn't worth
   paying.

2. **If you need higher throughput**, scale horizontally per
   `PHASE_B_HORIZONTAL_SCALING_POC.md` (run N shard processes, route PK
   queries via the new `sqlrustgo-shard-router`). This is the proven
   9.78× path on the same 10-core box.

3. **Consider batch:N only if**:
   - You have a write-heavy workload (>50% UPDATE/INSERT)
   - You can tolerate the durability loss window (up to N tx)
   - Multi-process sharding is infeasible (deployment constraint)

4. **Future work** (not in v4.0.0):
   - **Group commit**: coalesce N concurrent fsync() syscalls from N threads
     into one fsync, similar to InnoDB `innodb_flush_log_at_trx_commit=2`.
     This is the production-grade solution; estimated 5-10× throughput gain
     on multi-thread workloads.
   - **Time-based batch** (e.g. fsync every 1000ms regardless of N): bounds
     the durability loss window to 1s instead of N tx.
   - **`fdatasync()` on Linux**: skips metadata sync, ~30% faster than fsync
     on ext4 when only data integrity is needed.

## Files affected

- **Modified**: `scripts/soak/v400_1h_soak.sh` (add WAL_SYNC env var + pass-through)
- **No code changes** to server

## References

- `PHASE_B_HORIZONTAL_SCALING_POC.md` — multi-process POC that achieves 9.78×
- `PHASE_B_SERVER_READ_PERF_CLOSE.md` — Phase B summary, single-process ceiling
- `crates/mysql-server/src/lib.rs::parse_wal_sync_mode` — `--wal-sync` CLI parser
- `crates/storage/src/parallel_wal_storage.rs::commit_transaction` — sync logic
- `crates/storage/src/wal/file_backed_wal_manager.rs::sync` — actual fsync call
- A/B test results:
  - `results/soak-v400-batch100-5min-clean/STABILITY_REPORT.md`
  - `results/soak-v400-every-5min-clean/STABILITY_REPORT.md`