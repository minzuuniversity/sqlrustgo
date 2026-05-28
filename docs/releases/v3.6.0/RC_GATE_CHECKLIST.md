# v3.6.0 RC Gate Checklist

> **版本**: 1.0
> **创建日期**: 2026-05-29
> **维护人**: hermes-agent
> **分支**: `develop/v3.6.0`
> **前置条件**: Beta Gate 全部通过

---

## 一、检查清单

### R1 ~ R5: 基础构建（继承 v3.5.0）

| # | 检查项 | 命令 | 标准 | 通过 |
|---|--------|------|------|------|
| R1 | 编译检查 | `cargo build --release --workspace` | exit_code=0 | ⬜ |
| R2 | 单元测试 | `cargo test --workspace` | 100% 通过 | ⬜ |
| R3 | Clippy 检查 | `cargo clippy --all-features -- -D warnings` | 零警告 | ⬜ |
| R4 | 格式化检查 | `cargo fmt --all -- --check` | 无格式错误 | ⬜ |
| R5 | 覆盖率检查 | `cargo llvm-cov test L1_CRATES` | ≥80%（L1，综合方法） | ⬜ |

### R6 ~ R8: 架构完成（v3.6.0 RC 目标）

| # | 检查项 | 命令 | 标准 | 通过 |
|---|--------|------|------|------|
| R6 | Pipeline 100% 迁移 | `cargo test -p sqlrustgo-executor` | 所有 operator 已迁移 | ⬜ |
| R7 | SIMD 4/4 完成 | `cargo test -p sqlrustgo-executor -- vectorized` | 4/4 模块 | ⬜ |
| R8 | Spill 三大 Operator | `cargo test -p sqlrustgo-executor -- spill` | HashJoin/Sort/Agg 可 spill | ⬜ |

### R9 ~ R10: 分布式地基 + Evidence AI（v3.6.0 新增）

| # | 检查项 | 命令 | 标准 | 通过 |
|---|--------|------|------|------|
| R9 | Exchange 接口 | `cargo build -p sqlrustgo-executor` | ExchangeOperator trait 定义 | ⬜ |
| R10 | Evidence AI P0 | `cargo test -p sqlrustgo-gmp-llm` | SQL-Verified 流程完成 | ⬜ |

### R11 ~ R12: Benchmark Gate（v3.6.0 新增）

| # | 检查项 | 命令 | 标准 | 通过 |
|---|--------|------|------|------|
| R11 | TPC-H SF=1 | `./target/release/sqlrustgo-bench-cli tpch --sf 1` | 22/22 PASS ≤180s | ⬜ |
| R12 | Benchmark 回归 | `cargo bench --compare-to main` | 无 >15% 回退 | ⬜ |

---

## 二、覆盖率要求

> **RC 覆盖率阈值**: ≥80%（L1 crates，综合方法）

**覆盖率命令**:
```bash
for crate in "${L1_CRATES[@]}"; do
    cargo llvm-cov test --package "$crate" --all-features --tests 2>/dev/null | grep "^TOTAL"
done
```

---

## 三、RC 通过条件

| 条件 | 说明 |
|------|------|
| **R1~R5 全部 PASS** | 基础构建无退化 |
| **R6~R8 全部 PASS** | 架构统一完成 |
| **R9~R10 全部 PASS** | 新能力就绪 |
| **R11~R12 全部 PASS** | Benchmark Gate 完成 |
| **覆盖率 ≥80%** | L1 crates 综合覆盖率 |

---

## 四、执行记录

| 执行时间 | 执行者 | 结果 | 备注 |
|----------|--------|------|------|
| — | — | — | — |
