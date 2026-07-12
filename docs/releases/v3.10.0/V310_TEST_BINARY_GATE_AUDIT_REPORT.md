# SQLRustGo v3.10.0 — 测试 + 二进制 + 门禁 全面分析报告

> **目标读者**: DeepSeek（用于分析与改进建议）
> **生成时间**: 2026-07-13 00:55 CST
> **代码快照**: `develop/v3.10.0 @ ab756e337f`
> **当前阶段**: ALPHA (DRAFT → ALPHA promotion: 2026-07-11)
> **作者**: Claude Code (hermes-agent)

---

## 0. 摘要 (TL;DR)

| 维度 | 数量 | 状态 |
|------|------|------|
| 顶层 bin (`[[bin]]` in workspace) | **7** | ⚠️ 多入口, 非"统一 server+client" |
| 顶层 example | 24 | 主要在 storage |
| 顶层 test (`[[test]]`) | **309** | 跨 11 个 manifest, 高度碎片化 |
| 顶层 bench | 34 | 分散在 root + 4 子 crate |
| 辅助 bin (tools/, test/) | **7** (非 workspace) | 4 个独立 crate, 各 1-2 个 bin |
| `scripts/gate/` 门禁脚本 | **89** | 含历史遗留 + v3.10.0 specific |
| `#[ignore]` 标记 | 49 (20 个文件) | 远低于 v3.9.0 44 个，V310-12 子任务目标清零 |
| STAGE_CONFIG.yaml 5 阶段门禁 | 完整框架 | DRAFT/ALPHA/BETA/RC/GA, 每阶段 4-15 个 gate |
| v3.10.0 ALPHA 专用 gate | `check_alpha_v3.10.0.sh` (15/15 PASS) | ✅ |
| BETA/RC/GA 必备测试计划 | **缺失** `TEST_PLAN.md` + `FEATURE_CHECKLIST.md` | ❌ |

**核心结论**: v3.10.0 的 binary / test / gate 三层架构存在 **架构分散、命名不一致、缺少 BETA+ 阶段的统一测试规划文档** 三大问题，需要重构。

---

## 1. 二进制现状 (Bins)

### 1.1 顶层 workspace bins (7 个)

| # | Manifest | Package | Bin name | Source path | 角色 |
|---|----------|---------|----------|-------------|------|
| 1 | `Cargo.toml` | `sqlrustgo` | `sqlrustgo` | `src/bin/sqlrustgo/main.rs` | **DEPRECATED shim** (delegate 到 sqlrustgo-cli) |
| 2 | `crates/admin/Cargo.toml` | `sqlrustgo-admin` | `sqlrustgo-admin` | `crates/admin/src/main.rs` | backup/restore/verify/pitr |
| 3 | `crates/sqlrustgo-cli/Cargo.toml` | `sqlrustgo-cli` | `sqlrustgo-cli` | `crates/sqlrustgo-cli/src/main.rs` | "统一 CLI" (serve/exec/repl/... shells to mysql-server) |
| 4 | `crates/mysql-server/Cargo.toml` | `sqlrustgo-mysql-server` | `sqlrustgo-mysql-server` | `crates/mysql-server/src/main.rs` | **真正的 MySQL server** (canonical binary) |
| 5 | `crates/server/Cargo.toml` | `sqlrustgo-server` | `sqlrustgo-server` | `crates/server/src/main.rs` | **DEPRECATED stub** (声明 "This module is deprecated") |
| 6 | `crates/cli/Cargo.toml` | `sqlrustgo-soak` | `sqlrustgo-soak` | `crates/cli/src/main.rs` | CLI REPL/exec/soak 客户端 (connect to mysql-server) |
| 7 | `crates/tools/Cargo.toml` | `sqlrustgo-tools` | `tbl2bin` | `crates/tools/src/bin/tbl2bin.rs` | TPC-H `.tbl` → binary 转换工具 |

### 1.2 问题: 二进制不统一

#### A. 7 个顶层 bin, 没有 "server + client" 二元清晰架构

期望 (per V310_CLI_BINARY_PLAN.md 战略):
- `sqlrustgo` (= 客户端) — 单条 SQL 客户端
- `sqlrustgo-server` — 真正的 MySQL 兼容 server

实际 (混乱):
- `sqlrustgo` (root): **DEPRECATED shim** delegate to `sqlrustgo-cli`
- `sqlrustgo-cli` (`crates/sqlrustgo-cli/`): 9 subcommands, 7 个 shell out to `sqlrustgo-mysql-server`
- `sqlrustgo-soak` (`crates/cli/`): 不同目录, 同样客户端功能
- `sqlrustgo-mysql-server` (`crates/mysql-server/`): 真正 server
- `sqlrustgo-server` (`crates/server/`): **DEPRECATED stub**, 文档声明非功能
- `sqlrustgo-admin` (`crates/admin/`): backup/restore 工具
- `sqlrustgo-tools / tbl2bin` (`crates/tools/`): TPC-H 转换工具

