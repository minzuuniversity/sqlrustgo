# v3.9.0 测试计划补充 — 性能 / Sysbench / QPS / 稳定性 / 崩溃

> **作者**: Hermes Agent (审计)
> **日期**: 2026-06-05
> **目的**: 检查 v3.9.0 总体计划 + governance 规则, 补充缺失的性能/Sysbench/QPS/稳定性/崩溃测试

---

## 一、现状审计

### 1.1 v3.9.0 现有计划 (基于 V390_TEST_PLAN.md)

**G1-G10 门禁设计** (G1-G10 已有):

| Gate | 主题 | 工作量 | 状态 |
|------|------|--------|------|
| G1 | TPC-H 保持 (22/22) | 5h | 已 PASS |
| G2 | INT-2 ParallelExecutor 集成 | 30h | opencode 关闭 |
| G3 | INT-3 Single Expression | 32h | Hermes 关闭 |
| G4 | ARCH-3 Complete (VtuGuard) | 40h | Hermes 关闭 |
| G5 | SEM-1 Savepoint MVCC | 28h | Hermes 关闭 |
| G6 | Backup/Restore (100+ scenarios) | 40h | opencode 关闭 |
| G7 | 24h Soak Test | 48h | Hermes 关闭 (压缩时间) |
| G8 | Crash Matrix (100+ scenarios) | 40h | Hermes 关闭 |
| G9 | Upgrade Test (v3.8 → v3.9) | 40h | Hermes 关闭 |
| G10 | Audit Log + Time Travel | 44h | Hermes 关闭 |
| **合计** | | **347h** | |

### 1.2 governance 规则 (从 GATE_CONDITIONS.md + RC_TO_GA_GATE_CHECKLIST.md)

**GA 门禁要求 (R1-R4 + G1-G6)**:
- **G3**: 覆盖率 L1 avg ≥ 85%, 每 crate ≥ 80%
- **G4**: TPC-H SF=1 → 22/22 PASS
- **G5**: `cargo audit` + 手动审计 PASS
- **G6**: 文档完整 (API ref / CHANGELOG / UPGRADE_GUIDE)
- 性能验收: 响应时间 SLA / 并发测试达标 / 资源占用符合预期

**RC_TO_GA 清单 §4.2**:
- 响应时间符合 SLA
- 并发测试达标
- 资源占用符合预期

### 1.3 缺失审计 (V390_TEST_PLAN.md **未包含**)

按 GATE_CONDITIONS GA §4.2 要求, **当前 V390_TEST_PLAN.md 严重缺失以下 5 类测试**:

| 类别 | V390_TEST_PLAN 覆盖 | GA §4.2 强制要求 | 状态 |
|------|---------------------|------------------|------|
| 性能基准 (QPS/TPS) | ❌ 缺失 | ✅ 强制 | **缺** |
| Sysbench OLTP | ❌ 缺失 (仅 1 个 point_select.sh) | ✅ 推荐 | **缺** |
| QPS 测试 | ❌ 缺失 | ✅ 强制 | **缺** |
| 长时间稳定性 (≥24h) | ⚠️ G7 Soak 提及但未实施 | ✅ 强制 | **缺** |
| 真实崩溃测试 (无压缩时间) | ⚠️ G8 Crash 提及但 harness 是 mock | ✅ 强制 | **缺** |

**结论**: V390_TEST_PLAN 缺失 **40-60%** 真实性能/稳定性/崩溃测试 — 全部被压缩时间 mock 替代。GA §4.2 验收无法通过。

---

## 二、补充测试设计 (5 个新增 G11-G15 门禁)

### G11: 性能基准测试 (QPS / TPS) [16h, ~2 天]

#### 目标
真实测量 SQLRustGo 在不同负载下的吞吐量和延迟, 提供 GA 门禁的量化 SLA 指标。

#### 测试范围

