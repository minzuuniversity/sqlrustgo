# v3.11.0 历史遗留问题审计报告

> **状态**: ARCHIVED (2026-07-18) - 历史审计报告，已被 PROGRESS.md 替代
> **生成日期**: 2026-07-16
> **生成人**: openclaw (Claude Code)
> **基于**: debt-registry.yaml (v3.10.0 snapshot), FEATURE_CHECKLIST.md, 代码审计
> **覆盖版本**: v3.6.0 ~ v3.10.0 遗留 → v3.11.0 处置

---

## 1. 历史遗留债务总览 (v3.6.0 ~ v3.10.0)

### 1.1 各版本闭环统计

| 版本 | INT | ARCH | SEM | F-XX ISOLATED | F-XX NOT_IMPL | Ext Crates | 总关闭率 |
|------|-----|------|-----|----------------|---------------|------------|---------|
| v3.6.0 | — | — | — | — | — | — | — |
| v3.7.0 | — | — | — | — | — | — | — |
| v3.8.0 | 2/4 | 2/3 | 1/4 | 0/10 | 0/5 | 0/11 | — |
| v3.9.0 | 2/4 | 2/3 | 1/4 | 0/10 | 0/5 | 0/11 | — |
| v3.10.0 | **4/4** ✅ | **3/3** ✅ | 2/4 | 1/10 (F-16) | 2/5 (T-19,T-20) | 0/11 | 大幅改善 |
| **v3.11.0 DRAFT** | **4/4** | **3/3** | 3/4 | **5/10** (F-23,24,31,32,36) | **3/3** (F-03,30,36→CLOSED) | **7/11** (5删+1归档+1集成) | 进行中 |

### 1.2 debt-registry.yaml 完整状态

```
version: "3.10.0-snapshot-2026-07-15"
总计 45 项债务:
  CLOSED:       17 (38%)    ← v3.6.0 ~ v3.10.0 累计
  VERIFIED:     8 (18%)     ← 待集成 (F-XX ISOLATED)
  IN_PROGRESS:  2 (4%)      ← SEM-3, SEM-4
  PARTIAL:      2 (4%)      ← admin (F-32)
  DEFERRED:     4 (9%)      ← F-03, F-30, #3136, #3423
  SCOPE_DEFERRED: 8 (18%)   ← extension crates
  SCOPE_INTERNAL: 1 (2%)    ← vector
  SUPERSEDED:   2 (4%)      ← #3648, #3423
  FROZEN:       1 (2%)      ← distributed
```

---

## 2. v3.11.0 是否定义所有历史遗留问题的开发和集成工作？

### 2.1 对照表: 遗留债务 → v3.11.0 任务映射

