# v4.1.0 server-read-perf SOAK — 1h

**Date**: 2026-10-08T10:43:15Z–11:43:30Z
**Duration**: 3600s / 60min — completed
**Source commit**: `b1e9a98ee6bac1e863ab8826c54a41500455b71d` (develop/v4.1.0)
**Port**: 3423  **Threads**: 16 driver + 16 server  **Storage**: file
**Data dir**: `/private/tmp/sqlrustgo-v410-soak-1h` (clean)
**Rows**: 10,000 preloaded  **WAL sync**: `every` (durable)
**Workload**: 80% reads / 20% writes (10% INSERT), same driver + harness as the 5min run
**Harness**: `scripts/soak/v400_1h_soak.sh` 60min mode

## Verdict

| Metric | Value |
|--------|-------|
| Server alive at end | 1 |
| Trigger panics | 00 |
| Driver queries done | **582,585** |
| Driver errors | 00 |
| High-CPU episodes | 0 |
| Sustained throughput | ~162 QPS |
| FD count | constant 28 (12 after shutdown) — no leak |
| **Row-count integrity after restart** | **PASS — 68,288 = 10,000 + 58,288 INSERTs** |

## Latency (driver log)

| | min | p50 | p90 | p95 | p99 | max | mean |
|---|---:|---:|---:|---:|---:|---:|---:|
| ms | 1.0 | 79.9 | 204.3 | 251.3 | 370.1 | 2933.4 | 98.9 |

Query mix: point_select 290,606 / range_scan 116,900 / count_agg 58,831 /
insert 58,288 / point_update 57,960.

## RSS — no convergence over 60 minutes

| window (min) | mean MB | min | max |
|---|---:|---:|---:|
| 0–10 | 276.7 | 106.0 | 365.0 |
| 10–20 | 348.9 | 277.3 | 403.2 |
| 20–30 | 375.4 | 332.9 | 446.7 |
| 30–40 | 459.8 | 336.2 | 574.8 |
| 40–50 | 560.6 | 482.8 | 655.4 |
| 50–60 | 583.0 | 495.8 | 662.8 |

**RSS rises monotonically across the whole hour with no GC plateau.** Decadal means:
277 → 349 → 375 → 460 → 561 → 583 MB. The last 20 minutes still add ~2 MB/min. Peak
662.8 MB.

Contrast the 2026-09-19 v4.0.0 5min run, which had already entered a GC sawtooth
(311–636 MB oscillation) by t=151s and held it.

### Consequence for the 168h gate

Linear extrapolation from the last two decadal means (+2.0 MB/min) reaches the macOS
~1.7 GB per-process jetsam ceiling at roughly **t ≈ 940 min ≈ 15.6h**. That is a
projection, not a measurement — MVCC GC may engage later than 60min and reclaim the
accumulation, exactly as it did within 151s on the 2026-09-19 build. But on the
evidence available, **v4.1.0 at HEAD has not demonstrated a stable memory equilibrium**,
and that is the prerequisite the 168h SOAK is meant to establish.

Compare: v4.0.0's 1h and 8h runs were jetsam-killed at 42min and 52min. This run
survived 60min at 663 MB peak — better, but the underlying curve has the same shape.

## Data integrity (post-restart)

Server stopped, restarted against the same data dir:

```
SELECT COUNT(*) FROM sbtest1;                -> 68288   (10,000 + 58,288 INSERTs)
SELECT COUNT(*) FROM sbtest1 WHERE id>10000; -> 58288
```

Exact match. Every acknowledged INSERT survived.

## Defect found: cold full-table scan after restart

The verification query behaved very differently cold vs warm:

| | elapsed |
|---|---:|
| first `SELECT COUNT(*) FROM sbtest1` on a freshly restarted server | **24 min 22 s** |
| identical query immediately after | **0.08 s** |

Reproduced on a second clean restart: the first full-table scan is again pathologically
slow. Data dir state: `sbtest1.json` 13.6 MB, `sbtest1_idx_id.json` 1.3 MB,
`sqlrustgo.wal` 8.0 MB.

### Root cause (confirmed by stack sample + filesystem observation)

Recovery is not lazy-on-first-access in the benign sense — it is **quadratic in the
number of replayed WAL entries**, and it blocks startup.

Stack sample of the wedged server (`sample`, pid 74061), taken 28 min into recovery:

