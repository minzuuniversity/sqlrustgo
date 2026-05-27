# SQLRustGo v3.5.0 门禁规范

> **版本**: 1.0
> **日期**: 2026-05-27
> **状态**: 生效
> **supersedes**: v3.4.0 门禁规范（gate_spec_v340.md）

---

## 一、概述

### 1.1 四级门禁模型

```
A-Gate → B-Gate → R-Gate → G-Gate
 (α入口)  (β入口)  (RC入口)  (GA入口)
```

### 1.2 门禁通过条件

| 条件类型 | 说明 |
|----------|------|
| **全部通过** | 所有门禁检查项必须 PASS |
| **有豁免记录** | 不满足的项必须在 `GATE_EXEMPTIONS.md` 有审批通过的豁免记录 |
| **无阻塞性 OPEN Issue** | milestone 下的 OPEN Issue 不能阻塞门禁 |

### 1.3 v3.5.0 新增检查范围

| Crate | A-Gate | B-Gate | R-Gate | G-Gate |
|-------|--------|--------|--------|--------|
| `sqlrustgo-compliance-engine` | 必须通过 | 必须通过 | 必须通过 | 必须通过 |
| `sqlrustgo-gmp-api` | 必须通过 | 必须通过 | 必须通过 | 必须通过 |
| `sqlrustgo-gmp` | 必须通过 | 必须通过 | 必须通过 | 必须通过 |
| `sqlrustso-gmp-retrieval` | 必须通过 | 必须通过 | 必须通过 | 必须通过 |

---

## 二、Coverage SSOT 测量规范

### 2.1 测量命令（强制）

```bash
cargo llvm-cov test --lib \
  -p sqlrustgo \
  -p sqlrustgo-executor \
  -p sqlrustgo-optimizer \
  -p sqlrustgo-storage \
  -p sqlrustgo-types \
  -p sqlrustgo-parser \
  -p sqlrustgo-planner \
  -p sqlrustgo-transaction \
  -p sqlrustgo-catalog \
  --llc-cov --codecov
```

### 2.2 L1_CRATES 定义

| Crate | Alpha 阈值 | Beta 阈值 | GA 阈值 |
|-------|-----------|-----------|---------|
| `sqlrustgo` (workspace) | ≥70% | ≥75% | ≥80% |
| `sqlrustgo-executor` | ≥70% | ≥80% | ≥85% |
| `sqlrustgo-optimizer` | ≥80% | ≥85% | ≥90% |
| `sqlrustgo-storage` | ≥80% | ≥85% | ≥90% |
| `sqlrustgo-types` | ≥80% | ≥85% | ≥90% |
| `sqlrustgo-parser` | ≥50% | ≥60% | ≥70% |
| `sqlrustgo-planner` | ≥40% | ≥50% | ≥60% |
| `sqlrustgo-transaction` | ≥60% | ≥70% | ≥80% |
| `sqlrustgo-catalog` | ≥50% | ≥60% | ≥70% |
| **整体 L1** | **≥70%** | **≥80%** | **≥85%** |

### 2.3 v3.5.0 新增 Crate 覆盖率要求

| Crate | Alpha | Beta | GA |
|-------|-------|------|-----|
| `sqlrustgo-compliance-engine` | — | ≥40% | ≥50% |
| `sqlrustgo-gmp-api` | — | ≥40% | ≥50% |
| `sqlrustgo-gmp` | — | ≥30% | ≥40% |
| `sqlrustso-gmp-retrieval` | — | ≥40% | ≥50% |

> 注: 新增 crate 的覆盖率测量在 Beta 阶段开始纳入。

---

## 三、A-Gate (Alpha Gate)

### 3.1 检查清单

| # | 检查项 | 命令/方法 | 通过标准 | 证据格式 |
|---|--------|----------|---------|---------|
| A1 | 编译检查 | `cargo build --release --workspace` | 无错误 | `{command, exit_code}` |
| A2 | 单元测试 | `cargo test --lib` | 全部通过 | `{passed, failed, exit_code}` |
| A3 | Clippy 检查 | `cargo clippy --all-features -- -D warnings` | 零警告 | `{warnings, exit_code}` |
| A4 | 格式化检查 | `cargo fmt --all -- --check` | 无格式错误 | `{diff_count, exit_code}` |
| A5 | 覆盖率检查 | `cargo llvm-cov` | L1 ≥70% | `{total_pct}` |

### 3.2 Alpha Trust Infrastructure 检查

