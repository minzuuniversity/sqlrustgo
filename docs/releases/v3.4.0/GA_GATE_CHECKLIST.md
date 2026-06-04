# v3.4.0 GA Gate Checklist

> **版本**: v3.4.0-ga-gate
> **创建日期**: 2026-05-21
> **维护人**: hermes-agent
> **阶段**: GA (General Availability)
|> **分支**: `origin/develop/v3.4.0` (commit `bd39e787`)
|> **起点**: `a1ddb44c` (v3.3.0 GA release point)
> **关联门禁规范**: `docs/governance/GATE_SPEC_MASTER.md`

---

## 一、门禁信息

### 1.1 版本概述

**v3.4.0 战略定位**: GMP Management Suite（管理套件）

**核心价值**: 在 v3.3.0 Trust Infrastructure 可信内核基础上，构建面向终端用户的 GMP 管理界面（Web Dashboard + EBR + 设备集成）

**版本演进**:
```
v3.2.x: 内核修复（覆盖率/性能/MySQL协议）
    ↓
v3.3.0: Industrial Trust Platform（可信性工业闭环）✅ GA
    ↓
v3.4.0: GMP Management Suite（管理套件）← 当前
    ↓
v3.5.0: AI Native GMP Platform
```

### 1.2 入口条件

- [x] v3.3.0 GA 通过（v3.3.0 tag at `a1ddb44c`）
- [x] 所有 Trust Infrastructure 内核稳定
- [x] Alpha Gate 通过（✅ 16/16 PASS, 2026-05-22）
- [x] Beta Gate 通过（✅ 14/14 PASS, 2026-05-22）

### 1.3 新增 Crates（v3.4.0）

| Crate | 版本 | 关键功能 |
|-------|------|----------|
| `sqlrustgo-gmp-api` | 0.1.0 | GMP Management REST API (Batch/ Audit/ Device/ Signature/ Export/ Dashboard/ RuleEngine) |
| `sqlrustgo-gmp-retrieval` | — | BM25 + RRF Fusion + Ollama Reranker + LLM Chat (v2) |

### 1.4 增强 Crates（v3.4.0）

| Crate | 增强内容 |
|-------|----------|
| `sqlrustgo-gmp` | Workflow V2 types + integration tests, Trust Visualization CLI module |
| `sqlrustgo-mysql-server` | Built-in function support for SELECT |

---

## 二、Pre-Gate 自检清单

### 2.1 代码质量检查

| 检查项 | 命令 | 期望结果 | 状态 |
|--------|------|----------|------|
| cargo build | `cargo build --release --workspace` | 编译成功 | ✅ 38.09s |
| cargo clippy | `cargo clippy --all-features -- -D warnings` | 零警告 | ✅ (manifest unused key warning 除外) |
| cargo fmt | `cargo fmt --check` | 通过 | ✅ |
| cargo test | `cargo test --lib` | 全部通过 | ⏳ (workspace 测试运行中) |

### 2.2 覆盖率检查

> **覆盖率测量方法**: L1 CRATES 使用以下命令：
> ```bash
> cargo llvm-cov test \
>     -p sqlrustgo-types \
>     -p sqlrustgo-parser \
>     -p sqlrustgo-planner \
>     -p sqlrustgo-optimizer \
>     -p sqlrustgo-executor \
>     -p sqlrustgo-storage \
>     -p sqlrustgo-transaction \
>     -p sqlrustgo-catalog \
>     --lib
> ```

| 检查项 | 命令 | 期望结果 | 实际结果 | 状态 |
|--------|------|----------|----------|------|
| L1 覆盖率 | `cargo llvm-cov test L1_CRATES --lib` | ≥85% | 待测量 | ⏳ |

---

## 三、正式门禁检查 (GA)

### 3.1 核心检查 (G1-G12)

|| # | 检查项 | 命令 | 期望结果 | 状态 |
|---|--------|------|----------|------|
| ✅ | G1 | Build | `cargo build --release` | 成功 | ✅ 38.09s |
| ⏳ | G2 | Test | `cargo test --lib` | 全部通过 | ⏳ workspace 测试运行中 |
| ✅ | G3 | Clippy | `cargo clippy --all-features -- -D warnings` | 零警告 | ✅ (manifest unused key 除外) |
| ✅ | G4 | Format | `cargo fmt --check` | 通过 | ✅ |
| ⏳ | G5 | Coverage | `cargo llvm-cov test L1_CRATES --lib` | ≥85% | ⏳ |
| ⏳ | G6 | Security | `cargo audit` | 无漏洞 | ⏳ |
| ⏳ | G7 | GMP API Build | `cargo build -p sqlrustgo-gmp-api` | 成功 | ⏳ |
| ⏳ | G8 | GMP Retrieval Build | `cargo build -p sqlrustgo-gmp-retrieval` (if exists) | 成功 | ⏳ |
| ✅ | G9 | MySQL Server | `cargo build -p sqlrustgo-mysql-server` | 成功 | ✅ |
| ⏳ | G10 | TPC-H SF=1 | `check_tpch.sh --sf1` | 22/22 | ⏳ |
| ⏳ | G11 | Proofs | TLA+ model check | ≥30 | ⏳ |
| ⏳ | G12 | Docs | All OO docs | 全部存在 | ⏳ |