```
run_server_with_listener_and_shutdown
  StatefulRecoveryEngine<FileStorage>::recover
    FileStorage::insert_direct            <- crates/storage/src/file_storage.rs:4437
      FileStorage::save_table_full        <- crates/storage/src/file_storage.rs:1589
        serde_json::to_writer_pretty / write_all_cold
```

Every `WalEntryType::Insert` replayed by `recovery_force_insert`
(`crates/storage/src/recovery_engine.rs:199`, called from `apply_wal_entry` at :753)
routes into `FileStorage::insert_direct`, which calls `save_table_window`
(`file_storage.rs:1533`). On the recovery path `last_saved_row_count` is 0, so
`save_table_window` takes its cold-start branch:

```rust
if total_rows == 0 || last_saved == 0 || total_rows <= last_saved {
    if let Some(data) = st.tables.get(&scoped) {
        return self.save_table_full(db, table_name, data);   // full rewrite
    }
}
```

`save_table_full` serialises the **entire table** with `to_writer_pretty` and then
removes any pending delta. The delta-append fast path is never reached during recovery,
because `last_saved == 0` holds until the first full save completes — and each
full save resets the same state.

Filesystem observation over 60 s while recovery was still running:

```
20:47:24  sbtest1.json   1,048,576 bytes
20:47:34  sbtest1.json           0 bytes
20:47:44  sbtest1.json  13,602,635 bytes
20:47:54  sbtest1.json   7,340,026 bytes
20:48:04  sbtest1.json           0 bytes
20:48:14  sbtest1.json  13,602,594 bytes
(no .delta file at any sample)
```

The 13.6 MB snapshot is rewritten roughly **every 10 seconds, hundreds of times**,
never converging to a delta append. With 58,288 INSERT entries against a table growing
to 68,288 rows, that is O(entries x table_size) JSON serialisation — hence the 24 min.

The cost grows with soak length, so it gets worse in exactly the scenario the 168h
SOAK is meant to run.

Impact: any restart-then-query path (crash recovery, failover, backup restore) pays a
startup cost proportional to (rows replayed x table size). The server is not reachable
until it completes — recovery runs before the listener accepts, so this is a hard
availability outage, not a slow query.

Note this also explains a harness blind spot: `v400_1h_soak.sh` never restarts the
server mid-run, so no recorded SOAK — v4.0.0 or v4.1.0 — exercises this path. The
1h run only surfaced it because verification restarts the server to check row counts.

Two related observations from the same code path:
- `save_table_full` uses `to_writer_pretty`; a compact writer would cut the file size
  roughly 2-3x for this shape of data.
- The recovery replay path could batch entries per table and take the delta-append
  branch, or mark `last_saved_row_count` before replay so the cold-start branch is
  entered once rather than per entry.

## Comparison

| Run | Code | Queries | p50 | p99 | max | RSS peak | Duration reached |
|---|---|---:|---:|---:|---:|---:|---|
| **v4.1.0 1h** | `b1e9a98ee6` | **582,585** | 79.9 | 370.1 | 2,933.4 | **662.8 MB** | **60 / 60 min** |
| v4.1.0 5min | `b1e9a98ee6` | 129,017 | 29.6 | 170.7 | 505.6 | 281.9 MB | 5 / 5 min |
| v4.0.0 1h | `39fb79164a` | 107,880 | n/a | n/a | n/a | 1,680 MB | 42 / 60 min — **jetsam killed** |
| v4.0.0 8h | v4.0.0 | 135,916 | n/a | n/a | n/a | 1,500 MB | 52 / 480 min — **jetsam killed** |

Latency degrades with run length (p50 29.6 → 79.9 ms) as the table grows from ~23k to
~68k rows — expected for full-scan aggregates, but it means the 5min and 1h numbers are
not comparable as a single trend line.

## What this run does NOT prove

- Still not 168h. The memory equilibrium question is open.
- BLK-3 (AUTO_INCREMENT id reuse) uncovered — driver assigns explicit disjoint ids.
- `WAL_SYNC=batch:N` not exercised; durable `every` path only.
- Single 10-core host, file storage, no replication (#4937 semi-sync never under load).
- `check_anti_fabrication.sh` full run not completed within 30 min on this host (its
  sqlancer + test-runner build phase); CHECK 4 (the check this PR set touches) was
  verified in isolation and passes.

## Files

- `metrics.csv` — 15s samplings (235 rows)
- `summary.json` — aggregate stats
- `driver.log.gz` — per-query CSV (582,585 rows)
- `server.log` — `--log-level warn`, 0 ERROR/WARN/panic
- `STABILITY_REPORT.md` — this file
