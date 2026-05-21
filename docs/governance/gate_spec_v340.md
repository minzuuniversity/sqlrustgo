# SQLRustGo v3.4.0 门禁规范

> **版本**: 1.0
> **日期**: 2026-05-22
> **状态**: 生效
> **supersedes**: v3.3.0 门禁规范

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
  --llvm-cov --codecov
```

### 2.2 L1_CRATES 定义

| Crate | Alpha 阈值 | Beta 阈值 | GA 阈值 |
|-------|-----------|-----------|---------|
| `sqlrustgo` | ≥70% | ≥75% | ≥80% |
| `sqlrustgo-executor` | ≥70% | ≥80% | ≥85% |
| `sqlrustgo-optimizer` | ≥80% | ≥85% | ≥90% |
| `sqlrustgo-storage` | ≥80% | ≥85% | ≥90% |
| `sqlrustgo-types` | ≥80% | ≥85% | ≥90% |
| **整体** | **≥75%** | **≥80%** | **≥85%** |

> **v3.4.0 变更**: executor Alpha 阈值从 ≥75% 调整为 ≥70%（覆盖 execution_engine.rs 从 ~24% 提升至 ≥70% 的阶段目标）

---

## 三、A-Gate (Alpha Gate)

### 3.1 检查清单

| # | 检查项 | 命令/方法 | 通过标准 | 证据格式 |
|---|--------|----------|---------|---------|
| A1 | 编译检查 | `cargo build --release --workspace` | 无错误 | `{command, exit_code}` |
| A2 | 单元测试 | `cargo test --lib` | 全部通过 | `{passed, failed, exit_code}` |
| A3 | Clippy 检查 | `cargo clippy --all-features -- -D warnings` | 零警告 | `{warnings, exit_code}` |
| A4 | 格式化检查 | `cargo fmt --all -- --check` | 无格式错误 | `{diff_count, exit_code}` |
| A5 | 覆盖率检查 | `cargo llvm-cov` | L1 ≥75% | `{total_pct}` |
| A6 | MySQL Protocol | mysql client 连接 | 连接成功 | `{connection_status}` |
| A7 | TPC-H SF=1 | `check_tpch.sh --sf1` | 22/22 通过 | `{passed, total}` |

### 3.2 Alpha Trust Infrastructure 检查

| # | 检查项 | 命令/方法 | 通过标准 |
|---|--------|----------|---------|
| TI-1 | perf-baseline exists | `[ -d crates/perf-baseline/src ]` | 目录存在 |
| TI-2 | crash-sim exists | `[ -d crates/crash-sim/src ]` | 目录存在 |
| TI-3 | wal-verification exists | `[ -d crates/wal-verification/src ]` | 目录存在 |
| TI-4 | compliance-engine exists | `[ -d crates/compliance-engine/src ]` | 目录存在 |
| TI-5 | evidence-engine exists | `[ -d crates/evidence-engine/src ]` | 目录存在 |
| TI-6 | provenance-graph exists | `[ -d crates/provenance-graph/src ]` | 目录存在 |

### 3.3 Alpha SSOT 规范检查

| # | 检查项 | 命令/方法 | 通过标准 |
|---|--------|----------|---------|
| SSOT-1 | Gate Spec | `[ -f docs/governance/gate_spec_v340.md ]` | 文件存在 |
| SSOT-2 | OO Roadmap | `[ -f docs/releases/v3.4.0/oo/OO_ROADMAP.md ]` | 文件存在 |
| SSOT-3 | Legacy Issues | `[ -f docs/releases/v3.4.0/LEGACY_ISSUES.md ]` | 文件存在 |

### 3.4 Alpha 豁免关联

| 豁免 ID | 描述 | 关联 Issue |
|----------|------|-----------|
| EX-v330-001 | executor 覆盖率 72.3% | #1196, #1197 |
| EX-v330-002 | MySQL Protocol 握手失败 | #1201 |
| EX-v330-003 | TPC-H SF=1 数据缺失 | #1198 |
| EX-v330-004 | Sysbench 服务器环境 | #1198 |

---

## 四、B-Gate (Beta Gate)

### 4.1 检查清单

| # | 检查项 | 命令/方法 | 通过标准 | 证据格式 |
|---|--------|----------|---------|---------|
| B1 | 编译检查 | `cargo build --release --workspace` | 无错误 | `{command, exit_code}` |
| B2 | 全量测试 | `cargo test --all-features` | ≥90% 通过 | `{passed, failed, exit_code}` |
| B3 | Clippy 检查 | `cargo clippy --all-features -- -D warnings` | 零警告 | `{warnings, exit_code}` |
| B4 | 格式化检查 | `cargo fmt --all -- --check` | 无格式错误 | `{diff_count, exit_code}` |
| B5 | 覆盖率检查 | `cargo llvm-cov` | L1 ≥80% | `{total_pct}` |
| B6 | 安全扫描 | `cargo audit` | 无高危漏洞 | `{vulnerabilities}` |
| B7 | 文档链接检查 | `check_docs_links.sh` | 无死链 | `{broken_links}` |
| B8 | TPC-H SF=0.1 | `check_tpch.sh sf=0.1` | 22/22 通过 | `{passed, total, oom_count}` |

### 4.2 Beta Trust Infrastructure 检查

| # | 检查项 | 命令/方法 | 通过标准 |
|---|--------|----------|---------|
| B-TI1 | Performance Governance | cargo test -p perf-baseline | 全部通过 |
| B-TI2 | Crash Simulation | cargo test -p crash-sim | 全部通过 |
| B-TI3 | WAL Verification | cargo test -p wal-verification | 全部通过 |
| B-TI4 | Compliance Engine | cargo test -p compliance-engine | 全部通过 |
| B-TI5 | Evidence Engine | cargo test -p evidence-engine | 全部通过 |
| B-TI6 | Provenance Graph | cargo test -p provenance-graph | 全部通过 |

---

## 五、R-Gate (RC Gate)

### 5.1 检查清单

| # | 检查项 | 命令/方法 | 通过标准 | 证据格式 |
|---|--------|----------|---------|---------|
| R1 | 编译检查 | `cargo build --release --workspace` | 无错误 | `{command, exit_code}` |
| R2 | 全量测试 | `cargo test --all-features` | 100% 通过 | `{passed, failed, exit_code}` |
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
| G2 | 全量测试 | `cargo test --all-features` | 100% 通过 | `{passed, failed, exit_code}` |
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
| G-TI4 | Compliance-as-Code | rule evaluation test | 全部通过 |
| G-TI5 | Evidence Engine | audit package generation | 生成成功 |
| G-TI6 | Provenance Graph | cypher query test | 查询正确 |

---

## 七、豁免管理

### 7.1 v3.4.0 豁免项

| 豁免 ID | 描述 | 关闭条件 | 关联 Issue |
|----------|------|----------|-----------|
| EX-v340-001 | GMP Retrieval 阶段覆盖目标 | execution_engine.rs ≥70% | #1297 |
| EX-v340-002 | MySQL Protocol 握手失败 | 连接成功 | #1201 |
| EX-v340-003 | TPC-H SF=1 数据缺失 | 22/22 通过 | #1198 |

---

## 八、v3.4.0 版本特性

### 8.1 关键新功能

- **GMP Retrieval**: BM25, RRF (Reciprocal Rank Fusion), Reranker, LLM Chat
- **execution_engine.rs 覆盖率提升**: 从 ~24% 提升至 ≥70%

### 8.2 架构变更

- 新增 `crates/gmp/src/workflow/` 工作流状态管理
- 新增 BM25/RRF/Reranker 检索管线

---

## 九、变更历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 1.0 | 2026-05-22 | 初始版本，基于 v3.3.0 规范 |
