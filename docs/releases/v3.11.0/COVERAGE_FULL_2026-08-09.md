# v3.11.0 完整覆盖率报告

**生成时间**: 2026-08-09
**测量方法**: `cargo llvm-cov --lib -p <crate>` (llvm-cov, lib tests only)
**分支**: `develop/v3.11.0` @ `bc4d2143e`
**磁盘**: Mac Mini 228GB (清理 llvm-cov-target 后实测)

## GA Gate G3 覆盖率要求

| 阶段 | 阈值 | 说明 |
|------|------|------|
| RC C5 | L1_8 平均 ≥ 75% | 8个核心crate平均 |
| **GA G3** | **每crate ≥ 80%** | 全部crate独立达标 |
| GA G3 (加强) | L1 平均 ≥ 85% | 全部crate平均 |

> ⚠️ **RC C5 和 GA G3 是两套不同标准**。RC 通过不等于 GA 通过。

## 完整覆盖率数据（26 crates, 2026-08-09 实测）

| # | Crate | Line% | GA ≥80% | 备注 |
|---|-------|-------|---------|------|
| 1 | sqlrustgo-network | **100.00%** | ✅ | |
| 2 | sqlrustgo-cache | **99.47%** | ✅ | |
| 3 | sqlrustgo-wal-verification | **97.20%** | ✅ | |
| 4 | sqlrustgo-telemetry | **96.67%** | ✅ | |
| 5 | sqlrustgo-rag | **96.58%** | ✅ | |
| 6 | sqlrustgo-types | **90.91%** | ✅ | |
| 7 | sqlrustgo-common | **89.64%** | ✅ | |
| 8 | sqlrustgo-catalog | **84.94%** | ✅ | |
| 9 | sqlrustgo-planner | **84.91%** | ✅ | L1_8 |
| 10 | sqlrustgo-transaction | **84.29%** | ✅ | |
| 11 | sqlrustgo-security | **82.74%** | ✅ | |
| 12 | sqlrustgo-optimizer | **87.25%** | ✅ | |
| 13 | sqlrustgo-storage | **83.59%** | ✅ | L1_8 |
| 14 | sqlrustgo-tools | **80.39%** | ✅ | L1_8 |
| 15 | sqlrustgo-spill | 75.17% | ❌ | -4.83pp |
| 16 | sqlrustgo_gis | 75.00% | ❌ | -5.00pp |
| 17 | sqlrustgo-executor | 76.41% | ❌ | -3.59pp |
| 18 | sqlrustgo-server | 74.67% | ❌ | -5.33pp |
| 19 | sqlrustgo-vector | 70.65% | ❌ | -9.35pp |
| 20 | sqlrustgo-gmp | 73.32% | ❌ | -6.68pp |
| 21 | sqlrustgo-admin | 63.01% | ❌ | -16.99pp |
| 22 | sqlrustgo-parser | 63.03% | ❌ | -16.97pp |
| 23 | sqlrustgo-mysql-server | 40.62% | ❌ | -39.38pp |
| 24 | sqlrustgo-mysql-client | 31.56% | ❌ | -48.44pp |
| 25 | sqlrustgo-cli | 0.00% | ❌ | 无lib测试 |
| 26 | sqlrustgo-sql-corpus | 0.00% | ❌ | 无lib测试 |

**汇总**: 14/26 ✅  12/26 ❌

## L1_8 核心 Crate (GA G3 重点考核)

| Crate | RC C5 (2026-07-18) | GA G3 当前 | 差值 | GA≥80% |
|-------|-------------------|-----------|------|---------|
| sqlrustgo-storage | 85.58% | **83.59%** | -1.99pp | ✅ |
| sqlrustgo-planner | 84.91% | **84.91%** | 0% | ✅ |
| sqlrustgo-tools | 63.84% | **80.39%** | +16.55pp | ✅ |
| sqlrustgo-executor | 76.45% | **76.41%** | -0.04pp | ❌ -3.59pp |
| sqlrustgo-admin | 83.14% | **63.01%** | -20.13pp | ❌ -16.99pp |
| sqlrustgo-parser | 71.22% | **63.03%** | -8.19pp | ❌ -16.97pp |
| sqlrustgo-mysql-server | 51.53% | **40.62%** | -10.91pp | ❌ -39.38pp |
| sqlrustgo-mysql-client | 43.79% | **31.56%** | -12.23pp | ❌ -48.44pp |
| **L1_8 平均** | **80.60%** | **62.94%** | -17.66pp | ❌ |

## GA G3 分析

### 当前状态: ❌ FAIL

- **L1_8 平均**: 62.94% (要求 ≥85%)
- **每crate≥80%**: 4/8 L1_8 crate 通过 (storage/planner/tools/新4个)
- **全workspace 26 crate**: 14/26 通过

### RC C5 vs GA G3 差异原因

1. **测试策略不同**: RC 用 `cargo llvm-cov test` (含integration/e2e)，GA G3 用 `cargo llvm-cov --lib` (仅lib)
2. **阈值不同**: RC C5 要求 L1_8 平均 ≥75%；GA G3 要求每crate ≥80%
3. **数据陈旧**: RC 数据 2026-07-18 采集，当前实测 2026-08-09，部分crate有退化

### 关键发现

- tools 从 RC 63.84% 提升到 80.39% ✅ (+16.55pp)，主要来自 PR #3542 (+141 tests)
- admin 从 RC 83.14% **退化**到 63.01% ❌ (-20.13pp)，原因待查（测试策略变化？）
- parser/mysql-server/mysql-client 持续低于 80%

## G3 通过所需最小增量

| Crate | 当前 | 目标 | 需增加 | 最快路径 |
|-------|------|------|--------|---------|
| executor | 76.41% | 80% | +3.59pp | 22个0%文件 |
| spill | 75.17% | 80% | +4.83pp | 中等工作量 |
| gis | 75.00% | 80% | +5.00pp | 中等工作量 |
| server | 74.67% | 80% | +5.33pp | 中等工作量 |
| gmp | 73.32% | 80% | +6.68pp | 中等工作量 |
| vector | 70.65% | 80% | +9.35pp | 较多工作量 |
| admin | 63.01% | 80% | +16.99pp | 大量工作 |
| parser | 63.03% | 80% | +16.97pp | 大量工作 |
| mysql-server | 40.62% | 80% | +39.38pp | 大量工作 |
| mysql-client | 31.56% | 80% | +48.44pp | 大量工作 |
| cli | 0.00% | 80% | 80pp | 需要新增lib测试 |
| sql-corpus | 0.00% | 80% | 80pp | 需要新增lib测试 |

## 建议 G3 修复路径

1. **立即可达**: executor (76.41%, 差3.59pp) — 22个0%文件，集中补测
2. **短中期**: admin+parser+server+gmp+gis+spill+vector (63-75%, 差5-17pp)
3. **长期**: mysql-server + mysql-client (差距>39pp)
4. **不推荐**: cli/sql-corpus 设为非考核项（无lib测试本身是设计问题）