#### B. Server 二元有两个, 一个真一个假

- ✅ `sqlrustgo-mysql-server` (crates/mysql-server) - 真, **canonical**
- ❌ `sqlrustgo-server` (crates/server) - 假, **deprecated stub** (代码顶部 `//! # DEPRECATED`)

按 v3.8.0 计划, `crates/server` 应在 v3.8.0+ 移除, 但仍残留在 workspace, 占用编译时间。

#### C. Client 二元有三个, 功能重复

| 客户端 bin | 主要功能 | 状态 |
|----------|--------|------|
| `sqlrustgo` (root) | 9-subcommand CLI shim | DEPRECATED, 14 行代码 |
| `sqlrustgo-cli` | 9-subcommand CLI shell to mysql-server | ACTIVE |
| `sqlrustgo-soak` | 客户端, 走 wire protocol | ACTIVE 但功能与 cli 重叠 |

`crates/cli` 与 `crates/sqlrustgo-cli` 是不同目录的不同 package, 名字相近功能重叠。

### 1.3 辅助工具 (非 workspace, 7 个 bin)

| Manifest | Package | Bin | 状态 |
|----------|---------|-----|------|
| `tools/graph-cli/Cargo.toml` | `graph-cli` | `gate`, `ingest` | 独立 (非 workspace) |
| `tools/sqlrustgo-gate/Cargo.toml` | `sqlrustgo-gate` | `graph-gate`, `graph-ingest` | 独立 (非 workspace) |
| `tools/xtask/Cargo.toml` | `xtask` | `architecture-check`, `dead-modules` | 独立 (非 workspace) |
| `test/sql92/Cargo.toml` | `sql92-test-suite` | `sql92-runner` | 独立 (非 workspace) |

> 4 个独立 crate, 共 7 个 bin, 与 workspace 隔离, 不在 `cargo build --all-features` 范围内。

### 1.4 目标统一架构 (基于 V310_CLI_BINARY_PLAN.md Phase 1+2 已完成)

**建议的最终统一结构** (8 → 5 个):

```
sqlrustgo-mysql-server (canonical server, crates/mysql-server)
  ├── serve, exec, repl, bench, gmp, diag, backup, restore (subcommands)

sqlrustgo-cli (canonical client, crates/sqlrustgo-cli)  ← 保留
  ├── exec, repl, bench, gmp (subcommands, 使用 sqlrustgo-mysql-client lib)

sqlrustgo-admin (canonical admin tool, crates/admin)
  ├── backup, restore, verify, pitr (subcommands)

sqlrustgo-tools (TPC-H + dev utilities, crates/tools)
  ├── tbl2bin (TPC-H .tbl → binary)

[删除/归档]:
  - sqlrustgo (root src/bin/sqlrustgo/main.rs) — DEPRECATED shim, 14 行
  - sqlrustgo-server (crates/server) — DEPRECATED stub
  - sqlrustgo-soak (crates/cli) — 合并到 sqlrustgo-cli
```

**这是 V310_CLI_BINARY_PLAN.md Phase 4 目标**, 但从 v3.10.0 现状看 **未完全执行**。

---

## 2. 测试现状 (Tests)

### 2.1 顶层 [[test]] 数量: 309

按 root workspace 聚合, 跨 11 个 crate manifest 分布:

| Manifest | 数量 | 占比 |
|----------|------|------|
| `Cargo.toml` (root) | ~244 | 79% |
| `crates/executor` | 28 | 9% |
| `crates/storage` | 12 | 4% |
| `crates/distributed` | 5 | 1.6% |
| `crates/parser` | 4 | 1.3% |
| `crates/mysql-server` | 2 | 0.6% |
| `crates/optimizer` | 2 | 0.6% |
| `crates/transaction` | 2 | 0.6% |
| `crates/wal-verification` | 2 | 0.6% |
| `crates/planner` | 2 | 0.6% |
| `crates/server` | 1 | 0.3% |
| `crates/bench` | 2 | 0.6% |
| `crates/gmp` | 1 | 0.3% |
| `crates/graph` | 1 | 0.3% |
| `crates/sql-corpus` | 1 | 0.3% |

### 2.2 测试文件目录结构

```
tests/                                    223 files (root-level, root manifest)
crates/executor/tests/                    27 files
crates/storage/tests/                     11 files
crates/distributed/tests/                  5 files
crates/parser/tests/                       4 files
crates/mysql-server/tests/                 2 files
crates/optimizer/tests/                    2 files
crates/planner/tests/                      2 files
crates/transaction/tests/                  2 files
crates/wal-verification/tests/             2 files
crates/bench/tests/                        2 files
crates/server/tests/                       1 file
crates/gmp/tests/                          1 file
crates/graph/tests/                        1 file
crates/sql-corpus/tests/                   1 file
TOTAL                                    286 files
```

### 2.3 `#[ignore]` 标记统计

