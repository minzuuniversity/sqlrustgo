# 72h SOAK Design (Issue #3265)

> **Date**: 2026-06-27
> **Issue**: #3265 — [GA-P0/S3] Execute 72h long-running soak (WAL growth, memory leak, thread leak)
> **Author**: claude-macmini (brainstorming skill output)
> **Status**: APPROVED by user (2026-06-27)
> **Target**: v3.9.0 GA gate
> **Server**: Z6G4 (192.168.0.252)

---

## 1. Goal + Scope

### 1.1 Goal
Execute a **72-hour real wall-clock SOAK** (NOT compressed-time) to surface slow leaks and verify WAL checkpoint behavior per Issue #3265.

### 1.2 Success Criteria (from Issue #3265)
- **WAL file count**: must stabilize after checkpoint (not grow unboundedly)
- **Memory**: should plateau within 4-6 hours, not grow linearly
- **Thread count**: must stabilize (no thread leaks)
- **File descriptors**: must stabilize (no fd leaks)

### 1.3 Phased Approach (3 stages)
| Stage | Duration | Purpose | Exit Criteria |
|-------|----------|---------|---------------|
| **Stage 1** | 6h | smoke: verify WAL fix + ServerThreadPool stability under load | RSS < 500 MB, WAL < 1 GB, 0 OOM, FD < 100 |
| **Stage 2** | 24h | extended: observe memory plateau + thread stability | RSS does not grow > 10%, WAL truncate every ≤ 60s |
| **Stage 3** | 72h | full: long-term stability + slow leak detection | RSS < 1 GB, WAL < 100 MB, FD < 200, Thread count stable ±10% |