| ID | 遗留债务 | v3.10.0 状态 | v3.11.0 处置 | Issue | 状态 | 定义完整性 |
|----|---------|-------------|-------------|-------|------|----------|
| SEM-3 | ALTER TABLE 不完整 | IN_PROGRESS | V311-13 | #3433 | ✅ CLOSED | ✅ 完整 |
| SEM-4 | 覆盖率 ≥85% | IN_PROGRESS | V311-14 | #3493 | ⏳ TODO | ✅ 完整 |
| F-23 | Clustered Index ISOLATED | VERIFIED | V311-01 | — | ✅ CLOSED | ⚠️ **部分实现** |
| F-24 | Adaptive Hash Index ISOLATED | VERIFIED | V311-02 | — | ✅ CLOSED | ⚠️ **部分实现** |
| F-25 | Change Buffer ISOLATED | VERIFIED | V311-03 | #3491 | ⏳ TODO | ✅ 完整 |
| F-26 | Double-Write Buffer ISOLATED | VERIFIED | V311-04 | #3492 | ⏳ TODO | ✅ 完整 |
| F-27 | Table Compression ISOLATED | VERIFIED | V311-12 | #3498 | ⏳ TODO | ✅ 完整 |
| F-29 | Row-Level Security ISOLATED | VERIFIED | V311-05 | #3494 | ⏳ TODO | ✅ 完整 |
| F-31 | Performance Schema ISOLATED | VERIFIED | V311-06 | — | ✅ CLOSED | ⚠️ **部分实现** |
| F-32 | MySQL Admin PARTIAL | PARTIAL | V311-07 | — | ✅ CLOSED | ⚠️ **部分实现** |
| F-35 | Password Rotation ISOLATED | VERIFIED | V311-08 | #3495 | ⏳ TODO | ✅ 完整 |
| F-03 | GIS NOT_IMPL | DEFERRED | V311-11 | #3497 | ⏳ TODO | ✅ 完整 |
| F-30 | CREATE SEQUENCE NOT_IMPL | DEFERRED | V311-10 | #3496 | ⏳ TODO | ✅ 完整 |
| F-36 | 列级权限 NOT_IMPL | DEFERRED | V311-09 | — | ✅ CLOSED | ✅ 完整 |
| T-19 | Disk I/O delay | CLOSED | — | — | ✅ CLOSED | ✅ |
| T-20 | Process kill -9 | CLOSED | — | — | ✅ CLOSED | ✅ |
| Ext: agentsql | SCOPE_DEFERRED | SCOPE_DEFERRED | V311-19 | — | ✅ DELETED | ✅ 完整 |
| Ext: gmp | SCOPE_DEFERRED | SCOPE_DEFERRED | V311-19 | — | ⏳ ARCHIVED | ✅ 完整 |
| Ext: rag | SCOPE_DEFERRED | SCOPE_DEFERRED | V311-19 | — | ✅ DELETED | ✅ 完整 |
| Ext: distributed | FROZEN | FROZEN | V311-19 | — | ✅ DELETED | ✅ 完整 |
| Ext: graph | SCOPE_DEFERRED | SCOPE_DEFERRED | V311-19 | — | ✅ ARCHIVED | ✅ 完整 |
| Ext: qmd-bridge | SCOPE_DEFERRED | SCOPE_DEFERRED | V311-19 | — | ✅ DELETED | ✅ 完整 |
| Ext: evidence-graph | SCOPE_DEFERRED | SCOPE_DEFERRED | V311-19 | — | ✅ DELETED | ✅ 完整 |
| Ext: unified-query | SCOPE_DEFERRED | SCOPE_DEFERRED | V311-19 | — | ✅ DELETED | ✅ 完整 |
| Ext: unified-storage | SCOPE_DEFERRED | SCOPE_DEFERRED | V311-19 | — | ✅ DELETED | ✅ 完整 |
| Ext: admin | PARTIAL | PARTIAL | V311-07+V311-19 | — | ✅ CLOSED | ✅ 完整 |
| Ext: vector | SCOPE_INTERNAL | SCOPE_INTERNAL | — | — | ✅ RETAINED | ✅ |
| PERF-5 | 高并发 INSERT | NEW | V311-23 | — | ✅ CLOSED | ✅ 完整 |
| PERF-1 | Hash Semi Join | NEW | V311-15 | — | ✅ CLOSED | ✅ 完整 |
| PERF-2 | Hash Anti Join | NEW | V311-17 | — | ✅ CLOSED | ✅ 完整 |
| PERF-3 | CTE 物化 | NEW | V311-18 | #3499 | ⏳ TODO | ✅ 完整 |
| PERF-4 | Decorrelation | NEW | V311-16 | — | ✅ CLOSED | ✅ 完整 |
| GA-P0 #3423 | TPC-H SF=1 | SUPERSEDED | V311-20 | #3431 | ⏳ TODO | ✅ 完整 |
| GA-P0 #3648 | SOAK 跨平台 | SUPERSEDED | V311-21 | #3500 | ⏳ TODO | ✅ 完整 |
| #3136 | Gate script 升级 | DEFERRED | V311-19 | — | ⚠️ **未跟踪** | ❌ **缺失** |

**结论**: v3.11.0 定义了 32/33 项历史遗留债务的开发和集成工作。

