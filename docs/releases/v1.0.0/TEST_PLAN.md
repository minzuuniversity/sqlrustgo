# v1.0.0 测试计划

> **类型**: 事后补充（Retrospective）
> **Truthfulness Score**: 60%（历史遗留）
> **生成日期**: 2026-05-29
> **说明**: 无独立测试计划，合并到 RELEASE_GATE_CHECKLIST

---

## 测试策略

v1.0.0 测试计划合并到 **RELEASE_GATE_CHECKLIST** 门禁清单中统一管理。

### 测试层级

| 层级 | 范围 | 门禁要求 |
|------|------|----------|
| L0 | 编译 + 格式检查 | Alpha Gate |
| L1 | 单元测试 (core crates) | Beta Gate |
| L2 | 集成测试 | Beta Gate |
| L3 | E2E 测试 | RC Gate |

---

## 门禁测试映射

### Alpha Gate 测试项

| 检查项 | 测试内容 | 命令 |
|--------|----------|------|
| A1 Build | 编译通过 | `cargo build --all-features` |
| A2 Format | 格式检查 | `cargo fmt -- --check` |
| A3 Clippy | 静态分析 | `cargo clippy -- -D warnings` |

### Beta Gate 测试项

| 检查项 | 测试内容 | 命令 |
|--------|----------|------|
| B1 Unit Test | 单元测试 | `cargo test --lib --all` |
| B2 Integration | 集成测试 | `cargo test --workspace --test '*'` |
| B3 Doctest | 文档测试 | `cargo test --doc` |

### RC Gate 测试项

| 检查项 | 测试内容 | 命令 |
|--------|----------|------|
| R1 Full Test | 全量测试 | `cargo test --all-features` |
| R2 Coverage | 覆盖率检查 | `cargo llvm-cov test --lib` |

### GA Gate 测试项

| 检查项 | 测试内容 | 命令 |
|--------|----------|------|
| G1 Final | 最终验收测试 | `cargo test --all-features --release` |
| G2 Security | 安全扫描 | `cargo audit` |

---

## 测试覆盖

### 核心模块覆盖率目标

| 模块 | 覆盖率目标 | 状态 |
|------|------------|------|
| parser | ≥ 70% | ✅ |
| planner | ≥ 70% | ✅ |
| executor | ≥ 70% | ✅ |
| storage | ≥ 70% | ✅ |
| transaction | ≥ 70% | ✅ |

---

## 测试命令

```bash
# 编译测试
cargo build --all-features

# 单元测试
cargo test --lib --all

# 集成测试
cargo test --workspace --test '*'

# 覆盖率
cargo llvm-cov test --lib

# 安全扫描
cargo audit
```

---

## 测试文档

完整测试计划见: `docs/v1.0/rc1/验收文档/门禁验收/RC1门禁验收清单.md`

---

*最后更新: 2026-02-21*
*Truthfulness Score: 60%*