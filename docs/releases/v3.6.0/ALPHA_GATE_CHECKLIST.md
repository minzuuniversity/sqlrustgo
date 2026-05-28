# v3.6.0 Alpha Gate Checklist

> **版本**: 1.0
> **创建日期**: 2026-05-29
> **维护人**: hermes-agent
> **分支**: `develop/v3.6.0`
> **目标**: Architecture Convergence — Unified Execution Architecture

---

## 一、检查清单

### A1 ~ A5: 基础构建（继承 v3.5.0）

| # | 检查项 | 命令 | 标准 | 证据模板 | 通过 |
|---|--------|------|------|---------|------|
| A1 | 编译检查 | `cargo build --release --workspace` | exit_code=0 | `{command, exit_code}` | ⬜ |
| A2 | 单元测试 | `cargo test --lib` | 100% 通过 | `{passed, failed, exit_code}` | ⬜ |
| A3 | Clippy 检查 | `cargo clippy --all-features -- -D warnings` | 零警告 | `{warnings, exit_code}` | ⬜ |
| A4 | 格式化检查 | `cargo fmt --all -- --check` | 无格式错误 | `{diff_count, exit_code}` | ⬜ |
| A5 | 文档链接检查 | `bash scripts/gate/check_docs_links.sh` | 无死链 | `{broken_links}` | ⬜ |

### A6 ~ A8: 架构统一（v3.6.0 新增）

| # | 检查项 | 命令 | 标准 | 证据模板 | 通过 |
|---|--------|------|------|---------|------|
| A6 | Chunk 模型编译 | `cargo build -p sqlrustgo-executor` | 新增 chunk/ 模块编译通过 | `{module, exit_code}` | ⬜ |
| A7 | Pipeline 基础编译 | `cargo build -p sqlrustgo-executor` | 新增 pipeline/ 模块编译通过 | `{module, exit_code}` | ⬜ |
| A8 | SIMD 向量化编译 | `cargo build -p sqlrustgo-executor` | vectorized/ 模块编译通过 | `{module, exit_code}` | ⬜ |

### A9 ~ A11: Benchmark Gate 基础（v3.6.0 新增）

| # | 检查项 | 命令 | 标准 | 证据模板 | 通过 |
|---|--------|------|------|---------|------|
| A9 | Benchmark 框架 | `cargo bench --list` | 列出所有 benchmark | `{count, scenarios}` | ⬜ |
| A10 | TPC-H SF=1 基线 | `./target/release/sqlrustgo-bench-cli tpch --sf 1` | 22/22 PASS | `{passed, total, time}` | ⬜ |
| A11 | Point Select 基线 | `./target/release/sqlrustgo-bench-cli point-select` | QPS 记录 | `{qps, latency}` | ⬜ |

---

## 二、覆盖率要求

> **Alpha 覆盖率阈值**: ≥72%（L1 crates，综合方法）

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

---

## 三、执行记录

| 执行时间 | 执行者 | 结果 | 备注 |
|----------|--------|------|------|
| 2026-05-29 | hermes-agent | ⬜ 进行中 | initial |

---

## 四、Alpha 通过条件

| 条件 | 说明 |
|------|------|
| **A1~A5 全部 PASS** | 基础构建无退化 |
| **A6~A8 全部 PASS** | 架构模块编译通过 |
| **A9~A11 全部 PASS** | Benchmark 框架就绪 |
| **覆盖率 ≥72%** | L1 crates 综合覆盖率 |

---

## 五、文档要求

| 文档 | 状态 | 说明 |
|------|------|------|
| DEV_PLAN.md | ⬜ | v3.6.0 开发计划 |
| ARCHITECTURE_UNIFICATION.md | ⬜ | 架构统一设计方案 |
| CHUNK_DESIGN.md | ⬜ | Chunk 数据结构设计 |
| PIPELINE_DESIGN.md | ⬜ | Pipeline 执行器设计 |