### 3.2 GMP Management API 测试 (G-API1~G-API7)

| # | 检查项 | 命令 | 期望结果 | 状态 |
|---|--------|------|----------|------|
| G-API1 | gmp-api build | `cargo build -p sqlrustgo-gmp-api` | 成功 | ⏳ |
| G-API2 | gmp-api lib test | `cargo test -p sqlrustgo-gmp-api --lib` | 全部通过 | ⏳ |
| G-API3 | Batch CRUD | `cargo test -p sqlrustgo-gmp-api -- batch` | 通过 | ⏳ |
| G-API4 | Audit API | `cargo test -p sqlrustgo-gmp-api -- audit` | 通过 | ⏳ |
| G-API5 | Device API | `cargo test -p sqlrustgo-gmp-api -- device` | 通过 | ⏳ |
| G-API6 | Signature API | `cargo test -p sqlrustgo-gmp-api -- signature` | 通过 | ⏳ |
| G-API7 | Dashboard API | `cargo test -p sqlrustgo-gmp-api -- dashboard` | 通过 | ⏳ |

### 3.3 GMP 核心测试 (G-GMP1~G-GMP8)

| # | 检查项 | 命令 | 期望结果 | 状态 |
|---|--------|------|----------|------|
| G-GMP1 | gmp lib tests | `cargo test -p sqlrustgo-gmp --lib` | 全部通过 | ⏳ |
| G-GMP2 | Audit Chain | `cargo test -p sqlrustgo-gmp --test gmp_audit_chain_verify_test` | 通过 | ⏳ |
| G-GMP3 | Digital Signature | `cargo test -p sqlrustgo-gmp --test gmp_digital_signature_test` | 通过 | ⏳ |
| G-GMP4 | Electronic Signature | `cargo test -p sqlrustgo-gmp --test gmp_electronic_signature_test` | 通过 | ⏳ |
| G-GMP5 | Workflow V2 | `cargo test -p sqlrustgo-gmp --test gmp_workflow_v2_test` | 通过 | ⏳ |
| G-GMP6 | Evidence Engine | `cargo test -p sqlrustgo-gmp --test evidence_export_test` | 通过 | ⏳ |
| G-GMP7 | Immutable Record | `cargo test -p sqlrustgo-gmp --test gmp_immutable_record_test` | 通过 | ⏳ |
| G-GMP8 | Provenance | `cargo test -p sqlrustgo-gmp --test gmp_provenance_test` | 通过 | ⏳ |

### 3.4 Trust Infrastructure 稳定性 (G-TI1~G-TI8)

| # | 检查项 | 命令 | 期望结果 | 状态 |
|---|--------|------|----------|------|
| G-TI1 | evidence-engine | `cargo test -p sqlrustgo-evidence-engine --lib` | 通过 | ⏳ |
| G-TI2 | provenance-graph | `cargo test -p sqlrustgo-provenance-graph --lib` | 通过 | ⏳ |
| G-TI3 | compliance-engine | `cargo test -p sqlrustgo-compliance-engine --lib` | 通过 | ⏳ |
| G-TI4 | workflow-v2 | `cargo test -p sqlrustgo-workflow-v2 --lib` | 通过 | ⏳ |
| G-TI5 | trust-viz | `cargo test -p sqlrustgo-trust-viz --lib` | 通过 | ⏳ |
| G-TI6 | perf-baseline | `cargo test -p sqlrustgo-perf-baseline --lib` | 通过 | ⏳ |
| G-TI7 | crash-sim | `cargo test -p sqlrustgo-crash-sim --lib` | 通过 | ⏳ |
| G-TI8 | wal-verification | `cargo test -p sqlrustgo-wal-verification --lib` | 通过 | ⏳ |

### 3.5 遗留豁免复审