- **49 个 `#[ignore]`** 出现在 20 个不同测试文件
- V3.9.0 是 44 个, V310-12 子任务目标 = 0
- 当前 49 > V3.9.0 44, 因为增加了新的故障测试, 没有完全收敛

### 2.4 测试结构性问题

#### A. 309 个 test 目标分布在 11 个 manifest, 高度碎片化

理想架构: 按功能领域分组, 例如
- `tests/sql_core/` (核心 SQL 解析/执行)
- `tests/dml/` (DML 完整性)
- `tests/transaction/` (ACID)
- `tests/wire_protocol/` (MySQL wire protocol)
- `tests/soak/` (长期稳定性)
- `tests/tpch/` (TPC-H)
- `tests/bench/` (基准)
- `tests/cli/` (命令行集成)

现状: 223 个文件直接放在 `tests/` root, 命名随机 (adaptive_hash_index_test, diag_22_on_sf01, four_way_compare_test, q13_subquery_repro, repro_3282_orderby_desc, ...), 没有任何目录分类。

#### B. 命名规范不一致

观察到的命名 (root `tests/`):
- `*_test.rs` (主流, e.g. `aggregate_functions_test.rs`)
- `*_harness.rs` (e.g. `audit_log_harness.rs`, `crash_test_harness.rs`)
- `diag_*` (诊断, e.g. `diag_22_on_sf01.rs`, `diag_q11.rs`)
- `oracle_*` (oracle 对比, e.g. `oracle_g11_qps.rs`)
- `*_regression_test.rs` (回归, e.g. `q21_cell_regression_test.rs`)
- `*_bench*.rs` (基准, e.g. `sprint8_hash_chain_bench.rs`)
- `*_e2e_test.rs` (端到端, e.g. `cte_e2e_test.rs`)
- `*_contract_test.rs` / `*_harness.rs` (契约, e.g. `tx_wal_contract_tests.rs`)

缺少统一前缀或分类。

#### C. 测试与生产代码耦合

例如:
- `tests/adaptive_hash_index_test.rs` — 测的是 `crates/executor/src/window_executor.rs` 不存在的内容
- `tests/clustered_index_test.rs` — 测的是 storage 但 root manifest 引用
- `tests/change_buffer_test.rs` — 测的是已废弃功能

这违反 INT-3 (历史债务), 测试应只在主路径集成后才保留, 否则是"isolated test code"。

#### D. "Oracle" 测试机制分散

```
tests/oracle_g*.rs  (10 个) - 外部 SQLite/PG 对比 oracle
tests/diag_*.rs     (15+ 个) - 单条 SQL 诊断
```

这表明有 ad-hoc 的 oracle 机制, 但没有统一接口。

#### E. 集成测试 vs 单元测试混杂

Cargo test 区分:
- `cargo test --lib` (unit, in src/)
- `cargo test --test <name>` (integration, in tests/)
- `cargo test --doc` (doctest)

但 309 个 test 全部用 `[[test]]` 注册, **没有 --lib 单元测试, 全部是 integration**。这导致编译时间长 (309 个二进制), 调试难, 覆盖率测量受限。

### 2.5 现有 v3.10.0 测试目标 (V310_ISSUES_PLAN.md)

V310-12 子任务 12: "其他 ignore 测试 + 跨版本债" 估时 60h:

> **V310-12**: 其他 ignore 测试 + 跨版本债
> **估时**: 60h
> **依赖**: 无
> **子任务**:
> - V310-12a: M-5 INT-2 ParallelExecutor 生产路径 — 20h
> - V310-12b: M-6 INT-3 stored_proc 重构 — 20h
> - V310-12c: H-2 ARCH-3 VTU 剩余 5% — 20h
> **完成判据**: P2 债务全部关闭

但这个任务 (V310-12c) 的具体 `#[ignore]` 清单未在 plan 中列出, 需要从 `IGNORE_REGISTRY_2026-06-25.md` 重新生成。

### 2.6 现有 v3.10.0 测试计划文档

| 文档 | 状态 | 内容 |
|------|------|------|
| `V310_DEVELOPMENT_PLAN.md` | ✅ 14KB | 26 任务 / 500h / 4 阶段, 提到 G1-G8 门禁 |
| `V310_ISSUES_PLAN.md` | ✅ 12KB | 12 个子 ISSUE 详述, G1-G10 验收 |
| `V310_10_COVERAGE_PLAN.md` | ✅ 8KB | 覆盖率提升 ≥80%, 3 个 crate |
| `V310_CLI_BINARY_PLAN.md` | ✅ 7.6KB | 4 阶段 CLI 重构 |
| `V310_VERSION_PLAN.md` | ✅ 8.8KB | 战略定位, 不做项 |
| `PARALLEL_MAIN_PATH.md` | ✅ 4.8KB | I-12 集成文档 |
| **TEST_PLAN.md** | ❌ 缺失 | BETA 阶段 STAGE_CONFIG.yaml 必需 |
| **FEATURE_CHECKLIST.md** | ❌ 缺失 | BETA 阶段 STAGE_CONFIG.yaml 必需 |
| **VERSION_PLAN.md** (root) | ⚠️ 935B | 仅占位符, 缺战略定位 |

