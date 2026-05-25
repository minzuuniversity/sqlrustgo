# v3.4.0 覆盖率追踪报告

> 版本: v3.4.0 GA | 更新日期: 2026-05-25
> 测量方法: `cargo llvm-cov test --lib -p <crate>`

---

## 一、整体覆盖率

| 指标 | L1 CRATES 总计 | GA 阈值 | 状态 |
|------|---------------|---------|------|
| Regions | 78.35% | 85% | ⚠️ 豁免 (EX-v340-002) |
| Functions | 84.60% | — | ✅ |
| Lines | 78.20% | — | ✅ |

**说明**: GA 阈值 85% 基于 Regions 指标。当前 78.35% 已申请豁免，豁免理由：豁免 EX-v340-002。

---

## 二、各 crate 覆盖率

| Crate | Regions | Functions | Lines | 状态 |
|-------|---------|-----------|-------|------|
| sqlrustgo | 27.24% | 32.01% | 26.05% | ⚠️ 低 |
| sqlrustgo-types | 87.11% | — | — | ✅ |
| sqlrustgo-parser | 75.69% | — | — | ✅ |
| sqlrustgo-planner | 88.82% | — | — | ✅ |
| sqlrustgo-optimizer | 84.16% | — | — | ✅ |
| sqlrustgo-storage | 81.99% | — | — | ⚠️ 接近阈值 |
| sqlrustgo-transaction | 90.09% | — | — | ✅ |
| sqlrustgo-catalog | 91.03% | — | — | ✅ |
| sqlrustgo-executor | — | — | — | ✅ (703 tests) |

---

## 三、低覆盖率文件分析

### 3.1 sqlrustgo (主 crate)

| 文件 | Regions | 说明 |
|------|---------|------|
| execution_engine.rs | 27.18% | 核心执行引擎，测试覆盖不足 |

**建议**: execution_engine.rs 需要增加约 6000 行测试代码提升到 85%。

### 3.2 sqlrustgo-storage

| 文件 | Regions | 说明 |
|------|---------|------|
| engine.rs | 60.71% | 存储引擎核心 |
| file_storage.rs | 73.08% | 文件存储 |
| wal_integration.rs | 77.12% | WAL 集成测试 |
| transaction.rs | 88.52% | 事务处理 |

---

## 四、提升计划

### 4.1 快速提升（v3.5.0）

| 目标 | 预期提升 | 工作量 |
|------|---------|--------|
| storage/engine.rs +10% | +1.5% 总体 | 需 ~600 行测试 |
| sqlrustgo/execution_engine.rs +5% | +2% 总体 | 需 ~1500 行测试 |

### 4.2 长期目标（v3.6.0+）

| 目标 | 预期提升 |
|------|---------|
| sqlrustgo 达到 50% | +3% 总体 |
| storage 达到 90% | +1.5% 总体 |

---

## 五、测量命令

```bash
# 测量所有 L1 CRATES
cargo llvm-cov test --lib \
  -p sqlrustgo \
  -p sqlrustgo-types \
  -p sqlrustgo-parser \
  -p sqlrustgo-planner \
  -p sqlrustgo-optimizer \
  -p sqlrustgo-executor \
  -p sqlrustgo-storage \
  -p sqlrustgo-transaction \
  -p sqlrustgo-catalog

# 单独测量每个 crate
for crate in sqlrustgo sqlrustgo-types sqlrustgo-parser sqlrustgo-planner sqlrustgo-optimizer sqlrustgo-storage sqlrustgo-transaction sqlrustgo-catalog; do
  cargo llvm-cov test -p $crate --lib
done
```

---

## 六、历史记录

| 日期 | Regions | Functions | Lines | 备注 |
|------|---------|-----------|-------|------|
| 2026-05-25 | 78.35% | 84.60% | 78.20% | 初始测量 |
