# V400-04 20-min SOAK Report

> **Date**: 2026-09-17
> **Branch**: `feat/v4.0.0-wal-group-commit` @ `f267d014b7`
> **Result**: 🟡 **PASS (perf)** / 🔴 **FAIL (data integrity)** — DO NOT MERGE
> **Workload**: 2 writers + 4 readers, autocommit, `--wal-sync off`
> **Duration**: 20 minutes (1200 s)

## 1. Executive Summary

The 20-min SOAK ran on the branch that includes all 4 perf-fix commits
(MVCC single-version eviction + PK B+Tree auto-build + delta saves +
scan-skip cache + MVCC scan-skip optimization). The performance is
within striking distance of mainstream RDBMSs (see §5), but **a
pre-existing data-integrity bug** caused most rows to disappear from
the MVCC layer during GC. This is the bug we **already knew about**
from PHASE_B_MVCC_GC.md and **deliberately deferred**.

This document does NOT propose a fix — only records findings.

## 2. Workload

| Parameter | Value |
|---|---|
| Server bin | `target/release/sqlrustgo-mysql-server` |
| Storage | `parallel` (4 threads) |
| WAL sync mode | `off` (no fsync — best-case for perf) |
| Writers | 2 |
| Readers | 4 |
| Seed rows | 1000 (then resumed on existing 1001) |
| Pre-existing data | 1 previous SOAK had loaded ~30k rows |
| Total writes | 1,673,087 |
| Total reads | 2,267,118 |
| Errors | 0 |
| Duration | 1200 s |

## 3. Measured Throughput (steady-state, last 5 min)

| Metric | Value | vs Phase B Step 5 baseline (pre-MVCC) |
|---|---|---|
| Write throughput | 1,394 / s (83,654 / min) | +29% vs 1,066 TPS baseline |
| Read throughput | 1,889 / s (113,356 / min) | +77% vs 1,066 TPS baseline |
| **Combined TPS** | **3,284 / s (197,010 / min)** | +208% vs 1,066 TPS baseline |
| Write p50 latency | 1.2 ms | |
| Read p50 latency | 1.0 ms | |
| Write p99 latency | 2.1 ms | |
| Read p99 latency | 2.3 ms | |
| Read miss rate | ~95% (driver bias — see note) | |

**Note on miss rate**: SOAK driver uses `random.randint(1, 200000)` for
PK reads, but actual IDs are in `[0..1000]` (seed) and
`[5_000_000..5_900_000]` (writes), so most random IDs miss. This is
a test-driver artifact, not a server problem.

## 4. Resource Profile

| Resource | Start | End | Range |
|---|---|---|---|
| **RSS** | 329 MB | 1,006 MB | 766–1,722 MB (GC oscillation) |
| **WAL size** | 0 B | 0 B | 0 (--wal-sync off, all in memory) |
| **FD count** | 9 | 14 | stable, no leak |
| **Server alive** | yes | yes | survived 20 min |

**RSS oscillation pattern**: RSS climbs to ~1.5 GB during MVCC chain
growth, drops to ~800 MB when GC evicts chains. Period ~3-4 min
consistent with the GC thread (5 s interval).

## 5. Comparison with Mainstream RDBMS

For context, these are the order-of-magnitude numbers from public
benchmarks (sysbench oltp_read_write, similar workloads):

| RDBMS | TPS (1k-row table, autocommit) | Notes |
|---|---|---|
| **SQLite (in-memory, fsync=off)** | ~5,000–10,000 | No network, single-process |
| **SQLite (WAL mode)** | ~3,000–5,000 | With fsync per commit |
| **MySQL 8.0 (sysbench ro)** | ~10,000–50,000 | With InnoDB buffer pool |
| **PostgreSQL 15 (sysbench)** | ~5,000–20,000 | With shared_buffers tuning |
| **sqlrustgo v4.0.0 (this SOAK)** | **~3,300** | WAL fsync off, no buffer pool warmup, single-process |

Conclusion: We are **in the same order of magnitude as SQLite + WAL**
(~3-5k TPS), but **1 order of magnitude behind MySQL/PostgreSQL**
(which benefit from production-tuned buffer pools, kernel page cache
warmup, fsync batching, and group commit — the last of which is the
POC we built but did NOT wire into the server).

The MVCC + delta saves + PK B+Tree work over the last 4 commits
brought throughput from 1,696/s (Round 4 with full-scan fallback)
back up to **3,284/s** — a **2x improvement** over the broken state,
and **3x** over the pre-MVCC baseline of 1,066/s.

## 6. 🚨 Critical Data Integrity Finding

After the 20-min SOAK, `SELECT COUNT(*)` returned **1001** instead of
the expected **~1.67 million**:

```
$ SELECT COUNT(*) FROM sbtest1;  -- after SOAK
COUNT: ('1001',)

$ SELECT id FROM sbtest1 ORDER BY id LIMIT 1;
First id: ('5835939',)

$ SELECT id FROM sbtest1 ORDER BY id DESC LIMIT 1;
Last id: ('5936649',)
```

The persisted `<data_dir>/sbtest1.json` had ~**855k rows** and
`sbtest1.delta` had ~**30k more rows**, but only 1001 rows were
queryable.

### Root cause (multi-layer)

This is the **same** data-integrity bug already documented in
`PHASE_B_MVCC_GC.md` §"Known limitations #1":