| 工作负载 | 数据量 | 线程数 | 目标 QPS | 目标 P99 延迟 |
|----------|--------|--------|----------|----------------|
| **Point SELECT** (主键查询) | 10K rows | 1, 4, 8, 16 | ≥ 5,000 / ≥ 15,000 / ≥ 25,000 / ≥ 35,000 | < 5ms / < 10ms / < 15ms / < 20ms |
| **Range SELECT** (索引扫描) | 100K rows | 1, 4, 8 | ≥ 1,000 / ≥ 3,000 / ≥ 5,000 | < 20ms / < 50ms / < 100ms |
| **INSERT** (写) | 10K rows | 1, 4, 8 | ≥ 2,000 / ≥ 5,000 / ≥ 8,000 | < 10ms / < 30ms / < 50ms |
| **UPDATE** (索引) | 10K rows | 1, 4, 8 | ≥ 1,500 / ≥ 4,000 / ≥ 6,000 | < 15ms / < 40ms / < 80ms |
| **Mixed OLTP** (sysbench-like) | 10K rows | 8 | ≥ 3,000 | < 50ms |

#### 测试工具 (新建 + 借力)

1. `scripts/bench/run_qps_benchmarks.sh` (新) — 编排 + 报告
2. `crates/bench/src/db/sqlrustgo.rs` (已有) — QPS measurement
3. `crates/bench/src/runner/concurrent_runner.rs` (已有) — 并发执行
4. `crates/bench/src/metrics/latency.rs` (已有) — P50/P95/P99

#### 验证
- [ ] 5 类工作负载 × 4 线程数 = 20 个 measurement
- [ ] QPS 不退化 (vs v3.8.0 baseline)
- [ ] 报告生成: `docs/releases/v3.9.0/perf/QPS_REPORT.md`

---

### G12: Sysbench OLTP 测试 [8h, ~1 天]

#### 目标
借力 Sysbench 标准 OLTP 工作负载, 与 MySQL/PostgreSQL 对比, 验证 SQLRustGo 实用级数据库引擎的 OLTP 性能。

#### 测试范围 (借力 sysbench + sqlrustgo)

| Sysbench 工作负载 | 说明 | 数据集 | 目标 |
|------------------|------|--------|------|
| `oltp_point_select` | 主键点查 | 10K / 100K / 1M rows | ≥ 5,000 QPS |
| `oltp_read_only` | 只读事务 | 10K rows | ≥ 1,000 TPS |
| `oltp_read_write` | 读写事务 | 10K rows | ≥ 500 TPS |
| `oltp_write_only` | 只写事务 | 10K rows | ≥ 1,000 TPS |
| `oltp_insert` | 批量插入 | 100K rows | 持续 ≥ 5,000 TPS |

#### 实施

1. **真实 sysbench** (`scripts/sysbench/oltp_*.sh` 新建 5 个):
```bash
#!/bin/bash
# scripts/sysbench/oltp_read_write.sh
THREADS=${1:-8}
TIME=${2:-60}
sysbench oltp_read_write \
  --db-driver=mysql \
  --mysql-host=127.0.0.1 --mysql-port=3306 \
  --mysql-user=mysql --mysql-password=... \
  --mysql-db=sbtest --table-size=10000 \
  --threads=$THREADS --time=$TIME \
  --report-interval=5 \
  run
```

2. **Rust 原生 sysbench** (借力 `crates/bench/src/workload/oltp*.rs`):
   - 7 个 oltp_*.rs 已存在, 7 个 #[test] 通过
   - 扩展: 增加 30+ #[test] 覆盖 read_only / read_write / write_only

3. **G12 Gate** (`scripts/gate/check_g12_sysbench.sh` 新建, 7 checks):
   - 5 个 sysbench 脚本存在
   - OLTP tests 30+ 存在
   - 借力 crates/bench 7 workload 文件
   - cargo test 全部 PASS
   - 性能对比: QPS ≥ 50% v3.8.0 baseline
   - TPC-H 22/22 维持
   - 报告归档: `docs/releases/v3.9.0/perf/SYSBENCH_REPORT.md`

