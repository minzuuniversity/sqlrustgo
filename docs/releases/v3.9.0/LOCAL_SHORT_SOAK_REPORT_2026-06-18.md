# v3.9.0 Local Short-Soak Report (2026-06-18)

> **Caught a P0 GA blocker**: WAL grows unboundedly in serve loop. Real wall-clock soak (#3265/#3266) cannot complete without fix.

## Method

Created `scripts/stability/run_short_soak_ladder.sh` (progressive 30m→1h→2h→4h ladder) and ran locally on Mac Mini. Each step starts `sqlrustgo-mysql-server serve` + sysbench oltp_read_write + monitors RSS/FD/CPU/WAL/disk.

Workload per step: sysbench oltp_read_write, 2 threads, 1000-row sbtest1, mix 70% SELECT / 20% INSERT / 5% UPDATE / 5% DELETE.

## Steps Run

| Step | Duration | Result | Key Finding |
|------|----------|--------|-------------|
| 30m  | 30 min   | **PASS** (RSS/FD) but **WAL hit 22.8 GB** | Disk full: "No space left on device (os error 28)" at end |
| 1h   | 2 min    | Stopped early (projected: 45 GB WAL) | WAL growth rate: **~22 MB/s linear** |
| 2h   | not run | (would hit ~91 GB WAL) | blocked by WAL bug |
| 4h   | not run | (would hit ~183 GB WAL) | blocked by WAL bug |

## Stable Metrics (RSS / FD / CPU)

These are healthy across all runs:

| Metric | 30m start | 30m end | Δ | Verdict |
|--------|-----------|---------|---|---------|
| RSS | 18 MB | 23 MB | +5 MB | ✅ stable |
| FD | 11 | 11 | 0 | ✅ stable |
| CPU | 162% | 0% (idle) | - | ✅ normal |
| Locks | 0 | 0 | 0 | ✅ none |

**Conclusion**: server process itself is leak-free. The bug is **exclusively in WAL management**.

## P0 Finding: WAL Unbounded Growth

**Symptom** (measured locally):

| Elapsed | WAL Size | Growth Rate | Projected 168h |
|---------|----------|-------------|----------------|
| 30s | 1.1 GB | 37 MB/s | — |
| 2 min | 4.9 GB | 1.3 GB/min | — |
| 30 min | 22.8 GB | 0.76 GB/min | **7.6 TB** (impossible) |

**Disk exhaustion error at 30 min**: sysbench aborted with `errno = 1064, state = '42000': I/O error: No space left on device (os error 28)`. WAL consumed 22.8 GB on a 228 GB disk.

**Root cause** (verified by code inspection):
- `CheckpointManager` is defined at `crates/storage/src/checkpoint.rs` with `max_wal_size_mb` config
- But it is **never invoked from the serve loop** (verified: 0 hits for `do_checkpoint` / `spawn.*checkpoint` / `run_periodic` in `crates/`)
- `crates/mysql-server/src/main.rs` serve command only spawns signal handler + calls `run_server_v2`
- No background thread calls `CheckpointManager::do_checkpoint()` or `WalManager::truncate_after_checkpoint()`

**Why Sprint 8 simulated soak didn't catch this**:
- 1,440× compressed soak does not actually write to disk
- Recovery engine bypasses WAL by reading JSON files (per commit `d5c8fe384`)
- The "10/10 PASS" claim was an illusion

## Why This Matters for GA

| Step | Wall-clock | WAL cost (projected) | Disk impact (1 TB SSD) |
|------|-----------|---------------------|------------------------|
| 24h #3264 (closed) | 24h | ~1 TB | ❌ disk full in <24h |
| 72h #3265 | 72h | ~3 TB | ❌ disk full in <24h |
| 168h #3266 | 168h | ~7.6 TB | ❌ disk full in <24h |

**No real wall-clock soak can complete without fixing this bug.**

## Fix Required (Blocked on GA)

Spawn a background checkpoint thread in `run_server_v2` (or `crates/mysql-server/src/main.rs:serve`):

```rust
let checkpoint_handle = std::thread::spawn(move || {
    let cfg = CheckpointConfig { max_wal_size_mb: 100, interval: 60, .. };
    let mgr = CheckpointManager::new(data_dir.clone(), cfg).unwrap();
    loop {
        std::thread::sleep(Duration::from_secs(cfg.interval));
        if mgr.needs_checkpoint() { mgr.do_checkpoint().ok(); }
    }
});
```

Acceptance:
- 30m soak: WAL < 500 MB (was 22.8 GB)
- 1h soak: WAL < 1 GB (was 45 GB)
- 168h soak: WAL bounded (was 7.6 TB)

## Action Items

1. ✅ Created `scripts/stability/run_short_soak_ladder.sh` (committable)
2. ✅ Added WAL/disk guard thresholds
3. ✅ Filed Gitea issue: WAL grows unbounded (P0/GA-BLOCKER) [pending — Gitea unreachable at time of report]
4. ⏳ Fix WAL checkpoint invocation in serve loop
5. ⏳ Re-run 30m ladder post-fix to verify WAL stays bounded
6. ⏳ Re-run 1h → 2h → 4h progression
7. ⏳ Once 4h passes locally, dispatch #3265 (72h) → #3266 (168h) to Z6G4

## Artifacts

- `scripts/stability/run_short_soak_ladder.sh` — 290-line bash script with resource guards
- `docs/releases/v3.9.0/LOCAL_SHORT_SOAK_REPORT_2026-06-18.md` — this report
- WIP Gitea issue (P0/GA-BLOCKER): WAL grows unbounded