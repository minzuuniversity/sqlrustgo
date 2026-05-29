# v3.6.0 GA Gate Checklist

> **版本**: v3.6.0
> **分支**: develop/v3.6.0
> **HEAD**: 1b2a3c71
> **日期**: 2026-05-30
> **状态**: GA 入口审查
> **SSOT**: docs/governance/SSOT_CROSS_CHECK.md

---

## SSOT 门禁阈值参考

| 阶段 | 条件 | 阈值 |
|------|------|------|
| Alpha | 单 crate 最低 | >= 50% |
| Beta | L1 8 crates 平均 | >= 75% |
| RC/GA | L1 8 crates 平均 | >= 85% |

**当前 L1 平均覆盖率**: 83.01% (--lib 模式)

---

## GA 入口条件

### 1. 构建门禁

| ID | 检查项 | 方法 | 状态 |
|----|--------|------|------|
| G1 | Release 构建 | cargo build --release --workspace | ❌ PENDING |
| G2 | 全特性构建 | cargo build --all-features | ❌ PENDING |
| G3 | 文档构建 | cargo doc --no-deps | ❌ PENDING |

### 2. 测试门禁

| ID | 检查项 | 标准 | 状态 |
|----|--------|------|------|
| G4 | 单元测试 | cargo test --lib --workspace 100% PASS | ❌ PENDING |
| G5 | 集成测试 | cargo test --workspace >= 90% PASS | ❌ PENDING |
| G6 | 文档测试 | cargo test --doc 100% PASS | ❌ PENDING |
| G7 | Parser 测试 | 169 tests ALL PASS | ✅ 已通过 |
| G8 | Executor 测试 | 250 tests ALL PASS | ✅ 已通过 |
| G9 | WAL 测试 | 63 tests ALL PASS | ✅ 已通过 |

### 3. 代码质量门禁

| ID | 检查项 | 标准 | 状态 |
|----|--------|------|------|
| G10 | Clippy | cargo clippy --all-features -- -D warnings | ✅ 零警告 |
| G11 | Format | cargo fmt --all -- --check | ❌ PENDING |
| G12 | Coverage L1 | avg >= 85% | ❌ 83.01% 未达标 |

### 4. 文档门禁

| ID | 检查项 | 标准 | 状态 |
|----|--------|------|------|
| G13 | GA_GATE_CHECKLIST.md | 存在 | ✅ |
| G14 | USER_MANUAL.md | 存在 | ✅ |
| G15 | API_REFERENCE.md | 存在 | ✅ |
| G16 | RELEASE_NOTES.md | 存在 | ✅ |
| G17 | CHANGELOG.md | 存在 | ✅ |
| G18 | UPGRADE_GUIDE.md | 存在 | ✅ |
| G19 | BENCHMARK.md | 存在 | ✅ |
| G20 | TEST_REPORT.md | 存在 | ✅ |
| G21 | SECURITY_ANALYSIS.md | 存在 | ✅ |

### 5. 性能门禁

| ID | 检查项 | 标准 | 状态 |
|----|--------|------|------|
| G22 | TPC-H SF=1 | 22/22 PASS | ❌ PENDING |
| G23 | QPS 回归 | <= 5% 退化 | ❌ PENDING |

---

## 结果汇总

| 类别 | 通过 | 未通过 | 待定 |
|------|------|--------|------|
| 构建门禁 | 0 | 0 | 3 |
| 测试门禁 | 4 | 0 | 2 |
| 代码质量 | 1 | 1 | 1 |
| 文档门禁 | 9 | 0 | 0 |
| 性能门禁 | 0 | 0 | 2 |
| **总计** | **14** | **1** | **8** |

---

## 阻塞项

1. **Coverage 83.01% < 85%** — 需提升 parser + executor 覆盖率
2. **TPC-H SF=1 未运行** — 需要在 Z6G4 上执行
3. **Release 构建未验证** — 需在 CI 上执行

---

*SSOT 参考: docs/governance/SSOT_CROSS_CHECK.md*
*更新日期: 2026-05-30*