### 1.4 Out of Scope
- ❌ v3.10 features (Issue #3575 sysbench prepared stmt) — deferred per ADR-013
- ❌ `v390_join_reorder` feature flag — Q10-Q22 timeouts may still occur

---

## 2. Prerequisites

### 2.1 Code Status (verified)
- ✅ Issue #3531 WAL checkpoint fix (commit `83c840c46`) merged in develop/v3.9.0 (e15c4ba35)
- ✅ Issue #3522 Q8/Q9 wire perf fix merged (feature flag `v390_join_reorder`, default OFF)
- ✅ PR #3608 --server-threads CLI + ServerThreadPool merged
- ✅ GRUB fix applied: `/proc/cmdline` no longer contains `mce=0` or `hest_disable`

### 2.2 Server Health (Z6G4 @ 2026-06-27 08:19)
- ✅ Uptime: 7h 27m, 5/7 days since 2026-06-22 power loss
- ✅ Memory: 404 GB total / 393 GB available
- ✅ Disk: 728 GB available (60% used)
- ✅ CPU: 80 cores, load avg 0.28
- ✅ No new hardware incidents in last 7 days
- ❌ IPMI unavailable (HP Z6 G4 BMC limitation; hardware monitoring blind spot)

### 2.3 Known Limitations
- **Issue #3575** (sysbench prepared stmt): deferred to v3.10 per ADR-013. `sysbench oltp_read_write` will fail on prepared statements. Workaround: use `--db-ps-mode=disable` or rely on TPC-H rotation.
- **TPC-H Q10-Q22**: may timeout (Q8/Q9 fixed but feature flag off by default).

---

## 3. Architecture + Components

### 3.1 Reused Components
- `scripts/stability/run_wired_soak.sh` — Main driver (fractional HOURS support)
- `crates/mysql-server` binary (PR #3608: `--server-threads` CLI)
- `tests/common/tpch_cli_harness.rs` — Test client
- `crates/storage/src/wal_storage.rs` + `checkpoint.rs` (Issue #3533)

### 3.2 New Components
- **`scripts/stability/run_soak_staged.sh`** — Stage-gate runner
  - Launches Stage 1 (6h) → verifies → Stage 2 (24h) → verifies → Stage 3 (72h)
  - Each stage emits `metrics.csv` + `incidents.log`
- **`scripts/stability/aggregate_soak_report.sh`** — Report generator
  - Input: 3 stages of metrics.csv
  - Output: `SOAK_72H_REPORT.md` (per Issue #3265 spec)

### 3.3 Architecture Diagram

```
Z6G4 (192.168.0.252)
├─ sqlrustgo-mysql-server (--server-threads 16, port 3396)
│  ├─ WalStorage + FileStorage (persistent)
│  ├─ CheckpointManager thread (PR #3533) — every 30s
│  └─ ServerThreadPool (16 workers, panic isolation)
│
├─ sysbench oltp_read_write (16 threads, 10k rows)
│  └─ Issue #3575: prepared stmt fails → use --db-ps-mode=disable
│
├─ tpch_22_rotate.sh (background, Q1-Q22 round-robin)
│  └─ Q10-Q22 may timeout → log + skip
│
└─ metrics sampler (60s interval)
   ├─ RSS, FD count, Thread count
   ├─ WAL file size, checkpoint count
   └─ query latency (per-query)
```

---

## 4. Data Flow

### 4.1 Per-Stage Flow
```
run_soak_staged.sh --stage=N --hours=H
  ├─ Launch sqlrustgo-mysql-server (background)
  ├─ Wait for server ready (handshake probe)
  ├─ Load TPC-H fixture (LOAD DATA LOCAL INFILE)
  ├─ Launch sysbench (background, 16 threads)
  ├─ Launch tpch_22_rotate.sh (background, 600s interval)
  ├─ Start metrics sampler (60s interval, background)
  ├─ Sleep $HOURS hours
  ├─ Stop all background processes (graceful)
  └─ Emit metrics.csv + incidents.log
```

### 4.2 Stage-Gate Verification
After each stage, run health checks:
- `ps -p <pid>` → server alive
- `ls -lh <data>/sqlrustgo.wal` → WAL size
- `cat /proc/<pid>/status | grep VmRSS` → memory
- `ls /proc/<pid>/fd | wc -l` → FD count
- `ls /proc/<pid>/task | wc -l` → thread count
- `journalctl --since "${HOURS}h ago" | grep -i oom` → OOM events

### 4.3 Final Report
`SOAK_72H_REPORT.md` includes:
- Per-stage summary table
- Time-series plots (RSS, FD, Thread, WAL)
- Issue #3265 checklist (4 success criteria)
- Incident log
- Recommendation: PASS / FAIL / NEEDS_MORE_WORK

---

## 5. Error Handling

| Failure Mode | Handling |
|--------------|----------|
| server OOM-killed | Restart server, resume from checkpoint, log to `incidents.log` |
| sysbench prepared stmt fail (Issue #3575) | Use sysbench `--db-ps-mode=disable` or skip sysbench, rely on TPC-H rotation |
| Disk full (>95%) | Auto-rotate old logs, alert, abort stage |
| Server power loss | Resume from boot, 72h clock continues (downtime NOT counted toward 72h) |
| TPC-H Q10-Q22 timeout | Skip query, log to `errors.log`, continue other queries |
| Stage exit criteria fail | Halt, report failure, do not proceed to next stage |

### 5.1 Recovery Procedures
- Server crash → auto-restart with `systemd` or `nohup` wrapper
- WAL corruption → restore from checkpoint (Issue #3533 WAL truncate thread)
- Metrics loss → regenerate from `ps` / `lsof` snapshots

---

## 6. Testing + Verification

### 6.1 Pre-Launch Verification
- `cargo check --all-features`: PASS (5-30s incremental)
- `cargo fmt --check --all`: PASS (0 diffs)
- `cargo test --workspace --no-run`: PASS (compiles)
- `cargo clippy --all-features -- -D warnings`: PASS (no warnings)

### 6.2 Per-Stage Monitoring (every 6h)
- `ps aux | grep sqlrustgo-mysql-server` — process alive
- `lsof -p <pid> | wc -l` — FD count
- `ls -lh <data_dir>/sqlrustgo.wal` — WAL size
- `journalctl --since "6h ago" | grep -iE "oom|mce|aer"` — events
- `journalctl --since "6h ago" | grep -i hardware` — hardware warnings

### 6.3 Post-Completion Report
- `SOAK_72H_REPORT.md` per Issue #3265 spec
- Time-series plots (RSS, FD, Thread, WAL)
- Issue #3265 comment with report URL + git log

### 6.4 Success Gates
- Stage 1 (6h) → Stage 2 only if exit criteria met
- Stage 2 (24h) → Stage 3 only if exit criteria met
- Stage 3 (72h) → Issue #3265 close only if all 4 success criteria met

---

## 7. References

- Issue #3265 — [GA-P0/S3] 72h SOAK
- Issue #3531 — WAL grows unbounded (CLOSED, PR #3533)
- Issue #3522 — TPC-H Q8/Q9 fix (CLOSED, feature flag)
- Issue #3575 — sysbench prepared stmt (CLOSED, deferred to v3.10 ADR-013)
- ADR-013 — v3.10 wire protocol repair (PR #3574)
- PR #3608 — `--server-threads` CLI + ServerThreadPool (merged)
- `docs/audit/issues/ISSUE-3577_z6g4_power_loss_2026-06-22.md` — Z6G4 power loss report
- `docs/audit/status/SOAK_1H_BASELINE_SMOKE_2026-06-26.md` — SOAK 1h baseline
- `scripts/stability/run_wired_soak.sh` — Main driver
- `scripts/stability/run_72h_soak_v2.sh` — V2 runner (reference)

---

## 8. Open Questions

None at design time. All blockers (Issue #3531, GRUB fix) verified resolved.

---

## 9. Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-06-27 | Use `run_wired_soak.sh` as base | Already has fractional HOURS + TPC-H rotation |
| 2026-06-27 | Phased 6h/24h/72h approach | Risk mitigation: detect issues early, abort before long runs |
| 2026-06-27 | Skip Issue #3575 (sysbench prepared) | v3.10 milestone per ADR-013; SOAK focus is leak detection not throughput |
| 2026-06-27 | Default `v390_join_reorder=OFF` | Avoid regression risk; TPC-H rotation tolerates timeouts |
| 2026-06-27 | No new worktree | Run on Z6G4 directly (existing test infrastructure) |

---

**Status**: APPROVED → commit to git → proceed to writing-plans skill