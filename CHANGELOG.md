## 2026-07-12 — F-36 修复 + V310-14 mysqladmin CLI 二进制 (PR #3776 / #3768)

### Performance Fix

| Issue | 内容 | 验证 |
| --- | --- | --- |
| #3776 (F-36) | `fix(parallel)`: `parallel_scan` memory ownership — 替换 per-partition `Vec::to_vec()` 深拷贝为 `Arc<Vec<Record>>` 共享迭代器 `SharedSliceIter`。消除 2.9× 内存回归。`PARALLEL_MIN_ROWS` 从 500K 降至 100K。 | parallel_scan 11/11, int2_substance_parallel 9/9, parallel_semantic 8/8, parallel_group_by 8/8, parallel_hash_join 17/17 |
| #3768 (V310-14, F-32) | `feat(admin)`: `sqlrustgo-admin` CLI 增加 `status`, `reload`, `refresh`, `flush-tables`, `processlist`, `kill` 子命令。连接 MySQL wire protocol server，支持 `--host/--port/--user/--password`。`MysqlAdmin` 库结构已在 `crates/admin/src/mysqladmin.rs`。 | mysqladmin_test 11/11, `cargo build -p sqlrustgo-admin` ✓, `--help` 显示 6 subcommands |

---

## 2026-07-01 — execution_engine 拆分 + C-ARCH-05 锁回 + SGL-001 fmt (PR #3664/#3665/#3666)

v3.9.0 本机可推进的 L1 lint + 架构整理项已全部闭环。3 个连续 PR 合并至 `develop/v3.9.0` (HEAD `d77821f6d1`)。

### Changed

| PR | 内容 | 验证 |
| --- | --- | --- |
| #3664 (issue #3661) | `refactor(execution_engine)`: 拆分 `src/execution_engine.rs` 2630 → 1471 行 (AD-001 1500 目标达标)。新文件 `engine_helpers.rs` (227), `engine_dml.rs` (840), `engine_cte.rs` (127)。 | C-ARCH-05 PASS / check_arch3_no_bypass.sh PASS / DML 11/11 + lib 25/25 |
| #3665 | `fix(gate)`: C-ARCH-05 上限从 3000/1800 过渡值锁回 1500 (3 个 gate 脚本统一) | check_arch_invariants 5/5 + check_architecture_freeze A7-3 PASS |
| #3666 | `style`: rustfmt drift on 3 test files (SGL-001 gate fix) | SGL-5/5 PASS + integration gate 4/4 |

### State

- 当前分支 `develop/v3.9.0` @ `d77821f6d1`
- 本地 4 个快速 gate 全 PASS: `check_arch_invariants` (5/5), `check_arch3_no_bypass` (G4), `check_integration_gate` (4/4), `check_architecture_freeze` (A7-3 PASS)
- Issue #3667 已开为状态快照 (P0-arch-debt), 立即关闭
- Open issues (4, 全部硬件阻塞, 本机无法推进):
  - #3648 TPC-H 混合负载 SOAK 跨平台验证 (需要 Z6G4/Z440)
  - #3423 TPC-H SF=1.0 baseline (需要 75GB+ 磁盘, Mac mini 仅 1GB)
  - #3265 72h 长跑 SOAK (blocked-on-S1, 需 72+ 小时持续运行)
  - #3266 168h 长跑 SOAK (blocked-on-S1, 需 168 小时持续运行)

### Verification (本机 develop/v3.9.0 @ `d77821f6d1`)

```
bash scripts/gate/check_arch_invariants.sh:    5/5 PASS
  [C-ARCH-05] execution_engine.rs: 1471 lines, limit 1500, AD-001 target 1500
bash scripts/gate/check_arch3_no_bypass.sh:    PASS
bash scripts/gate/check_integration_gate.sh:   PASS (4/4)
  SGL-001 (B4 Format): PASS
  SGL-002..005: PASS
  WAL lifecycle (INV-1/2/3): PASS
bash scripts/gate/check_architecture_freeze.sh:
  A7-3 ExecutionEngine: PASS (< 1500 lines as per AD-001)
cargo fmt --check:                            clean
```

