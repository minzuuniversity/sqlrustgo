# GA-Coverage-80 Baseline Report

**测量时间**: 2026-06-18
**worktree**: `/home/ai/sqlrustgo/.worktrees/ga-cov80`
**分支**: `feat/ga-coverage-80`
**HEAD**: `78e28dab0` (docs(agents): add GitNexus code intelligence section)
**应用状态**: parser.rs 改动已 stash pop（+155/-18 行），openspec/ 目录已复制

## 工具

- `cargo-llvm-cov` v0.8.5
- `rustc` 1.96.0

## Per-Crate Baseline

| Crate | Regions | Lines | Functions | vs design.md | 备注 |
|-------|---------|-------|-----------|--------------|------|
| **sqlrustgo (main)** | 13.56% (1193/8798) | 14.64% (793/5418) | 15.54% (71/457) | 14.54% ✅ 一致 | **P0 重点** |
| **sqlrustgo-parser** | 35.52% (3974/11189) | 36.61% (2341/6395) | 53.57% (165/308) | 37.53% ✅ 一致 | parser.rs 本身 27.83% |
| **sqlrustgo-executor** | 64.95% (10489/16150) | 63.12% (5807/9200) | 68.97% (680/986) | 64.95% ✅ 完全一致 | 7 文件 0% |
| **sqlrustgo-mysql-server** | 44.41% (2071/4663) | 41.37% (1124/2717) | 55.29% (141/255) | 51.10% ⚠️ 略低 | 跳过 1 pre-existing 失败 |

## design.md 中声称的 workspace 69.9% 尚未完整测量

- **原因**: workspace 整体 `cargo llvm-cov test --workspace --lib` 超过 30 分钟未完成
- **可执行替代**: 已完成 4 个主要 crate 单独测量（覆盖了 design.md 中识别的所有低覆盖率 crate）
- **待补**: 完整 workspace 测量留到 Step 6（最终验证）

## Pre-existing 测试失败（与 ga-coverage-80 无关）

| Crate | Test | 失败原因 |
|-------|------|----------|
| sqlrustgo-mysql-server | `integration_tests::test_col_type_from_string_varchar` | `col_type_from_string("VARCHAR(255)")` 返回 15 而非预期 253 (0xfd) |

**按 ADR-008 Policy 2 处理**: 这是非 gate 测试的 pre-existing 失败，**不应在 ga-coverage-80 范围内修复**。
- baseline 测量用 `--skip test_col_type_from_string_varchar` 绕过
- 建议另开 issue / branch 跟踪

## 缺口文件（0% 覆盖）— P0 提升目标

### sqlrustgo-executor (0% 覆盖，5661 missed regions 的一部分)
- `execution/facade.rs` (21 regions)
- `execution/recovery.rs` (191 regions)
- `execution/result.rs` (11 regions)
- `execution/telemetry.rs` (303 regions)
- `mutation_compiler.rs` (91 regions)
- `predicate_compiler.rs` (96 regions)
- `update_compiler.rs` (127 regions)
- `trigger_eval/resolver.rs` (3 regions)

### sqlrustgo (main, 14.64% 覆盖, 7625 missed regions)
- **engine_select.rs**: 5550 regions missed（最大缺口）
- **execution_engine.rs**: 待测量
- **engine_builder.rs**: 187 regions missed

## 下一步

进入 Step 4：按 P0 → P1 顺序编写测试用例
- P0: main crate (engine_select.rs, execution_engine.rs, engine_builder.rs) → +5,468 regions
- P1: parser SELECT/INSERT/UPDATE/DELETE → +2,400 regions
- P1: executor trigger/aggregate/join/scan → +2,423 regions
- P1: mysql-server MySQL 协议 handshake/COM_QUERY/COM_STMT_PREPARE → +1,236 regions