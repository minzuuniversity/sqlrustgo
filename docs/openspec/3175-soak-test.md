# openspec/3175 - P1-3 Soak Test (WIRED E2E)

> **Issue**: #3175
> **作者**: Hermes Agent
> **日期**: 2026-06-21 (rewrite)
> **Phase**: 4 (W7-8)
> **工作量**: 48h (per V390_DEVELOPMENT_PLAN)
> **状态**: G7 gate requires wired E2E (sqlrustgo-mysql-server + sysbench). In-process simulation decommissioned 2026-06-21.

## 一、问题分析

### 1.1 历史问题 (2026-06-21 audit)

The previous G7 Soak Test gate (`scripts/gate/check_p13_soak_test.sh`) only ran
in-process Rust unit tests on a `MemoryExecutionEngine` with **1,440x
compressed-time equivalence**:

- 24h → 60s of in-process queries
- 72h → 180s of in-process queries
- 168h → 420s of in-process queries

This compressed-time approach had three structural problems that the
2026-06-17 truthfulness audit (see `docs/audit/status/2026-06-17-truthfulness-current-state.md`)
flagged as a "PARTIALLY UNTRUSTED" claim:

1. **Wire protocol was not exercised.** The harness used a Rust API
   directly; no `mysql`/`mariadb` client ever connected to a real server.
2. **Buffer pool, connection manager, lock manager, WAL fsync,
   catalog caching, MVCC, prepared-statement cache, TLS handshake,
   and error-path recovery were not exercised.** The MemoryEngine
   is single-threaded, single-connection, in-memory.
3. **"Equivalence" is unfalsifiable.** There is no metric that ties
   "100 in-process queries at 5 q/s" to "100 wire queries at 5 q/s".
   The 1,440x factor was a label, not a proof.

The fix: **G7 must launch the real `sqlrustgo-mysql-server` binary,
hit it with the industry-standard `sysbench oltp_read_write` workload
over the MySQL wire protocol, and parse a real `STABILITY_REPORT.md`.**

### 1.2 G7 Soak Test 3 levels (per ChatGPT review #三)

| Level | Duration | Trigger | E2E form |
|-------|----------|---------|----------|
| 24h Soak | 24h | CI before each release | wired: `HOURS=24 bash scripts/stability/run_wired_soak.sh` |
| 72h Soak | 72h | RC stage | wired: `HOURS=72 bash scripts/stability/run_wired_soak.sh` |
| 168h Soak | 168h (1 week) | GA gate | wired: `HOURS=168 bash scripts/stability/run_wired_soak.sh` |

### 1.3 Gate (CI default: 5 min)

`scripts/gate/check_p13_soak_test.sh` runs a 5-minute (default;
`SOAK_MINUTES=5`) wired soak as the G7 gate. The gate produces a real
`STABILITY_REPORT.md` and enforces:

| Criterion | Threshold |
|-----------|-----------|
| Crashes | 0 |
| sysbench errors (FATAL/ERROR) | 0 |
| Max RSS | ≤ `SOAK_RSS_HARD_MB` (default 6144 MB) |
| RSS growth (scaled from 24h) | ≤ `SOAK_MINUTES * 200 MB` |
| FD growth (scaled from 24h) | ≤ `SOAK_MINUTES * 5` |
| Resource alerts | 0 |
| STABILITY_REPORT.md present | required |

Gate exit codes:
- 0 = PASS
- 1 = FAIL (server crashed / sysbench errors / RSS over hard cap / report missing)
- 2 = SKIPPED (no sysbench, no binary, or `SOAK_SKIP=1`)

### 1.4 Monitoring metrics (per #3175)

| Metric | Threshold | Source |
|--------|-----------|--------|
| Memory usage (RSS MB) | hard cap | `ps -p $PID -o rss=` |
| File descriptor count | `SOAK_FD_LIMIT` (default 512) | `lsof -p $PID` |
| WAL size | `< 10 GB` end-of-run | `du -sm $DATA_DIR` |
| Query P99 latency | sysbench-reported | `sysbench.log` |
| sysbench errors | 0 | `grep FATAL\|ERROR sysbench.log` |

## 二、实施方案

### 2.1 范围限定

按治理 §2.1 最小修改 + 复用现有 stability scripts:

**本次 PR 范围 (3 大块)**:

1. **重写** `scripts/gate/check_p13_soak_test.sh` 为 wired E2E 门禁 (本 SPEC 重点)
2. **降级** `tests/soak_test.rs` 10 个 `#[test]` 全部 `#[ignore]`, in-process 模拟不再用于 gate
3. **更新** 文档: 本 SPEC + `V390_TEST_PLAN.md §G7` + `SOAK_72H_REPORT.md` 头部

**延后 (推 v3.10+)**:
- 真实 24h/72h/168h 持续测试 (需 CI scheduled runner, Z6G4 hardware)
- Prometheus/Grafana dashboard 集成
- 多节点 soak (cluster scenario)
- 自动 Soak 终止策略 (当 P99 退化超阈值)

### 2.2 Gate 设计 (wired E2E)

The gate `check_p13_soak_test.sh` is a self-contained script that:

1. Pre-flight: check `sysbench` in PATH, `sqlrustgo-mysql-server` built.
   If either missing, exit 2 (SKIPPED).
