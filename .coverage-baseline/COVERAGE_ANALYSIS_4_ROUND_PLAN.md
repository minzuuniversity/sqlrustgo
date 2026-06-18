# Coverage Analysis & 4-Round Improvement Plan

**Generated**: 2026-06-18
**Source Issue**: #3534 (sqlrustgo Gitea)
**Related PR**: #3528
**Branch**: `feat/ga-coverage-80`
**HEAD**: 283697de3

## 背景

Workspace 当前覆盖率 **70.29% regions / 69.50% lines**，距 GA gate 目标 80-85% 仍有约 10-15pp 缺口。

本次会话通过 PR #3528 完成：
- 3 个 pre-existing test failures 解决（无需 `--skip`）
- 3 个 ignored tests un-ignored（mmap module + 三值逻辑 + FK parser）
- 7 个 perf benchmarks 移到 benches/（净减少 8 个 lib tests ignored）
- 4 个 production bugs 修复
- workspace total: 70.03% → 70.29% (+0.23pp)

## 当前状态

| 指标 | 数值 |
|------|------|
| Workspace total | 70.29% regions / 69.50% lines |
| GA gate 目标 | 80-85% |
| 缺口 | ~10-15pp |
| 顶层集成测试 | 184 个 `tests/*.rs` 文件 |
| E2E 测试 | 3 个 (e2e_query, monitoring, observability) |
| Crate 内测试 | 32 个 (`crates/*/tests/*.rs`) |
| Ignored lib tests | 1 (mmap - seccomp 环境限制) |

## Per-Crate Baseline

| Crate | Lines | Status | Notes |
|-------|-------|--------|-------|
| sqlrustgo (main) | 26.45% | +11.81pp in this PR | 6 engine_builder + 5 engine_select tests |
| sqlrustgo-parser | 36.61% | +155 lines from stash (109 tests) | FK test un-ignored |
| sqlrustgo-executor | 63.12% | +5 tests (370 total) | 7 small 0% files remain |
| sqlrustgo-mysql-server | 41.37% | pre-existing test fixed | skip test_col_type_from_string_varchar no longer needed |
| sqlrustgo-vector | ~85% | 6 perf tests moved to benches/ | 1 mmap test ignored (env) |
| sqlrustgo-storage | 73.47% | +65% mmap_vector_store.rs | 1 mmap_save_to_file ignored |
| sqlrustgo-bench | 100% | perf test removed | 0 ignored |

## SQL 功能覆盖矩阵

| SQL 类别 | 集成测试数 | 状态 | 详情 |
|----------|-----------|------|------|
| DQL 基础 (SELECT/WHERE/JOIN) | 62+ | ✅ 强 | join, subquery, group_by, cte |
| DQL 高级 (CTE, Window) | 4-90 | ✅ | CTE 强，Window 仅 4 files 偏少 |
| **DDL** (CREATE/DROP/ALTER) | **2** | ⚠️ 弱 | 只有 alert_tests / firewall_tests |
| **DML** (INSERT/UPDATE/DELETE) | **0** | ❌ 缺失 | 没有专门的 DML 集成测试文件 |
| Trigger | 15 | ✅ | |
| View | 8 | ✅ | |
| Index | 23 | ✅ | |
| Constraint | 9 | ✅ | 含 FK（已修） |
| MVCC/Isolation | 18 | ✅ | |
| WAL/Recovery | 39 | ✅ | |
| Checkpoint | 9 | ✅ | |
| Vector | 5 | ✅ | |
| Graph/Cypher | 2 | ⚠️ 弱 | |
| **UNION/INTERSECT/EXCEPT** | **0** | ❌ 完全缺失 | set operations 0 测试 |
| Replication | 2 | ⚠️ 弱 | 在 distributed crate |

## 0% 覆盖率文件清单（最大缺口）

### Executor crate (1091 missed lines)

| 文件 | Lines | 影响 | ROI |
|------|-------|------|-----|
| `ast_adapter.rs` | 138 | AST↔Internal 转换，影响所有 SQL 路径 | 🔴 极高 |
| `execution/telemetry.rs` | 303 | 收集执行遥测数据 | 🔴 极高 |
| `execution/recovery.rs` | 191 | 执行恢复逻辑 | 🟠 高 |
| `predicate_compiler.rs` | 96 | 谓词编译 | 🟡 中 |
| `mutation_compiler.rs` | 91 | Mutation 编译 | 🟡 中 |
| `execution/facade.rs` | 21 | 简单 facade | 🟢 简单 |
| `execution/result.rs` | 11 | Result 处理 | 🟢 简单 |
| `trigger_eval/resolver.rs` | 3 | 几乎免费 | 🟢 简单 |

### Storage crate (800+ missed lines)

| 文件 | Lines | 影响 | ROI |
|------|-------|------|-----|
| `vtu_ir/predicate_ir.rs` | 286 | Vectorized update 谓词 IR | 🟠 高 |
| `vtu_ir/update_plan.rs` | 57 | VTU update plan | 🟡 中 |
| `vtu_ir/mutation_ir.rs` | 35 | VTU mutation IR | 🟡 中 |
| `wal/mod.rs` | 73 (23% 覆盖) | WAL 接口 | 🟡 中 |