**关键 gap**: BETA 阶段 STAGE_CONFIG.yaml 要求 `TEST_PLAN.md` + `FEATURE_CHECKLIST.md` 必备, 但 v3.10.0 都没有。

---

## 3. 门禁现状 (Gates)

### 3.1 `scripts/gate/` 总览: 89 个脚本

按用途分类 (大致):

| 类别 | 数量 | 代表 |
|------|------|------|
| 5-Principle 审计 | 4 | `check_5_principles.sh`, `check_10_principles.sh` |
| Per-version stage gates | 3 | `check_alpha.sh`, `check_alpha_v380.sh`, `check_alpha_v3.10.0.sh` |
| 架构 / 集成债务 | 5 | `check_arch_invariants.sh`, `check_arch2_no_bypass.sh`, `check_arch3_no_bypass.sh`, `check_arch_sem_debt.sh`, `check_int_debt.sh` |
| TPC-H / G1-G16 | 9 | `check_g1_tpch_*.sh`, `check_g11_qps.sh`, `check_g13_stability.sh`, ... |
| INT-2/INT-3 单任务 | 2 | `check_int2_no_orphan.sh`, `check_int3_single_expr.sh` |
| 跨版本 / 漂移 | 3 | `check_cross_version_debt.sh`, `check_drift_not_pass.sh`, `check_5_principles.sh` |
| 文档 / 一致性 | 7 | `check_docs.sh`, `check_docs_links.sh`, `check_docs_consistency.sh`, `check_document_completeness.sh`, `check_r1_r10_content.sh` |
| 覆盖率 / 反虚假 | 3 | `check_coverage.sh`, `check_anti_fabrication.sh`, `check_evidence_binding.sh` |
| 测试自身 | 4 | `check_test_inventory.sh`, `check_test_inventory_gate_test.sh`, `check_test_count_monotonic.sh`, `check_gate_self_verification.sh` |
| Anti-bypass / 治理 | 5 | `check_drift_not_pass.sh`, `check_architecture_freeze.sh`, `check_mainline.sh`, `check_attack_surface.sh`, `check_gate_test_integrity.sh` |
| 性能 / 备份 / 安全 | 6 | `check_performance.sh`, `check_perf_baseline.sh`, `check_backup_restore.sh`, `check_security.sh`, `check_sql_compat.sh`, `check_anti_fabrication.sh` |
| 通信 / Webhook | 3 | `gate_webhook.sh`, `send_gate_alert.sh`, `send_gate_failure_webhook.sh` |
| 流程 / 杂项 | 35+ | `audit_*.sh`, `log_gate.sh`, `update_changelog.sh`, `update_proof_registry.sh`, `pre-commit-env-blocker.sh`, ... |

### 3.2 STAGE_CONFIG.yaml 阶段门禁 (version-agnostic SSOT)

`docs/governance/STAGE_CONFIG.yaml` 定义 5 阶段 (DRAFT/ALPHA/BETA/RC/GA) 的 `required_gates` 列表:

| 阶段 | Required Gates (摘要) |
|------|----------------------|
| DRAFT | `check_docs_links.sh`, `cargo build` |
| ALPHA | `check_alpha_v{VER}.sh`, `check_arch_invariants.sh`, `check_arch3_no_bypass.sh`, `cargo build/test/fmt` |
| BETA | `check_beta_gate.sh`, `check_arch_invariants.sh`, `check_arch3_no_bypass.sh`, `check_arch_sem_debt.sh`, `check_cross_version_debt.sh`, `check_int_debt.sh`, `cargo build/test/fmt/clippy` |
| RC | `check_rc_ga_gate.sh`, `check_arch_invariants.sh`, `check_arch3_no_bypass.sh`, `check_integration_gate.sh`, `check_anti_fabrication.sh`, `check_arch_sem_debt.sh`, `check_cross_version_debt.sh`, `check_int_debt.sh`, `check_full_gate_verification.sh`, `check_drift_not_pass.sh`, `cargo build/test/fmt/clippy` |
| GA | (RC 全部) + `check_architecture_freeze.sh`, `check_gate_self_verification.sh`, `check_gate_test_integrity.sh`, `cargo build/test/fmt/clippy` |

**Stage Driver**: `scripts/gate/check_stage.sh` 根据 `docs/releases/vX.Y.Z/STAGE.yaml` 的 `current_stage` 自动选择 + 串联该阶段的 `required_gates`。

### 3.3 v3.10.0 当前门禁状态

`docs/releases/v3.10.0/STAGE.yaml`:
- `current_stage: ALPHA`
- 实际 ALPHA promotion 2026-07-11 完成 (`15/15 check_alpha_v3.10.0.sh`)
- 但 `blubber_issues: []` (空), 实际还有 4 个 v3.9.0 债务
- `gate-results/2026-07-01-DRAFT-init.md`: 只有 DRAFT 阶段初次结果, **BETA 之后无任何 gate-results**

