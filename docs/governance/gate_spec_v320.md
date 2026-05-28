# v3.2.0 GA Gate 规范 (Gate Specification)

> **版本**: 1.0
> **创建日期**: 2026-05-29（基于 check_ga_v320.sh 反向工程）
> **维护人**: hermes-z440
> **适用版本**: v3.2.0
> **前置版本**: v3.1.0 GA
> **依据**: `scripts/gate/check_ga_v320.sh` + `docs/releases/v3.2.0/GA_GATE_CHECKLIST.md`

> **SSOT 声明**: `gate_spec_v320.md` 是 v3.2.0 GA 门禁定义的唯一权威来源。

---

## 一、门禁概述

v3.2.0 GA Gate 共定义 14 个检查项，分 4 类：

| 类别 | 检查项 | 说明 |
|------|--------|------|
| 代码质量 (G1-G6) | Build / Test / Clippy / Format / Coverage / Security | 无错误通过 |
| 文档完整性 (G11) | 所有 OO 文档存在性 | 14/14 存在 |
| 性能基准 (G7-G9) | SQL Compat / TPC-H SF=1 / Sysbench | 达标 |
| 功能验证 (G10, G12) | Formal Proofs / MySQL Protocol | 验证通过 |

---

## 二、G1-G14 检查清单

### 2.1 代码层 Gate（G1-G6）

| # | 检查项 | 命令/方法 | 通过标准 | 证据格式 |
|---|--------|----------|---------|---------|
| G1 | Build | `cargo build --release --workspace` | 无错误 | `{exit_code}` |
| G2 | Test | `cargo test --lib` (workspace, lib tests only) | 全部通过 | `{passed, failed}` |
| G3 | Clippy | `cargo clippy --all-features -- -D warnings` | 零警告 | `{warnings, exit_code}` |
| G4 | Format | `cargo fmt --all -- --check` | 无格式错误 | `{diff_count}` |
| G5 | Coverage | `cargo llvm-cov test L1_CRATES --lib` | **≥85%** | `{total_pct}` |
| G6 | Security | `cargo audit` | 无高危漏洞 | `{vulnerabilities}` |

> **G5 实测结果**: 85.81% (GA_GATE_CHECKLIST.md L67)
> **注意**: v3.2.0 使用 `--lib` 而非 `--all-features --workspace`。L1_CRATES 包含 5 个核心 crate。

### 2.2 功能层 Gate（G7-G9）

| # | 检查项 | 命令/方法 | 通过标准 | 证据格式 |
|---|--------|----------|---------|---------|
| G7 | SQL Compat | `bash scripts/gate/check_sql_compat.sh` | ≥85% MySQL | `{pass_rate}` |
| G8 | TPC-H SF=1 | `bash scripts/gate/check_tpch.sh sf=1` | 22/22 无 OOM | `{passed, total}` |
| G9 | Sysbench | sysbench point/update/delete | 目标 QPS | `{qps}` |

> **G8 实测结果**: 22/22 通过 (GA_GATE_CHECKLIST.md L70)
> **G9 注意**: 需服务器环境，GA_GATE_CHECKLIST 标记为"需服务器"（⬜ 未执行）

### 2.3 特殊验证 Gate（G10-G12）

| # | 检查项 | 命令/方法 | 通过标准 | 证据格式 |
|---|--------|----------|---------|---------|
| G10 | Formal Proofs | TLA+ model check | ≥30 proofs | `{proof_count}` |
| G11 | OO Docs | 检查 14 个 OO 文档 | 全部存在 | `{missing}` |
| G12 | MySQL Protocol | `cargo test -p sqlrustgo-mysql-server --test mysql_protocol` | 验证通过 | `{exit_code}` |

> **G11 实测结果**: 14/14 全部存在 (GA_GATE_CHECKLIST.md L73)
> **G10 注意**: TLA+ 检查待人工执行，GA_GATE_CHECKLIST 标记为"需人工执行"

### 2.4 QA Enhancement Gate（G-QA14）

| # | 检查项 | 命令/方法 | 通过标准 |
|---|--------|----------|---------|
| G-QA14 | QA Enhancement Suite | `GATE_STAGE=ga bash scripts/gate/check_qa_enhancement.sh` | 全部 PASS |

---

## 三、L1_CRATES 定义（Coverage 测量范围）

v3.2.0 Coverage 测量范围仅限以下 5 个 crate：

| Crate | 说明 | GA 阈值 |
|-------|------|---------|
| `sqlrustgo` | 主 crate | ≥85% (overall) |
| `sqlrustgo-executor` | 执行器 | 包含在 overall |
| `sqlrustgo-optimizer` | 优化器 | 包含在 overall |
| `sqlrustgo-storage` | 存储引擎 | 包含在 overall |
| `sqlrustgo-types` | 类型系统 | 包含在 overall |

**测量命令**:
```bash
cargo llvm-cov test --lib \
  -p sqlrustgo \
  -p sqlrustgo-executor \
  -p sqlrustgo-optimizer \
  -p sqlrustgo-storage \
  -p sqlrustgo-types \
  --llvm-cov --codecov
```

---

## 四、门禁结果统计

| 类别 | 通过 | 总计 | 通过率 |
|------|------|------|--------|
| 核心检查 G1-G12 | 10 | 12 | **83.3%** |
| 其中 G9, G10 需人工/环境 | — | 2 | — |
| G-QA14 QA Enhancement | — | 1 | — |

---

## 五、版本间对比

| 检查项 | v3.0.0 规范 | v3.2.0 规范 | 变化 |
|--------|------------|------------|------|
| G5 Coverage 阈值 | ≥85% | ≥85% | 不变 |
| G5 测量命令 | `cargo llvm-cov --all-features --lcov` | `cargo llvm-cov test L1_CRATES --lib` | **命令变化** |
| G6 Security | `cargo audit` | `cargo audit` | 不变 |
| G7 SQL Compat | RC 阶段有，GA 无专项 | GA 有 | **新增** |
| G9 Sysbench | G-Gate 性能门槛 | G9（服务器环境） | 下调（环境依赖） |

---

## 六、已知缺口

| 缺口 | 说明 | 影响 |
|------|------|------|
| **G10 TLA+ 未执行** | 需人工执行 TLA+ model checker | 无法验证 proofs |
| **G9 Sysbench 未执行** | 需 Z6G4 服务器环境 | 性能基线未验证 |
| **G6 cargo audit** | advisory db 不可达时 SKIP | 安全漏洞可能被忽略 |

---

*本文档由 hermes-z440 根据 check_ga_v320.sh 和 GA_GATE_CHECKLIST.md 反向工程生成，日期：2026-05-29*