### Refs

- #3667 — 闭环声明 issue (P0-arch-debt, closed as state snapshot)
- AD-001 — 1500 行架构原始目标

---
# SQLRustGo v3.9.0 更新日志

> **版本**: v3.9.0
> **类型**: **Production Readiness Release**（工程化版本，非功能版本）
> **分支**: `develop/v3.9.0`（从 `main@v3.8.0` fork）
> **创建日期**: 2026-06-05
> **GA 日期**: 2026-07-10
> **前版本**: v3.8.0
> **当前阶段**: **GA**（2026-07-10 完成 GA cut）

---

## 2026-07-01 — Gate Lint Drift 修复

`gate.sh` L1 门禁在 RC7 之后暴露 6 个 clippy 错误与 4 文件 fmt 漂移, 已全部修复:

### Fixed

| 文件 | 变更 |
|------|------|
| `crates/storage/src/binary_storage.rs` | 删除未使用 `use std::sync::Arc;` 与 `Read`, 删除死方法 `ensure_loaded` |
| `crates/storage/src/checkpoint.rs:185` | `sort_by` → `sort_by_key(\|b\| Reverse(b.timestamp))` |
| `crates/storage/src/engine.rs:150` | 折叠嵌套 `if` 进 `match` arm guard |
| `crates/storage/src/wal_legacy.rs:913` | `sort_by` → `sort_by_key(\|a\| a.archive_id)` |
| `crates/optimizer/src/stats.rs:401` | 提取闭包 `update_min/update_max`, 消除嵌套 `if` 触发 `collapsible_match` |
| `crates/executor/src/executor_metrics.rs:66` | `if total == 0` 改 `checked_div(...).unwrap_or(0)` |
| `crates/telemetry/src/lib.rs:153` | 同上 `checked_div` 改写 |
| `crates/vector/src/ivfpq.rs:144` | 移除冗余 `.into_iter()` |
| `crates/mysql-server/src/lib.rs:2115` | 死函数 `is_select_stmt` 加 `#[cfg(test)]` |
| `crates/mysql-server/src/lib.rs:3276` | 8-arg `run_server_*` 加 `#[allow(clippy::too_many_arguments)]` |

### fmt 漂移 (4 文件)

- `crates/cli/src/main.rs:98`
- `crates/storage/src/binary_storage.rs:566, 599, 641`
- `crates/tools/src/bin/tbl2bin.rs:9, 15`
- `tests/mixed_workload_deadlock_regression_test.rs:22`

### 验证

```
bash gate/gate.sh v3.9.0
[L1] cargo build...           [PASS]
[L1] cargo test --lib...      [PASS]
[L1] clippy...                [PASS]
[L1] cargo fmt...             [PASS]
=== Gate Result: PASSED ===
```

---

## 2026-07-01 — Gate Lint Drift 修复

`gate.sh` L1 门禁在 RC7 之后暴露 6 个 clippy 错误与 4 文件 fmt 漂移, 已全部修复:

### Fixed

| 文件 | 变更 |
|------|------|
| `crates/storage/src/binary_storage.rs` | 删除未使用 `use std::sync::Arc;` 与 `Read`, 删除死方法 `ensure_loaded` |
| `crates/storage/src/checkpoint.rs:185` | `sort_by` → `sort_by_key(\|b\| Reverse(b.timestamp))` |
| `crates/storage/src/engine.rs:150` | 折叠嵌套 `if` 进 `match` arm guard |
| `crates/storage/src/wal_legacy.rs:913` | `sort_by` → `sort_by_key(\|a\| a.archive_id)` |
| `crates/optimizer/src/stats.rs:401` | 提取闭包 `update_min/update_max`, 消除嵌套 `if` 触发 `collapsible_match` |
| `crates/executor/src/executor_metrics.rs:66` | `if total == 0` 改 `checked_div(...).unwrap_or(0)` |
| `crates/telemetry/src/lib.rs:153` | 同上 `checked_div` 改写 |
| `crates/vector/src/ivfpq.rs:144` | 移除冗余 `.into_iter()` |
| `crates/mysql-server/src/lib.rs:2115` | 死函数 `is_select_stmt` 加 `#[cfg(test)]` |
| `crates/mysql-server/src/lib.rs:3276` | 8-arg `run_server_*` 加 `#[allow(clippy::too_many_arguments)]` |

