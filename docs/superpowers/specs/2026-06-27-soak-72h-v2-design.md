# 72h SOAK v2 Design (Issue #3265)

> **Date**: 2026-06-27
> **Issue**: #3265 — [GA-P0/S3] Execute 72h long-running soak
> **Author**: claude-macmini (brainstorming skill output)
> **Status**: APPROVED by user (2026-06-27)
> **Supersedes**: `2026-06-27-soak-72h-design.md` (v1, abandoned due to ineffective load)
> **Server**: Z6G4 (192.168.0.252)

---

## 1. Goal + Scope

### 1.1 Goal
Run **72-hour real wall-clock SOAK** with **real load** (standard sysbench oltp_read_write + TPC-H 22 rotation) on Z6G4 to surface slow leaks and verify WAL/thread/FD stability per Issue #3265.

### 1.2 Why v2 (vs v1)
v1 (commit 0f72f5761) used `run_wired_soak.sh` with `SKIP_SYSBENCH=1` to bypass Issue #3575. **This resulted in 0 effective load**:
- sysbench SKIPPED (Issue #3575 bulk_insert failure)
- TPC-H fixture failed to load (tpch-sf001 dummy data, INSERT mode failed, LOAD DATA mode failed)
- TPC-H rotation couldn't connect (mysql CLI TLS negotiation failed)

**v2 fixes the root causes** by patching sqlrustgo to handle:
1. multi-VALUES INSERT (sysbench bulk_insert)
2. mysql CLI TLS handshake (TPC-H rotation client)

### 1.3 Success Criteria (from Issue #3265)
- WAL file count: stabilized after checkpoint (not growing unboundedly)
- Memory: plateaued within 4-6 hours, not growing linearly
- Thread count: stabilized (no thread leaks)
- File descriptors: stabilized (no fd leaks)
- **TPS/QPS measured** (sysbench report-interval output)

### 1.4 Phased Approach (3 stages)
| Stage | Duration | Purpose | Exit Criteria |
|-------|----------|---------|---------------|
| Stage 1 | 6h | smoke: real load verification | RSS < 500 MB, WAL < 1 GB, FD < 100, sysbench QPS > 10, TPC-H QPS > 1 |
| Stage 2 | 24h | extended: memory plateau | RSS < 1 GB, WAL truncate every ≤ 60s |
| Stage 3 | 72h | full: long-term stability | RSS < 1 GB, WAL < 100 MB, FD < 200 |

### 1.5 Out of Scope
- ❌ v3.10 features (Issue #3575 v3.10 milestone)
- ❌ `v390_join_reorder` feature flag (default OFF)
- ❌ Modifying sysbench or mysql CLI client code
- ❌ Modifying TPC-H 22 query content

---

## 2. Prerequisites + Root Cause

### 2.1 Root Cause Analysis

| Bug | Location | Cause |
|-----|----------|-------|
| **sysbench bulk_insert** | `crates/executor/src/sql_executor.rs` or `crates/parser/src/parser.rs` | sqlrustgo fails on multi-VALUES INSERT (single query with ~5000 rows, ~30KB); connection drops, server stays alive (no panic) |
| **mysql CLI TLS** | `crates/mysql-server/src/lib.rs` | sqlrustgo defaults to TLS self-signed; mysql CLI doesn't send TLS handshake by default → "cannot reach server" |
| **TPC-H fixture** | `scripts/stability/load_tpch_fixture*.sh` | tpch-sf001 = 168KB dummy data; INSERT mode fails; LOAD DATA mode fails |

### 2.2 Code Status (verified)
- ✅ Issue #3531 WAL checkpoint fix (commit `83c840c46`) in develop/v3.9.0
- ✅ PR #3608 --server-threads CLI + ServerThreadPool merged
- ✅ PR #3617 SKIP_SYSBENCH option (no longer needed in v2)
- ✅ PR #3622 actually applied SKIP_SYSBENCH (superseded by v2)
- ✅ GRUB fix (mce=0/hest_disable removed)

### 2.3 Server Health (Z6G4 @ 2026-06-27)
- Uptime: stable since 2026-06-27 00:52
- Memory: 404 GB total / 393 GB available
- Disk: 728 GB available
- CPU: 80 cores

---

## 3. Architecture + Components

### 3.1 Architecture Diagram

```
sqlrustgo-mysql-server (修复后)
├─ Wire Protocol (crates/mysql-server/src/lib.rs)
│  ├─ TLS 协商: 接受 mysql CLI 连接 (--ssl-mode=DISABLED 支持)
│  └─ Query 接收: 处理任意长度 multi-VALUES INSERT
│
├─ Parser (crates/parser/src/parser.rs)
│  └─ INSERT statement: 接受 N 行 VALUES (N 任意)
│
├─ Executor (crates/executor/src/sql_executor.rs)
│  └─ INSERT execution: 单 query 处理所有 rows (无 buffer 限制)
│
└─ Storage (crates/storage/src/wal_storage.rs)
   └─ WAL checkpoint (PR #3533 已就位)

SOAK Driver (scripts/stability/run_soak_staged.sh)
├─ Stage 1 (6h) → Stage 2 (24h) → Stage 3 (72h)
├─ sysbench oltp_read_write (16 threads, TABLE_SIZE=10000)
└─ TPC-H 22 rotation (interval=600s, tpch-sf01 dataset)
```

### 3.2 Components

**Reused**:
- `scripts/stability/run_wired_soak.sh` (modified: remove SKIP_SYSBENCH branch since we fixed root cause)
- `scripts/stability/tpch_22_rotate.sh` (mysql CLI client — works after TLS fix)
- `scripts/stability/run_soak_staged.sh` (stage-gate orchestrator, v1)
- `tests/data/tpch-sf01/` (3.6MB real TPC-H data)

**Modified**:
- `crates/executor/src/sql_executor.rs` (multi-VALUES INSERT)
- `crates/parser/src/parser.rs` (multi-VALUES INSERT parsing)
- `crates/mysql-server/src/lib.rs` (mysql CLI TLS)
- `scripts/stability/load_tpch_fixture*.sh` (use tpch-sf01)
- `scripts/stability/run_wired_soak.sh` (remove SKIP_SYSBENCH)

**New**:
- `crates/executor/tests/bulk_insert_regression_test.rs` (prevent regression)
- `crates/mysql-server/tests/tls_handshake_test.rs` (mysql CLI TLS)

---

## 4. Data Flow + Auto-Verification

### 4.1 Phase 0: Pre-SOAK
1. Rebuild sqlrustgo-mysql-server (cargo build --release -p sqlrustgo-mysql-server)
2. Start server (--server-threads 16, port 3396)
3. Load TPC-H fixture (LOAD DATA mode + tpch-sf01)
4. sysbench oltp_read_write prepare (TABLE_SIZE=10000)
5. Verify all 3 connections work

### 4.2 Phase 1-3: SOAK Execution
```
sysbench 16 threads ─┐
                     ├─→ sqlrustgo ─→ WAL checkpoint (PR #3533)
TPC-H rotation (mysql CLI) ─┘
                            ↓
                       metrics sampler (60s)
                            ↓
                       metrics.csv + incidents.log
```

### 4.3 Auto-Verification (per 60s sample)

```bash
# Auto-verify checks
1. Server alive: kill -0 $SERVER_PID
2. WAL size: < 1024 MB (Stage 1) / 1024 MB (Stage 2) / 100 MB (Stage 3)
3. sysbench alive: kill -0 $SYSBENCH_PID + QPS > 0 (last 60s)
4. TPC-H rotation alive: kill -0 $TPCH_ROTATE_PID + last_query_log_age < INTERVAL * 2
5. RSS absolute: < 1024 MB (or alert threshold)
6. FD count: stable ±10% over last 5 samples
```

### 4.4 Auto-Recovery

| Failure | Action |
|---------|--------|
| sysbench crash | auto-restart 1 attempt + log incident |
| TPC-H rotation crash | auto-restart 1 attempt + log incident |
| Server crash | log fatal + abort phase (no auto-restart) |
| Fixture load failure | **FATAL** — abort phase immediately |

### 4.5 Incident Logging

```
$RESULTS_DIR/$PHASE/incidents.log

Format: [TIMESTAMP] [LEVEL] [TYPE] message
[2026-06-27 09:30:00] [WARN] [SYSBENCH_QPS] QPS=0 for 3 samples — restarting
[2026-06-27 10:15:00] [INFO] [AUTO_RECOVER] sysbench restarted, PID=12345
[2026-06-27 11:00:00] [FATAL] [SERVER_DEAD] Server crashed, exit code 134
```

---

## 5. Error Handling

| Failure Mode | Handling |
|--------------|----------|
| Fixture load fail | **FATAL** (was: continuing in v1) |
| sysbench prepare fail | **FATAL** (was: continuing in v1) |
| sysbench run crash | Auto-restart 1x + log |
| TPC-H rotation crash | Auto-restart 1x + log |
| TPC-H Q10-Q22 timeout | log + skip (not fatal) |
| Server crash | FATAL + abort phase |
| RSS > threshold | warn + alert |
| Disk > 95% | abort + cleanup |
| Server power loss | Auto-recovery + continue clock |

---

## 6. Testing + Verification

### 6.1 Pre-Launch (ALL must PASS)
- `cargo check --all-features`: PASS
- `cargo fmt --check --all`: PASS
- `cargo test --workspace --no-run`: PASS
- `cargo clippy --all-features -- -D warnings`: PASS
- **NEW** `cargo test bulk_insert`: PASS (regression test)
- **NEW** `cargo test mysql_cli_tls`: PASS (TLS handshake test)
- **NEW** `cargo test sysbench_e2e`: PASS (full sysbench oltp_read_write prepare + run)

### 6.2 Per-Stage Monitoring (every 60s)
- Server alive, RSS, FD, Threads
- WAL size, checkpoint count
- sysbench QPS (10s report-interval)
- TPC-H query latency (Q1-Q22, 600s round)
- OOM events (journalctl | grep -i oom)
- Hardware events (journalctl | grep -i hardware)

### 6.3 Post-Completion Report
- `SOAK_72H_REPORT.md` per Issue #3265 spec
- Per-stage metrics summary
- TPS/QPS analysis
- Issue #3265 comment with report URL

### 6.4 Success Gates
- Stage 1 → Stage 2 only if exit criteria met (incl. sysbench QPS > 10)
- Stage 2 → Stage 3 only if exit criteria met
- Stage 3 → Issue #3265 close only if all 4 success criteria met

---

## 7. References

- Issue #3265 — [GA-P0/S3] 72h SOAK
- Issue #3531 — WAL grows unbounded (CLOSED, PR #3533)
- Issue #3575 — sysbench prepared stmt (CLOSED, deferred to v3.10 ADR-013)
- ADR-013 — v3.10 wire protocol repair (PR #3574)
- PR #3608 — --server-threads CLI + ServerThreadPool (merged)
- PR #3614 — SOAK design + plan + scripts (merged)
- PR #3617, #3622 — SKIP_SYSBENCH workaround (merged, superseded by v2)
- v1 design: `docs/superpowers/specs/2026-06-27-soak-72h-design.md` (superseded)
- Server health: `docs/audit/issues/ISSUE-3577_z6g4_power_loss_2026-06-22.md`
- Baseline: `docs/audit/status/SOAK_1H_BASELINE_SMOKE_2026-06-26.md`
- pymysql fallback: `scripts/stability/pymysql_soak.py` (no longer primary)

---

## 8. Open Questions

None at design time. All blockers identified with specific fix paths.

---

## 9. Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-06-27 | Fix root cause (not workaround) | User choice: 修复 Issue #3575 + 标准 sysbench |
| 2026-06-27 | Use tpch-sf01 (not tpch-sf001) | sf001 = 168KB dummy; sf01 = 3.6MB real data |
| 2026-06-27 | Add auto-verification (every 60s) | User feedback: avoid silent failures |
| 2026-06-27 | Fail-fast on fixture load | Was "continuing" in v1, caused 0-load SOAK |
| 2026-06-27 | Remove SKIP_SYSBENCH from v2 | After fix, sysbench should work; no workaround needed |
| 2026-06-27 | mysql CLI TLS handshake | sqlrustgo currently breaks mysql CLI connections |
| 2026-06-27 | Multi-VALUES INSERT support | Required for sysbench bulk_insert |

---

**Status**: APPROVED → commit to git → proceed to writing-plans skill