### 3.4 v3.10.0 ALPHA gate (`check_alpha_v3.10.0.sh`) 内容

15 个检查:
- A1: `cargo build --all-features`, `cargo test --all-features --lib`, `cargo fmt --check`
- A2: `check_arch_invariants.sh`, `check_arch3_no_bypass.sh`
- A3: 8 个必备文件 (CHANGELOG, RELEASE_NOTES, STAGE.yaml, V310_VERSION_PLAN.md, V310_DEVELOPMENT_PLAN.md, ARCHITECTURE.md, V310_ISSUES_PLAN.md, DRAFT_ASSESSMENT)
- A4: branch sanity (`develop/v3.10.0`), DRAFT 阶段产物

✅ 已 PASS。

### 3.5 BETA gate 缺口 (v3.10.0)

BETA 阶段 STAGE_CONFIG.yaml 要求:
1. **必备文件**: `STAGE.yaml`, `TEST_PLAN.md`, `FEATURE_CHECKLIST.md`
   - ❌ `TEST_PLAN.md` 缺失
   - ❌ `FEATURE_CHECKLIST.md` 缺失
2. **Required gates**: `check_beta_gate.sh`, `check_arch_invariants.sh`, `check_arch3_no_bypass.sh`, `check_arch_sem_debt.sh`, `check_cross_version_debt.sh`, `check_int_debt.sh`, `cargo build/test/fmt/clippy`
3. **评审**: 2 个 reviewer (codeowner + governance)

**Risk**: 当前 `check_beta_gate.sh` 不存在, 而 STAGE_CONFIG 引用它。

### 3.6 跨 5 阶段通用的门禁脚本

- `check_arch_invariants.sh` (C-ARCH-01~05)
- `check_arch3_no_bypass.sh` (VTU 强制)
- `check_arch_sem_debt.sh` (D8 Architecture-Semantic-Debt)
- `check_cross_version_debt.sh` (跨版本债务)
- `check_int_debt.sh` (集成债务)
- `check_integration_gate.sh` (集成)
- `check_anti_fabrication.sh` (反虚假)
- `check_full_gate_verification.sh` (full gate)
- `check_drift_not_pass.sh` (防漂移)
- `check_architecture_freeze.sh` (RC+)
- `check_gate_self_verification.sh` (GA)
- `check_gate_test_integrity.sh` (GA)

### 3.7 3.10.0 范围内还未创建/不适用的 gates

按 STAGE_CONFIG.yaml, 下列脚本可能缺失或需 per-version 实现:
- `check_beta_gate.sh` — 缺 (BETA 阶段必需)
- `check_rc_ga_gate.sh` — 存在, 但可能需要 per-version 调整
- `check_full_gate_verification.sh` — 存在, 是 BETA 共享

---

## 4. 测试计划与门禁脚本的"统一"差距

### 4.1 Per-Version 测试计划缺口 (按 STAGE_CONFIG.yaml 要求)

| 阶段 | 必备文件 | v3.10.0 状态 | 影响 |
|------|---------|--------------|------|
| DRAFT | `VERSION_PLAN.md`, `ARCHITECTURE.md` | ✅ (ARCHITECTURE.md 8.5KB, VERSION_PLAN.md 935B 偏小) | OK |
| ALPHA | `CHANGELOG.md`, `RELEASE_NOTES.md`, `STAGE.yaml` | ✅ | OK |
| **BETA** | `STAGE.yaml`, **`TEST_PLAN.md`**, **`FEATURE_CHECKLIST.md`** | ❌ **缺** | **阻塞 BETA promotion** |
| RC | `STAGE.yaml`, `RELEASE_NOTES.md`, `CHANGELOG.md`, `GA_GATE_REPORT.md` | ⚠️ 缺 GA_GATE_REPORT.md | 阻塞 RC |
| GA | `STAGE.yaml` (state=GA), `RELEASE_NOTES.md`, `GA_GATE_REPORT.md`, `GA_RELEASE_TIMELINE.md`, `CHANGELOG.md` (GA entry) | ⚠️ 缺多个 | 阻塞 GA |

**结论**: **v3.10.0 没有 BETA 阶段测试计划, 无法推进到 BETA**。

### 4.2 应用集成目标缺失

`V310_DEVELOPMENT_PLAN.md` 提到 26 个任务和 12 个子 ISSUE, 但:
- **没有"应用集成测试"的概念**: 缺少端到端的 "真实客户端连接到 server, 执行真实业务场景" 的集成测试
- **缺少性能回归 baseline**: 任何性能回归如何判定? 没有基线 (V310_DEVELOPMENT_PLAN §0.3 提到"~500h" 但没有 perf budget)
- **缺少兼容性矩阵**: MySQL 5.7 的哪些命令必须支持? V310_DEVELOPMENT_PLAN 提到"替代 MySQL 5.7" 但没有具体清单