**缺失项**: `#3136 check_cross_version_debt.sh 升级` — V311_DEBT_CLOSURE_PLAN §1.7 声称已通过 V311-19 完成，但 V311-19 仅处理了 Extension Crate 删除，未见 gate script 升级代码。

---

## 3. 功能孤岛审计 (Feature Islands)

### 3.1 已声称集成但验证通过 (CORRECTED)

以下 F-XX ISOLATED 项目被标记为 CLOSED，代码审计确认主执行路径可达：

| ID | 名称 | PR | 集成验证 |
|----|------|-----|---------|
| F-23 | Clustered Index | #3461 | ✅ `scan_with_ahi()` 中检查 `clustered_tables` registry；`execute_create_table()` 写入 clustered_tables |
| F-24 | Adaptive Hash Index | #3478 | ✅ `scan_with_ahi()` 调用 `adaptive_hash_index.record_access()` 在单表 SELECT 和 JOIN 基表扫描路径 |
| F-31 | Performance Schema | #3479 | ✅ `instrumentation.on_seq_scan_start()` 在 `scan_with_ahi()` 中被调用 |
| F-32 | MySQL Admin | #3481 | ✅ binary 发版，wire protocol 部分集成 |

> **更正**: 2026-07-16 重新审计发现 `scan_with_ahi()` 在 `src/engine_select.rs:282` 和 `:1328` 被调用，
> 包含 `record_access()` 调用 (L1304)。F-23 ClusteredTable 读取路径在 commit `74b36ccf7` 中修复。
| F-31 | Performance Schema | V311-06 添加 `instrumentation.rs` trait | `InstrumentationHook` trait 存在但无任何执行算子实际调用 hooks | 🟡 **中等** |
| F-32 | MySQL Admin | V311-07 添加 `mysqladmin.rs` | `MysqlAdmin` 在 `crates/admin/` 中，未在 `crates/mysql-server/` 中被使用 | 🟡 **中等** |

**审计方法**: 对每个声称集成的功能，执行 `grep -rl "SymbolName" crates/` 全局搜索跨模块调用。

### 3.2 仍然孤岛 (正确识别)

以下 5 项 ISOLATED 尚未开始，issue 已创建：

| ID | 名称 | 当前实现位置 | Issue |
|----|------|------------|-------|
| F-25 | Change Buffer | `tests/integration/sql/change_buffer_test.rs` 内联实现 | #3491 |
| F-26 | Double-Write Buffer | `tests/integration/sql/double_write_buffer_test.rs` 内联实现 | #3492 |
| F-27 | Table Compression | `tests/integration/sql/table_compression_test.rs` (RLE only) | #3498 |
| F-29 | Row-Level Security | `tests/integration/sql/row_level_security_test.rs` 内存 catalog | #3494 |
| F-35 | Password Rotation | `tests/integration/sql/password_rotation_test.rs` 内存实现 | #3495 |

### 3.3 未实现功能孤岛 (正确识别)

| ID | 名称 | 实现状态 | Issue |
|----|------|---------|-------|
| F-03 | GIS (POINT + WITHIN) | 零代码 | #3497 |
| F-30 | CREATE SEQUENCE | 零代码 | #3496 |

---

## 4. 测试遗漏审计 (Test Gaps)

### 4.1 新增测试文件 (v3.11.0 声称)

| 测试文件 | 声称覆盖 | 实际检查 | 问题 |
|---------|---------|---------|------|
| `tests/integration/storage/adaptive_hash_main_path_test.rs` | 7 tests (ahi_accessor, scan_with_ahi, etc.) | 文件存在于 `tests/integration/storage/` | 🟡 仅测试 harness 调用，非真实算子集成 |
| `tests/integration/sql/clustered_index_test.rs` | 7/7 PASS | 215 行内联 BTreeMap 实现，无 crate 集成 | 🔴 测试的是孤岛代码，不是主路径 |
| `tests/integration/sql/change_buffer_test.rs` | 5/5 PASS | 内联 ChangeBuffer struct，无 crate 依赖 | 🔴 同上 |

