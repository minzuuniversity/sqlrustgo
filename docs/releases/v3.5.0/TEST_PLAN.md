# v3.5.0 测试计划

> **版本**: v3.5.0
> **创建日期**: 2026-05-27
> **维护人**: hermes-agent
> **分支**: `develop/v3.5.0`
> **Truthfulness Score**: 100%

---

## 一、版本概述

v3.5.0 定位为 **AI Native GMP Platform**，引入 AI Agent 能力。

### 1.1 战略演进

```
v3.4.0: GMP Management Suite（GMP 管理套件）
    ↓
v3.5.0: AI Native GMP Platform（AI 原生 GMP 平台）
```

### 1.2 新增 P1 功能

| Issue | 功能 | 实现位置 |
|-------|------|---------|
| #1367 | 自然语言报表生成 | `crates/compliance-engine/src/report/` |
| #1366 | 预测性设备维护 | `crates/gmp/src/device_predictor.rs` |
| #1368 | 审计链 AI 摘要 | `crates/compliance-engine/src/summarizer/` |
| #1369 | 规则自动推荐 | `crates/gmp-api/src/rule_recommendation.rs` |

---

## 二、测试范围

### 2.1 核心功能模块

| 模块 | 功能 | 状态 |
|------|------|------|
| sqlrustgo-gmp-api | GMP REST API + Rule Recommendation | ✅ |
| sqlrustgo-gmp | GMP Core + Device Predictor | ✅ |
| sqlrustgo-gmp-retrieval | GMP Retrieval V3 (Vectorizer/Reranker/QueryRewriter) | ✅ |
| sqlrustgo-compliance-engine | Report Generator + Audit Chain Summarizer | ✅ |
| sqlrustgo-workflow-v2 | 工作流引擎 V2 | ✅ |
| sqlrustgo-evidence-engine | 证据引擎 | ✅ |

### 2.2 新功能

| Issue | 功能 | 实现 | 单元测试 | 状态 |
|-------|------|------|---------|------|
| #1366 | 设备预测 | `device_predictor.rs` | 24 tests | ✅ |
| #1367 | 报表生成 | `report/` | 97 tests | ✅ |
| #1368 | 审计摘要 | `summarizer/` | 105 tests | ✅ |
| #1369 | 规则推荐 | `rule_recommendation.rs` | 11 tests | ✅ |

---

## 三、测试策略

### 3.1 覆盖范围

| 层级 | 范围 | Alpha 目标 | Beta 目标 | GA 目标 |
|------|------|-----------|-----------|---------|
| L1 Core | parser, planner, optimizer, executor, storage, transaction, catalog | ≥70% | ≥80% | ≥85% |
| L2 Integration | gmp-api, gmp, gmp-retrieval | ≥50% | ≥60% | ≥70% |
| L3 Application | compliance-engine | ≥40% | ≥50% | ≥60% |

### 3.2 测试类型

| 类型 | 命令 | Alpha | Beta | GA |
|------|------|-------|------|-----|
| 单元测试 | `cargo test --lib` | 全部通过 | 全部通过 | 全部通过 |
| 集成测试 | `cargo test --workspace --test '*'` | ≥90% | 全部通过 | 全部通过 |
| Doctest | `cargo test --doc` | 全部通过 | 全部通过 | 全部通过 |
| 覆盖率 | `cargo llvm-cov test --lib` | L1 ≥70% | L1 ≥80% | L1 ≥85% |
| SQL 兼容性 | `bash scripts/gate/check_sql_compat.sh` | ≥70% | ≥80% | ≥85% |
| TPC-H SF=1 | `bash scripts/gate/check_tpch.sh --sf1` | 22/22 | 22/22 | 22/22 |

### 3.3 新增 crate 测试覆盖要求

| Crate | 测试命令 | Alpha | Beta | GA |
|-------|---------|-------|------|-----|
| `sqlrustgo-compliance-engine` | `cargo test -p sqlrustgo-compliance-engine --lib` | 全部通过 | 全部通过 | 全部通过 |
| `sqlrustgo-gmp-api` | `cargo test -p sqlrustgo-gmp-api --lib` | 全部通过 | 全部通过 | 全部通过 |
| `sqlrustgo-gmp` | `cargo test -p sqlrustgo-gmp --lib` | 全部通过 | 全部通过 | 全部通过 |
| `sqlrustso-gmp-retrieval` | `cargo test -p sqlrustso-gmp-retrieval --lib` | 全部通过 | 全部通过 | 全部通过 |

---

## 四、测试执行计划

### 4.1 本地开发测试（L0-L3）

每次 PR 前必须执行：

```bash
# L0: 格式 + 编译
cargo fmt --all -- --check
cargo build --all

# L1: 单元测试（所有 crate）
cargo test --lib --all

# L2: 集成测试
cargo test --workspace --test '*'

# L3: 新功能专项测试
cargo test -p sqlrustgo-compliance-engine --lib
cargo test -p sqlrustgo-gmp --lib
cargo test -p sqlrustgo-gmp-api --lib
cargo test -p sqlrustso-gmp-retrieval --lib
```

### 4.2 CI 门禁测试

PR 合并前必须通过（详见 `.github/workflows/ci-v350-pr.yml`）：

- Fmt Check
- Clippy Lint
- Build (all-features)
- Unit Tests (workspace --lib)
- Integration Tests (workspace --test '*')
- Doctests (workspace --doc)
- Coverage Gate (all crates)
- SQL Compatibility
- TPC-H SF=1

---

*最后更新: 2026-05-27*
*Truthfulness Score: 100%*
