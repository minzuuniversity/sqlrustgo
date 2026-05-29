# SQLRustGo v3.6.0 发布说明

> **版本**: v3.6.0
> **分支**: develop/v3.6.0
> **HEAD**: 1b2a3c71
> **日期**: 2026-05-30
> **状态**: GA 入口审查
> **SSOT**: docs/governance/SSOT_CROSS_CHECK.md

---

## 版本概述

v3.6.0 是 SQLRustGo 的**知识增强 + 存储可靠性**版本，聚焦于四大领域：

1. **WALVerifier**: WAL 验证框架，确保事务日志完整性
2. **SIMD 向量化**: 利用 CPU SIMD 指令加速向量操作
3. **Knowledge OS 集成**: qmd-bridge 桥接知识操作系统
4. **Parser 修复**: sql-corpus 编译修复 + 窗口函数架构完善

---

## 新增功能

### 1. WALVerifier — WAL 验证框架

WAL (Write-Ahead Log) 是事务持久化的核心。v3.6.0 引入专门的 WAL 验证模块：

| 组件 | 说明 |
|------|------|
| crates/wal-verification/ | 独立 crate，专用 WAL 验证逻辑 |
| LSN 连续性检查 | 验证日志序列号无断层 |
| Checksum 校验 | 每条 WAL 条目的 CRC 校验 |
| 恢复正确性 | 模拟崩溃恢复，验证数据一致性 |
| 形式化验证 | WAL_Recovery.tla TLA+ 模型，修复无限集合问题 |

**测试**: 63 个 WAL 测试全部 PASS。

### 2. SIMD 向量化加速

| 特性 | 说明 |
|------|------|
| SIMD lanes 检测 | 自动检测 CPU 支持的 SIMD 宽度 (AVX2=8, AVX-512=16) |
| 向量距离计算 | L2 距离、余弦相似度 SIMD 加速 |
| 加速比目标 | 标量 vs SIMD ≥ 2x |

**提交**: `11ffbb98 v3.6.0-fix4: SIMD integration + WAL verification + compile fixes`

### 3. Knowledge OS 集成 (qmd-bridge)

| 特性 | 说明 |
|------|------|
| qmd-bridge 混合测试 | QmdDataType 导入修复 |
| 知识库接口 | 预留 Knowledge OS 桥接接口 |

**提交**: `86ff2a3d fix: qmd-bridge hybrid test QmdDataType import`

### 4. Parser 修复与增强

| 修复 | 说明 |
|------|------|
| sql-corpus 编译 | DROP COLUMN / MODIFY COLUMN 分支补齐，cargo check PASS |
| 窗口函数架构 | PercentRank / CumeDist 函数添加 + 测试断言修复 |
| 覆盖率测试 | 32 个 parser 覆盖集成测试 (0 失败) |

**提交**: `86f7cb13`, `4a5a7f19`, `045d4f3c`

---

## 测试结果

| 类别 | 用例数 | 通过 | 失败 | 通过率 |
|------|--------|------|------|--------|
| Parser 测试 | 169 | 169 | 0 | **100%** |
| Executor 测试 | 250 | 250 | 0 | **100%** |
| WAL 测试 | 63 | 63 | 0 | **100%** |
| Clippy | — | — | — | **零警告** |

---

## 覆盖率现状

| Crate | 覆盖率 |
|-------|--------|
| types | 87.62% |
| parser | 47.16% |
| planner | 92.23% |
| optimizer | 91.26% |
| executor | 72.04% |
| storage | 81.76% |
| transaction | 91.51% |
| catalog | 92.17% |
| **L1 平均** | **83.01%** |

> 注: GA 门槛 85%，当前 parser (47.16%) 和 executor (72.04%) 需要专项冲刺。

---

## 相比 v3.5.0 的改进

| 指标 | v3.5.0 | v3.6.0 | 变化 |
|------|--------|--------|------|
| WAL 测试 | ~12 | 63 | +425% |
| Parser 测试 | ~120 | 169 | +41% |
| L1 覆盖率 | ~76% | 83.01% | +7pp |
| SIMD 支持 | 基础 | 集成验证 | 增强 |
| Clippy 警告 | 0 | 0 | 一致 |

---

## 已知问题

1. **Coverage < 85%**: parser 覆盖率仅 47.16%，需大量补充测试
2. **TPC-H 未运行**: SF=0.1 和 SF=1 基准测试尚未执行
3. **MySQL 协议测试**: mysql-server tests2 仍有 43 个错误
4. **SSI 降级**: 串行化隔离级别降级为 READ COMMITTED

---

## 后续计划

| 版本 | 重点 |
|------|------|
| v3.7.0 | Parser 覆盖冲刺 (47% → 75%+) |
| v3.8.0 | Executor 覆盖冲刺 (72% → 85%+) |
| v4.0.0 | TPC-H 完整基准 + SSI 恢复 |

---

*SSOT 参考: docs/governance/SSOT_CROSS_CHECK.md*
*更新日期: 2026-05-30*