### 4.2 测试覆盖缺口

| 领域 | 缺口描述 | 影响 |
|------|---------|------|
| **F-23 ClusteredTable** | 无 integration test 验证 ClusteredTable 从 ExecutionEngine 可达 | 主路径未验证 |
| **F-24 AHI** | 无 integration test 验证 `ahi().lookup()` 从 BTreeIndex scan 被调用 | 热路径未验证 |
| **F-25 Change Buffer** | 测试文件仅覆盖 isolated 实现，主路径集成后需重新测试 | 重复工作 |
| **ALTER TABLE** | PR #3444 修复了 3 个 parser/executor bug，应有对应非回归测试 | 遗漏 |

### 4.3 已被禁用的集成测试

22 个集成测试因 API 重构被 `#![cfg(any())]` 禁用 (Issue #3421):

```
crates/executor/tests/*.rs    数量未知
crates/mysql-server/tests/*   数量未知
```

这些测试的禁用导致旧功能的回归覆盖缺失。

---

## 5. 错误集成审计 (Integration Errors)

### 5.1 虚假集成 — PR 声称但代码不支持

**F-24 Adaptive Hash Index (V311-02 v2, PR #3478)**

PR 描述声称:
> `src/engine_select.rs: scan_with_ahi() — instruments single-table SELECT and JOIN base tables with stable FNV-1a hash...`

实际代码 diff:
```diff
+                        // V311-02 v2: AHI access was recorded at scan time via
+                        // `scan_with_ahi()`. Adding per-row hooks here would be
+                        // redundant noise; the table-level access is sufficient
+                        // for the production-hook metric (touched_pages, hit_rate).
```

**问题**: diff 仅添加了注释声称功能已连接，但 `scan_with_ahi()` 函数本身不存在于 `src/engine_select.rs` 中。PR 添加的代码是:
- `src/engine_select.rs`: +4 行 (全部为注释)
- `src/execution_engine.rs`: +9 行 (一个 `pub fn ahi()` accessor 方法)

这是一个**注释驱动的集成** — 声称已连接但代码中无实际调用链。

**验证**:
```bash
grep -rn "scan_with_ahi\|ahi().lookup\|ahi().record_access" \
    crates/ src/ --include='*.rs' | grep -v "test\|#"
# 结果: 无匹配
```

**F-23 Clustered Index (V311-01, PR #3461)**

PR 添加了 `crates/storage/src/clustered_table.rs` (215 行)，但:
- `ExecutionEngine` 无 `ClusteredTable` 引用
- `planner` 无 `ClusteredTable` 查询计划支持
- `CREATE TABLE ... CLUSTERED` 语法未被解析

**验证**:
```bash
grep -rn "ClusteredTable" crates/executor/src/ crates/planner/src/ crates/optimizer/src/
# 结果: 无匹配
```

### 5.2 版本漂移 — PR 合并后代码回退

PR #3478 (V311-02 v2) merge commit `d60eaaab3` 声称修改了:
- `src/engine_select.rs`
- `src/execution_engine.rs`
- `tests/integration/storage/adaptive_hash_main_path_test.rs`

但 `develop/v3.11.0` HEAD commit `d314c0278` 中:
- `src/engine_select.rs` 不包含 `adaptive_hash` 相关代码
- `tests/integration/storage/adaptive_hash_main_path_test.rs` 不存在

**根因**: develop/v3.11.0 分支在 PR 合并后经历了 rebasing 或 patches 导致实际代码与 merge commit 声明不符。

---

## 6. 文档不一致审计

| 项目 | debt-registry.yaml 状态 | FEATURE_CHECKLIST.md 状态 | 正确值 |
|------|------------------------|--------------------------|--------|
| F-23 Clustered Index | CLOSED (PR #3461) | ✅ DONE | CLOSED ✅ |
| F-24 Adaptive Hash Index | CLOSED (PR #3478) | ✅ DONE | CLOSED ✅ |
| F-25 Change Buffer | CLOSED (PR #3512) | ✅ DONE | CLOSED ✅ (2026-07-16 更新) |
| F-26 Double-Write Buffer | CLOSED (PR #3514) | ✅ DONE | CLOSED ✅ (2026-07-16 更新) |
| F-27 Table Compression | VERIFIED | ✅ DONE | VERIFIED ✅ |
| F-29 Row-Level Security | VERIFIED | ✅ DONE | VERIFIED ✅ |
| F-31 Performance Schema | CLOSED | ✅ DONE | CLOSED ✅ |
| F-32 MySQL Admin | CLOSED | ✅ DONE | CLOSED ✅ |
| F-35 Password Rotation | VERIFIED | ✅ DONE | VERIFIED ✅ |

> **更正 (2026-07-16)**: debt-registry.yaml 已更新到 v3.11.0-snapshot，F-25/F-26 状态从 VERIFIED 修正为 CLOSED。
> #3136 (check_cross_version_debt.sh 升级) 已关闭 — 新 gate 脚本使用 debt-registry.yaml SSOT。
| F-23 Clustered Index | CLOSED (PR #3461) | ✅ DONE | CLOSED (代码已合入) |
| F-24 Adaptive Hash Index | CLOSED (PR #3465/#3478) | ✅ DONE | CLOSED (accessor 已合入) |
| F-25 Change Buffer | VERIFIED (待集成) | ⏳ TODO | VERIFIED ✅ |
| F-26 Double-Write Buffer | VERIFIED | ⏳ TODO | VERIFIED ✅ |
| F-27 Table Compression | VERIFIED | ⏳ TODO | VERIFIED ✅ |
| F-29 Row-Level Security | VERIFIED | ⏳ TODO | VERIFIED ✅ |
| F-31 Performance Schema | CLOSED | ✅ DONE | CLOSED ✅ |
| F-32 MySQL Admin | CLOSED | ✅ DONE | CLOSED ✅ |
| F-35 Password Rotation | VERIFIED | ⏳ TODO | VERIFIED ✅ |

**问题**: debt-registry.yaml 仍为 v3.10.0 snapshot (version: "3.10.0-snapshot-2026-07-15")，未更新 v3.11.0 已完成项。

### 6.2 ISOLATED_MODULES.md 需更新

`docs/releases/v3.11.0/ISOLATED_MODULES.md` 需反映 F-23/24/31/32 的 CLOSED 状态。

| F-23 | Clustered Index | CLOSED | ✅ CLOSED | ✅ CLOSED |
| F-24 | Adaptive Hash Index | CLOSED | ✅ DONE | ✅ CLOSED |
| F-25 | Change Buffer | ~~VERIFIED~~ CLOSED | ✅ DONE | ~~VERIFIED~~ ✅ CLOSED |
| F-26 | Double-Write Buffer | ~~VERIFIED~~ CLOSED | ✅ DONE | ~~VERIFIED~~ ✅ CLOSED |

> **更正 (2026-07-16)**: debt-registry.yaml 已更新到 v3.11.0-snapshot。F-25/F-26 状态从 VERIFIED 修正为 CLOSED。
> #3136 已关闭 — 新 gate 脚本 (check_alpha/beta/rc_v3.11.0.sh) 使用 debt-registry.yaml SSOT。

### 7.3 建议行动

1. **更新 debt-registry.yaml**: 添加 v3.11.0 snapshot，将已完成的 12 项标记为 CLOSED，8 项保留 VERIFIED/DEFERRED
2. **创建 #3136 跟踪 issue**: gate script 升级未完成，需独立 issue
3. **更新 ISOLATED_MODULES.md**: 反映 F-23/24/31/32 的当前真实状态（部分集成）
4. **V311-02 v3 计划**: 实现 `scan_with_ahi()` 实际调用链，不是仅加注释
5. **V311-01 v2 计划**: 将 ClusteredTable 集成到 CREATE TABLE 解析和执行路径

---

*Report generated by Claude Code audit pipeline — 2026-07-16*
