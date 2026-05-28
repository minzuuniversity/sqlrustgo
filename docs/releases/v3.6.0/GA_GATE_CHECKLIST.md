# v3.6.0 GA Gate Checklist

> **版本**: 1.0
> **创建日期**: 2026-05-29
> **维护人**: hermes-agent
> **分支**: `develop/v3.6.0`
> **前置条件**: RC Gate 全部通过

---

## 一、检查清单

### G1 ~ G5: 基础构建（继承 v3.5.0）

| # | 检查项 | 命令 | 标准 | 通过 |
|---|--------|------|------|------|
| G1 | Release 编译 | `cargo build --release -p sqlrustgo` | exit_code=0 | ⬜ |
| G2 | 全 workspace 测试 | `cargo test --workspace` | 100% 通过 | ⬜ |
| G3 | Clippy 零警告 | `cargo clippy --all-features -- -D warnings` | 零警告 | ⬜ |
| G4 | Format 合规 | `cargo fmt --all -- --check` | 无 diff | ⬜ |
| G5 | 覆盖率检查 | `cargo llvm-cov test L1_CRATES`（综合方法） | ≥85% | ⬜ |

### G6 ~ G8: 架构统一完成（v3.6.0 GA）

| # | 检查项 | 命令 | 标准 | 通过 |
|---|--------|------|------|------|
| G6 | Pipeline 100% | `cargo test -p sqlrustgo-executor` | 所有 operator 走 Pipeline | ⬜ |
| G7 | SIMD 完整 | `cargo test -p sqlrustgo-executor -- vectorized` | 4/4 模块 + 性能提升 | ⬜ |
| G8 | Spill Framework | `cargo test -p sqlrustgo-executor -- spill` | 三大 Operator 可 spill | ⬜ |

### G9 ~ G10: 分布式地基 + Evidence AI

| # | 检查项 | 命令 | 标准 | 通过 |
|---|--------|------|------|------|
| G9 | Exchange Operator | `cargo test -p sqlrustgo-executor -- exchange` | LocalExchange 可用 | ⬜ |
| G10 | Evidence AI | `cargo test -p sqlrustgo-gmp-llm` | SQL-Verified + G20 Evidence Gate | ⬜ |

### G11 ~ G12: Benchmark Gate

| # | 检查项 | 命令 | 标准 | 通过 |
|---|--------|------|------|------|
| G11 | TPC-H SF=1 | `./target/release/sqlrustgo-bench-cli tpch --sf 1` | 22/22 PASS ≤180s | ⬜ |
| G12 | Benchmark 回归 | `cargo bench --compare-to main` | 无 >15% 回退 | ⬜ |

---

## 二、覆盖率要求

> **GA 覆盖率阈值**: ≥85%（L1 crates，综合方法）

**L1 Crates**:
```bash
L1_CRATES=(
    sqlrustgo-types
    sqlrustgo-parser
    sqlrustgo-planner
    sqlrustgo-optimizer
    sqlrustgo-executor
    sqlrustgo-storage
    sqlrustgo-transaction
    sqlrustgo-catalog
)
```

**覆盖率命令**:
```bash
# 综合方法：--tests 优先，--lib fallback
for crate in "${L1_CRATES[@]}"; do
    cargo llvm-cov test --package "$crate" --all-features --tests 2>/dev/null | grep "^TOTAL"
done
```

**Per-crate 覆盖率**:
| Crate | 目标 |
|-------|------|
| sqlrustgo-types | ≥85% |
| sqlrustgo-parser | ≥85% |
| sqlrustgo-planner | ≥85% |
| sqlrustgo-optimizer | ≥85% |
| sqlrustgo-executor | ≥85% |
| sqlrustgo-storage | ≥85% |
| sqlrustgo-transaction | ≥85% |
| sqlrustgo-catalog | ≥85% |
| **平均** | **≥85%** |

---

## 三、覆盖率测量方法说明

**方法**: 综合方法（`--tests` 优先，`--lib` fallback）

> 原因：`--lib only` 会忽略外部集成测试的覆盖贡献，最高可达 +6.30pp（executor）。此方法已在 RC2 确立（commit aa830bcd），所有 gate 统一使用。

```bash
# 每个 crate 单独测量
for crate in "${L1_CRATES[@]}"; do
    # 优先使用 --tests（更广覆盖）
    cargo llvm-cov test --package "$crate" --all-features --tests 2>/dev/null | grep "^TOTAL"
done
```

---

## 四、GA 通过条件

| 条件 | 说明 |
|------|------|
| **G1~G5 全部 PASS** | 基础构建无退化 |
| **G6~G8 全部 PASS** | 架构统一完成 |
| **G9~G10 全部 PASS** | 新能力就绪 |
| **G11~G12 全部 PASS** | Benchmark Gate 完成 |
| **覆盖率 ≥85%** | L1 crates 综合覆盖率 |

---

## 五、执行记录

| 执行时间 | 执行者 | 结果 | 备注 |
|----------|--------|------|------|
| — | — | — | — |

---

## 六、文档要求

| 文档 | 状态 | 说明 |
|------|------|------|
| GA_GATE_REPORT.md | ⬜ | GA 门禁报告 |
| ARCHITECTURE_CONVERGENCE.md | ⬜ | 架构统一总结 |
| PIPELINE_MIGRATION_REPORT.md | ⬜ | Pipeline 迁移报告 |
| BENCHMARK_REPORT.md | ⬜ | Benchmark 结果报告 |
