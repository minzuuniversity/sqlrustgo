# v4.0.0 168h Multi-Model SOAK — Kickoff Report

> **Date**: 2026-09-19T22:11:53Z
> **Worktree**: `/Users/liying/dev/sqlrustgo`
> **Branch**: `develop/v4.0.0` HEAD = `3fa3bb811c`
> **Source commit**: `3fa3bb811c` (post-V400-05/06/07 + WP + 5min SOAK baseline)
> **Duration target**: 168 hours (10080 minutes)
> **Reference**: docs/releases/v4.0.0/{RC_GATE_REPORT,GA_GATE_REPORT,SOAK_BASELINE_5MIN_2026-09-19}.md

---

## 1. Configuration

| Field | Value |
|------|-------|
| Workload | mixed read/write (80% reads / 20% writes, 10% INSERT) |
| Dataset | sbtest1 with 10000 rows |
| Driver threads | 16 |
| Server threads | 16 |
| WAL sync | `batch:100` (vs default `every`; ~43% p50 latency cut per PHASE_B_WAL_BATCH.md) |
| Storage | file (`/tmp/sqlrustgo-v400-soak`) |
| Target duration | 168h = 604800s = 10080min |
| Hardware | 10-core macOS arm64, ~12 GB free disk |
| RSS budget | < 1.7 GB (macOS per-process jetsam cap) |
| macOS cap ratio | 636 MB peak (5min) vs 1700 MB cap = 37% utilization |

## 2. Launch Command

```bash
bash scripts/soak/v400_1h_soak.sh 10080 3411
# Equivalent: WAL_SYNC=batch:100 bash scripts/soak/v400_1h_soak.sh 10080 3411
```

Process tree:

| PID | Role | Status |
|-----|------|--------|
| 41637 | bash wrapper | ALIVE |
| 41654 | sqlrustgo-mysql-server | ALIVE |
| ... | pymysql driver | ALIVE |

## 3. First 60s Metrics (warmup window)

| t (s) | RSS (MB) | queries | errors | panics |
|------:|---------:|--------:|-------:|-------:|
|  15   | 219.5    | 7,916   | 0      | 0      |
|  30   | 355.6    | 13,515  | 0      | 0      |
|  45   | 467.7    | 21,858  | 0      | 0      |
|  60   | 460.9    | 24,527  | 0      | 0      |

**RSS range**: 220–468 MB (well under 1.7 GB cap; similar to 5min baseline).

**QPS**: ~408 sustained (24,527 queries / 60s).

## 4. Monitoring Plan

### 4.1 Status script

```bash
bash scripts/soak/v400_168h_soak_status.sh
```

Prints:
- Server PID + alive status
- Last 10 metric samples
- STABILITY_REPORT.md content (auto-generated on SOAK end)
- Warning if RSS > 1.7 GB

### 4.2 Live tail

```bash
# Latest metrics
tail -f /Users/liying/dev/sqlrustgo/results/soak-v400-1h-*/metrics.csv

# Server log
tail -f /Users/liying/dev/sqlrustgo/results/soak-v400-1h-*/server.log

# Driver log
tail -f /Users/liying/dev/sqlrustgo/results/soak-v400-1h-*/driver.log
```

### 4.3 Checkpoints

| Time elapsed | Action |
|-------------:|--------|
|   1h         | Mid-warmup sanity check |
|  24h         | Day-1 report (1/7 of duration) |
|  72h         | 3-day report (3/7) |
| 120h         | 5-day report (5/7) |
| 168h         | Final report + GA claim sign-off |

Each checkpoint writes a STABILITY_REPORT_<TS>.md in `results/`.

## 5. Acceptance Criteria

| Criterion | Required | Status |
|-----------|----------|--------|
| Server alive at end | Yes | ⏳ in-progress |
| 0 panics | Yes | ✅ so far |
| 0 driver errors | Yes | ✅ so far |
| RSS peak < 1.7 GB | Yes | ✅ 468 MB (sample at 60s) |
| No jetsam kill | Yes | ✅ |
| ≥ 1M queries/day sustained | Yes | ✅ on track (408 QPS × 86400s = 35.3M/day) |
| Final STABILITY_REPORT.md | Yes | ⏳ on completion |

## 6. Failure Modes & Mitigation

| Mode | Detection | Mitigation |
|------|-----------|------------|
| RSS growth > 1.7 GB | metrics.csv threshold alert | Kill + reduce `gc_lag` to 128 |
| Server crash | pid file shows dead | Restart with same config |
| macOS jetsam | server log shows `Killed: 9` | Reduce dataset or threads |
| Driver errors | driver_q vs driver_e mismatch | Restart driver, log to STABILITY_REPORT |

## 7. Roll-back plan

If 168h SOAK fails at any checkpoint:

1. Capture STABILITY_REPORT_<TS>.md with failure mode
2. Tag `ga/v4.0.0` rolls back to `ga/v4.0.0-rc-cond` (snapshot at 5min PASS)
3. GA claim downgrade: "5min baseline PASS; 168h deferred to v4.0.1"
4. Update CLAIM_DOWNGRADE_MANIFEST.md with new boundary
5. Re-tag `ga/v4.0.0` and re-push

## 8. References

- `docs/releases/v4.0.0/SOAK_BASELINE_5MIN_2026-09-19.md` (pre-flight PASS)
- `docs/releases/v4.0.0/PHASE_B_WAL_BATCH.md` (WAL batch:100 trade-off)
- `docs/releases/v4.0.0/PHASE_B_MVCC_GC.md` (GC tuning rationale)
- `docs/releases/v4.0.0/GA_GATE_REPORT.md` (GA promotion verdict)
- Commit `b87997d4a1` (GC 1s/256, RSS -58%)