# v3.6.0 Beta Gate Checklist

> **版本**: 1.0
> **创建日期**: 2026-05-29
> **维护人**: hermes-agent
> **分支**: `develop/v3.6.0`
> **前置条件**: Alpha Gate 全部通过

---

## 一、检查清单

### B1 ~ B5: 基础构建（继承 v3.5.0）

| # | 检查项 | 命令 | 标准 | 通过 |
|---|--------|------|------|------|
| B1 | 编译检查 | `cargo build --release --workspace` | exit_code=0 | ⬜ |
| B2 | 单元测试 | `cargo test --workspace` | 100% 通过 | ⬜ |
| B3 | Clippy 检查 | `cargo clippy --all-features -- -D warnings` | 零警告 | ⬜ |
| B4 | 格式化检查 | `cargo fmt --all -- --check` | 无格式错误 | ⬜ |
| B5 | 文档链接检查 | `bash scripts/gate/check_docs_links.sh` | 无死链 | ⬜ |

### B6 ~ B8: 架构迁移（v3.6.0 Beta 目标）

| # | 检查项 | 命令 | 标准 | 通过 |
|---|--------|------|------|------|
| B6 | Pipeline 迁移进度 | `cargo test -p sqlrustgo-executor` | 30% operator 已迁移 | ⬜ |
| B7 | SIMD 向量化 | `cargo test -p sqlrustgo-executor -- vectorized` | 3/4 模块完成 | ⬜ |
| B8 | Spill Framework | `cargo test -p sqlrustgo-executor -- spill` | HashJoin/Sort/Agg 可 spill | ⬜ |

### B9 ~ B10: Benchmark Gate（v3.6.0 新增）

| # | 检查项 | 命令 | 标准 | 通过 |
|---|--------|------|------|------|
| B9 | Benchmark CI | `cargo bench --save-baseline` | 6/8 场景完成 | ⬜ |
| B10 | Regression CI | `cargo bench --compare-to main` | 无性能回退 | ⬜ |

---

## 二、覆盖率要求

> **Beta 覆盖率阈值**: ≥75%（L1 crates，综合方法）

**覆盖率命令**:
```bash
for crate in "${L1_CRATES[@]}"; do
    cargo llvm-cov test --package "$crate" --all-features --tests 2>/dev/null | grep "^TOTAL"
done
```

---

## 三、Beta 通过条件

| 条件 | 说明 |
|------|------|
| **B1~B5 全部 PASS** | 基础构建无退化 |
| **B6~B8 全部 PASS** | 架构迁移达标 |
| **B9~B10 全部 PASS** | Benchmark Gate 就绪 |
| **覆盖率 ≥75%** | L1 crates 综合覆盖率 |

---

## 四、执行记录

| 执行时间 | 执行者 | 结果 | 备注 |
|----------|--------|------|------|
| — | — | — | — |