**总和 ~1900 missed lines = 约 1.5-2pp workspace coverage 提升空间**

## 4 轮改进计划

### Round 1: High-ROI 0% Files (5h, 预期 +1.5-2pp) - Issue #3535

- [ ] **task-1.1** `ast_adapter.rs` (138 lines) - 1-2h, 预期 +0.3-0.5pp
- [ ] **task-1.2** `execution/telemetry.rs` (303 lines) - 1h, 预期 +0.5-0.8pp
- [ ] **task-1.3** `vtu_ir/predicate_ir.rs` (286 lines) - 2h, 预期 +0.5-0.7pp
- [ ] **task-1.4** `execution/recovery.rs` (191 lines) - 1-2h, 预期 +0.3-0.5pp

### Round 2: 集成测试空白 (3h, 预期 +0.5-0.8pp + 填补功能空白) - Issue #3536

- [ ] **task-2.1** UNION/INTERSECT/EXCEPT 集成测试 - 1h, 预期 +0.2-0.3pp
  - 创建 `tests/union_set_operations_test.rs`
- [ ] **task-2.2** DML 集成测试 (INSERT/UPDATE/DELETE) - 2h, 预期 +0.3-0.5pp
  - 创建 `tests/dml_integration_test.rs`

### Round 3: Small 0% Files + 高级功能 (4h, 预期 +0.7-1pp) - Issue #3537

- [ ] **task-3.1** 7 个 small executor 0% 文件 - 1h, 预期 +0.3-0.5pp
- [ ] **task-3.2** 3 个 vtu_ir 0% 文件 - 1-2h, 预期 +0.2-0.3pp
- [ ] **task-3.3** Graph/Cypher 集成测试 - 2h, 预期 +0.2-0.3pp + 填补空白

### Round 4: 清理与验证 (3h) - Issue #3538

- [ ] **task-4.1** 检查并删除 dead code - 1h
- [ ] **task-4.2** 重新跑完整 workspace 覆盖率验证 - 30min
- [ ] **task-4.3** 更新 PR #3528 描述 + 准备 GA 门禁提交 - 1.5h

## 预期总结果

| 阶段 | Workspace | Delta |
|------|-----------|-------|
| 当前 | 70.29% / 69.50% | baseline |
| After Round 1 | ~71.5-72% | +1.5-2pp |
| After Round 2 | ~72-73% | +0.5-0.8pp |
| After Round 3 | ~73-74% | +0.7-1pp |
| After Round 4 | ~74-75% | dead code 清理 |

**根本限制**: 即使完成全部 4 轮（约 15h），workspace 仍可能距 80% 差 5-6pp。要达到 80% 目标需要**激进的 dead code 清理 + 减少 ignored perf 依赖**。

## 风险评估

1. **P12 ignore count 真实性**: 当前 35 ignored (含 1 个 mmap 环境限制)，需确保都是合理 ignored
2. **功能 vs 覆盖率 tradeoff**: 80% 不等于"所有功能都有测试"
3. **MVCC 隔离级别**: 18 files 中是否有跨隔离级别对比？需抽样验证
4. **Error path**: happy path 充分，error path (OOM、超时、锁竞争) 可能不足

## Issue Hierarchy

```
#3534 (parent: full analysis + 4-round plan)
├── #3535 (Round 1: 4 high-ROI 0% files, 5h)
├── #3536 (Round 2: UNION + DML integration tests, 3h)
├── #3537 (Round 3: small files + Graph, 4h)
└── #3538 (Round 4: cleanup + verify, 3h)
```

## PR 累积成就 (4 commits in #3528)

| Commit | 描述 | Impact |
|--------|------|--------|
| `2fdc7aa7b` | test(ga-coverage-80): main crate +11.81pp coverage (11 new tests) | +11 tests, +11.81pp main crate |
| `eb45bb45d` | fix(tests): 3 pre-existing test failures (cosine simd, sort direction, test_benchmark_run_short) | 3 production bugs fixed |
| `d5a0196e2` | fix(tests): 3 more ignored (mmap module, three-value logic, FK parser) | 4 production bugs fixed |
| `283697de3` | test(vector/bench): remove 7 redundant perf benchmarks from lib tests | 7 ignored moved to benches/ |

**Net result**:
- 8 ignored lib tests → 1 ignored lib test
- Workspace runs 100% clean by default with no `--skip` flags
- 4 production bugs fixed (cosine SIMD norm, three-value NULL logic, FK LParen, orphan mmap module)

## 参考

- PR #3528: http://192.168.0.252:3000/openclaw/sqlrustgo/pulls/3528
- Parent Issue #3534: http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3534
- Design: `openspec/changes/ga-coverage-80/design.md`
- Tasks: `openspec/changes/ga-coverage-80/tasks.md`
- Gate scripts: `scripts/gate/check_coverage.sh`, `check_full_gate_verification.sh`
- baseline report: `.coverage-baseline/BASELINE_REPORT.md`
- handoff: `.coverage-baseline/HANDOFF.md`
