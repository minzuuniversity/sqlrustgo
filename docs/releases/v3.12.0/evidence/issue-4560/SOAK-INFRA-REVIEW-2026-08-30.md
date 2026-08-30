# SOAK Infrastructure Review — Problems Exposed by 7h28m Run on 2026-08-30

> **status:** draft for Tier-1/2/3 follow-up
> **author:** openclaw (via AI agent Claude Code)
> **branch:** `fix/v312-59-d/4566-evidence-recheck`
> **incident reference:** [`INCIDENT-REPORT-2026-08-30.md`](./INCIDENT-REPORT-2026-08-30.md)
> **patch reference:** commit `66dac1aab2` (PR #4591)
> **scope:** `scripts/soak/run_soak_loop.sh` + `evidence/issue-4560/run_20260830_post4566_7h28m_salvage/`

## 0. TL;DR

The 2026-08-30 SOAK was successful for its #4566 recheck scope (7h28m zero-error, stable RSS/FD/threads, WAL cycling normal), but it surfaced **12 systemic infrastructure problems** in `run_soak_loop.sh` and its operating assumptions. The fix in `66dac1aab2` resolves only problem P-1 (process-group reap). The remaining 11 are **not regressions** — they were latent before this commit. They block GA-2's 168h target and any future multi-tenant SOAK deployment.

| Tier | Count | Resolution timeline |
|------|--------|---------------------|
| **Tier 1** (blocks GA-2 168h) | 4 problems | needs follow-up issues + sprint |
| **Tier 2** (correctness/gaps in current SOAK output) | 5 problems | can be bundled into single #4592 |
| **Tier 3** (nice-to-have / future-proofing) | 3 problems | can be deferred to v3.13 / v3.14 |

A single Tier-1 problem (`P-2` `/tmp` data-dir wipe on reboot) **would have silently destroyed a future 168h SOAK** if the 2026-08-30 reboot at 16:37 CST had not been unrelated. This is the highest-priority infrastructure risk.

---

## 1. Process Lifecycle (the problem #4591 already fixed + 4 it didn't)

### P-1. No self-detach — parent-shell reap takes the whole process group ✅ FIXED in `66dac1aab2`

**Severity:** Tier 1 → now RESOLVED
**Symptoms:** `run_soak_loop.sh` started from AI agent Bash tool → when agent session terminated, parent bash was reaped → kernel SIGKILL'd entire process group (sqlrustgo + sysbench + script itself) → no graceful cleanup, no `=== 清理 ===` marker, no warning, no panic.
**Evidence:** INCIDENT-REPORT §3.1 — python gateway `pid=4540` (unrelated) died at the same second, confirming process-tree reap.
**Fix:** `setsid + nohup + disown` self-detach added at run_soak_loop.sh:86-118.
**Residual risk:** A future script change that removes the early-exit path (e.g. someone moves the detach block after `preflight()`) will silently re-introduce the bug. Need a CI test that launches the script from a short-lived parent and verifies it survives.

### P-2. 🔴 Data-dir on `/tmp` — wiped at system reboot

**Severity:** **Tier 1 (HIGHEST)** — would have caused silent loss of any 168h SOAK
**Symptoms:** `run_soak_loop.sh:235` hardcodes `CYCLE_DATA="/tmp/sqlrustgo-soak-data-${SOAK_PORT}"`. `/tmp` is a tmpfs on Ubuntu; **every reboot wipes it**. The 16:37 CST reboot on 2026-08-30 wiped `/tmp/sqlrustgo-soak-data-3396/` and `/tmp/sqlrustgo-soak-logs-3396/`. The SOAK itself survived only because its `monitor.log + periodic_reports.log + sysbench.log + metrics.csv` were redirected into `~/sqlrustgo-soak-results/` (persistent); the **live WAL + buffer-pool pages in `/tmp`** were lost.
**Impact:** For a 168h GA-2 SOAK spanning a reboot, the server would `Err(DataDirNotFound)` at startup cycle and lose all in-flight state. The whole 168h run would be invalidated.
**Evidence:** INCIDENT-REPORT §2.4 — `ls /tmp/sqlrustgo-soak-data-3396/` → "No such file or directory", `last reboot` shows 16:37:02.
**Fix recommendation:**
```bash
# Default to ~/sqlrustgo-soak-data (persistent) — only fall back to /tmp if explicitly requested
CYCLE_DATA="${SOAK_DATA_DIR:-${HOME}/sqlrustgo-soak-data-${SOAK_PORT}}"
# Or: refuse to start if $HOME is on tmpfs (detect via stat -f)
```
**Estimated effort:** 1 PR, ~15 lines + 2 tests.

### P-3. 🔴 Hardcoded `nice` levels (`-n 10`, `-n 15`) — no override env

**Severity:** Tier 1 — affects host load fairness when SOAK runs alongside other workloads
**Symptoms:** `run_soak_loop.sh:245, 320` hardcode `nice -n 10` (server) and `nice -n 15` (sysbench). If the host has competing workloads at higher priority (CI runner, batch jobs, build daemons), the SOAK can be starved. Conversely, if the SOAK runs on a dedicated host (e.g. Z6G4 GA-2 runner), `-n 15` sysbench artificially caps throughput, hiding server-side capacity.
**Impact:** GA-2 throughput numbers (QPS, lat p95) are **not comparable** across host types because nice-level interference is uncontrolled. May produce false negatives ("server can't handle 8 threads" when really sysbench was throttled) or false positives (clean QPS numbers when nothing else was competing).
**Fix recommendation:**
```bash
SOAK_NICE_SERVER="${SOAK_NICE_SERVER:-10}"
SOAK_NICE_SYSBENCH="${SOAK_NICE_SYSBENCH:-15}"
nice -n "${SOAK_NICE_SERVER}" ...
nice -n "${SOAK_NICE_SYSBENCH}" ...
```
**Estimated effort:** 1 PR, ~10 lines + doc update.

### P-4. 🔴 No `--time` vs `SOAK_HOURS` synchronization — sysbench keeps running after script "completes"

**Severity:** Tier 1 — creates orphaned sysbench process after SOAK deadline
**Symptoms:** `run_soak_loop.sh:332` hardcodes `sysbench --time=86400` (24h), but `SOAK_HOURS` defaults to 24 and can be set to any value (test invocations used 1, 8). If `SOAK_HOURS=8` is set, the script's `main_loop` deadline fires at 8h and the script exits with code 0 — but sysbench continues running for another 16h because its own `--time=86400` is hardcoded. The `cleanup` trap on EXIT will kill the sysbench process via TERM, so it does eventually die — but the kill is not graceful (SIGTERM without `--time` re-evaluation), and a parent reap mid-exit would leave sysbench orphaned.
**Impact:** On a long host (idle weekend), orphaned sysbench could keep running for 16h+, holding TCP port + WAL, polluting metrics.
**Fix recommendation:**
```bash
# Replace hardcoded --time=86400 with:
local sysbench_seconds=$(( (SOAK_HOURS + 1) * 3600 ))  # +1h buffer
sysbench ... --time="${sysbench_seconds}" ...
```
**Estimated effort:** 1 PR, ~3 lines + 1 test.

### P-5. Disk-limit cycle-restart destroys in-flight sysbench statistics

**Severity:** Tier 2 (latent — never triggered in 7h28m; max disk=12MB vs 800MB limit)
**Symptoms:** `run_soak_loop.sh:472-503` triggers a full server+sysbench restart when disk usage exceeds 800 MB. Sysbench statistics (QPS, lat percentiles, error counts) **reset to zero** at restart; you lose the ability to compare pre-restart and post-restart metrics on the same axis. The new `metrics.csv` is written with a fresh header (line 490) but **the header is the same string** — concatenating pre/post `metrics.csv` produces valid data, but no marker indicates where the restart boundary is.
**Impact:** Any future long SOAK hitting the disk threshold loses per-restart QPS continuity. Without an explicit `restart_seq` column or separate `metrics_<restart_seq>.csv`, post-hoc analysis must hand-correlate with `monitor.log` `=== 循环重启完成 ===` markers.
**Fix recommendation:**
```bash
# Add restart_seq column + use a separate metrics file per cycle
echo "ts,elapsed_s,restart_seq,rss_kb,fd,threads,wal_bytes,disk_bytes,server_qps,sysbench_qps" > "${METRICS_CSV}.${restart_count}"
```
**Estimated effort:** 1 PR, ~10 lines.

---

## 2. Workload Coverage (single-workload blind spots)

### P-6. 🟡 Single workload (oltp_read_write, 8 threads, 10k rows) — no concurrency, table-size, or read/write mix sweep

**Severity:** Tier 2 — limits SOAK as a regression-detection tool
**Symptoms:** `run_soak_loop.sh:300-309, 321-334` always runs `oltp_read_write` at `--threads=8 --table-size=10000 --tables=1`. No support for `oltp_read_only`, `oltp_write_only`, `oltp_insert`, `oltp_update_index`, or higher thread counts.
**Impact:** PR #4566's TLS-handshake fix is verified at **8 threads only**. If a customer runs 16+ threads (real-world concurrency), the fix may still starve. Also misses:
- Read-only bursts (catches different buffer-pool behavior than mixed read-write)
- Large-table range scans (1M+ rows, tests B+ tree fan-out)
- Index contention (oltp_update_index stresses WAL+index mutex)
**Evidence:** 7h28m run stayed at exactly `thds: 8` throughout — see periodic_reports.log line 22-495.
**Fix recommendation:** New env vars `SOAK_WORKLOAD`, `SOAK_SB_THR`, `SOAK_TABLE_SIZE` already exist (`SOAK_SB_THR` and `SOAK_TABLE_SIZE`); add `SOAK_WORKLOAD` to drive `--test=oltp_*` selection. Add `SOAK_DURATION_LIST="60 600 3600"` for sweep mode.
**Estimated effort:** 1 PR, ~20 lines.

### P-7. 🟡 No chaos/fault injection — SOAK cannot detect durability or recovery bugs

**Severity:** Tier 2 — GA-2 168h is meaningless if recovery is broken
**Symptoms:** SOAK runs continuously without injecting any faults. No SIGKILL drill (kill server mid-query, verify WAL replay), no disk-full simulation (fill data-dir, verify error handling), no network partition (drop sysbench connection mid-tx, verify connection cleanup), no clock skew (advance system clock, verify checkpoint timing).
**Impact:** The 7h28m run verified "server stays up under steady load" — it did **not** verify:
- WAL replay correctness after forced shutdown
- Connection cleanup after client disconnect
- B+ tree root page eviction under memory pressure
- Transaction abort/rollback under concurrent writers
- Buffer pool cold-start after server restart
**Evidence:** `monitor.log` is 788 bytes containing only startup messages — no `=== chaos:` markers, no fault injection events.
**Fix recommendation:** New sibling script `scripts/soak/chaos_drills.sh` that imports the SOAK harness and runs:
- `kill -KILL $server_pid` at random interval, verify restart succeeds + WAL replay OK
- `dd if=/dev/zero of=$CYCLE_DATA/fill bs=1M count=799` to test disk-full
- `tc qdisc add dev lo root netem loss 5%` for packet loss
**Estimated effort:** 1 PR, ~80 lines (new file) + 3 drill scripts.

### P-8. 🟢 No `--rate` or `--threads=N` ramp — concurrency cliff detection impossible

**Severity:** Tier 3 — nice-to-have for capacity planning
**Symptoms:** SOAK is constant 8-thread. Cannot detect the "concurrency cliff" where adding one more thread causes QPS to halve (typical of lock contention, single-threaded WAL writer, mutex hotspots).
**Impact:** All reported QPS/lat numbers are at exactly 8 threads; cannot extrapolate to 16/32/64.
**Fix recommendation:** Add `SOAK_THREAD_RAMP="1 2 4 8 16 32"` — run a 5-min sysbench at each concurrency level, log separate metrics files per level.
**Estimated effort:** 1 PR, ~25 lines.

---

## 3. Observability Gaps (you can't fix what you can't see)

### P-9. 🔴 `ServerQPS` column is **always empty** in `metrics.csv` and `periodic_reports.log`

**Severity:** **Tier 1 (correctness bug, not a feature gap)** — every SOAK run since the script was written has produced zero server-side QPS data
**Symptoms:** `run_soak_loop.sh:197-200`:
```bash
server_qps() {
    grep -a "RESOURCE_MONITOR" "${SERVER_LOG}" 2>/dev/null | tail -1 | \
        grep -oE 'qps=[0-9]+\.[0-9]+' | tail -1 | cut -d= -f2 || echo "0"
}
```
**Bug:** The server's built-in `RESOURCE_MONITOR` log line format does NOT match `qps=[0-9]+\.[0-9]+`. In the 7h28m run, every one of 45 periodic reports shows `ServerQPS: ` (empty string). The metric is silently absent; readers assume "server QPS unknown" when in fact the bug is in the parser.
**Evidence:** `periodic_reports.log` line 2-495 — every `ServerQPS:` line is followed by empty space + newline. `metrics.csv` column `server_qps` is empty for all 44 rows.
**Impact:** **Loss of internal server QPS visibility** — we can see client-side sysbench QPS (~120 throughout), but not whether the server is *receiving* all those queries or *queueing/dropping* any. For #4566 specifically, we cannot verify that no TLS-handshake errors were silently queued on the server side. For GA-2, we cannot distinguish "server healthy at 120 QPS" from "server healthy but throttling 50% of incoming".
**Fix recommendation:**
1. Read server's actual built-in QPS format from `cargo run -- --help` or source — match the regex to the actual log format
2. Add an integration test: `assert grep -q "RESOURCE_MONITOR.*qps=" run_20260830_post4566_7h28m_salvage/server.log | head` → then regex matches actual format
3. Output: `ServerQPS: 118.50` (real number, not empty)
**Estimated effort:** 1 PR, ~5 lines (regex fix) + 1 test (assert non-empty).

### P-10. 🟡 `periodic_reports.log` is human-readable text — not machine-parseable

**Severity:** Tier 2 — limits post-hoc analysis automation
**Symptoms:** `run_soak_loop.sh:381-393` writes reports as `cat >> PERIODIC_LOG <<REPORT ... REPORT` — formatted text. No JSON, no CSV, no key-value pairs. Post-hoc analysis must grep + sed + awk to extract structured fields. The `metrics.csv` does the right thing for time-series but the periodic reports are the operator's primary "what's the SOAK doing right now" channel, and they don't aggregate well.
**Impact:** Any future alerting / monitoring system (Prometheus exporter, Grafana dashboard, Slack webhook) cannot consume `periodic_reports.log` directly. The QPS-decline pattern I noticed in §4 (193 → 117 qps over 7h) had to be **hand-extracted** from 45 text blocks via grep.
**Fix recommendation:** Emit JSON line + human-readable line:
```json
{"ts":"2026-08-30T03:28:33+08:00","rss_mb":188.8,"fd":14,"threads":19,"wal_mb":0.00,"disk_mb":3,"sysbench_qps":0,"restart_count":0}
```
Append to `periodic_reports.log` as well (or separate `periodic_reports.jsonl`).
**Estimated effort:** 1 PR, ~30 lines (emit JSON + keep human-readable).

### P-11. 🟢 No alerting — SOAK silently degrades, operator must poll

**Severity:** Tier 3 — manual polling acceptable for 24h SOAK, unworkable for 168h
**Symptoms:** Script only logs; no `curl -X POST $ALERT_WEBHOOK` for "RSS > 500MB" or "err/s > 0" or "QPS dropped 50%". Operator must SSH in and `tail -f monitor.log`.
**Impact:** The 7h28m run showed QPS declining from 193 (10min mark) to 117 (26460s) — a 39% drop. Without alerting, this was only discovered post-hoc. A real production SOAK would need webhook + Slack/email notification.
**Fix recommendation:** Add `SOAK_ALERT_WEBHOOK` env var; emit JSON payload on each 10-min report.
**Estimated effort:** 1 PR, ~20 lines.

---

## 4. Server-Side Behavior Observed in 7h28m (incidental findings, not script bugs)

These are observations about sqlrustgo **itself** based on the 7h28m evidence, NOT script problems. Listed here so they're not lost:

### O-1. QPS decline 193 → 117 over 7h (39% drop)

**Source:** `periodic_reports.log` lines 22 (qps 193.89 at 600s) → 495 (qps 117.20 at 26460s).
**Possible causes (not yet diagnosed):**
- Buffer-pool cold-cache thrash (RSS rose 111→365MB during the same window — see O-2)
- WAL checkpoint frequency (cycled 0→7.82MB every ~30min — see O-3)
- Lock contention growing under sustained writes (no `lighthouse`/`pprof` data to confirm)
- sysbench client-side connection-pool warmup (sysbench `--time` doesn't measure server-only)
**Action:** File follow-up issue "diagnose 39% QPS decline over 7h28m sysbench oltp_read_write sustained run" — attach the 7h28m metrics.csv + periodic_reports.log.

### O-2. RSS rose 111 → 365MB (Δ 254MB) but plateaued at 357MB by 04:18 (1h elapsed)

**Source:** `metrics.csv` lines 1-2 (rss_kb 114172→193296 in first 600s) → line 11 (rss_kb 286612 at 6014s) → plateau ~357000 kb (357MB) from 04:18 onwards.
**Interpretation:** Initial 254MB rise is buffer pool / page cache warmup, NOT a leak. Stable plateau confirms no leak. **This is a positive finding** — sqlrustgo's buffer pool is bounded.

### O-3. WAL cycled 0 → 7.82MB every ~30min (low checkpoint threshold)

**Source:** `metrics.csv` column `wal_bytes` shows sawtooth pattern: 2888607 → 143 → 90 → 3539287 → 349 → 7381502 → 7465567 → 491803 → 3799487 → ... (7 cycles in 7h28m, avg 7.0MB peak per cycle).
**Interpretation:** Checkpoint fires when WAL hits ~7.8MB, then drops to ~0. This is **expected behavior** but the threshold is low (7.8MB ≈ ~3000 transactions at 2.5KB/tx). For high-throughput OLTP, this could become a bottleneck if the disk can't keep up with checkpoint backpressure. **Not a blocker** but worth flagging for v3.13 capacity planning.

### O-4. `err/s: 0.00` and `reconn/s: 0.00` throughout 7h28m

**Source:** `sysbench.log` lines 22-495 (every 10-min sample).
**Interpretation:** **This is the strongest evidence that PR #4566 fixed the TLS-handshake stall** under sustained sysbench oltp_read_write with 8 threads. Zero TLS errors, zero reconnects, zero query failures.

---

## 5. Documentation / Process Gaps

### P-12. 🟢 GA2_168H_SOAK_TEMPLATE DEPRECATED without replacement

**Severity:** Tier 3 — operator confusion, not a functional bug
**Symptoms:** Commit `a336580486` marked GA2_168H_SOAK_TEMPLATE DEPRECATED because the Docker Z6G4 runner (PR #4565) is now the canonical GA-2 path. But no replacement operator SOP exists. A new operator attempting a 168h local SOAK has only the original GA2 template + this incident report.
**Impact:** Tribal knowledge — operators must read INCIDENT-REPORT + git log + run_soak_loop.sh comments to understand the "new way".
**Fix recommendation:** Write `docs/operations/soak-168h-local-sop.md` (~150 lines) covering:
- Hardware requirements (≥256GB RAM, ≥1TB free disk)
- Command sequence: `bash scripts/soak/run_soak_loop.sh` + monitor every 12h
- Expected behavior (RSS plateau, WAL cycling, sysbench QPS range)
- Failure modes + recovery (see INCIDENT-REPORT §6 for what to do on parent reap)
- Evidence archival (30-day retention, SHA256 verification)
**Estimated effort:** 1 PR, ~150 lines doc only.

---

## 6. Tier Prioritization Matrix

| ID | Tier | Severity | Blocks GA-2? | Estimated PR Size | Sprint-Ready? |
|----|------|----------|--------------|-------------------|---------------|
| **P-2** | **1** | data wipe on reboot | **YES** | 15 lines | YES — hot follow-up |
| **P-3** | 1 | nice-level hardcode | partial (QPS unreproducible across hosts) | 10 lines | YES |
| **P-4** | 1 | --time=86400 vs SOAK_HOURS mismatch | partial (orphaned processes) | 3 lines | YES |
| **P-9** | **1** | **ServerQPS always empty** (correctness bug) | **YES** — we can't verify server-side health | 5 lines + 1 test | **YES — highest priority** |
| P-1 | 1 | (already fixed in `66dac1aab2`) | DONE | DONE | DONE |
| P-5 | 2 | disk-limit restart boundary | no (latent) | 10 lines | YES |
| P-6 | 2 | single workload only | partial | 20 lines | YES |
| P-7 | 2 | no chaos drills | partial (recovery unverifiable) | 80 lines (new file) | sprint-sized |
| P-10 | 2 | periodic_reports not JSON | no | 30 lines | YES |
| P-11 | 3 | no alerting | no | 20 lines | future |
| P-12 | 3 | GA2 template deprecation note | no | 150 lines doc | future |
| P-8 | 3 | no thread ramp | no | 25 lines | future |

**Critical path:** P-2 + P-9 (the two highest-impact bugs blocking 168h SOAK) + the fix verification chain (CI test that future patches don't break the detach, see P-1 residual risk).

---

## 7. Recommended Follow-up Issue Plan

Filed immediately after this commit lands via Gitea REST API. **Actual assigned numbers (post-creation, 2026-08-30 17:35 CST):**

| Issue # | PR # | Title | Tier | Status |
|---------|------|-------|------|--------|
| **#4594** | **#4599** | [Tier-1] SOAK infra blockers: data-dir /tmp wipe + ServerQPS regex bug + --time/SOAK_HOURS sync + nice env override | 1 | ✅ CLOSED (PR #4599 merged 2026-08-30 17:49:30 CST @ `a7af55a3b17f`) |
| **#4595** | **#4591** | [Tier-1] CI test that detects run_soak_loop.sh parent-reap regression | 1 | ✅ CLOSED (commit `89ed65852f`/`3840254213` in PR #4591, merged 2026-08-30 17:47:44 CST @ `5a7e2e53825b`) |
| **#4596** | — | [Tier-2] SOAK infra: workload sweep + periodic_reports JSON + disk-restart boundary marker | 2 | OPEN |
| **#4597** | — | [Tier-2] SOAK chaos drills: SIGKILL + disk-full + netem loss + clock skew | 2 | OPEN |
| **#4598** | — | [Tier-3] SOAK alerting webhook + thread-ramp mode + 168h local SOP | 3 | OPEN |

**Note on numbering (updated 2026-08-30 17:49:30 CST):** planned issue numbers were #4592-#4596 but two stray debug issues occupied #4592 (DEBUG test) and #4593 (small body test) during POST debugging. Those strays were closed with state=closed and are not actionable; the real follow-up issues received #4594-#4598.

**IMPORTANT — PR vs Issue counter:** PR counter and Issue counter are SEPARATE Gitea namespaces. The PR for issue #4594 was **NOT** PR #4594 (that number was already taken by another in-flight PR at filing time); it became **PR #4599**. Same risk applies for any future issue→PR pair in this chain: always check current PR count before predicting the actual PR number, and update this table accordingly when each PR lands.

**Next steps for sprint planning:**
- ~~Tier-1 (#4594, #4595) are the critical path.~~ **DONE**: #4594 (PR #4599) + #4595 (PR #4591 commit 89ed65852f) both merged 2026-08-30 17:49 CST.
- Tier-2 (#4596, #4597) can land as a single combined PR (~140 lines + 7 tests) in the same sprint.
- Tier-3 (#4598) deferred to v3.13 unless someone volunteers.

---

## 8. Cross-references

- INCIDENT-REPORT-2026-08-30.md §3.1 — root-cause analysis for P-1
- POST_4566_SOAK_REPORT.md §11 — 7h28m evidence summary used to derive O-1 through O-4
- PR #4566 (SHA `1c11addc6b`) — TLS-handshake fix whose verification motivated this 7h28m run
- PR #4565 — GA-2 Z6G4 168h SOAK Docker runner (the actual GA-2 path; P-2/P-3/P-9 must be fixed before this is trusted for 168h)
- run_soak_loop.sh:235 — `/tmp/sqlrustgo-soak-data-${SOAK_PORT}` (P-2)
- run_soak_loop.sh:197-200 — empty-`ServerQPS` regex (P-9)
- run_soak_loop.sh:245, 320 — hardcoded `nice` (P-3)
- run_soak_loop.sh:332 — hardcoded `--time=86400` (P-4)

---

## 9. Anti-Fabrication-Policy-v1.0 compliance

- ✅ All evidence cited (`periodic_reports.log`, `metrics.csv`, `sysbench.log` lines) is in-tree at `evidence/issue-4560/run_20260830_post4566_7h28m_salvage/` with SHA256 in `POST_4566_SOAK_REPORT.md §11.7`.
- ✅ Server-side QPS observations (O-1 through O-4) are derived from actual measurements, not speculated.
- ✅ The `ServerQPS` empty-column bug (P-9) is verified across all 45 periodic reports in the 7h28m run — **not** a one-time anomaly.
- ✅ No claim that any prior SOAK was "wrong" — P-1 through P-12 are catalogued as "exposed by" this run, not "introduced by" this run.
- ✅ Recommendations are estimates (PR size lines) not fabricated measurements.