| ID | 豁免项 | 原版本 | 复审条件 | 状态 |
|----|--------|--------|----------|------|
| EX-v330-001 | executor 覆盖率 <85% | v3.3.0 | ≥85% | ⏳ |

---

## 四、检查结果汇总

### 4.1 结果汇总表

|| 类别 | 通过数 | 总数 | 通过率 | 状态 |
|------|--------|------|--------|------|
| 核心检查 G1-G12 | 4 | 12 | 33% | 🔄 |
| GMP API G-API1~7 | 0 | 7 | 0% | ⏳ |
| GMP 核心 G-GMP1~8 | 0 | 8 | 0% | ⏳ |
| Trust Infra G-TI1~8 | 0 | 8 | 0% | ⏳ |
| **总计** | **4** | **35** | **11%** | **🔄** |

### 4.2 GA Gate 执行模板

```bash
#!/usr/bin/env bash
# check_ga_v340.sh — v3.4.0 GA Gate
set -e
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

echo "=== v3.4.0 GA Gate ==="
echo "分支: $(git branch --show-current)"
echo "Commit: $(git rev-parse --short HEAD)"
echo ""

# Pre-check
cargo build --release --workspace
cargo test --all-features --lib
cargo clippy --all-features -- -D warnings
cargo fmt --all -- --check

# Coverage (Z6G4)
cargo llvm-cov test --lib \
    -p sqlrustgo-types \
    -p sqlrustgo-parser \
    -p sqlrustgo-planner \
    -p sqlrustgo-optimizer \
    -p sqlrustgo-executor \
    -p sqlrustgo-storage \
    -p sqlrustgo-transaction \
    -p sqlrustgo-catalog

# GMP API tests
cargo test -p sqlrustgo-gmp-api --lib
cargo test -p sqlrustgo-gmp --lib

# Trust Infrastructure
cargo test -p sqlrustgo-evidence-engine --lib
cargo test -p sqlrustgo-provenance-graph --lib
cargo test -p sqlrustgo-compliance-engine --lib
cargo test -p sqlrustgo-workflow-v2 --lib
cargo test -p sqlrustgo-trust-viz --lib
cargo test -p sqlrustgo-perf-baseline --lib
cargo test -p sqlrustgo-crash-sim --lib
cargo test -p sqlrustgo-wal-verification --lib
```

---

## 五、Issue 状态

### 5.1 v3.4.0 完成 Issue

| # | Issue | 标题 | PR | 状态 |
|---|-------|------|-----|------|
| #1260 | P0 | EBR core implementation | #1260 | ✅ |
| #1261 | P0 | Electronic Signature Service | #1261 | ✅ |
| #1262 | P1 | OPC UA Device Integration | #1262 | ✅ |
| #1263 | P1 | Rule Engine Visual Editor | #1263 | ✅ |
| — | P0 | gmp-retrieval v2 (BM25+RRF+Reranker+LLM) | #1297 | ✅ |
| — | P1 | Workflow V2 integration tests | — | ✅ |

### 5.2 待完成 Issue

| # | Issue | 标题 | 优先级 |
|---|-------|------|--------|
| TBD | P0 | Web Dashboard UI | 🔄 |
| TBD | P1 | MQTT Device Integration | 🔄 |
| TBD | P2 | Mobile Approval API | 🔄 |
| TBD | P2 | Compliance Template Library | 🔄 |

---

## 六、附录

### A.1 相关文档

- 门禁规范: `docs/governance/GATE_SPEC_MASTER.md`
- 开发计划: `docs/releases/v3.4.0/DEV_PLAN.md`
- v3.3.0 GA Gate: `docs/releases/v3.3.0/GA_GATE_CHECKLIST.md`
- Trust Infrastructure: `docs/releases/v3.3.0/OO/Trust-Infrastructure/`

### A.2 关键文件路径

| 文件 | 路径 |
|------|------|
| GMP API | `crates/gmp-api/src/lib.rs` |
| GMP Retrieval v2 | `crates/gmp-retrieval/` |
| GMP Core | `crates/gmp/src/lib.rs` |
| GA Gate 脚本 | `scripts/gate/check_ga_v340.sh` |

### A.3 GMP-Platform 集成待办（来自 ANALYSIS_REPORT）

| 优先级 | 任务 | 说明 |
|--------|------|------|
| P0 | 向量索引全量构建 | sqlrustgo-vector HNSW 替换 SQLite BLOB |
| P0 | GraphChannel 替换 | sqlrustgo-graph Cypher 替换关键词匹配 |
| P1 | 消除 search_sync Runtime 泄漏 | 统一 Runtime 管理 |

---

*最后更新: 2026-05-21*