### fmt 漂移 (4 文件)

- `crates/cli/src/main.rs:98`
- `crates/storage/src/binary_storage.rs:566, 599, 641`
- `crates/tools/src/bin/tbl2bin.rs:9, 15`
- `tests/mixed_workload_deadlock_regression_test.rs:22`

### 验证

```
bash gate/gate.sh v3.9.0
[L1] cargo build...           [PASS]
[L1] cargo test --lib...      [PASS]
[L1] clippy...                [PASS]
[L1] cargo fmt...             [PASS]
=== Gate Result: PASSED ===
```

---

## 2026-07-01 — Gate Lint Drift 修复

`gate.sh` L1 门禁在 RC7 之后暴露 6 个 clippy 错误与 4 文件 fmt 漂移, 已全部修复:

### Fixed

| 文件 | 变更 |
|------|------|
| `crates/storage/src/binary_storage.rs` | 删除未使用 `use std::sync::Arc;` 与 `Read`, 删除死方法 `ensure_loaded` |
| `crates/storage/src/checkpoint.rs:185` | `sort_by` → `sort_by_key(\|b\| Reverse(b.timestamp))` |
| `crates/storage/src/engine.rs:150` | 折叠嵌套 `if` 进 `match` arm guard |
| `crates/storage/src/wal_legacy.rs:913` | `sort_by` → `sort_by_key(\|a\| a.archive_id)` |
| `crates/optimizer/src/stats.rs:401` | 提取闭包 `update_min/update_max`, 消除嵌套 `if` 触发 `collapsible_match` |
| `crates/executor/src/executor_metrics.rs:66` | `if total == 0` 改 `checked_div(...).unwrap_or(0)` |
| `crates/telemetry/src/lib.rs:153` | 同上 `checked_div` 改写 |
| `crates/vector/src/ivfpq.rs:144` | 移除冗余 `.into_iter()` |
| `crates/mysql-server/src/lib.rs:2115` | 死函数 `is_select_stmt` 加 `#[cfg(test)]` |
| `crates/mysql-server/src/lib.rs:3276` | 8-arg `run_server_*` 加 `#[allow(clippy::too_many_arguments)]` |

### fmt 漂移 (4 文件)

- `crates/cli/src/main.rs:98`
- `crates/storage/src/binary_storage.rs:566, 599, 641`
- `crates/tools/src/bin/tbl2bin.rs:9, 15`
- `tests/mixed_workload_deadlock_regression_test.rs:22`

### 验证

```
bash gate/gate.sh v3.9.0
[L1] cargo build...           [PASS]
[L1] cargo test --lib...      [PASS]
[L1] clippy...                [PASS]
[L1] cargo fmt...             [PASS]
=== Gate Result: PASSED ===
```

---

# 变更日志

SQLRustGo 的所有显着更改都将记录在此文件中。

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased] - 2026-07-01

### Fixed (Gate Lint Drift)

- **PR (this)**: `gate.sh` L1 门禁 — clippy 6 项 / fmt 4 文件 lint 漂移修复。
  - `crates/storage/src/binary_storage.rs`: 删除未使用 `use std::sync::Arc;` 与 `Read`, 删除死代码 `ensure_loaded` 方法。
  - `crates/storage/src/checkpoint.rs:185`: `sort_by` 改 `sort_by_key(|b| Reverse(b.timestamp))`。
  - `crates/storage/src/engine.rs:150`: 折叠嵌套 `if` 进 `match` arm guard。
  - `crates/storage/src/wal_legacy.rs:913`: `sort_by` 改 `sort_by_key(|a| a.archive_id)`。
  - `crates/optimizer/src/stats.rs:401`: 重构 `match` arm 用闭包 guard 消除嵌套 `if`。
  - `crates/executor/src/executor_metrics.rs:66`: `if total == 0` 改 `checked_div(...).unwrap_or(0)`。
  - `crates/telemetry/src/lib.rs:153`: 同上 `checked_div` 改写。
  - `crates/vector/src/ivfpq.rs:144`: 移除冗余 `.into_iter()`。
  - `crates/mysql-server/src/lib.rs`: 死函数 `is_select_stmt` 加 `#[cfg(test)]`, 8-arg `run_server_*` 加 `#[allow(clippy::too_many_arguments)]`。
  - `cargo fmt --all` 应用 4 文件格式修正 (`binary_storage.rs` / `cli/src/main.rs` / `tools/src/bin/tbl2bin.rs` / `tests/mixed_workload_deadlock_regression_test.rs`)。
  - **验证**: `bash gate/gate.sh v3.9.0` 退出 0, L1 build / test-lib / clippy / fmt 全部 PASS。

