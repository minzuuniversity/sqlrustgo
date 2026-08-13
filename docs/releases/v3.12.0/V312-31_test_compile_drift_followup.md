# V312-31: Test Compile Drift 后续整改 — 15 个剩余 broken binary 子 ISSUE 定义

> **provenance:** generated_by=v3.12.0-remediation-round-4, generated_at=2026-08-13T..., commit=10d7ab11e4 (HEAD), source_repo=openclaw/sqlrustgo, branch=fix/v3943-r2-4-drift-status, base=develop/v3.12.0, policy=Anti-Fabrication-Policy-v1.0

> **Status**: 🟡 ANALYSIS COMPLETE — AWAITING REMEDIATION SPRINTS
> **Scope**: V312-19 (#3943) 第二轮 ripple — V312-19 PR #4140 之后仍有 15 个 test binary 因 API drift 编译失败
> **Method**: STRICT PROOF MODE — `cargo test --workspace --all-features --no-run > /tmp/build.log` (exit=101), 247 errors 实测

## 摘要

V312-19 / #3943 第一轮 ripple（commit `90baf34bc5`）修复了 `benches/*`、`test_data.rs`、以及 16 个 test 文件中的 `TableInfo` / `ColumnDefinition` 字段扩展问题。但 PR #4077 / #4140 的 API drift 还有更深一层的传播：executor/storage/page API rename、模块重组、`ExecutionEngine` generic 参数等。`cargo test --workspace --no-run` 实测发现 **15 个 test binary 仍编译失败**，分布在 `tests/{stress,integration,unit,anomaly}/`，共 **247 个 compiler error**。

> 注：用户表述为 16 个；本轮实测 15 个。差额来自 `teaching_scenario_client_server_test` 仅 1 个 error（独立子集），以及早期若干已被部分修复但仍报 error 的项。

## 实测证据（commit 10d7ab11e4 HEAD）

```
$ cargo test --workspace --all-features --no-run
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

### Top 15 failing binaries（by error count）

| # | binary | errors | 主因 | 文件 |
|---|--------|--------|------|------|
| 1 | `stress_test` | 47 | `Arc<Page>` 缺 `id()`、`ExecutionEngine::default()` 缺、`MemoryStorage` 缺 `set_cancel_flag` 等 | `tests/stress/stress_test.rs` |
| 2 | `server_integration_test` | 43 | `ExecutionEngine::default()` 缺、`Page` 缺 `calculate_checksum` / `verify_checksum`、`BufferPoolStats::new()` 缺、`PoolConfig` / `ConnectionPool` / `HttpServer` 等找不到 | `tests/integration/server_integration_test.rs` |
| 3 | `checksum_corruption_test` | 31 | `Page::calculate_checksum` / `verify_checksum` 缺 | `tests/integration/checksum_corruption_test.rs` |
| 4 | `performance_test` | 27 | `HttpServer` / `ConnectionPool` / `TeachingHttpServer` 找不到 | `tests/integration/performance_test.rs` |
| 5 | `mysql_compatibility_test` | 23 | `ExecutionEngine::default()` + 多种方法漂移 | `tests/integration/mysql_compatibility_test.rs` |
| 6 | `teaching_scenario_test` | 21 | 多模块漂移 | `tests/integration/teaching_scenario_test.rs` |
| 7 | `foreign_key_test` | 17 | `ExecutionEngine::execute_plan` 缺、`ForeignKeyConstraint` 缺 `referenced_column` 字段、`KillStatement` 找不到 | `tests/integration/foreign_key_test.rs` |
| 8 | `view_test` | 13 | `MemoryStorage::create_view` / `list_views` / `get_view` 缺 | `tests/anomaly/view_test.rs` |
| 9 | `boundary_test` | 10 | 多处 type drift | `tests/anomaly/boundary_test.rs` |
| 10 | `buffer_pool_test` | 8 | `BufferPoolStats::new` / `BufferPoolStats::misses()` 缺 | `tests/unit/buffer_pool_test.rs` |
| 11 | `columnar_storage_test` | 2 | `sqlrustgo_storage::columnar` unresolved | `tests/integration/columnar_storage_test.rs` |
| 12 | `production_scenario_test` | 2 | type drift | `tests/stress/production_scenario_test.rs` |
| 13 | `parquet_test` | 1 | `Page::calculate_checksum` 缺 | `tests/integration/parquet_test.rs` |
| 14 | `vector_storage_integration_test` | 1 | `sqlrustgo_executor::vectorization` unresolved import | `tests/integration/vector_storage_integration_test.rs` |
| 15 | `teaching_scenario_client_server_test` | 1 | `TeachingEndpoints` 找不到 | `tests/integration/teaching_scenario_client_server_test.rs` |

## 子 ISSUE 分组（5 个 V312-N）

### V312-32 / #3944: `ExecutionEngine` API drift (generic + method changes)

**owner**: minimax | **priority**: P1 | **scope**: 8 binaries

**问题摘要**:
- `ExecutionEngine<S>` 改为 generic over storage；缺 `default()` impl（18 × E0599）
- 缺 `execute_plan()` 方法（10 × E0599）
- 缺 `new_with_session()`（6 × E0599）
- `engine.storage` 字段 private，需用 `storage_ref()` accessor（1 × E0616）
- `TransactionManager::begin()` 在 `RwLockWriteGuard` 上需重新设计（4 × E0599）

**影响 binary**: stress_test, server_integration_test, mysql_compatibility_test, teaching_scenario_test, foreign_key_test, view_test, boundary_test, performance_test, production_scenario_test

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

**影响 binary**: server_integration_test, checksum_corruption_test, parquet_test, production_scenario_test, stress_test, boundary_test

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

**影响 binary**: performance_test, server_integration_test, stress_test, columnar_storage_test, vector_storage_integration_test, teaching_scenario_client_server_test, foreign_key_test, teaching_scenario_test, boundary_test

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

**影响 binary**: stress_test, server_integration_test, teaching_scenario_test, foreign_key_test, view_test, buffer_pool_test, mysql_compatibility_test, boundary_test

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

**影响 binary**: foreign_key_test, mysql_compatibility_test, teaching_scenario_test

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

- 全部 15 个 binary 编译通过（`cargo test --workspace --no-run` exit=0）
- AFP gate 仍 PASS（不许新增 allowlist 条目除非 issue 显式标 P3-DEFERRABLE）
- R2 invariants 7/8 PASS（除已知的 C-ARCH-05 execution_engine.rs 1770 > 1600 不在本次 scope）

## 不在本次 scope 的项

- C-ARCH-05（execution_engine.rs 1770 lines > 1600 limit）— 需独立 refactor sprint
- bench crate `tpch_run_query` example — 与本次 test binary scope 分开
- sqlrustgo 主 binary 启动失败 — 与本次 scope 分开

## Provenance hash

本报告生成自 commit `10d7ab11e4`（HEAD）on `fix/v3943-r2-4-drift-status`，base = `develop/v3.12.0`。
实测命令：`cargo test --workspace --all-features --no-run > /tmp/build.log 2>&1; echo EXIT=$?`  → EXIT=101