2. Pick a free port in 3396-3496 (override via `SOAK_PORT`).
3. Launch the real binary:
   ```bash
   ( ulimit -v $((SOAK_RSS_HARD_MB * 1024)); ulimit -n $SOAK_FD_LIMIT;
     exec ./target/release/sqlrustgo-mysql-server serve
       --host $SOAK_HOST --port $SOAK_PORT
       --data-dir $DATA_DIR
       --log-level info ) &
   ```
4. Wire sanity: `mysql --ssl=0 -e 'SELECT 1'`. Catches TLS-handshake
   regressions in 1 round-trip.
5. `sysbench oltp_read_write ... prepare` (20 rows, threshold to avoid
   server TLS flush race, see known issues).
6. `sysbench oltp_read_write ... run` for `SOAK_SECS` (= `SOAK_MINUTES * 60`).
7. Monitor loop every `SOAK_INTERVAL` seconds: RSS, FD, CPU, WAL.
8. After sysbench finishes, kill server, write `STABILITY_REPORT.md`.
9. Enforce 7 hard criteria; exit 0/1.

### 2.3 Three 等级 Wired Soak Scripts (long-run)

| Level | Real duration | Script | Trigger |
|-------|---------------|--------|---------|
| 24h | 86400s | `HOURS=24 bash scripts/stability/run_wired_soak.sh` | RC pre-release |
| 72h | 259200s | `HOURS=72 bash scripts/stability/run_wired_soak.sh` | GA candidate |
| 168h | 604800s | `HOURS=168 bash scripts/stability/run_wired_soak.sh` | GA-final |

These wrap the same `sqlrustgo-mysql-server` + `sysbench oltp_read_write`
+ `tpch_22_rotate` + `monitor_server.sh` shape as the gate, with
longer duration, larger tables, and TPC-H rotation in parallel.

### 2.4 已知问题 / Limitations

- **Server TLS flush race**: sqlrustgo's `TlsStream::flush_pending()`
  in `crates/mysql-server/src/lib.rs` can stall on the first
  multi-batch INSERT after TLS upgrade. The gate uses `table_size=20`
  to stay below the threshold. Tracked separately.
- **macOS /proc absent**: gate uses `ps` + `lsof` for cross-platform
  metrics (works on Linux + macOS).
- **sysbench 1.0.20** uses `libmariadb` (not `libmysqlclient`);
  `MYSQL_OPT_SSL_MODE` is ignored. The server advertises SSL
  capability, so sysbench upgrades opportunistically. Wire sanity
  uses `mysql --ssl=0` to bypass.

## 三、风险评估

| 风险 | 影响 | 缓解 |
|------|------|------|
| 5-min gate 慢 | CI 超时 | 5 min 默认值; SOAK_MINUTES=1 用于本地快速验证 |
| 资源耗尽 | 跑飞 host | ulimit -v (RSS hard cap) + ulimit -n (FD limit) + 资源监控告警 |
| sysbench 不在 PATH | 误报 FAIL | 显式 SKIP (exit 2) 而不是 FAIL |
| 真实跑 24h+ 需要 Z6G4 | CI 不可达 | 5-min gate 在 CI 跑; 24h+ 用 dedicated host + cron |
| TLS flush race 触发 | 误报 FAIL | 显式 hint 提示用户降低 table_size 或修 server |

## 四、验收标准 (G7 门禁)

```
✅ 5-min 默认 wired soak: 0 crash, 0 sysbench errors, RSS < hard cap
✅ sysbench 实际跑了 ≥10 个 transactions (E2E 证据)
✅ STABILITY_REPORT.md 完整生成 (含 acceptance, sysbench, artifacts 章节)
✅ Wire sanity SELECT 1 通过
✅ 资源 limits: max RSS ≤ SOAK_RSS_HARD_MB, FD ≤ SOAK_FD_LIMIT
✅ 871 L1 tests 不回归 (1555 当前)
✅ TPC-H 22/22 (G1 维持)
```

## 五、Subsumed Issues

- #3175 本身 (本任务)
- 与 P1-4 Upgrade Test (#3176) 互补 (24h 升级 + 重启稳定性)
- 解决 2026-06-17 audit 的 "G7 simulated" finding

## 六、回滚计划

如 wired gate 编译失败或不可运行:

1. `SOAK_SKIP=1 bash scripts/gate/check_p13_soak_test.sh` → exit 2 (skip)
2. In-process `tests/soak_test.rs` 仍可手动 `cargo test -- --ignored` 跑
3. Long-run wired scripts (`run_wired_soak.sh`) 不依赖 gate, 可独立使用

## 七、依赖

**上游**: P1-2 Crash Test (借力 harness 设计)
**下游**: P1-4 Upgrade Test (复用 monitoring)

## 八、参考资料

- Issue #3175
- V390_DEVELOPMENT_PLAN.md §P1-3
- V390_TEST_PLAN.md §G7
- `scripts/gate/check_p13_soak_test.sh` (wired E2E gate, current)
- `scripts/stability/run_wired_soak.sh` (24h+ long-run shape)
- `crates/mysql-server/src/main.rs` (server binary)
- `crates/mysql-server/src/lib.rs:TlsStream` (known flush race, see #issue)
- `tests/soak_test.rs` (in-process harness, all `#[ignore]`)
- `tests/soak_test_harness.rs` (shared harness; 5 self-tests)
- `docs/audit/status/2026-06-17-truthfulness-current-state.md` (audit that triggered this rewrite)
- ADR-013 (v310-wired-soak-ddl-and-wire-protocol-repair)
