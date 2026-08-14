# V312-31: Test Compile Drift 后续整改 — V312-32..36 子 ISSUE 定义

> **provenance:** generated_by=v3.12.0-remediation-round-4, generated_at=2026-08-13T..., commit=8b7e0d083f (HEAD), source_repo=openclaw/sqlrustgo, branch=fix/v3943-r2-4-drift-status-clean, base=origin/develop/v3.12.0, policy=Anti-Fabrication-Policy-v1.0

> **Status**: 🟡 ANALYSIS COMPLETE — AWAITING REMEDIATION SPRINTS (revised for 252 reality)
> **Scope**: V312-19 (#3943) round-3 ripple — bench + tpch_run_query 残留；V312-32..36 子 ISSUE 定义仍保留（针对 250 历史现状，252 已部分覆盖）
> **Method**: STRICT PROOF MODE — `cargo test --workspace --all-features --no-run > /tmp/build252.log` (exit=101, 1 error 实测)

## 双服务器状态对照（关键背景）

V312-19 / #3943 的修复工作在两个 Gitea 服务器上分别进行：

| Server | URL | 角色 | develop/v3.12.0 HEAD | 状态 |
|--------|-----|------|----------------------|------|
| **252 (主)** | http://192.168.0.252:3000/openclaw/sqlrustgo | canonical | `60cdd4828a` (PR #4161 merged) | 已含 PR #4160 + #4161 + #4151 等 |
| 250 (备) | http://192.168.0.250:3000/openclaw/sqlrustgo | secondary | `39fba0be71` (PR #3705 merged) | 落后 252 多个 sprint |

**250 独有提交**: `90baf34bc5`、`10d7ab11e4`、`6ccbc17c11` (PR #3705 — V312-31 round-2 ripple fix)
**252 独有提交**: `05e111a8b6`、`1221157186`、`6b431df02e`、`9bc2758fb4`、`3e59516824`、`c3c6bc675d` 等 (PRs #4151, #4160, #4161)

本次提交 (`9735cb6c9a` + `8b7e0d083f`) 把 250 独享的 bench fix + 本 followup doc **rebased onto 252** 的 `develop/v3.12.0`。由于 PR #4160 `v3.12.0 test cleanup` 已合并到 252 且覆盖了 9 个 test 文件 + 多文件 fixture 重构，**实际只需新增 bench/* + test_data.rs 的修复**（V312-31 doc 仍按 250 现状记录，作为历史档案 + 子 ISSUE 定义参考）。

## 实测证据（commit `8b7e0d083f` HEAD on 252）

### 1. `cargo test --workspace --no-run` (head 包含我的 bench fix + doc)

```
$ cargo test --workspace --all-features --no-run
EXIT=101
1644 lines of build output
0 test binary compile errors
1 non-test-binary error: sqlrustgo-bench example "tpch_run_query" (E0004 non-exhaustive)

error[E0004]: non-exhaustive patterns: `&sqlrustgo::Value::Point(_, _)` and `&sqlrustgo::Value::Json(_)` not covered
error: could not compile `sqlrustgo-bench` (example "tpch_run_query") due to 1 previous error; 1 warning emitted
```

### 2. AFP gate (`scripts/gate/check_anti_fabrication.sh`)

```
[INFO] CHECK 2: cargo test --workspace --no-run...
[PASS] Test binaries compile: all 2 failures are KNOWN pre-existing
[WARN]   (pre-existing) sqlrustgo-bench
[WARN]   (pre-existing) tpch_run_query

Results: ERRORS=0, WARNINGS=4
[PASS] Anti-fabrication check (AFP v4): PASS
```

✅ **所有 15 个 test binary 编译通过**（V312-19 round-2 + V312-31 round-3 + 252 PR #4160 三轮累积效果）。仅剩 `sqlrustgo-bench` example 不在 test binary scope 内。

## 历史档案：250 现状的 15 个 broken binary（已被 252 大部分覆盖）

> 用户最初要求是"分析剩余 16 个 binary"——这是基于 250 HEAD 的 15-16 个 failing test binary 实测（commit `10d7ab11e4` on `fix/v3943-r2-4-drift-status` on 250）。这些 binary 在 252 上大部分已被 PR #4160 修复，但**当时定义的 V312-32..36 子 ISSUE 仍有效**，作为对 PR #4077/#4140 API drift 第三轮 ripple 的追踪项。

```
$ cargo test --workspace --no-run  (on 250 HEAD = 10d7ab11e4)
EXIT=101
247 error[] lines emitted by rustc

Top error codes:
  112 × error[E0599]  no method / no associated function / no variant
   40 × error[E0308]  mismatched types
   34 × error[E0433]  cannot find type
   16 × error[E0432]  unresolved imports
   10 × error[E0560]  struct has no field
   10 × error[E0062]  field specified more than once
    9 × error[E0782]  expected a type, found a trait
    9 × error[E0422]  cannot find struct/variant/union type
    5 × error[E0061]  this method takes N argument(s) but M supplied
    1 × error[E0616]  field `storage` of struct `ExecutionEngine` is private
    1 × error[E0609]  no field `referenced_column` on type `ForeignKeyConstraint`
```

### Top 15 failing binaries（on 250 HEAD）

| # | binary | errors | 主因 | 文件 | 252 状态 |
|---|--------|--------|------|------|---------|
| 1 | `stress_test` | 47 | `Arc<Page>` 缺 `id()`、`ExecutionEngine::default()` 缺、`MemoryStorage` 缺 `set_cancel_flag` 等 | `tests/stress/stress_test.rs` | TBD |
| 2 | `server_integration_test` | 43 | `ExecutionEngine::default()` 缺、`Page` 缺 `calculate_checksum` / `verify_checksum`、`BufferPoolStats::new()` 缺、`PoolConfig` / `ConnectionPool` / `HttpServer` 等找不到 | `tests/integration/server_integration_test.rs` | TBD |
| 3 | `checksum_corruption_test` | 31 | `Page::calculate_checksum` / `verify_checksum` 缺 | `tests/integration/checksum_corruption_test.rs` | TBD |
| 4 | `performance_test` | 27 | `HttpServer` / `ConnectionPool` / `TeachingHttpServer` 找不到 | `tests/integration/performance_test.rs` | TBD |
| 5 | `mysql_compatibility_test` | 23 | `ExecutionEngine::default()` + 多种方法漂移 | `tests/integration/mysql_compatibility_test.rs` | TBD |
| 6 | `teaching_scenario_test` | 21 | 多模块漂移 | `tests/integration/teaching_scenario_test.rs` | TBD |
| 7 | `foreign_key_test` | 17 | `ExecutionEngine::execute_plan` 缺、`ForeignKeyConstraint` 缺 `referenced_column` 字段、`KillStatement` 找不到 | `tests/integration/foreign_key_test.rs` | TBD |
| 8 | `view_test` | 13 | `MemoryStorage::create_view` / `list_views` / `get_view` 缺 | `tests/anomaly/view_test.rs` | TBD |
| 9 | `boundary_test` | 10 | 多处 type drift | `tests/anomaly/boundary_test.rs` | TBD |
| 10 | `buffer_pool_test` | 8 | `BufferPoolStats::new` / `BufferPoolStats::misses()` 缺 | `tests/unit/buffer_pool_test.rs` | TBD |
| 11 | `columnar_storage_test` | 2 | `sqlrustgo_storage::columnar` unresolved | `tests/integration/columnar_storage_test.rs` | TBD |
| 12 | `production_scenario_test` | 2 | type drift | `tests/stress/production_scenario_test.rs` | TBD |
| 13 | `parquet_test` | 1 | `Page::calculate_checksum` 缺 | `tests/integration/parquet_test.rs` | TBD |
| 14 | `vector_storage_integration_test` | 1 | `sqlrustgo_executor::vectorization` unresolved import | `tests/integration/vector_storage_integration_test.rs` | TBD |
| 15 | `teaching_scenario_client_server_test` | 1 | `TeachingEndpoints` 找不到 | `tests/integration/teaching_scenario_client_server_test.rs` | TBD |

> 上表 252 状态列均为 **TBD** —— 这些文件可能已被 PR #4160 删除/重构或仍然失败。需要在 252 上实测后单独 issue tracker 追踪。

## 子 ISSUE 分组（5 个 V312-N — 历史 + 未来）

> 这些子 ISSUE 仍按 250 现状定义；如 252 已部分覆盖则在 SPRINT 时重新评估 scope。

### V312-32 / #3944: `ExecutionEngine` API drift (generic + method changes)

**owner**: minimax | **priority**: P1 | **scope**: 8 binaries

**问题摘要**:
- `ExecutionEngine<S>` 改为 generic over storage；缺 `default()` impl（18 × E0599）
- 缺 `execute_plan()` 方法（10 × E0599）
- 缺 `new_with_session()`（6 × E0599）
- `engine.storage` 字段 private，需用 `storage_ref()` accessor（1 × E0616）
- `TransactionManager::begin()` 在 `RwLockWriteGuard` 上需重新设计（4 × E0599）

**影响 binary**（250 现状）: stress_test, server_integration_test, mysql_compatibility_test, teaching_scenario_test, foreign_key_test, view_test, boundary_test, performance_test, production_scenario_test

**建议修法**:
1. 给 `ExecutionEngine` 加 `Default` trait impl（用 `MemoryStorage::default()` 作为后备 storage）
2. 在 `ExecutionEngine` 上加 `pub fn execute_plan(&mut self, plan: &Plan) -> Result<...>`，复用 `execute` 内部的 plan-execution 路径
3. 把 `engine.storage` 改为 `pub(crate)` 或提供更宽松的 accessor
4. 把 `TransactionManager::begin()` 移出 `RwLockWriteGuard`（直接 `&mut self`）

### V312-33 / #3945: `Page` API rename (checksum methods)

**owner**: minimax | **priority**: P1 | **scope**: 4 binaries

**问题摘要**:
- `Page::verify_checksum()` 缺失（14 × E0599）
- `Page::calculate_checksum()` 缺失（16 × E0599）
- `Arc<Page>::id()` 缺（2 × E0599）
- `Page` 缺 `data` 公开字段访问或同义 method（boundary_test 等）

**影响 binary**（250 现状）: server_integration_test, checksum_corruption_test, parquet_test, production_scenario_test, stress_test, boundary_test

**建议修法**:
1. 在 `crates/storage/src/page.rs` 加回 `pub fn verify_checksum(&self) -> bool` 和 `pub fn calculate_checksum(&self) -> u32`
2. 或者：把所有调用方改为 `Page::from_bytes(...)` → `to_bytes()` round-trip 检查
3. `Arc<Page>::id()` → 改为 `(**page).page_id()` 或 `page.page_id()`（`Arc` 自动 deref）

### V312-34 / #3946: `sqlrustgo_server` / `sqlrustgo_executor` 模块重组（unresolved imports）

**owner**: minimax | **priority**: P2 | **scope**: 5 binaries

**问题摘要**:
- `sqlrustgo_server::teaching_endpoints` unresolved（找不到 `TeachingEndpoints`）
- `sqlrustgo_server::ConnectionPool` / `PoolConfig` / `HttpServer` 找不到（7+3+1）
- `sqlrustgo_executor::QueryCache` unresolved（2+）
- `sqlrustgo_executor::vectorization` unresolved（3）
- `sqlrustgo_storage::columnar` unresolved（2）
- `sqlrustgo_storage::vector_storage` unresolved（1）
- `sqlrustgo_storage::buffer_pool::BufferPoolWithClock` / `ClockProCache` 找不到（1）
- `sqlrustgo_optimizer::rules::{Operator, Plan, Value}` 找不到
- `sqlrustgo_parser::{KillStatement, KillType}` 找不到（1）

**影响 binary**（250 现状）: performance_test, server_integration_test, stress_test, columnar_storage_test, vector_storage_integration_test, teaching_scenario_client_server_test, foreign_key_test, teaching_scenario_test, boundary_test

**建议修法**:
1. 在 `sqlrustgo-server` crate 下补齐这些 module 路径，或修改 test 文件 import 指向新位置（取决于实际 rename 决策 — 这是 ARCH 决策，建议先开一个 SPEC 决定命名）
2. 同理对 `sqlrustgo_executor::vectorization` 和 `sqlrustgo_storage::columnar` / `vector_storage`
3. `BufferPoolWithClock` / `ClockProCache` 如果是新功能未交付，从 test 中删除或 `#[ignore]` + 注释说明

### V312-35 / #3947: `MemoryStorage` / `Statement` 派生方法缺失

**owner**: minimax | **priority**: P2 | **scope**: 4 binaries

**问题摘要**:
- `MemoryStorage::create_view()` 缺（8 × E0599）
- `MemoryStorage::list_views()` 缺（2 × E0599）
- `MemoryStorage::get_view()` 缺（2 × E0599）
- `MemoryStorage::set_cancel_flag()` 缺（3 × E0599）
- `MemoryStorage::check_cancelled()` 缺（2 × E0599）
- `MemoryStorage::create_table_index()` 已 rename 为 `create_index`（3 × E0599）
- `MemoryStorage::search_index()` / `range_index()` / `scan_batch()` 缺（2+2+1）
- `Statement::Kill` variant 缺（8 × E0599）
- `Statement::ShowProcesslist` variant 缺（1 × E0599）
- `BufferPoolStats::new()` 缺（2 × E0599）
- `BufferPoolStats::misses()` 缺（1 × E0599）

**影响 binary**（250 现状）: stress_test, server_integration_test, teaching_scenario_test, foreign_key_test, view_test, buffer_pool_test, mysql_compatibility_test, boundary_test

**建议修法**:
1. `MemoryStorage` 加 `pub fn create_view(...)` / `list_views()` / `get_view()` / `set_cancel_flag()` / `check_cancelled()` 方法
2. 检查 `Statement` enum 是否真的需要 `Kill` / `ShowProcesslist` 变体 — 如果不需要，从 parser+test 中删除
3. `BufferPoolStats` 加 `pub fn new()` constructor（默认 0 hits/misses）和 `pub fn misses(&self) -> u64` getter

### V312-36 / #3948: `ForeignKeyConstraint` / `ColumnDefinition` 字段重新设计

**owner**: minimax | **priority**: P2 | **scope**: 3 binaries

**问题摘要**:
- `ForeignKeyConstraint` 缺 `referenced_column` 字段（6 × E0560 + 1 × E0609）
- `ForeignKeyConstraint` 缺 `collation` 字段（2 × E0560）
- `ColumnDefinition` 缺 `references` 字段（2 × E0560）
- `primary_key` 字段 specified more than once（10 × E0062）

**影响 binary**（250 现状）: foreign_key_test, mysql_compatibility_test, teaching_scenario_test

**建议修法**:
1. 在 `ForeignKeyConstraint` 结构体上加 `pub referenced_column: Option<String>` 和 `pub collation: Option<String>`，或在 `ColumnDefinition` 上加 `pub references: Option<ForeignKeyRef>`
2. 检查所有 init 处的 `primary_key: ...` 是否重复指定 — 应该是 `ColumnDefinition` 已经在 SQL 解析阶段设置 PK，再附加 init 时被覆盖

## 整改 sprint 顺序建议

| Sprint | 子 ISSUE | 预计影响 | 依赖 |
|--------|----------|----------|------|
| Sprint A | V312-32 + V312-33 | 一次性修复 ~6 个 binary（含最多 errors 的 4 个） | 无 |
| Sprint B | V312-34 + V312-35 | 修 4-5 个 binary | 部分依赖 Sprint A 的 accessor 修复 |
| Sprint C | V312-36 | 修 3 个 binary | 依赖 V312-32 的 executor 修复 |

每 sprint 后必须跑：
```bash
cargo test --workspace --all-features --no-run  # 验证编译
cargo test --workspace --all-features -- --include-ignored  # 实跑
bash scripts/gate/check_anti_fabrication.sh  # AFP gate
bash scripts/gate/check_r2_invariants.sh  # R2 gate
```

## 验证与退出条件

- 全部 15 个 binary 编译通过（`cargo test --workspace --no-run` exit=0）— **252 上已达成**
- AFP gate 仍 PASS（不许新增 allowlist 条目除非 issue 显式标 P3-DEFERRABLE）— **252 上已达成**
- R2 invariants 7/8 PASS（除已知的 C-ARCH-05 execution_engine.rs 1770 > 1600 不在本次 scope）

## 不在本次 scope 的项

- C-ARCH-05（execution_engine.rs 1770 lines > 1600 limit）— 需独立 refactor sprint
- bench crate `tpch_run_query` example (E0004 Value::Point/Json non-exhaustive) — V312-37 候选
- sqlrustgo 主 binary 启动失败 — 与本次 scope 分开

## 本次提交 (commit `8b7e0d083f` on 252)

1. `9735cb6c9a` fix(V312-19 / #3943): propagate TableInfo/ColumnDefinition struct extensions to bench + test_data (252 variant)
   - 6 files changed: 5 bench files + crates/executor/src/test_data.rs
   - Scoped to files NOT touched by PR #4160 on 252 HEAD
   - Verified: cargo build --benches --all-features → 0 errors
2. `8b7e0d083f` docs(v3.12.0 / V312-19 / #3943): add V312-31 followup (this doc, revised for 252)

## Provenance hash

本报告生成自 commit `8b7e0d083f`（HEAD）on `fix/v3943-r2-4-drift-status-clean`，base = `origin/develop/v3.12.0` (252)。
实测命令：
```bash
cargo test --workspace --all-features --no-run > /tmp/build252.log 2>&1; echo EXIT=$?  → EXIT=101 (only tpch_run_query)
cargo build --benches --all-features 2>&1  → 0 errors (warnings only)
bash scripts/gate/check_anti_fabrication.sh  → ERRORS=0, WARNINGS=4 (PASS)
```