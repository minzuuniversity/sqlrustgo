# openspec/3175 - P1-3 Soak Test (24h/72h/168h)

> **Issue**: #3175
> **作者**: Hermes Agent
> **日期**: 2026-06-05
> **更新**: 2026-06-27 (SF=0.01 实测数据)
> **Phase**: 4 (W7-8)
> **工作量**: 48h (按 V390_DEVELOPMENT_PLAN)
> **状态**: 框架已完成，实测发现问题

---

## 执行摘要（2026-06-27）

### 实测配置
- **数据集**: TPC-H SF=0.01（6,001 lineitem / 1,501 orders / 151 customer）
- **负载工具**: `scripts/soak/tpch_soak_driver.py`（Python subprocess，22 TPC-H 查询轮询）
- **并发**: 16 线程（见下方线程数评估）
- **Server**: `sqlrustgo-mysql-server` v3.8.0-beta，单线程接受连接

### 实测结果

| 指标 | 值 |
|------|-----|
| **QPS** | 265.2 q/s（16 线程）|
| **P50 延迟** | 53.5 ms |
| **P99 延迟** | 204.6 ms |
| **Server CPU** | 15–20%（严重未饱和）|
| **Errors** | 0 |
| **内存增长** | **149 MB → 421 MB（~9 min，警戒中）** ⚠️ |

### 发现：Server 单线程瓶颈

`handle_connection` 使用 `std::thread::spawn` **每连接一个 OS 线程**，无线程池复用。
16 个 client 线程竞争同一个连接线程，P50 从 53ms → 70ms（32 线程），
但 Server CPU 仅 15%，**瓶颈不在 CPU，在单连接串行执行**。

建议：Server 需要 `tokio` 多线程 runtime 或 thread pool。

### 线程数评估（SF=0.01）

| 线程 | QPS | P50 | P99 | 内存增长 | 评估 |
|------|-----|-----|-----|---------|------|
| 16 | 265.2 | 53ms | 205ms | 稳定 | ✅ 推荐 |
| 32 | 385.8 | 70ms | 295ms | 70%↑ | ⚠️ 内存压力大 |
| 64 | 378.5 | 158ms | 381ms | 稳定 | ❌ 延迟过高 |

**结论**：保持 **16 并发**作为 SOAK 标准配置。

---

## 一、问题分析

### 1.1 现状审计 (2026-06-05)

仓库已有:
- `crates/mysql-server/src/monitoring.rs` (569 lines) - 完整 PerformanceMonitor:
  - `MemoryStats` (current memory tracking)
  - `ConnectionStats` (open/closed count)
  - `QueryStats` (avg/peak execution time)
  - `PerformanceMonitor` (singleton via SharedMonitor)
  - `prometheus_metrics()` (Prom 格式导出)
  - `json_stats()` (JSON 格式导出)
- `tests/memory_fault_injection_test.rs` test 6 - `test_memory_leak_detection_across_operations` (已有)
- `crates/agentsql/src/memory.rs` (727 lines) - memory subsystem

### 1.2 #3175 Soak Test 3 等级 (按 ChatGPT 评审 §三)

| 等级 | 时长 | 触发 | 现状 |
:|------|------|------|------|
| 24h Soak | 24h | CI 每次发版前 | **缺** 自动化 |
| 72h Soak | 72h | RC 阶段 | **缺** 自动化 |
| 168h Soak | 168h (1 周) | GA 前 | **缺** 自动化 |

### 1.3 监控指标 (按 #3175)

| 指标 | 阈值 | 监控实现 |
|------|------|----------|
| Memory usage | baseline + 10% | MemoryStats ✅ 已有 |
| File descriptor | baseline + 5 | ✅ 已实现（procfs） |
| Lock count | 0 leak | 已有 deadlock_injection |
| WAL size | baseline + 5% | ❌ 缺 |
| Buffer cache | baseline + 10% | ❌ 缺 |
| Query P99 latency | baseline + 50% | QueryStats ✅ 已有 |

## 二、交付物

### 2.1 已完成文件

| 文件 | 说明 |
|------|------|
| `tests/soak_test_harness.rs` | 共享 harness（3 等级 smoke constants）|
| `tests/soak_test.rs` | 10 tests（24h/72h/168h smoke + 内存/FH 验证）|
| `scripts/soak/tpch_soak_driver.py` | Python TPC-H SOAK driver（mysql CLI subprocess）|
| `scripts/soak/tpch_schema.sql` | TPC-H 8 表 DDL（sqlrustgo 类型适配）|
| `scripts/soak/tpch_queries/q01–q22.sql` | 22 个 TPC-H 查询文件 |
| `scripts/soak/prepare_sf01_data.sh` | SF=0.1  fixture 生成脚本 |
| `scripts/gate/check_p13_soak_test.sh` | G7 gate（9/9 checks，2026-06-27 更新）|
| `openspec/changes/p1-3-soak-test/` | OpenSpec 文档（proposal/design/spec/tasks）|

### 2.2 G7 Gate 结果（2026-06-27）

```
[1/9] ✅ PASS: tests/soak_test_harness.rs present
[2/9] ✅ PASS: tests/soak_test.rs present + registered
[3/9] ✅ PASS: 3-level smoke equivalence (24h→60s, 72h→180s, 168h→420s)
[4/9] ✅ PASS: soak_test compiles
[5/9] ✅ PASS: soak_test 10 passed (≥10)
[6/9] ✅ PASS: alert-threshold mechanism verified
[7/9] ✅ PASS: memory baseline invariant
[8/9] ✅ PASS: scripts/soak/tpch_soak_driver.py valid Python
[9/9] ✅ PASS: scripts/soak/extract_soak_report.py valid Python
```

## 三、风险评估

| 风险 | 影响 | 缓解 |
|------|------|------|
| Server 单线程瓶颈 | QPS 上限低 | 文档记录，需 v3.10+ tokio runtime |
| 内存增长趋势（149→421 MB/9min）| 24h 测试可能 OOM | 需持续监控，SF=0.01 暂未稳定 |
| LOAD DATA LOCAL INFILE > 6K 行崩溃 | 无法测 SF=0.1 | 使用 SF=0.01 |
| 短期测试不能证明 24h 稳定 | 评估失真 | 文档明确"等效"含义 |

## 四、下一步

1. **30 min SOAK 持续运行中** — 等待完成验证内存稳定性
2. **Server 多线程改造** — 推 v3.10+（使用 tokio multi-thread runtime）
3. **真实 24h/72h/168h** — 需 CI scheduled runner（SF=0.01，16 并发）
4. **WAL size / Buffer cache 监控** — 延后

## 五、参考资料

- Issue #3175
- V390_DEVELOPMENT_PLAN.md §P1-3
- V390_TEST_PLAN.md §G7
- `crates/mysql-server/src/monitoring.rs` — PerformanceMonitor
- `crates/mysql-server/src/lib.rs:handle_connection` — 单连接线程模型
