# openspec/3175 - P1-3 Soak Test (24h/72h/168h)

> **Issue**: #3175
> **作者**: Hermes Agent
> **日期**: 2026-06-05
> **Phase**: 4 (W7-8)
> **工作量**: 48h (按 V390_DEVELOPMENT_PLAN)
> **状态**: 部分已存在 (monitoring.rs 569 lines + memory leak test), 本次做 Soak 框架 + G7 gate

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
|------|------|------|------|
| 24h Soak | 24h | CI 每次发版前 | **缺** 自动化 |
| 72h Soak | 72h | RC 阶段 | **缺** 自动化 |
| 168h Soak | 168h (1 周) | GA 前 | **缺** 自动化 |

### 1.3 监控指标 (按 #3175)

| 指标 | 阈值 | 监控实现 |
|------|------|----------|
| Memory usage | baseline + 10% | MemoryStats ✅ 已有 |
| File descriptor | baseline + 5 | ❌ 缺 |
| Lock count | 0 leak | 已有 deadlock_injection |
| WAL size | baseline + 5% | ❌ 缺 |
| Buffer cache | baseline + 10% | ❌ 缺 |
| Query P99 latency | baseline + 50% | QueryStats ✅ 已有 |

### 1.4 P1-3 任务真正需要做的 (按治理最小修改)

**A. Soak Test Framework** (新):
- 持续负载生成器 (queries/inserts/updates/deletes 循环)
- 资源采样器 (memory + FD + lock + WAL)
- 告警阈值定义 + 比较

**B. 短期 Soak 代理** (新):
- 24h 等效压缩: 高负载 1 分钟 (5 queries/s × 60 = 300 queries, 等效 24h 工作量)
- 资源采样: 每 5 秒
- 报告生成: baseline + final diff

**C. G7 Gate** (新):
- 7 项检查 (类似 G5/G8)

**D. 缺失监控**:
- FD count sampling (借用 nix crate 或 std)
- WAL size tracking (borrowing)

## 二、实施方案

### 2.1 范围限定

按治理 §2.1 最小修改 + 复用现有 monitoring 基础:

**本次 PR 范围 (4 大块)**:

1. **新文件**: `tests/soak_test_harness.rs` (shared harness)
2. **新文件**: `tests/soak_test.rs` (3 等级 smoke tests)
3. **新文件**: `scripts/gate/check_p13_soak_test.sh` (G7 gate)
4. **新文件**: `docs/openspec/3175-soak-test.md` (本文件)

**延后 (推 v3.10+)**:
- 真实 24h/72h/168h 持续测试 (需 CI scheduled runner)
- Prometheus/Grafana dashboard 集成
- 多节点 soak (cluster scenario)
- OOM 时的自动 Soak 终止策略

### 2.2 Soak Test Harness 设计

```rust
// tests/soak_test_harness.rs (shared)
pub struct SoakConfig {
    pub duration_seconds: u64,     // 短期 = 60s, 24h = 86400s
    pub queries_per_second: u32,    // 负载率
    pub memory_baseline_bytes: u64, // 启动 baseline
    pub fd_baseline: u32,           // 启动 FD 数
    pub memory_alert_threshold_pct: u32, // 默认 10
    pub fd_alert_threshold: u32,    // 默认 +5
}

pub struct SoakReport {
    pub duration_seconds: u64,
    pub queries_executed: u64,
    pub memory_baseline_bytes: u64,
    pub memory_final_bytes: u64,
    pub memory_growth_pct: f64,    // baseline → final
    pub fd_baseline: u32,
    pub fd_final: u32,
    pub fd_growth: i32,
    pub p50_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub alert_triggered: bool,
}

pub fn run_soak(config: SoakConfig) -> SoakReport;
```

### 2.3 3 等级 Smoke Tests

| # | Test | 时长 | 等效 |
|---|------|------|------|
| 1 | test_soak_24h_smoke_60s | 60s | 24h |
| 2 | test_soak_72h_smoke_180s | 180s | 72h |
| 3 | test_soak_168h_smoke_420s | 420s | 168h |

每测试负载 5 q/s, 断言:
- 内存增长 < 10%
- FD 增长 < 5
- P99 < 100ms baseline
- 0 leak (queries_executed 匹配预期)

### 2.4 G7 Gate (7 checks)

1. soak_test_harness.rs 存在
2. soak_test.rs 存在
3. Cargo.toml 注册 2 test targets
4. cargo check pass
5. 3 等级 tests pass
6. 监控 baseline+final 资源采样有效 (MemoryStats 集成)
7. 报告生成正确 (含 alert_triggered 字段)

## 三、风险评估

| 风险 | 影响 | 缓解 |
|------|------|------|
| 60s 测试慢 | CI 超时 | 5 q/s rate 限制, runtime ~60-65s |
| 内存监测不准 | 误报 | 启动后 5s 稳定期 + baseline 采集 |
| 短期不能证明 24h 稳定 | 评估失真 | 文档明确"等效"含义,真 24h 推 v3.10+ |
| P99 采样噪声 | 误判 | 50+ query 滑动窗口,丢弃启动 10 query |

## 四、验收标准 (G7 门禁)

```
✅ 3 等级 Soak tests (60s/180s/420s) PASS
✅ memory growth < 10% (baseline 验证)
✅ FD growth < 5
✅ queries_executed = 预期
✅ MemoryStats 集成 (PerformanceMonitor)
✅ G7 gate: 7/7 PASS
✅ 871 L1 tests 不回归 (1555 当前)
✅ TPC-H 22/22 (G1 维持)
```

## 五、Subsumed Issues

- #3175 本身 (本任务)
- 与 P1-4 Upgrade Test (#3176) 互补 (24h 升级 + 重启稳定性)

## 六、回滚计划

如 soak_test 编译失败:
1. 删除 `tests/soak_test*.rs`
2. G7 gate 标记 DEFER
3. 监控基础设施不动 (monitoring.rs 已存在)

## 七、依赖

**上游**: P1-2 Crash Test (借力 harness 设计)
**下游**: P1-4 Upgrade Test (复用 monitoring)

## 八、参考资料

- Issue #3175
- V390_DEVELOPMENT_PLAN.md §P1-3
- V390_TEST_PLAN.md §G7
- crates/mysql-server/src/monitoring.rs (569 lines, PerformanceMonitor)
- tests/memory_fault_injection_test.rs (test_memory_leak_detection_across_operations)
- crates/agentsql/src/memory.rs (727 lines, memory subsystem)
- P1-2 #3174 crash_test_harness (设计模型)