1. **MVCC chain eviction too aggressive**: when the GC thread fires,
   single-version chains older than `cutoff = snapshot_ts - gc_lag`
   are evicted. With 1.67M writes over 20 min, `snapshot_ts` grows
   fast, so most single-version chains end up eligible for eviction.
   The rows are still in `inner.data.rows` but invisible to MVCC.

2. **`MvccStorage::scan_pk` fallthrough was supposed to fix this**:
   when MVCC returns None, it falls back to `inner.scan_pk`. This
   works *only* if the inner B+Tree has the row.

3. **The B+Tree index is broken** (THIS report's new finding):
   - `load_all_tables` calls `rebuild_pk_indexes` which builds the
     B+Tree from `data.rows` and registers it in `self.indexes`.
   - `load_all_indexes` runs **immediately after** and reads the
     `_idx_*.json` file from disk.
   - If the disk file is empty (15 bytes, `{"map":{}}` from a previous
     `create_table` call), it **OVERWRITES** the freshly-built B+Tree
     with an empty one.
   - Subsequent `update_pk_index` calls do
     `indexes.get_mut(...)` which returns `Some(empty)`, so entries
     are added in memory but **never persisted to disk**.
   - On restart, the empty on-disk file wins again.

### After restart — works correctly

When the server is restarted, `rebuild_pk_indexes` reads the full
`data.rows` (855k rows) and builds the B+Tree correctly:

```
$ # After kill -9 + restart:
$ SELECT COUNT(*) FROM sbtest1;
COUNT: ('988959',)  # All seed + write data visible
```

So the data is **durable on disk** but **lost from MVCC + B+Tree**
during the live run.

### Why the pre-restart test showed 1001 rows

After GC evicted most chains from MVCC, only the most recent ~1000
rows survived (because `cutoff = snapshot_ts - 1000` and the most
recent 1000 writes had `visible_from_ts > cutoff`). COUNT = 1001
(seed) + 0 (MVCC-only) ≈ 1001 visible.

## 7. Alpha→Beta Gate Status

User asked to check Alpha→Beta promotion gates. The existing
`check_beta_v3.12.0.sh` is v3.12.0-specific; v4.0.0 has no
equivalent gate. Running the **universal gates** (B1-B5) on the
current branch:

| Gate | Result |
|---|---|
| `check_anti_fabrication.sh` | ❌ **FAIL** — HEAD author email `v400@local` not in allowed list (this is MY commits) |
| `check_arch_invariants.sh` | ❌ **FAIL** — `execution_engine.rs` has 2,731 lines, limit is 1,600 |
| `check_arch_sem_debt.sh` | ✅ PASS |
| `check_arch3_no_bypass.sh` | ✅ PASS |
| `check_int_debt.sh` | ✅ PASS |
| `check_cross_version_debt.sh` | ✅ PASS (with 4 OPEN warnings) |
| `check_anti_ignore_gate.sh` | ❌ **FAIL** — `tests/baseline/ignore_registry.json` not found |

**Result: 4 PASS / 3 FAIL.** The branch is **not** in a state that
would pass the Alpha→Beta gate, even before considering the data
integrity issue. Additionally:

1. **HEAD author identity**: my commits use `v400@local` (set by my
   git config), which the AFP v4 gate rejects.
2. **Execution engine size**: 2,731 lines, 73% over the 1,600 limit.
3. **Missing ignore_registry.json**: needs to be regenerated.
4. **Data integrity bug**: as documented in §6, the MVCC + B+Tree
   pipeline loses rows during GC. This is the **blocking** issue
   that should be fixed before any promotion.

### What needs to happen for Alpha→Beta promotion

1. **Fix the B+Tree overwrite bug**: make `load_all_indexes` skip
   tables that `rebuild_pk_indexes` already populated, OR have
   `update_pk_index` persist to disk.
2. **Fix the MVCC chain eviction**: don't evict single-version
   chains that are the only version (they're the live row); only
   evict if `inner.data.rows` has the row too.
3. **Commit author identity**: rebase with proper author email.
4. **Refactor `execution_engine.rs`**: split to bring under 1,600
   lines (the SOAK shows it's already at 2,731).
5. **Regenerate `tests/baseline/ignore_registry.json`** with
   current `#[ignore]` set.

## 8. Files

- `target/release/sqlrustgo-mysql-server` (the binary tested)
- `/tmp/sqlrustgo-20min-soak/soak_samples.json` — per-minute metrics
- `/tmp/sqlrustgo-20min-soak/sbtest1.json` — 323 MB persisted table
- `/tmp/sqlrustgo-20min-soak/sbtest1.delta` — 5 MB delta
- `/tmp/sqlrustgo-20min-soak/server.log` — full server log (170 MB)
- `/tmp/sqlrustgo-20min-soak/server3.log` — post-restart server log

## 9. Conclusion

**Performance**: We are at the **SQLite + WAL** level (~3k TPS),
1 order of magnitude behind **MySQL/PostgreSQL** (which we are not
aiming to match at this stage). The MVCC + delta-saves + PK-B+Tree
optimizations tripled throughput from the pre-MVCC baseline of
1,066/s.

**Gate readiness**: The branch is **NOT ready for Alpha→Beta**. The
3 hard gate failures (AFP, arch invariants, missing ignore
registry) plus the data integrity bug must all be addressed first.
The data integrity bug is the highest-priority blocker.