| # | 检查项 | 命令/方法 | 通过标准 |
|---|--------|----------|---------|
| TI-1 | perf-baseline exists | `[ -d crates/perf-baseline/src ]` | 目录存在 |
| TI-2 | crash-sim exists | `[ -d crates/crash-sim/src ]` | 目录存在 |
| TI-3 | wal-verification exists | `[ -d crates/wal-verification/src ]` | 目录存在 |
| TI-4 | compliance-engine exists | `[ -d crates/compliance-engine/src ]` | 目录存在 |
| TI-5 | evidence-engine exists | `[ -d crates/evidence-engine/src ]` | 目录存在 |
| TI-6 | provenance-graph exists | `[ -d crates/provenance-graph/src ]` | 目录存在 |
| TI-7 | gmp-api exists | `[ -d crates/gmp-api/src ]` | 目录存在 |
| TI-8 | gmp exists | `[ -d crates/gmp/src ]` | 目录存在 |
| TI-9 | gmp-retrieval exists | `[ -d crates/gmp-retrieval/src ]` | 目录存在 |

### 3.3 Alpha SSOT 规范检查

| # | 检查项 | 命令/方法 | 通过标准 |
|---|--------|----------|---------|
| SSOT-1 | Gate Spec | `[ -f docs/governance/gate_spec_v350.md ]` | 文件存在 |
| SSOT-2 | OO Roadmap | `[ -f docs/releases/v3.5.0/oo/OO_ROADMAP.md ]` | 文件存在 |
| SSOT-3 | Legacy Issues | `[ -f docs/releases/v3.5.0/LEGACY_ISSUES.md ]` | 文件存在 |
| SSOT-4 | Test Plan | `[ -f docs/releases/v3.5.0/TEST_PLAN.md ]` | 文件存在 |

---

## 四、B-Gate (Beta Gate)

### 4.1 检查清单

| # | 检查项 | 命令/方法 | 通过标准 | 证据格式 |
|---|--------|----------|---------|---------|
| B1 | 编译检查 | `cargo build --release --workspace` | 无错误 | `{command, exit_code}` |
| B2 | 全量测试 | `cargo test --all-features --workspace` | ≥90% 通过 | `{passed, failed, exit_code}` |
| B3 | Clippy 检查 | `cargo clippy --all-features -- -D warnings` | 零警告 | `{warnings, exit_code}` |
| B4 | 格式化检查 | `cargo fmt --all -- --check` | 无格式错误 | `{diff_count, exit_code}` |
| B5 | 覆盖率检查 | `cargo llvm-cov` | L1 ≥80% | `{total_pct}` |
| B6 | 安全扫描 | `cargo audit` | 无高危漏洞 | `{vulnerabilities}` |
| B7 | 文档链接检查 | `check_docs_links.sh` | 无死链 | `{broken_links}` |

### 4.2 Beta Trust Infrastructure 检查

| # | 检查项 | 命令/方法 | 通过标准 |
|---|--------|----------|---------|
| B-TI1 | Performance Governance | `cargo test -p perf-baseline` | 全部通过 |
| B-TI2 | Crash Simulation | `cargo test -p crash-sim` | 全部通过 |
| B-TI3 | WAL Verification | `cargo test -p wal-verification` | 全部通过 |
| B-TI4 | Compliance Engine | `cargo test -p sqlrustgo-compliance-engine --lib` | 全部通过 |
| B-TI5 | Evidence Engine | `cargo test -p evidence-engine` | 全部通过 |
| B-TI6 | Provenance Graph | `cargo test -p provenance-graph` | 全部通过 |
| B-TI7 | GMP API | `cargo test -p sqlrustgo-gmp-api --lib` | 全部通过 |
| B-TI8 | GMP Core | `cargo test -p sqlrustgo-gmp --lib` | 全部通过 |
| B-TI9 | GMP Retrieval | `cargo test -p sqlrustso-gmp-retrieval --lib` | 全部通过 |

---

## 五、R-Gate (RC Gate)

### 5.1 检查清单

| # | 检查项 | 命令/方法 | 通过标准 | 证据格式 |
|---|--------|----------|---------|---------|
| R1 | 编译检查 | `cargo build --release --workspace` | 无错误 | `{command, exit_code}` |
| R2 | 全量测试 | `cargo test --all-features --workspace` | 100% 通过 | `{passed, failed, exit_code}` |
| R3 | Clippy 检查 | `cargo clippy --all-features -- -D warnings` | 零警告 | `{warnings, exit_code}` |
| R4 | 格式化检查 | `cargo fmt --all -- --check` | 无格式错误 | `{diff_count, exit_code}` |
| R5 | 覆盖率检查 | `cargo llvm-cov` | L1 ≥85% | `{total_pct}` |
| R6 | 安全扫描 | `cargo audit` | 无漏洞 | `{vulnerabilities}` |
| R7 | SQL Corpus | `check_sql_compat.sh` | ≥85% | `{passed, total, pct}` |
| R8 | TPC-H SF=1 | `check_tpch.sh --sf1` | 22/22 通过 | `{passed, total}` |

### 5.2 RC 稳定性测试

| # | 检查项 | 命令/方法 | 通过标准 |
|---|--------|----------|---------|
| R-S1 | 16h 持续写入 | `test_sustained_write_16h` | 无崩溃 |
| R-S2 | 16h 并发读写 | `test_concurrent_read_write_16h` | 无崩溃 |
| R-S3 | 24h 持续写入 | `test_sustained_write_24h` | 无崩溃 |
| R-S4 | 24h 并发读写 | `test_concurrent_read_write_24h` | 无崩溃 |

