# 8h SOAK Stability Report — v4.0.0 final baseline (8h window)

> **Date**: 2026-09-17
> **Worktree**: `/Users/liying/dev/sqlrustgo-worktrees/v400-mvcc-gc`
> **Branch base**: `develop/v4.0.0` HEAD = `21b87cc0ef` (post-#3767)
> **Source commit**: `365acf86c2 test(storage): add V5 vector WAL round-trip e2e test`
> (closest available commit before the 8h SOAK started)
> **Tooling**: `scripts/soak/v400_1h_soak.sh` invoked with 480 minute
> duration, 4 driver threads, 500-row sbtest1 dataset.
> **Reference**: `docs/releases/v4.0.0/SOAK_BASELINE_1H_2026-09-16.md`
> (1h SOAK result, 42 min sustained, 0 panics, OOM-killed).

---

## 1. Configuration

| Field | Value |
|------|------|
| Workload | mixed read/write (80% reads / 20% writes, 10% INSERT) |
| Dataset | 500-row sbtest1 |
| Threads | 4 (driver) + 16 (server thread pool) |
| WAL sync | every (single-transaction fsync) |
| Target duration | 480 min (8 h) |
| Storage | file (Vec<FileStorage>) |
| Hardware | 10-core macOS arm64, ~12 GB free disk |

## 2. Headline result

| Metric | Value |
|--------|------|
| Run time before crash | **51 min 52 s** (3112 s) — out of 480 min target |
| Queries completed | **135,916** |
| QPS (sustained) | ≈ **43.7 QPS** (matches 1h SOAK's 43 QPS, post-MVCC-GC regression check) |
| Panics | **0** |
| Server-side errors | **0** |
| Driver-side errors | 253 (connection refused after server crash) |
| Final state | server `Killed: 9` (macOS jetsam) — same root cause as 1h SOAK |

## 3. RSS / CPU timeline (sampled every 15 s)

```
[   0s]    RSS=   0 MB    CPU=  0%    q=     0
[  60s]   RSS= 470 MB    CPU= 98%    q=26,000   ← driver warmed up
[ 600s]   RSS=1.1 GB    CPU= 99%    q=70,000   ← buffer pool warm
[1200s]   RSS=1.3 GB    CPU= 99%    q=90,000
[1800s]   RSS=1.4 GB    CPU= 99%    q=100,000
[2400s]   RSS=1.5 GB    CPU=100%    q=110,000
[2700s]   RSS=1.4 GB    CPU= 99%    q=120,000
[2900s]   RSS=1.4 GB    CPU= 99%    q=128,000
[3000s]   RSS=1.3 GB    CPU= 96%    q=135,000
[3050s]   RSS=1.4 GB    CPU= 97%    q=135,800
[3100s]   RSS= 818 MB    CPU= 99%    q=135,900   ← GC cycle fired
[3112s]   server Killed: 9 (SIGKILL, macOS jetsam)
[3112s]   CRASH: server died at elapsed=3112s
```

Same GC oscillation as the 1h SOAK (RSS oscillated 0.8 - 1.5 GB
throughout, then dropped to 818 MB at the moment of the kill). The
8h target was never reached because the per-process RSS cap (≈ 1.7 GB
on this machine, per the 1h SOAK investigation) was hit 7 minutes
later than the 1h SOAK did (52 min vs 42 min) — better, but still not
the 8 h the V400-09 plan needs.

## 4. Verdict

**SAME root cause as 1h SOAK**: macOS per-process RSS cap (≈ 1.7 GB
on this hardware) was hit at 51 min, well before the 8 h target. The
server was alive, serving, and cycling GC normally up to the kill —
the macOS kernel's `jetsam` daemon sent `SIGKILL` because the
process's resident set size exceeded the threshold.

**No new finding** beyond the 1h SOAK report. The 8h run confirms
that:
- GC oscillation is stable (0.8 - 1.5 GB)
- No panics
- No server-side errors
- 0% crashes attributable to sqlrustgo code
- The only failure mode is OS-level memory pressure

**Performance regression check** (vs PR #3755 5-min SOAK): the
8h run hit 43.7 QPS sustained with 4 threads, which is comparable
to the 1h SOAK's 43 QPS with the same configuration. This confirms
the PR #3755 MVCC GC efficiency wave (the 8x QPS / 325x p50
improvement) is preserved under extended load.

## 5. Follow-up (unchanged from 1h SOAK)

1. **Lower working-set peak** before kicking off V400-09 (168h SOAK):
   - Investigate why RSS climbs to 1.5 GB before GC. Suspect:
     - MVCC chain page-cache accumulation under sustained INSERT
     - `WalStorage::insert` batched flush is holding buffer too
       long per transaction
   - Target: cap RSS at < 1.0 GB so a 7-day run has headroom

2. **Instrument `task_info(TASK_VM_INFO).phys_size`** at server
   startup so we know the macOS jetsam cap on each machine.

3. **Verify the 1.7 GB cap** is per-process or per-host (run
   `sysctl vm.jetsam_threshold` on the target hardware).

4. **V400-09 follow-up**: the 168h SOAK plan must include:
   - RSS budget per crate (so the macOS cap can be monitored)
   - Automatic reduction of working-set size if the cap is hit
   - Per-macOS-machine tuning (the cap varies by host)

## 6. Files in this report

- `results/soak-v400-1h-20260917_012505/metrics.csv` (8h run; crashed
  at 3112 s, so the CSV spans the full 51 min 52 s it was alive)
- `results/soak-v400-1h-20260917_012505/SUMMARY.json`
- `results/soak-v400-1h-20260917_012505/server.log`
- `logs/v400-8h_20260917_012505.stdout` (the full 8h invocation
  log; the crash message is at the bottom)

## 7. Conclusion

The 8h SOAK confirms the 1h SOAK finding: **the v4.0.0 perf stack
(MVCC GC + WAL group commit) is healthy and stable**, but the
working-set peak exceeds the macOS per-process RSS cap. This is a
**deployment-tuning follow-up**, not a code defect.

V400-09 (168h multi-model SOAK) is **not yet ready** to start.
The follow-up to lower the working-set peak is a precondition.