#### 验证
- [ ] 5 类 sysbench OLTP 工作负载
- [ ] 30+ OLTP unit tests PASS
- [ ] 与 v3.8.0 baseline 对比报告
- [ ] TPC-H 22/22 维持 (G1)

---

### G13: 长时间稳定性测试 (24h+ 真实运行) [24h, ~3 天]

#### 目标
**真实运行** (非压缩时间) 24h+ 持续负载, 监控资源增长和崩溃次数。这是 v3.9.0 GA §4.2 的强制要求。

#### 与 G7 Soak Test 的区别

| 维度 | G7 Soak (已实现) | G13 Stability (本任务) |
|------|------------------|------------------------|
| 时长 | 60s/180s/420s (压缩) | **24h+ 真实** |
| 工具 | 单元级 mock | 真实 server 启动 |
| 资源监控 | 模拟采样 | 真实 RSS / FD / lock |
| CI 触发 | 每次 PR | **仅 RC/GA 前** (CI 不跑) |
| 借力 | harness 单元测试 | 真实 sysbench/tpch |
| 输出 | 报告 | **GA 验收必备** |

#### 测试设计

| 阶段 | 时长 | 工作负载 | 监控 | 终止条件 |
|------|------|----------|------|----------|
| 24h Soak | **24h** | sysbench oltp_read_write 8 thread | RSS / FD / lock / WAL | RSS 增长 > 50% OR 崩溃 > 0 |
| 72h Soak | **72h** | sysbench oltp_mixed 16 thread | 同上 | 同上 |
| 168h Soak | **168h** | sysbench oltp_read_write 16 thread | 同上 | 同上 |

#### 实施

1. **`scripts/stability/run_24h_soak.sh`** (新):
```bash
#!/bin/bash
HOURS=24
THREADS=8
# 1. 启动 sqlrustgo server
# 2. sysbench oltp_read_write --time=$((HOURS*3600))
# 3. 每 60s 采样 RSS/FD/lock/WAL
# 4. 异常阈值: 报警 + 终止
# 5. 生成报告
```

2. **`scripts/stability/run_72h_soak.sh`** + **`run_168h_soak.sh`** (新)

3. **`scripts/stability/collect_stability_results.sh`** (借力 + 扩展)

4. **G13 Gate** (`scripts/gate/check_g13_stability.sh` 新建, 7 checks):
   - run_24h_soak.sh 存在
   - run_72h_soak.sh 存在
   - run_168h_soak.sh 存在
   - 监控脚本存在 (RSS/FD/lock/WAL sampler)
   - cargo test PASS (基线)
   - TPC-H 22/22 维持
   - 报告: `docs/releases/v3.9.0/perf/STABILITY_REPORT.md`

#### 验证
- [ ] 24h real run (Z6G4 server, NOT local)
- [ ] 72h real run (Z6G4, RC 阶段)
- [ ] 168h real run (Z6G4, GA 阶段)
- [ ] 报告: PASS/FAIL, 资源增长曲线, 崩溃次数

---

### G14: 真实崩溃测试 (无压缩时间) [16h, ~2 天]

#### 目标
**真实进程级崩溃注入** (非单元 mock) 验证数据一致性。这是 v3.9.0 GA §4.2 的强制要求。

#### 与 G8 Crash Matrix 的区别

| 维度 | G8 Crash (已实现) | G14 Real Crash (本任务) |
|------|-------------------|--------------------------|
| 工具 | harness mock | **真实 sys_kill / dd / rm** |
| 进程 | 进程内 mock | **真实 sqlrustgo-mysql-server 进程** |
| 数据 | 内存 hashmap | **真实磁盘 WAL/Page** |
| 恢复验证 | mock assert | **TPC-H Q1 hash 比对** |
| CI 触发 | 每次 PR | **仅 RC/GA 前** (需 root + 慢) |

