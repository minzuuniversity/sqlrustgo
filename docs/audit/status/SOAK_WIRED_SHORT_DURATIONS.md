# Wired Soak — Short Durations (0.5 / 1 / 2h)

> **Status**: scripts ready, awaiting launch authorization
> **Date**: 2026-06-14
> **Companion**: test_integration_5min.sh (PR #3380, 5-min anti-OOM smoke)
> **Driver binary**: `target/release/sqlrustgo-mysql-server serve` (real, not in-process)
> **Frontend**: `sysbench oltp_read_write` (industry-standard wired) + `mysql` CLI for TPC-H 22
> **Closes**: #3225 (24h/72h) prerequisite + user-requested 0.5/1/2/4/8/12/16/48h gaps

## Why this exists

Two separate concerns that the GA team needs but neither the existing v2 nor the
5-min anti-OOM test alone covers:

1. **5-min anti-OOM test** (`test_integration_5min.sh`, PR #3380)
   - 60s real run with hard resource limits (RSS 2GB, FD 512, DB 500MB)
   - Goal: prove the integrated server binary + sysbench 1.0.20 oltp_read_write
     wire-protocol path works AT ALL (architecture verification)
   - NOT a soak — too short to detect growth/leak trends

2. **24h+ v2 soak** (`run_24h_soak_v2.sh` + `run_72h/168h_soak.sh`)
   - Long real wall-clock runs
   - But: minimum 24h means 24h+ before any signal — too slow for
     fast feedback when iterating on regressions

**Gap**: short-duration real soak (30m / 1h / 2h) that gives feedback in
minutes-to-hours, runs the FULL wired stack (no resource hard-limits), and
exercises TPC-H 22 + sysbench the same way v2 does.

This PR fills that gap.

## Scripts (all new in this PR)

| File | Role |
|------|------|
| `scripts/stability/run_wired_soak.sh` | Single-instance driver; HOURS accepts fractional values (0.5 / 1 / 2 / 4 / 8 / 12 / 16 / 24 / 48 / 72). Auto-scales INTERVAL + TPCH_ROTATE_INTERVAL for short runs. |
| `scripts/stability/load_tpch_fixture.sh` | Loads TPC-H 8-table schema + LOAD DATA via mysql CLI |
| `scripts/stability/tpch_22_rotate.sh` | Periodically runs TPC-H Q1..Q22 (per-query error logged) |
| `scripts/stability/launch_parallel_soak.sh` | Orchestrates 3+ instances across ports, writes PID map |
| `docs/audit/status/SOAK_WIRED_SHORT_DURATIONS.md` | This file |

The existing scripts (run_24h_soak.sh, run_24h_soak_v2.sh, run_72h/168h_soak.sh,
test_integration_5min.sh, capture_tpch_sha256.sh) are **left untouched** for
backwards compatibility.

## How it satisfies the "wired" requirement

1. `sqlrustgo-mysql-server serve --port N --data-dir D` — launches the real release binary
2. `sysbench oltp_read_write --db-driver=mysql --mysql-port=N ...` — connects via real MySQL wire protocol
3. `mysql -h HOST -p PORT --local-infile=1 -e "..."` — TPC-H 22 queries use real LOAD DATA + real query path
4. `LOAD DATA LOCAL INFILE` — uses the MySQL-spec bulk loader

No in-process engine calls. No synthetic workload generator inside the server.

## Recommended workflow (short first, then long)

```bash
cd /home/ai/sqlrustgo

# Step 0: 5-min architecture smoke (PR #3380)
bash scripts/stability/test_integration_5min.sh
# → if this fails, nothing else will work — fix it first

# Step 1: 30m observation (1 instance, port 3500)
HOURS=0.5 PORT=3500 bash scripts/stability/run_wired_soak.sh
# → review STABILITY_REPORT.md, check growth trends

# Step 2: 1h + 2h parallel (2 instances, ports 3501-3502)
PHASE_SHORT="1 2" BASE_PORT=3501 \
    bash scripts/stability/launch_parallel_soak.sh
# → ~2h wall, then evaluate growth deltas

# Step 3: longer runs only if Step 1+2 are clean
PHASE_A="4 8 12" BASE_PORT=3503 \
    bash scripts/stability/launch_parallel_soak.sh
PHASE_B="16 24" BASE_PORT=3506 \
    bash scripts/stability/launch_parallel_soak.sh
PHASE_C="48 72" BASE_PORT=3508 \
    bash scripts/stability/launch_parallel_soak.sh
```

## Outputs (per instance, under `test_results/wired_soak_<H>h_<ts>_p<PORT>/`)

| File | Content |
|------|---------|
| `STABILITY_REPORT.md` | Verdict + per-criterion PASS/WARN + Mode (SHORT_OBSERVATION vs FULL_SOAK) |
| `metrics.csv` | RSS/FD/CPU/WAL samples (auto-scaled interval) |
| `sqlrustgo.log` | server log |
| `sysbench.log` | sysbench output |
| `tpch_22_rotate.log` | per-query latency CSV (ts,query,elapsed_ms,status) |
| `sqlrustgo.pid` / `tpch_rotate.pid` | PIDs for cleanup |

`test_results/parallel_soak_status.json` is the master PID map for the
parallel launcher.

## Acceptance criteria (inherited from v2, plus TPC-H, time-scaled for short runs)

| Criterion | Threshold | Notes |
|-----------|-----------|-------|
| Crashes | 0 | v2 |
| RSS growth | < 50 MB (24h-equiv); auto-scaled to elapsed fraction for short runs | v2 + new scaling |
| FD growth | < 50 (24h); < 5 for short runs | v2 + new scaling |
| Final RSS | < 4096 MB | v2 |
| Final WAL | < 10240 MB | v2 |
| sysbench errors | 0 | v2 |
| TPC-H rounds | ≥ 1 (if rotation enabled) | v3 |
| TPC-H per-query errors | 0 ideal, WARN allowed for known-broken Q2/Q9/Q13/Q16/Q17/Q20/Q21/Q22 (PR-3262/3265) | v3 |

## Launch authorization

**NOT yet started.** Awaiting user review per "先 commit 留底再说" workflow.

Reasons to defer:
- Each instance uses ~150MB memory + 1 port; running 3+ in parallel
  on a single host (Z440) requires user confirmation that CPU/disk are
  not shared with other workloads
- Step 0 (5-min anti-OOM) is the prerequisite — if it fails, no point
  in long runs
- A 30m soak gives signal within an hour, useful for early regression
  detection without committing to multi-day test cycles

## Relationship to other stability tests

| Test | Duration | Goal | Resource limits |
|------|----------|------|-----------------|
| `test_integration_5min.sh` (PR #3380) | 60s | Architecture smoke | Hard: RSS 2GB, FD 512, DB 500MB |
| **`run_wired_soak.sh` HOURS=0.5** | 30 min | Short observation | None (real soak) |
| **`run_wired_soak.sh` HOURS=1** | 1 hour | Short observation | None |
| **`run_wired_soak.sh` HOURS=2** | 2 hours | Short observation | None |
| `run_wired_soak.sh` HOURS=4..12 | 4-12 hours | Medium soak | None |
| `run_24h_soak_v2.sh` | 24 hours | GA gate | None |
| `run_72h_soak.sh` | 72 hours | Post-GA | None |
| `run_168h_soak.sh` | 168 hours (7d) | Post-GA weekly | None |