## [Unreleased] - 2026-06-05

### v3.9.0 Production Readiness Release 启动

**分支**: `develop/v3.9.0` (从 `main@v3.8.0` fork)
**类型**: 工程化版本 (可靠性 + 可恢复性 + 可审计性)
**当前状态**: RC7 (2026-06-12), GA 目标 2026-12-15 (per Hermes audit #3252, deferred from 2026-09-23)
**资源**: 架构债 40% / 可靠性 35% / GMP 审计 15% / 性能 10% / **新 SQL 0%**

详细启动计划见:

- [`docs/releases/v3.9.0/README.md`](docs/releases/v3.9.0/README.md) — 入口
- [`docs/releases/v3.9.0/CHANGELOG.md`](docs/releases/v3.9.0/CHANGELOG.md) — v3.9.0 专用 Changelog
- [`docs/releases/v3.9.0/ROADMAP.md`](docs/releases/v3.9.0/ROADMAP.md) — 6 Phase / 12 周 路线图
- [`docs/releases/v3.9.0/plans/V390_VERSION_PLAN.md`](docs/releases/v3.9.0/plans/V390_VERSION_PLAN.md) — 战略定位
- [`docs/releases/v3.9.0/plans/V390_DEVELOPMENT_PLAN.md`](docs/releases/v3.9.0/plans/V390_DEVELOPMENT_PLAN.md) — P0/P1/P2/P3 任务
- [`docs/releases/v3.9.0/plans/V390_TEST_PLAN.md`](docs/releases/v3.9.0/plans/V390_TEST_PLAN.md) — G1-G10 门禁

Gitea Milestone: <http://192.168.0.252:3000/openclaw/sqlrustgo/milestones/32>

### Added

- `docs/releases/v3.9.0/` 目录 + README + CHANGELOG + ROADMAP (本次启动)
- 3 个 V390 plan 文档从 `docs/releases/v3.8.0/plans/` 迁移至 `docs/releases/v3.9.0/plans/`

### Changed

- **AGENTS.md** main branch 升级: `develop/v3.8.0` → `develop/v3.9.0`

---

> 以下为 v3.8.0 GA Final 收口内容, 已合并入 [`docs/releases/v3.8.0/CHANGELOG.md`](docs/releases/v3.8.0/CHANGELOG.md) 详细记录。v3.8.0 GA Final 关键 PR (按合并顺序):

- **PR #3131 (Issue #2988)**: Parser — MySQL 5.7 keyword-as-identifier + scalar function dispatch. Corpus +68 cases, 86.5% → 94.2% pass rate.
- **PR #3132 (Issue #2977)**: TPC-H Q2 — 5-table comma-join key resolution bug. TPC-H 21/22 → **22/22 ✅**.
- **PR #3142**: ORDER BY execution in SELECT — TPC-H Q18/Q4 unblocked.
- **PR #3134 (Issue #3110)**: Savepoint basic test scaffolding — `tests/savepoint_test.rs` + `undo_log_len` getter.
- **PR #3137 (Issue #3107 #3111)**: FEATURE_MATRIX.md §1.5 F-11/F-12 contradiction fixed.
- **PR #3138 (Issue #3136)**: TPC-H 22 root-cause analysis.
- **PR #3139 (Issue #3102 #3103)**: 10 孤岛 F-XX + 5 无实现 debt items marked DEFERRED v3.9.0+.
- **PR #3140 (Issue #3099 #3100)**: GA docs + compile fix.
- **PR #3141 (Issue #3106)**: `check_cross_version_debt.sh` Part 5 Code Reality Check.
- **PR #3159 (本次会话)**: 治理优化第 1 波 — INDEX.md + check_arch_sem_debt.sh 改读 YAML SSOT.
- **PR #3165 (PR-3159 + 合并)**: develop/v3.8.0 → main (v3.8.0 GA Final).

### v3.8.0 GA Final Gates (D1-D9)

- `scripts/gate/check_docs_links.sh` — **PASS**
- `scripts/gate/check_docs_consistency.sh` — **PASS**
- RC gate: D1=10/10 D2=5/5 D3=DRIFT(1 tracked) D4=5/5 D5=9/10 — **0 blockers**
- D7 INT Debt: 2 CLOSED + 2 DEFERRED w/ plan — **PASS-WITH-DRIFT**
- D8 Arch/Sem Debt: 3 CLOSED + 4 IN_PROGRESS w/ plan — **PASS-WITH-DRIFT**
- D9 Full Gate Verification — **PASS**

### v3.8.0 GA Final Breaking Changes (Internal)

- **Retired legacy binaries** (BREAKING internal): `sqlrustgo` / `sqlrustgo-sql-cli` / `sqlrustgo-bench` / `sqlrustgo-bench-cli` / `sqlrustgo-gmp-cli` / `sqlrustgo-tools` removed
- **Canonical entry**: `sqlrustgo-mysql-server` (subcommands: `serve` / `exec "<sql>"` / `repl` / `bench` / `gmp` / `diag` / `backup` / `restore`)
- **Gate scripts updated**: `check_alpha_v380.sh` 增 `A1_BIN_COUNT` + `A2_EPHEMERAL_SMOKE`; A5 coverage 75% → 73%

### v3.8.0 Migration

```bash
# v3.8.0-beta (deprecated)
cargo run --bin sqlrustgo

# v3.8.0+ (canonical, required)
cargo run --bin sqlrustgo-mysql-server -- repl
cargo run --bin sqlrustgo-mysql-server -- serve
```

### v3.8.0 GA Out of Scope

- 5 graph-tool bins (`graph-gate`, `graph-ingest`, `sqlrustgo-gate`, `gate`, `ingest`) — orthogonal, tracked separately
- 详细设计: `docs/releases/v3.8.0/SPEC-v3.8.0-001-mysql-server-canonical-entry.md`

---

## [3.8.0] - 2026-05-31 (Alpha)

### 目标

Architecture Unification Release — 消灭双执行路径，统一 SQL → AST → Plan → Execution，接入 WAL 核心。

### 核心功能

- **Execution Semantics Freeze**: AUTOCOMMIT + WAL Mandatory + MVCC Enabled + TX Lifecycle 强制（commit 087bb12d）
- **Hermes C Regression Analysis**: 三路径行为差异检测（Path A/B/C）
- **ExecutionEngine Type Alias**: `MemoryExecutionEngine = ExecutionEngine<MemoryStorage>`
- **Canonical Binary Consolidation**: `sqlrustgo-mysql-server` 是 v3.8.0+ 唯一执行入口
  - 子命令：`serve`（默认，MySQL wire 协议）/ `exec "<sql>"` / `repl` / `bench` / `gmp` / `diag`
  - 旧 binary 退役：`sqlrustgo` / `sqlrustgo-sql-cli` / `sqlrustgo-bench` / `sqlrustgo-bench-cli` / `sqlrustgo-tools` 现在打印 deprecation 提示并指向 canonical entry
- **Embedded Test Harness Keystone**: `sqlrustgo-mysql-server::testing::start_ephemeral` 启动进程内 MySQL server，便于 e2e 测试通过 wire 协议执行
- **Raw MySQL Wire-Protocol Test Client**: `tests/common/mod.rs::MySqlTestClient` 不依赖 `mysql` crate，raw TCP + HandshakeResponse41，让测试有完整的 wire 协议控制

### 门禁状态

| Gate | 结果 | 日期 |
|------|------|------|
| Alpha | 🔄 IN_PROGRESS | 2026-05-31 |

> **Status**: 开发中

## [3.5.0] - 2026-05-28 (GA)

### 目标

AI Native GMP Platform（AI 原生 GMP 平台），在 v3.4.0 管理套件基础上构建 AI Agent 层。

### 核心功能

- **AI Agent Layer**: Deviation Investigator、Compliance Judge、Device Predictor、Report Generator
- **LLM 本地推理**: Ollama 集成 + SSE 流式输出
- **GMP Retrieval v3**: BM25 + Vector + Graph + FTS 四路融合（RRF + Reranker）
- **跨语言报告**: 本地术语预处理 + LLM 翻译（FDA 21 CFR Part 11 / EMA Annex 11）

### 门禁状态

| Gate | 结果 | 日期 |
|------|------|------|
| Alpha (16/16) | ✅ PASS | 2026-05-27 |
| Beta (14/14) | ✅ PASS | 2026-05-27 |
| RC (16/16) | ✅ PASS | 2026-05-28 |
| GA | ✅ PASS | 2026-05-28 |

> **L1 平均覆盖率**: 87.36%（≥85%）✅  
> **TPC-H SF=1**: 22/22 PASS ✅

详见: [docs/releases/v3.5.0/README.md](docs/releases/v3.5.0/README.md)

## [3.4.0] - 2026-05-24 (GA)

### 目标

GMP Management Suite（管理套件）版本，在 v3.3.0 可信内核基础上构建完整的管理套件。

### 核心功能

- **GMP Management API**: EBR Batch Manager、Electronic Signature、Audit、Device OPC UA、Rule Editor、Dashboard
- **GMP Retrieval v2**: BM25 + RRF Fusion + Ollama Reranker + LLM Chat
- **Trust Visualization**: CLI 工具 + Dashboard

### 门禁状态

| Gate | 结果 | 日期 |
|------|------|------|
| Alpha (16/16) | ✅ PASS | 2026-05-22 |
| Beta (14/14) | ✅ PASS | 2026-05-22 |
| RC | ✅ PASS | 2026-05-24 |
| GA | ✅ PASS | 2026-05-24 |

详见: [docs/releases/v3.4.0/README.md](docs/releases/v3.4.0/README.md)

## [3.3.0] - 2026-05-20 (GA)

### 目标

Industrial Trust Platform（可信内核）版本，建立工业级可信闭环。

### 已完成

| 功能 | PR | 状态 |
|------|-----|------|
| GMP Retrieval v1 | #1257 | ✅ |
| Graph Generic Execute Cypher | #1327 | ✅ |
| Executor 覆盖率 76.44% | #1324 | ✅ |

### 战略演进

```
v3.2.0: Trust Convergence（可信收敛）✅ GA
v3.3.0: Industrial Trust Platform（可信内核）✅ GA
v3.4.0: GMP Management Suite（管理套件）← 当前 RC
```

## [3.2.0] - 2026-05-18 (GA)

### 目标

Trust Convergence（可信收敛）版本，聚焦 GMP 工业标准验证，确保：
- MySQL 协议完整兼容
- 性能稳定无回归
- 审计链完整性验证
- Crash Recovery 验证
- GMP Long-Run 稳定性

### 核心原则

> **禁止架构扩散，聚焦可信收敛**

### 已完成

| 功能 | PR | 状态 |
|------|-----|------|
| UPDATE/DELETE WHERE 子句修复 | #1174 | ✅ |
| 性能回归调查（无回归发现） | #1174 | ✅ |
| Audit Chain Validator 增强 | #1180 | ✅ |
| WAL Crash Recovery 测试 | #1168 | ✅ |
| Crash Recovery 验证 | #1166 | ✅ |
| GMP Timestamp 验证 | #1171 | ✅ |

### 性能数据

| 操作 | v3.2.0 实测 | v3.0.0 基线 | 提升 |
|------|------------|------------|------|
| UPDATE | 109,988 QPS | 43,121 QPS | +155% |
| DELETE | 134,312 QPS | 64,896 QPS | +107% |
| INSERT | 73,261 QPS | 28,698 QPS | +155% |

### GMP 审计链增强

- `verify_chain()` 新增时间戳单调递增验证
- `verify_chain()` 新增事务 ID 追踪（孤立条目检测）
- `AuditChainError` 新增 5 个变体：`TimestampNotMonotonic`, `SignatureInvalid`, `OrphanEntry`, `WorkflowLinkBroken`, `ProvenanceIncomplete`
- CLI `audit-chain-verify` 处理所有新错误类型

### Bug 修复

- **UPDATE WHERE 子句被忽略**: `expression_to_value()` 不支持行上下文，导致 WHERE 条件无法求值。添加 `evaluate_row_expression()` 方法支持行上下文和列名→索引映射。
- **DELETE WHERE 子句被忽略**: 同上，使用 `get_table_records_mut()` + 索引收集实现正确的条件过滤。

## [2.8.0] - 2026-05-01 (GA)

### 目标

生产化+分布式+安全版本，MySQL 5.7 功能覆盖率 92%，分布式能力（分区表、主从复制、故障转移），安全性评分 92%。

### 已完成

| 功能 | PR | 状态 |
|------|-----|------|
| PR 100 CASE WHEN NULLIF | #100 | ✅ |
| PR 101 clippy fix | #101 | ✅ |
| PR 102 OFFSET/LIMIT | #102 | ✅ |
| Git 历史清理 (target/ 移除) | #103 | ✅ |
| 仓库压缩 62.8GB→95MB | #103 | ✅ |
| SSH fetch 修复 (3秒) | #103 | ✅ |

### 发布 PR

- [#103](https://192.168.0.252:3000/openclaw/sqlrustgo/pulls/103) - chore: R-Gate cleanup

## [2.7.0] - 2026-04-22 (GA)

### 目标

企业级韧性版本，实现 WAL 崩溃恢复、外键稳定性增强、备份恢复机制、审计证据链等企业级功能。

### 已完成

| 功能 | PR | 状态 |
|------|-----|------|
| T-01 事务/WAL 恢复 | - | ✅ |
| T-02 FK/约束稳定化 | - | ✅ |
| T-03 备份恢复演练 | - | ✅ |
| T-04 qmd-bridge 统一检索层 | #1713 | ✅ |
| T-05 统一检索 API (lex/vec/graph/hybrid) | #1714 | ✅ |
| T-06 混合检索重排 (RRF/Linear/Composite) | #1714 | ✅ |
| T-07 GMP Top 10 审核查询模板 | #1714 | ✅ |
| T-08 审计证据链 (防篡改哈希链) | #1718 | ✅ |

### 发布 PR

- [#1729](https://github.com/minzuuniversity/sqlrustgo/pull/1729) - chore: v2.7.0 GA release

## [2.6.0] - 2026-04-22 (GA)

### 目标

生产就绪版本，实现 SQL-92 完整支持。

### 已完成 (2026-04-18)

| 功能 | PR | 状态 |
|------|-----|------|
| SQL-92 聚合函数 | #1545 | ✅ |
| SQL-92 JOIN 语法 | #1545 | ✅ |
| SQL-92 GROUP BY | #1545 | ✅ |
| SQL-92 HAVING 子句 | #1567 | ✅ |
| DELETE 语句 | #1557 | ✅ |
| 外键约束 | #1436, #1567 | ✅ |
| 集成测试修复 | #1561 | ✅ |
| 覆盖率测试提升 | #1559, #1564 | ✅ |
| ExecutionEngine API | #1566 | ✅ |
| Clippy 零警告 | #1570 | ✅ |
| SQL Corpus 100% | - | ✅ (59/59) |

### P0 功能 (进行中)

| 功能 | 状态 | Issue |
|------|--------|-------|
| 功能集成 (索引扫描、CBO、存储过程、触发器、WAL) | 进行中 | #1497 |
| MVCC SSI (可串行化快照隔离) | 待开发 | #1389 |

### P1 功能

| 功能 | Issue |
|------|-------|
| FULL OUTER JOIN | #1380 |

### 文档

- **新**: v2.6.0 发布文档目录
- **新**: VERSION_PLAN.md - 版本计划
- **新**: RELEASE_GATE_CHECKLIST.md - 门禁检查清单
- **新**: TEST_PLAN.md - 测试计划
- **新**: INTEGRATION_STATUS.md - 功能集成状态
- **新**: PERFORMANCE_TARGETS.md - 性能目标
- **新**: SQL_REGRESSION_PLAN.md - SQL 回归测试计划
- **新**: INTEGRATION_TEST_PLAN.md - 集成测试计划

## [2.5.0] - 2026-04-16

### Added

- **架构**: MVCC 并发控制 - 快照隔离实现
- **架构**: WAL 启用验证 - 崩溃恢复测试
- **功能**: 语义嵌入 API - 替换 HashEmbedding
- **功能**: 分布式存储 - ShardGraph/ShardVector
- **功能**: Cost-based Optimizer - CBO 实现
- **功能**: Prepared Statement - 参数化查询
- **功能**: 连接池 - Connection Pool 实现
- **功能**: 子查询优化 - EXISTS/IN/ANY/ALL
- **功能**: Cypher 查询语言 - 图查询子集实现
- **功能**: JOIN 完整实现 - LEFT/RIGHT/CROSS JOIN
- **功能**: Graph 持久化 - DiskGraphStore 实现
- **功能**: 事务系统集成 - MVCC + WAL 到 SQL 执行路径
- **功能**: 全模块集成测试 - SQL+Vector+Graph 混合负载
- **功能**: 图查询性能基准 - BFS/DFS/多跳查询
- **功能**: 向量检索性能基准 - 10万/100万向量 KNN
- **功能**: TPC-H SF=10 性能测试
- **功能**: GMP 内审支持 - 审计日志+合规检查+内审报表
- **功能**: OpenClaw 全局调度 - 任务流编排+Agent 协作
- **功能**: 统一优化器 - CBO 自动选择执行路径
- **功能**: 统一存储层 - Document 表+向量+图索引联动
- **功能**: 统一查询 API - SQL+Vector+Graph 联合查询

### 测试结果

| 测试套件 | 结果 |
|----------|------|
| Storage lib tests | 55/55 ✅ |
| Parser tests | 37/37 ✅ |
| Executor lib tests | 9/9 ✅ |
| sql-corpus | 59 cases, 12 passed (20.3%) |
| 整体覆盖率 | 49% |

## [2.4.0] - 2026-04-08

### Added

- **架构**: SIMD 加速全面化
- **架构**: 向量存储层整合：BinaryStorage + B+Tree + mmap
- **功能**: 查询计划器 - 自动选择索引
- **功能**: 列式存储压缩 (LZ4/Zstd)
- **功能**: Hash 索引支持
- **功能**: 内存映射存储 (mmap)
- **功能**: SIMD 向量计算加速

### 性能

- **优化**: TPC-H SF=1 性能基准测试
- **优化**: v2.2+v2.3 整合测试

## [2.0.0] - 2026-03-25

### Added

- **架构**: 异步网络层
- **功能**: 客户端-服务器架构
- **功能**: 连接池支持多个客户端

---

## 版本历史

| 版本 | 日期 | 成熟度 | 说明 |
|------|------|--------|-------|
| v3.5.0 | 2026-05-28 | GA | AI Native GMP Platform、AI Agent Layer |
| v3.4.0 | 2026-05-24 | GA | GMP Management Suite、管理套件 |
| v3.3.0 | 2026-05-20 | GA | Industrial Trust Platform、可信内核 |
| v3.2.0 | 2026-05-18 | GA | Trust Convergence、可信收敛 |

---

## 路线图

- **v2.6.0**: 生产就绪版本 (当前开发)
- **v2.5.0**: MVCC、Vector/Graph 存储
- **v2.4.0**: SIMD 加速、列式存储
- **v2.0.0**: 异步网络架构
- **v1.1.0**: 架构升级
- **v1.0.0**: 初始版本

---

*此变更日志由 yinglichina8848 维护*