#### 8 类真实崩溃场景 (与 G8 同分类)

| # | 崩溃 | 真实命令 | 验证 |
|---|------|----------|------|
| 1 | SIGKILL mid-INSERT | `kill -9 $PID` 期间 sysbench | TPC-H hash 比对 |
| 2 | SIGKILL mid-COMMIT | 同上, COMMIT flush 中 | 同上 |
| 3 | SIGKILL mid-ROLLBACK | 同上, ROLLBACK TO SAVEPOINT | 同上 |
| 4 | Power loss (rm -rf WAL) | `rm WAL/* ; restart` | 同上 |
| 5 | Disk full (dd 满) | `dd if=/dev/zero of=WAL/x` | 同上 |
| 6 | OOM (cgroup) | `systemd-run --memory-limit=100M` | 同上 |
| 7 | WAL corruption (字节翻转) | `dd conv=notrunc bs=1 seek=N < /dev/urandom` | 同上 |
| 8 | 进程 hang (kill -STOP) | `kill -STOP $PID` | 同上 |

#### 实施

1. **`scripts/crash/run_real_crash_test.sh`** (新):
```bash
#!/bin/bash
TEST_KIND=${1:-sigkill_insert}
# 1. 启动 sqlrustgo server
# 2. sysbench 启动 workload
# 3. 在指定时机注入崩溃
# 4. 重启 server
# 5. 跑 TPC-H Q1, hash 与 baseline 比对
# 6. PASS/FAIL
```

2. **`scripts/crash/run_*_test.sh`** — 8 个真实崩溃脚本

3. **G14 Gate** (`scripts/gate/check_g14_real_crash.sh` 新建, 7 checks):
   - run_real_crash_test.sh 存在
   - 8 个崩溃子脚本存在
   - TPC-H baseline 已记录
   - 资源 (dd / kill / cgroup) 可用
   - cargo test PASS
   - TPC-H 22/22 维持 (基线)
   - 报告: `docs/releases/v3.9.0/perf/CRASH_TEST_REPORT.md`

#### 验证
- [ ] 8 类真实崩溃场景
- [ ] TPC-H Q1 hash 一致性验证
- [ ] 报告: PASS/FAIL, 场景, 恢复时间

---

### G15: 综合性能报告 (Performance Report) [8h, ~1 天]

#### 目标
汇总 G11-G14 数据, 生成 **GA 验收必备** 的 `PERFORMANCE_REPORT.md` (GATE_CONDITIONS GA GE3 入口条件)。

#### 报告结构

```markdown
# SQLRustGo v3.9.0 Performance Report

## 1. 概述
- 测试环境: Z6G4, x86_64, 8 cores
- 测试时间: 2026-XX-XX
- Baseline: v3.8.0 (commit xxx)

## 2. QPS / TPS (G11)
| 工作负载 | 线程 | QPS v3.9.0 | QPS v3.8.0 | 变化 | 目标 | 状态 |
|----------|------|-----------|-----------|------|------|------|
| point_select | 1 | ... | ... | +X% | ≥5000 | ✅/❌ |
| point_select | 4 | ... | ... | +X% | ≥15000 | ✅/❌ |
| ... | | | | | | |

## 3. Sysbench OLTP (G12)
| 工作负载 | v3.9.0 TPS | v3.8.0 TPS | 变化 | 目标 | 状态 |
|----------|-----------|-----------|------|------|------|
| oltp_point_select | ... | ... | +X% | ... | ✅/❌ |
| ... | | | | | |

## 4. 24h Stability (G13)
| 指标 | 24h | 72h | 168h | 目标 |
|------|-----|-----|------|------|
| RSS 增长 | <5% | <10% | <15% | <10% |
| FD 句柄 | <100 | <200 | <500 | <1000 |
| 崩溃次数 | 0 | 0 | 0 | 0 |

## 5. Real Crash (G14)
| 场景 | 恢复时间 | 数据一致 | 状态 |
|------|----------|----------|------|
| SIGKILL mid-INSERT | Xms | ✅/❌ | ✅/❌ |
| ... | | | |

## 6. 结论
- ✅ GA 性能门禁 PASS / ❌ FAIL
- 已知瓶颈
- 优化建议 (v3.10+)
```

