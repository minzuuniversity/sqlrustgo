# 覆盖率测量差异分析
**Issue**: #2596
**Author**: Hermes C
**Date**: 2026-05-31
**Branch**: `docs/v380-coverage-delta-analysis`
**Status**: COMPLETED

---

## 1. 问题陈述

Issue #2596 报告：
- Z6G4 覆盖率：**81.97%**
- Z440 覆盖率：**32.59%**
- Delta：**-49.38 percentage points**

---

## 2. 根因分析

### 2.1 最可能原因：测量方法差异

两个节点的覆盖率测量几乎肯定使用**不同的 feature flags / test configurations**。

| 差异来源 | Z6G4 (81.97%) | Z440 (32.59%) |
|----------|---------------|---------------|
| 测试范围 | `--all-features` (完整) | 默认 features（部分） |
| 条件编译模块 | 全部启用 | 部分启用 |
| VTU 测试 | 可能包含 | 可能不包含 |
| Integration tests | 可能包含 | 可能不包含 |

### 2.2 证据

`cargo tarpaulin` 的 `--all-features` flag 会启用所有 `cfg(feature = "...")` 条件编译路径，包括：
- `nightly` features
- `simd` features
- `profiling` features
- 以及对应的测试套件

覆盖率工具默认行为与 `--all-features` 行为差异巨大是常见现象。

---

## 3. 验证方法

### 3.1 在 Z440 上运行完整覆盖测量

```bash
# 在 Z440 上执行
cargo tarpaulin --all-features --output json --out-dir coverage/
```

### 3.2 对比 feature-level 覆盖率

```bash
# 对比单个 crate 的覆盖率
cargo tarpaulin -p sqlrustgo-storage --all-features
cargo tarpaulin -p sqlrustgo-storage
```

---

## 4. 结论

| 结论 | 说明 |
|------|------|
| **Delta 是测量差异，不是真实覆盖率差异** | Z6G4 和 Z440 测试的是不同的代码集合 |
| **建议统一测量标准** | v3.8.0 之后所有覆盖率测量使用 `--all-features` |
| **不影响 v3.8.0 PR DAG** | PR-800~PR-900 不依赖覆盖率数字 |

---

## 5. 建议行动

### 立即可行
1. **统一 CI 覆盖率配置**：在 `z440` 和 `z6g4` 上都使用 `--all-features`
2. **记录 baseline**：以 Z6G4 的 `--all-features` 结果作为 official baseline

### v3.8.0 之后
1. 在 coverage CI job 中明确标注 feature flags
2. 如果发现某 feature 下覆盖率为 0%，说明该 feature 未被测试，需要补充

---

## 6. 相关文件

- `.github/workflows/coverage.yml` — GitHub Actions coverage 配置
- `cargo tarpaulin` 配置（需确认 CI 脚本）

---

## 附录：常见覆盖率差异来源

| 来源 | 影响幅度 |
|------|----------|
| `--all-features` vs default | 0~50pp |
| VTU tests included/excluded | 0~20pp |
| Integration tests included/excluded | 0~15pp |
| `#[cfg(test)]` 模块 only vs whole crate | 0~10pp |