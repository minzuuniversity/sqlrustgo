# openspec/3176 - P1-4 Upgrade Test (v3.8 → v3.9)

> **Issue**: #3176
> **作者**: Hermes Agent
> **日期**: 2026-06-05
> **Phase**: 4 (W7-8)
> **工作量**: 40h (按 V390_DEVELOPMENT_PLAN)
> **状态**: 借力已存在 upgrade.rs (799 lines, 8 unit tests), 本次做 v3.8→v3.9 集成 + G9 gate

## 一、问题分析

### 1.1 现状审计 (2026-06-05)

仓库已有:
- `crates/tools/src/upgrade.rs` (799 lines) - 完整 Upgrade 工具:
  - `VersionInfo` (parse "2.0.0" / "v2.1.0" 格式)
  - `UpgradeManifest`, `UpgradeStatus`, `MigrationStep`
  - `UpgradePlan`, `UpgradeCommand` enum
  - `run_with_opt`, `check_upgrade`, `execute_upgrade`, `execute_rollback`
  - `show_status`, `list_history`
  - 8 unit tests (version parse, can_upgrade_to, cannot_downgrade)
- `crates/storage/src/pitr_recovery.rs` (PITR, 已由 P1-1 close)
- 已有: 11 tests 涵盖 version compatibility
- 缺: 真实数据 fixture (v3.8 WAL format) + 升级 scenarios 框架

### 1.2 #3176 8 类场景覆盖映射

| #3176 类别 | 已有覆盖 | 缺口 |
|------------|----------|------|
| 简单表升级 (1 表, 100 行) | ❌ | **完全缺** |
| 复杂表升级 (多表 JOIN INDEX) | ❌ | **完全缺** |
| 大量数据 (100K/1M/10M) | ❌ | **完全缺** |
| 类型覆盖 (NULL/BLOB/CLOB) | ❌ | **完全缺** |
| 对象 (VIEW/TRIGGER/CONSTRAINT) | ❌ | **完全缺** |
| 升级中崩溃 → 重启继续 | ❌ | **完全缺** (可借力 P1-2 crash) |
| 升级回滚 v3.9 → v3.8 | execute_rollback ✅ | **部分** |
| 数据可读性验证 | ❌ | **完全缺** |

### 1.3 P1-4 任务真正需要补的 (按治理最小修改)

**A. Upgrade Test Harness** (新):
- 数据 fixture 管理 (v3.8-format WAL/Page fixture)
- 升级 scenarios 枚举
- 数据完整性验证 (row count + checksum)

**B. 50+ Upgrade Scenarios** (新):
- 按 #3176 8 类分组, 至少 50 scenarios
- 复用 P1-2 crash_test_harness 的 spawn 模式
- 复用 P1-3 soak_test 的资源监控

**C. G9 Gate** (新):
- 7 项检查

## 二、实施方案

### 2.1 范围限定

按治理 §2.1 最小修改 + 复用现有 upgrade.rs 基础:

**本次 PR 范围 (4 大块)**:

1. **新文件**: `tests/upgrade_test_harness.rs` (共享 helper)
2. **新文件**: `tests/upgrade_test.rs` (50+ scenarios)
3. **新文件**: `scripts/gate/check_p14_upgrade_test.sh` (G9 gate)
4. **新文件**: `docs/openspec/3176-upgrade-test.md` (本文件)

**延后 (推 v3.10+)**:
- 真实跨版本 binary 测试 (需要发布 v3.8 binary)
- 网络升级 (multi-node v3.8 → v3.9 cluster)
- 性能对比 (v3.8 vs v3.9 same workload)

### 2.2 Upgrade Test Harness 设计