#### 实施

1. **`scripts/perf/generate_perf_report.sh`** (新) — 汇总 G11-G14 数据
2. **`docs/releases/v3.9.0/perf/PERFORMANCE_REPORT.md`** (新建)
3. **G15 Gate** (`scripts/gate/check_g15_perf_report.sh` 新建, 7 checks):
   - generate_perf_report.sh 存在
   - QPS_REPORT.md 存在 (G11)
   - SYSBENCH_REPORT.md 存在 (G12)
   - STABILITY_REPORT.md 存在 (G13)
   - CRASH_TEST_REPORT.md 存在 (G14)
   - PERFORMANCE_REPORT.md 存在 (汇总)
   - 报告含 baseline 对比 + PASS/FAIL

---

## 三、修正后 v3.9.0 总体测试工作量

### 当前 (V390_TEST_PLAN.md)
- G1-G10 = **347h** (单元级 mock 为主)
- 缺失: 性能 / Sysbench / QPS / 24h+ 稳定性 / 真实崩溃

### 补充后 (本方案)
- G1-G10 = **347h** (保持)
- G11 = **16h** (QPS/TPS)
- G12 = **8h** (Sysbench OLTP)
- G13 = **24h** (24h+ Stability)
- G14 = **16h** (Real Crash)
- G15 = **8h** (汇总报告)
- **新增小计: 72h**
- **总计: 419h** (12 周, 1 人满负载)

### 时间线 (W11-12, Phase 6 收口)

| 周 | 任务 | 关键产物 |
|----|------|----------|
| W11 | G11 QPS + G12 Sysbench | QPS_REPORT, SYSBENCH_REPORT |
| W12 | G13 24h + G14 Real Crash + G15 汇总 | STABILITY_REPORT, CRASH_TEST_REPORT, PERFORMANCE_REPORT |
| W12 end | GA 收口 (G15 报告) | v3.9.0 GA 候选 |

---

## 四、治理对齐 (governance 规则)

### 4.1 满足 GATE_CONDITIONS.md GA 入口条件

| 入口 | 当前状态 | 补充后 |
|------|----------|--------|
| GE1 RC PASS | ✅ | ✅ |
| GE2 RC_GATE_REPORT | ✅ | ✅ |
| **GE3 PERFORMANCE_REPORT** | ❌ 缺失 | ✅ G11-G15 提供 |
| GE4 SECURITY_AUDIT | ✅ | ✅ |
| GE5 Issue 关闭 | ✅ | ✅ |

### 4.2 满足 GA §4.2 性能指标

| 验收 | 当前 | 补充后 |
|------|------|--------|
| 响应时间 SLA | ❌ 无量化 | ✅ G11 + G15 |
| 并发测试达标 | ❌ mock | ✅ G12 + G13 |
| 资源占用符合 | ❌ mock | ✅ G13 |

### 4.3 满足 RC_TO_GA_GATE_CHECKLIST §4.2

- ✅ 4.2.1 响应时间符合 SLA
- ✅ 4.2.2 并发测试达标
- ✅ 4.2.3 资源占用符合预期

### 4.4 与 v3.8.0 governance 对比

v3.8.0 已有:
- `scripts/test/monitor_stability_test.sh` (12K chars) — 真实稳定性监控
- `scripts/sysbench/point_select.sh` (1 个) — 仅 1 个 sysbench
- `scripts/test/deploy_stability_test.sh` (5.7K chars) — 真实部署稳定性
- `scripts/test/collect_stability_results.sh` (7.9K chars) — 结果收集
- `scripts/benchmark/check_regression.sh` — 性能回归
- `scripts/gate/check_performance.sh` (DEPRECATED 注释) — 旧性能 gate