---

## 六、G-Gate (GA Gate)

### 6.1 检查清单

| # | 检查项 | 命令/方法 | 通过标准 | 证据格式 |
|---|--------|----------|---------|---------|
| G1 | 编译检查 | `cargo build --release --workspace` | 无错误 | `{command, exit_code}` |
| G2 | 全量测试 | `cargo test --all-features --workspace` | 100% 通过 | `{passed, failed, exit_code}` |
| G3 | Clippy 检查 | `cargo clippy --all-features -- -D warnings` | 零警告 | `{warnings, exit_code}` |
| G4 | 格式化检查 | `cargo fmt --all -- --check` | 无格式错误 | `{diff_count, exit_code}` |
| G5 | 覆盖率检查 | `cargo llvm-cov` | L1 ≥85% | `{total_pct}` |
| G6 | 安全扫描 | `cargo audit` | 无漏洞 | `{vulnerabilities}` |
| G7 | SQL Corpus | `check_sql_compat.sh` | ≥90% | `{passed, total, pct}` |
| G8 | TPC-H SF=1 | `check_tpch.sh --sf1` | 22/22 通过 | `{passed, total}` |

### 6.2 GA 性能基准

| # | 检查项 | 命令/方法 | 通过标准 |
|---|--------|----------|---------|
| G-P1 | Point Select QPS | sysbench point_select | ≥30K QPS |
| G-P2 | Update QPS | sysbench update_index | ≥10K QPS |
| G-P3 | Delete QPS | sysbench delete | ≥10K QPS |

### 6.3 GA 稳定性测试

| # | 检查项 | 命令/方法 | 通过标准 |
|---|--------|----------|---------|
| G-S1 | 48h 持续写入 | `test_sustained_write_48h` | 无崩溃、无内存泄漏 |
| G-S2 | 48h 并发读写 | `test_concurrent_read_write_48h` | 无崩溃、无内存泄漏 |
| G-S3 | 72h 持续写入 | `test_sustained_write_72h` | 无崩溃、无内存泄漏 |
| G-S4 | 72h 并发读写 | `test_concurrent_read_write_72h` | 无崩溃、无内存泄漏 |

### 6.4 GA Trust Infrastructure 验证

| # | 检查项 | 命令/方法 | 通过标准 |
|---|--------|----------|---------|
| G-TI1 | Performance Governance | perf-gate check | regression <10% |
| G-TI2 | Crash Simulation | crash-sim inject 1000x | 无数据损坏 |
| G-TI3 | WAL Formal Verification | TLC model check | 无 violation |
| G-TI4 | Compliance Engine | `cargo test -p sqlrustgo-compliance-engine --lib` | 全部通过 |
| G-TI5 | GMP API | `cargo test -p sqlrustgo-gmp-api --lib` | 全部通过 |
| G-TI6 | GMP Core | `cargo test -p sqlrustgo-gmp --lib` | 全部通过 |
| G-TI7 | GMP Retrieval | `cargo test -p sqlrustso-gmp-retrieval --lib` | 全部通过 |
| G-TI8 | Evidence Engine | audit package generation | 生成成功 |
| G-TI9 | Provenance Graph | cypher query test | 查询正确 |

---

## 七、豁免管理

### 7.1 v3.5.0 豁免项

| 豁免 ID | 描述 | 关闭条件 | 关联 Issue |
|----------|------|----------|-----------|
| EX-v350-001 | 新增 crate 覆盖率阈值 | Beta 阶段达标 | — |

---

## 八、v3.5.0 版本特性

### 8.1 关键新功能

- **AI Native GMP**: 设备预测 Agent、报表生成 Agent、审计链摘要 Agent、规则推荐 Agent
- **Report Generator**: 自然语言报表生成（`compliance-engine/report/`）
- **Device Predictor**: 预测性设备维护（`gmp/device_predictor.rs`）
- **Audit Chain Summarizer**: 审计链 AI 摘要（`compliance-engine/summarizer/`）
- **Rule Recommender**: 规则自动推荐（`gmp-api/rule_recommendation.rs`）

### 8.2 架构变更

- 新增 `crates/compliance-engine/src/report/` 报表生成模块
- 新增 `crates/compliance-engine/src/summarizer/` 审计链摘要模块
- 新增 `crates/gmp/src/device_predictor.rs` 设备预测模块
- 新增 `crates/gmp-api/src/rule_recommendation.rs` 规则推荐模块
- 新增 `crates/gmp-retrieval/src/ollama_vectorizer.rs` Ollama 向量化
- 新增 `crates/gmp-retrieval/src/query_rewriter.rs` 查询重写
- 新增 `crates/gmp-retrieval/src/reranker.rs` Reranker

---

## 九、变更历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 1.0 | 2026-05-27 | 初始版本，基于 v3.4.0 规范，扩展 AI Native GMP 检查项 |
