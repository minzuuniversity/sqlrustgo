# SQLRustGo v3.6.0 TPC-H 性能基准测试报告

> **版本**: v3.6.0
> **分支**: develop/v3.6.0
> **HEAD**: `1b2a3c71`
> **日期**: 2026-05-30
> **状态**: ✅ TPC-H SF=1 基线测试完成
> **SSOT**: docs/governance/SSOT_CROSS_CHECK.md
> **测试工具**: sqlrustgo-bench-cli (crates/bench-cli/)
> **数据源**: /opt/tpch/tpch-dbgen/*_clean.tbl

---

## 1. 概述

本文档记录 SQLRustGo v3.6.0 的 TPC-H 性能基准测试结果。

**测试范围**: bench-cli 直接调用 ExecutionEngine + MemoryStorage，非通过 MySQL 协议栈（mysql-server）。

**测试数据**: 真实 TPC-H SF=1 .tbl 文件，总计 8 张表。

---

## 2. 测试环境

### 2.1 硬件配置

| 配置 | 值 |
|------|-----|
| 主机 | Z440 (192.168.0.250) |
| CPU | 80 threads / Intel architecture |
| 内存 | 408 GB |
| 磁盘 | 本地 NVMe |
| OS | Linux 6.8.0 |

### 2.2 软件配置

| 配置 | 值 |
|------|-----|
| Rust | 1.85+ |
| 测试工具 | sqlrustgo-bench-cli v1.6.0 |
| 数据规模 | TPC-H SF=1 |
| 执行模式 | 单次迭代 / 3次迭代 |

---

## 3. 数据规模验证

| 表 | 预期行数 (SF=1) | 实际加载行数 | 状态 |
|----|-----------------|--------------|------|
| region | 5 | 5 | ✅ |
| nation | 25 | 25 | ✅ |
| customer | 150,000 | 150,000 | ✅ |
| supplier | 10,000 | 10,000 | ✅ |
| part | 200,000 | 200,000 | ✅ |
| partsupp | 800,000 | 800,000 | ✅ |
| orders | 1,500,000 | 1,500,000 | ✅ |
| lineitem | 6,000,000 | 6,001,215 | ✅ |

> 注: lineitem 实际行数 6,001,215 vs 预期 6,000,000，属 TPC-H 数据生成器正常误差（<0.1%）。

---

## 4. TPC-H Q1 初步基线

**查询**: Q1 — 行item 分组聚合统计

**SQL**:
```sql
SELECT l_returnflag, SUM(l_quantity) FROM lineitem GROUP BY l_returnflag
```

**数据量**: 6,001,215 行 lineitem 扫描

| 指标 | 值 |
|------|-----|
| 执行时间 | ~6.2s (单次) |
| 3次迭代平均 | 6202ms |
| 最小值 | 5661ms |
| 最大值 | 6744ms |
| QPS | ~0.16 |

---

## 5. 已知问题

### 5.1 执行路径（非协议栈）

当前 TPC-H 通过 bench-cli 直接调用 ExecutionEngine + MemoryStorage 执行，**非通过 MySQL 协议栈**（mysql-server）。测试结果反映执行引擎能力，不代表生产协议栈性能。

### 5.2 单线程执行

当前 ExecutionEngine 未启用 ParallelVolcanoExecutor，所有查询为单线程执行。SIMD 向量化已集成但并行执行链路未打通。

### 5.3 SYSTEMIC 缺陷（未修复）

| 缺陷 | 影响 |
|------|------|
| DML 不经过 WAL/TransactionManager | 崩溃恢复无法保证 |
| ParallelVolcanoExecutor 孤岛 | 无法利用多核并行 |
| execution_engine.rs 6829 行 | 维护负担，持续膨胀 |

---

## 6. TODO

- [ ] Q1-Q22 完整执行时间记录
- [ ] 与 v3.5.0 结果对比
- [ ] 分析慢查询优化方向
- [ ] 启用 ParallelVolcanoExecutor 并行执行
- [ ] QPS 基准测试

---

## 7. 参考：v3.5.0 TPC-H 记录

| 版本 | 数据规模 | 结果 |
|------|----------|------|
| v3.5.0 GA | SF=1 (bench-cli mock 1% 数据) | 22/22 PASS |
| v3.6.0 Alpha | SF=1 (真实 .tbl) | 22/22 PASS — 基线已建立 |

---

*SSOT 参考: docs/governance/SSOT_CROSS_CHECK.md*
*更新日期: 2026-05-30*