v3.9.0 本方案借力 + 扩展, 不重复造轮子。

---

## 五、关键决策记录

| 决策 | 选择 | 理由 |
|------|------|------|
| 真实 vs 压缩时间 | **24h+ 真实 (G13)** | G7 压缩时间仅 CI 用, GA 须真实 |
| Sysbench 真伪 | **双轨** (真 sysbench + Rust oltp) | 工具 + 自研双覆盖 |
| Real crash 工具 | **真实 sys_kill / dd / cgroup** | G8 mock 仅 CI 用, GA 须真实 |
| 报告汇总 | **PERFORMANCE_REPORT.md** | GATE_CONDITIONS GA GE3 入口强制 |
| CI vs GA 区分 | **G11-G14 仅 RC/GA 前跑** | 24h/72h/168h 不可 CI |
| 资源 | **Z6G4** (复用 v3.8.0) | 已有 16 核 64GB |
| 借力策略 | **复用 v3.8.0 真实脚本** | monitor_stability_test.sh, collect_stability_results.sh |

---

## 六、Subsumed / 依赖

### 6.1 依赖现有

- G1 TPC-H (Hermes 已 PASS 22/22)
- G6 Backup/Restore (opencode 已实现, 借力)
- G7 Soak (Hermes 已实现压缩时间, G13 扩展真实)
- G8 Crash (Hermes 已实现 mock, G14 扩展真实)
- G10 Audit/Time Travel (Hermes 已实现)

### 6.2 Subsumed Issues

- (无新 issue) — 这是 V390_TEST_PLAN 的补充, 不创建新 issue

### 6.3 跨阶段影响

- G11-G15 是 Phase 6 收口任务, 与 P3 性能优化 (P3-1~P3-5) 互补
- P3 已完成 CBO + Statistics + Parallel + SIMD (Hermes 11 任务)
- G11-G15 是 P3 优化的**验证步骤** (CBO 优化是否真提升 QPS?)

---

## 七、风险评估

| 风险 | 等级 | 缓解 |
|------|------|------|
| 24h 真实运行 CI 超时 | 🟠 中 | 仅 RC/GA 前跑, 标记 `<!-- env:blocked:no-ci -->` |
| Z6G4 资源被占用 | 🟡 中 | v3.8.0 已用, 协调 |
| Real crash 需 root | 🟠 中 | docker container 内跑 |
| Sysbench 安装 | 🟢 低 | 已有 `scripts/sysbench/point_select.sh` |
| 性能数据噪音 | 🟡 中 | 多次测量取中位数 |

---

## 八、文件清单 (本计划产出)

### 8.1 新建文件 (15 个)

| 文件 | 行数估算 | 用途 |
|------|----------|------|
| `scripts/bench/run_qps_benchmarks.sh` | 80 | G11 QPS 编排 |
| `scripts/sysbench/oltp_read_only.sh` | 60 | G12 |
| `scripts/sysbench/oltp_read_write.sh` | 60 | G12 |
| `scripts/sysbench/oltp_write_only.sh` | 60 | G12 |
| `scripts/sysbench/oltp_insert.sh` | 60 | G12 |
| `tests/oltp_extended_test.rs` | 400 | G12 30+ tests |
| `scripts/stability/run_24h_soak.sh` | 100 | G13 |
| `scripts/stability/run_72h_soak.sh` | 100 | G13 |
| `scripts/stability/run_168h_soak.sh` | 100 | G13 |
| `scripts/crash/run_real_crash_test.sh` | 120 | G14 |
| `scripts/crash/run_*_test.sh` × 8 | 8×80 = 640 | G14 |
| `scripts/gate/check_g11_qps.sh` | 70 | G11 |
| `scripts/gate/check_g12_sysbench.sh` | 70 | G12 |
| `scripts/gate/check_g13_stability.sh` | 70 | G13 |
| `scripts/gate/check_g14_real_crash.sh` | 70 | G14 |
| `scripts/gate/check_g15_perf_report.sh` | 70 | G15 |
| `scripts/perf/generate_perf_report.sh` | 80 | G15 |
| `docs/releases/v3.9.0/perf/QPS_REPORT.md` | 200 | G11 |
| `docs/releases/v3.9.0/perf/SYSBENCH_REPORT.md` | 200 | G12 |
| `docs/releases/v3.9.0/perf/STABILITY_REPORT.md` | 200 | G13 |
| `docs/releases/v3.9.0/perf/CRASH_TEST_REPORT.md` | 200 | G14 |
| `docs/releases/v3.9.0/perf/PERFORMANCE_REPORT.md` | 300 | G15 (汇总) |
| **合计** | **~3400 行** | |