### 4.3 统一规划缺失

`V310_DEVELOPMENT_PLAN.md` 已经存在 (2026-06-25 创建), 但:
- **测试目标分散在多处**: `V310_ISSUES_PLAN.md` 列了 G1-G10 验收, `V310_10_COVERAGE_PLAN.md` 只关注覆盖率, `V310_CLI_BINARY_PLAN.md` 只关注 binary, 没有任何单一文档统一所有测试目标
- **没有测试矩阵**: 应有"功能/性能/兼容性/稳定性" 4 维矩阵, 每维都有目标 + 测试方法 + 验收标准
- **没有"集成应用目标"**: 缺少 "1 个用户在 5 分钟内能用 sqlrustgo-cli 跑完 1 个 22-query TPC-H" 这种具体场景的端到端指标

### 4.4 统一应用集成 (3.10.0 范围) 应该覆盖

参考 MySQL 5.7 替代定位, 必须有以下端到端应用场景:

| # | 场景 | 入口 | 预期结果 |
|---|------|------|----------|
| 1 | 启动 + 连接 + SELECT 1 | sqlrustgo-mysql-server + mysql client | 1 row, "1" |
| 2 | 22-query TPC-H SF=0.1 | server + oltp benchmark | 22/22 PASS in <30s |
| 3 | 24h 稳定性 (G13) | server + sysbench 8-32 threads | 0 errors, 0 crashes |
| 4 | kill -9 崩溃恢复 (T-20) | server + kill -9 mid-tx | data consistency verified |
| 5 | Backup + Restore (G6) | sqlrustgo-admin backup/restore | roundtrip data identical |
| 6 | sysbench prepare/run 真接入 (V310-09) | wired sysbench via mysql-server | end-to-end prepare→run |
| 7 | ALTER TABLE RENAME | server + mysql client | schema modified, data preserved |
| 8 | ROLLBACK MVCC | server + tx-wal | snapshot restoration |
| 9 | UNION 集合操作 | server | INTERSECT/EXCEPT/UNION ALL |
| 10 | 168h 长期稳定性 (G13/G16) | server + sustained load | 0 errors, 0 resource leak |

这些场景的**具体测试方法、oracle、预期时间** 在当前 V3.10.0 文档中**完全缺失**。

---

## 5. 给 DeepSeek 的具体分析请求 (按重要性排序)

### 5.1 高优先级 (架构问题)

#### Q1: 二进制是否应该统一到 "server + client" 二元?
**现状**: 7 个顶层 bin (含 1 deprecated shim, 1 deprecated stub, 3 个功能重叠的 client)
**问题**:
- `sqlrustgo` (root) 是 DEPRECATED shim
- `sqlrustgo-server` (crates/server) 是 DEPRECATED stub
- `sqlrustgo-cli` vs `sqlrustgo-soak` vs `sqlrustgo` (root) 三个客户端入口功能重叠

**建议问题**:
- 是否应该合并 `crates/cli` + `crates/sqlrustgo-cli` 为一个 client crate?
- 是否应该从 workspace 移除 `crates/server` (DEPRECATED stub)?
- 是否应该将 root `src/bin/sqlrustgo/` (14 行 shim) 标记为 hidden feature flag, 默认不编译?

#### Q2: 是否应该把所有 integration tests 重组为分层结构?
**现状**: 309 个 test 全部为 integration (无 --lib unit), 跨 11 个 manifest, 223 个 root `tests/*.rs` 无目录分类
**问题**:
- 编译时间: 309 个 test target, 每次 `cargo test` 编译 309 个二进制
- 覆盖率测量: `cargo llvm-cov --tests` 失败, 只能 `--lib` 测 65.70% (executor), 38.68% (parser), 69.16% (storage)
- 命名混乱: test, harness, diag, oracle, regression, contract 各种后缀