```rust
// tests/upgrade_test_harness.rs (shared)
pub struct UpgradeScenario {
    pub name: String,
    pub from_version: &'static str,  // "3.8.0"
    pub to_version: &'static str,    // "3.9.0"
    pub tables: u32,                 // 1, 5, 10
    pub rows_per_table: u32,         // 100, 1K, 10K, 100K
    pub data_types: Vec<&'static str>, // ["int", "text", "blob"]
    pub objects: Vec<&'static str>,     // ["view", "trigger"]
    pub crash_midway: bool,          // simulate crash during upgrade
    pub rollback: bool,              // test rollback path
}

pub struct UpgradeResult {
    pub scenario: String,
    pub upgrade_time_ms: u64,
    pub tables_preserved: u32,
    pub rows_preserved: u64,
    pub checksum_match: bool,
    pub crash_resumable: bool,
    pub rollback_success: bool,
}

pub fn run_upgrade_scenario(scenario: &UpgradeScenario) -> UpgradeResult;
```

### 2.3 50+ Upgrade Scenarios (按 #3176 8 类)

| 类别 | Count | 示例 |
|------|-------|------|
| 简单表 (1 表) | 5 | rows=100/1K/10K, 4 simple types |
| 复杂表 (多表) | 8 | 3-5 表 + JOIN + INDEX, rows=100-10K |
| 大量数据 | 5 | 100K / 1M / 10M rows, single table |
| 类型覆盖 | 8 | NULL, BLOB, TEXT, JSON, BOOLEAN, FLOAT, DATE, TIMESTAMP |
| 对象 | 6 | VIEW, TRIGGER, PK, FK, UNIQUE, CHECK constraint |
| 升级中崩溃 | 8 | crash at 8 different points (WAL append, page flush, etc) |
| 回滚 | 5 | forward 3.8→3.9 then rollback 3.9→3.8 |
| 数据完整性 | 5 | checksum, row count, FK consistency, INDEX validity |
| **TOTAL** | **50** | |

### 2.4 G9 Gate (7 checks)

1. `tests/upgrade_test_harness.rs` 存在
2. `tests/upgrade_test.rs` 存在 + Cargo.toml 注册
3. `cargo check --test upgrade_test` pass
4. ≥50 tests pass
5. 8 类别全覆盖 (grep test_ names)
6. 借力 crates/tools/src/upgrade.rs (8 unit tests 仍 PASS)
7. 借力 P1-1 Backup/Restore (PITR 升级路径)

## 三、风险评估

| 风险 | 影响 | 缓解 |
|------|------|------|
| v3.8 binary 不可得 | 无法端到端 | 用 version_info + execute_upgrade API, mock data fixture |
| 数据 fixture 大 | CI 慢 | 限 10K-100K rows, 不用 1M+ |
| 50 测试运行慢 | CI 超时 | 单元级 (1ms-1s), 不用真实 fsync |
| 升级 fixture 互相污染 | flaky | 每个测试独立 tempdir |

## 四、验收标准 (G9 门禁)

```
✅ upgrade_test: ≥50 tests PASS
✅ 8 类别全覆盖
✅ 借力 crates/tools/src/upgrade.rs (8 unit tests 仍 PASS)
✅ G9 gate: 7/7 PASS
✅ 871 L1 tests 不回归
✅ TPC-H 22/22 (G1 维持)
```

## 五、Subsumed Issues

- #3176 本身 (本任务)
- 与 P1-1 Backup/Restore 互补 (升级需 backup + restore 配合)

## 六、回滚计划

如 upgrade_test 编译失败:
1. 删除 `tests/upgrade_test*.rs`
2. G9 gate 标记 DEFER
3. 现有 upgrade.rs + 8 unit tests 仍保留

## 七、依赖

**上游**: P1-1 Backup/Restore (#3173 close)
**下游**: GA 前的最终兼容性验证

## 八、参考资料

- Issue #3176
- V390_DEVELOPMENT_PLAN.md §P1-4
- V390_TEST_PLAN.md §G9
- crates/tools/src/upgrade.rs (799 lines, UpgradePlan, execute_upgrade/rollback)
- crates/storage/src/pitr_recovery.rs (PITR 升级恢复)
- P1-2 #3174 crash_test_harness (设计模型)
- P1-3 #3175 soak_test_harness (设计模型)