### 8.2 修改文件 (1 个)

- `docs/releases/v3.9.0/plans/V390_TEST_PLAN.md` — 加 G11-G15 章节

---

## 九、执行计划 (W11-12)

### W11 (Phase 6 中段)

| Day | 任务 |
|-----|------|
| D1 | 写 G11 编排 + cargo bench 集成 (run_qps_benchmarks.sh) |
| D2 | 写 5 个 sysbench 脚本 + 借力 crates/bench oltp |
| D3 | 写 G11 G12 gates + 跑 baseline |
| D4-5 | 跑 G11 (QPS) + G12 (Sysbench) — Z6G4 真实环境 |

### W12 (Phase 6 末段)

| Day | 任务 |
|-----|------|
| D1-2 | 24h Soak (G13) — 真实 server + sysbench |
| D3-4 | Real Crash (G14) — 8 类真实崩溃注入 |
| D5 | Generate Performance Report (G15) + 归档 |
| D6-7 | v3.9.0 GA 收口 (G11-G15 全部 PASS) |

---

## 十、验收标准 (W12 收口)

```
✅ G11 QPS: 5 workloads × 4 thread counts = 20 measurements
✅ G12 Sysbench: 5 OLTP workloads + 30+ Rust unit tests
✅ G13 24h Stability: RSS < 10% growth, 0 crashes
✅ G14 Real Crash: 8 categories, TPC-H Q1 hash 一致
✅ G15 Performance Report: QPS + Sysbench + Stability + Crash 4 个 sub-report + 汇总

✅ G1 TPC-H 22/22 维持 (G11-G14 期间不退化)
✅ L1 unit ≥ 85% coverage (G1-G10 已有)
✅ Governance: GA GE3 PERFORMANCE_REPORT.md 存在 (G15 提供)
✅ Performance Report 含 baseline 对比 (vs v3.8.0)
```

---

## 十一、对比: 治理要求 vs 现有

| 治理要求 (GATE_CONDITIONS.md) | V390_TEST_PLAN 当前 | 补充后 (G11-G15) |
|------------------------------|---------------------|-------------------|
| **GA 入口 GE3 PERFORMANCE_REPORT** | ❌ 缺 | ✅ G15 |
| **GA 验收 §4.2.1 响应时间 SLA** | ❌ 缺 | ✅ G11 |
| **GA 验收 §4.2.2 并发测试达标** | ❌ mock | ✅ G12 + G13 |
| **GA 验收 §4.2.3 资源占用符合** | ❌ mock | ✅ G13 |
| **RC_TO_GA §4.2 性能指标** | ⚠️ 部分 | ✅ 全部 |
| **G1 TPC-H 维持** | ✅ 22/22 | ✅ 22/22 |

**结论**: 本补充方案完整覆盖 governance GA 性能验收要求, 解决 V390_TEST_PLAN 缺失的 40-60% 测试覆盖。

---

**本计划完成审计, 准备进入 W11 实施阶段。**