**建议问题**:
- 是否应该按功能领域分层 (tests/sql/, tests/dml/, tests/transaction/, tests/wire/)?
- 是否应该把核心逻辑的测试移入 crates/*/src/ (unit test) 而不是 tests/?
- 是否应该定义统一命名规范 (e.g. `*_integration_test.rs`, `*_contract_test.rs`, `*_harness.rs`)?

#### Q3: 是否应该创建 v3.10.0 统一的 TEST_PLAN.md 文档?
**现状**: V310_DEVELOPMENT_PLAN.md 提到 "G1-G8 门禁" 但不细; V310_ISSUES_PLAN.md 列了 G1-G10 验收; 没有统一的"测试矩阵"文档
**问题**:
- BETA 阶段 STAGE_CONFIG.yaml 要求 `TEST_PLAN.md` 必备
- 现有的 V310_10_COVERAGE_PLAN.md 只关注覆盖率
- V310_CLI_BINARY_PLAN.md 只关注 binary 重构
- 没有任何文档列出**所有测试 + oracle + 预期时间 + 失败处理**

**建议问题**:
- 应该按什么结构编写 TEST_PLAN.md?
- 应该包含哪些章节 (测试矩阵, G1-G16 详细场景, 性能 baseline, 兼容性矩阵)?
- 与 STAGE_CONFIG.yaml 的关系 (TLDR/详细/单一真相)?

#### Q4: BETA gate `check_beta_gate.sh` 缺失, 是否应该创建?
**现状**: `STAGE_CONFIG.yaml` BETA 阶段 `required_gates` 引用 `scripts/gate/check_beta_gate.sh`, 但该文件不存在
**问题**:
- BETA promotion 会失败 (gate script missing)
- 类似的还有 `check_alpha_v{VER}.sh` 模式 (已经按 v3.8.0 / v3.10.0 实现), 但 BETA/RC/GA 的 per-version gate 没有

**建议问题**:
- 是否需要 per-version `check_beta_v{VER}.sh` / `check_rc_v{VER}.sh` / `check_ga_v{VER}.sh`?
- 还是让 v3.10.0 共用 v3.9.0 的 gate?

### 5.2 中优先级 (测试规划问题)

#### Q5: 是否应该明确"应用集成目标" (end-to-end 场景)?
**现状**: V310_ISSUES_PLAN.md 列了 10 个 G1-G10 验收 gate, 但都是 framework 层级 (TPC-H 22/22, 覆盖率 80%, 24h SOAK 0 errors)
**问题**:
- 缺少"用户视角" 的集成场景 (10 个具体 end-to-end 用例, 见 §4.4)
- 缺少"什么算 PASS" 的具体 oracle 验证 (目前是 framework-level metrics)
- 缺少"应用集成测试" 整体定位

**建议问题**:
- 是否应该新增"应用集成测试" 文档 / 章节?
- 是否应该将 10 个 end-to-end 场景加入 BETA gate?

#### Q6: 49 个 `#[ignore]` 应该如何收敛?
**现状**: V310-12 子任务目标 = 0 ignore, 但 V3.9.0 是 44, V3.10.0 是 49 (更多)
**问题**:
- 49 个 `#[ignore]` 来自 20 个不同测试文件
- 部分测试的 ignore 原因已不存在 (例如 `repro_3282_orderby_desc` #3282 已 closed)
- 应该有计划逐步 unignore

**建议问题**:
- 是否应该创建 `docs/releases/v3.10.0/IGNORE_REGISTRY.md` 列出每个 ignore 的去留计划?
- 是否应该把"unignore" 作为 V310-12 的子任务?

### 5.3 低优先级 (清理问题)

#### Q7: 89 个 gate scripts 中有多少冗余?
**问题**:
- `check_5_principles.sh` 和 `check_10_principles.sh` 听上去功能重叠
- 多个 `audit_*.sh` 类似目的
- `check_alpha.sh` (v2.9.0 硬编码) vs `check_alpha_v380.sh` vs `check_alpha_v3.10.0.sh` 三种版本共存, v2.9.0 已废弃但未删除

**建议问题**:
- 是否应该审计 89 个 gate scripts, 删除废弃的?
- 是否应该合并功能相似的?

#### Q8: tools/, test/, root src/bin 三个目录与 workspace 隔离, 是否应该整合?
**问题**:
- `tools/*` 4 个独立 crate, 不在 workspace, `cargo build --all-features` 不会编译
- `test/sql92` 独立, 同上
- `src/bin/sqlrustgo/` root shim

**建议问题**:
- 是否应将 tools/* 纳入 workspace (as `members = ["tools/*"]`)?
- 整合的好处: 单一 cargo 命令编译全部, 减少分散

---

## 6. 附录: 关键文件路径清单

### 6.1 配置文件
- `Cargo.toml` (root) — 顶层 workspace + package + bin
- `docs/governance/STAGE_CONFIG.yaml` — 5 阶段门禁 SSOT
- `docs/governance/debt/debt-registry.yaml` — 跨版本债务登记
- `docs/openspec/` — OpenSpec 变更集

### 6.2 v3.10.0 计划文档
- `docs/releases/v3.10.0/ARCHITECTURE.md` (8.5KB)
- `docs/releases/v3.10.0/ARCHITECTURE_DEBT_ANALYSIS.md` (14KB)
- `docs/releases/v3.10.0/CHANGELOG.md` (6.8KB)
- `docs/releases/v3.10.0/DRAFT_ASSESSMENT_AND_ALPHA_GATE.md` (7.7KB)
- `docs/releases/v3.10.0/LEGACY_DEBT_CLOSURE_TRACKING_REPORT.md` (30KB, 388 行)
- `docs/releases/v3.10.0/RELEASE_NOTES.md` (5.1KB)
- `docs/releases/v3.10.0/STAGE.yaml` (6.4KB)
- `docs/releases/v3.10.0/VERSION_PLAN.md` (935B - 偏小)
- `docs/releases/v3.10.0/plans/INDEX.md` (5.3KB)
- `docs/releases/v3.10.0/plans/PARALLEL_MAIN_PATH.md` (4.8KB)
- `docs/releases/v3.10.0/plans/V310_10_COVERAGE_PLAN.md` (8.7KB)
- `docs/releases/v3.10.0/plans/V310_CLI_BINARY_PLAN.md` (7.6KB)
- `docs/releases/v3.10.0/plans/V310_DEVELOPMENT_PLAN.md` (14.8KB)
- `docs/releases/v3.10.0/plans/V310_ISSUES_PLAN.md` (12.7KB)
- `docs/releases/v3.10.0/plans/V310_VERSION_PLAN.md` (8.9KB)
- **缺失**: `docs/releases/v3.10.0/TEST_PLAN.md`
- **缺失**: `docs/releases/v3.10.0/FEATURE_CHECKLIST.md`

### 6.3 门禁脚本
- 89 个 gate scripts in `scripts/gate/`
- 关键脚本:
  - `check_alpha_v3.10.0.sh` (15/15 PASS) - 当前 v3.10.0 专用
  - `check_alpha_v380.sh` - v3.8.0 模板
  - `check_alpha.sh` - v2.9.0 硬编码, 已废弃
  - `check_stage.sh` - STAGE_CONFIG.yaml driver
  - `check_arch_invariants.sh` - C-ARCH-01~05
  - `check_arch3_no_bypass.sh` - VTU 强制
  - `check_arch_sem_debt.sh` - 7-state debt
  - `check_cross_version_debt.sh` - 跨版本 debt
  - `check_int_debt.sh` - 集成 debt
  - `check_integration_gate.sh` - 集成
  - `check_test_inventory.sh` - 100% tests 集成
  - `check_drift_not_pass.sh` - 防漂移
  - `check_full_gate_verification.sh` - 完整门禁
  - `check_anti_fabrication.sh` - 反虚假
  - `check_tpch_sf1.sh` - v3.10.0 TPC-H SF=1
  - `check_g13_stability.sh` - 24h 稳定性

### 6.4 数据统计
- 工作空间成员: 43 个 crates
- 顶层 `[[bin]]`: 7 (sqlrustgo, sqlrustgo-admin, sqlrustgo-cli, sqlrustgo-mysql-server, sqlrustgo-server, sqlrustgo-soak, tbl2bin)
- 工具 `[[bin]]` (非 workspace): 7
- 顶层 `[[test]]`: 309 (跨 11 个 manifest)
- 顶层 `[[bench]]`: 34
- 顶层 `[[example]]`: 24
- 顶层测试文件目录:
  - `tests/` — 223 files
  - `crates/executor/tests/` — 27 files
  - `crates/storage/tests/` — 11 files
  - `crates/distributed/tests/` — 5 files
  - 等等
- `#[ignore]` 标记: 49 (在 20 个测试文件)
- Gate scripts: 89 (在 `scripts/gate/`)
- Per-version gate scripts:
  - `check_alpha_v3.10.0.sh` ✅
  - `check_alpha_v380.sh` ✅
  - `check_alpha.sh` (v2.9.0 已废弃, 未删除)
  - 缺失: `check_beta_v*.sh`, `check_rc_v*.sh`, `check_ga_v*.sh`

### 6.5 v3.10.0 状态指标
- 当前阶段: ALPHA (2026-07-11 达成)
- ALPHA gate: 15/15 PASS
- 历史债务 (从 v3.9.0 继承): 4 个 OPEN (V310-06~09 PR1-4)
- 已闭环 v3.9.0 GA-P0: 2 个 (#3265 72h SOAK, #3266 168h SOAK)
- 12 个 V310 子 ISSUE: 在 plan 中, Gitea 创建阻塞
- 核心目标: 替代 MySQL 5.7 生产级

---

## 7. 报告生成元数据

- **方法**: 静态代码分析 + Cargo metadata + 文档审阅
- **数据源**:
  - `cargo metadata --offline --no-deps` (43 packages, 7 bins, 309 tests, 34 benches, 24 examples)
  - 90 个 gate scripts 文件清单
  - 9 个 v3.10.0 plan/strategy docs (V310_DEVELOPMENT_PLAN, V310_ISSUES_PLAN, V310_CLI_BINARY_PLAN, V310_VERSION_PLAN, V310_10_COVERAGE_PLAN, PARALLEL_MAIN_PATH, INDEX, ARCHITECTURE, ARCHITECTURE_DEBT_ANALYSIS, LEGACY_DEBT_CLOSURE_TRACKING_REPORT)
  - 5 阶段 STAGE_CONFIG.yaml (DRAFT/ALPHA/BETA/RC/GA)
  - 49 个 `#[ignore]` 标记 in 20 个文件
- **未做**: 未实际跑 `cargo test` 或 `cargo llvm-cov` (环境限制)
- **致 DeepSeek**: 请基于上述事实回答 §5 的 8 个问题, 给出 v3.10.0 的 binary / test / gate 改进建议。

*Generated by Claude Code (hermes-agent) 2026-07-13 00:55 CST*
