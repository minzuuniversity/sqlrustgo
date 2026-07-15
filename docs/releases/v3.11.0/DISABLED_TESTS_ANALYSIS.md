# v3.11.0 禁用测试分析报告

**Issue**: #3420 SEM-4 Coverage ≥ 80%
**Issue**: #3421 Re-enable disabled integration tests (API drift fix)
**分支**: `fix/sem4-coverage-3420` (`250/develop/v3.11.0`)
**日期**: 2026-07-15
**分析人**: Claude Code + 2 Scout Agents

---

## 执行摘要

43 个测试文件被 `#![cfg(any())]` 禁用。

| 类别 | 数量 | 状态 |
|------|------|------|
| ✅ **已修复** | 1 | tpch_hash_test (JSON路径 + cfg移除) |
| ✅ **编译通过（去掉cfg即可）** | 29 | 需逐个运行 `cargo test` 验证 |
| ⚠️ **编译失败（可修复）** | 5 | expr_single_engine_test, savepoint_test |
| 🔴 **编译失败（API重构）** | 8 | 需要较大工作量 |

---

## 第一批修复：Batch 1 — `tpch_hash_test` ✅

**状态**: 已提交 (`4305f7cb65`)

修复内容:
```diff
- #![cfg(any())] // disabled for GA: API changed
- include_str!("tpch_hashes_v380.json")
+ include_str!("../../tpch_hashes_v380.json")
```

结果: 编译通过，`cargo test --test tpch_hash_test` → **16 passed, 2 failed (fixture缺失)**

---

## 编译失败分析（按根因分类）

### A. `ExecutionEngine::execute` 签名变更
**根因**: `execute(Statement)` → `execute(&str)`

| 文件 | 修复难度 | 说明 |
|------|---------|------|
| `tests/integration/savepoint_test.rs` | **LOW** | 5处 `execute(parse(...))` → `execute(...)` |
| `tests/integration/foreign_key_test.rs` | **HIGH** | 外键约束 struct 字段也变了 |
| `tests/integration/sql/expr_single_engine_test.rs` | **LOW** | duplicate字段 + execute签名 |

### B. `Value` 枚举变体移除
**根因**: `Value::Date`、`Value::Timestamp`、`Value::to_bool()` 等已移除

| 文件 | 修复难度 |
|------|---------|
| `tests/unit/types_value_test.rs` | **MEDIUM** — 删除 Date/Timestamp 测试，替换 to_bool |

### C. `StorageEngine` Trait API 重构
**根因**: `ColumnDefinition.references` 移除，ForeignKeyConstraint 字段变更

| 文件 | 修复难度 |
|------|---------|
| `tests/integration/foreign_key_test.rs` | **HIGH** — struct 字段名/类型全变了 |
| `tests/integration/sql/expr_single_engine_test.rs` | **LOW** — 仅 singular→plural |

### D. `TransactionManager` 重构（SSI引入）
**根因**: `begin_transaction` 返回 `Result<TxId>`，`commit_transaction` 不再接受 `TxId`

| 文件 | 修复难度 |
|------|---------|
| `tests/stress/concurrency_stress_test.rs` | **HIGH** — 变量名bug + API重构 |
| `tests/stress/stress_test.rs` | **HIGH** — 大量事务API变更 |
| `tests/anomaly/snapshot_isolation_test.rs` | **HIGH** — TxId/begin/commit API |

### E. `BufferPool` 类型移除
**根因**: `BufferPoolWithClock`、`ClockProCache` 已移除，`Page` API 变更

| 文件 | 修复难度 |
|------|---------|
| `tests/unit/buffer_pool_test.rs` | **HIGH** — 12个测试需重写 |

### F. 其他
| 文件 | 错误数 | 修复难度 |
|------|--------|---------|
| `tests/anomaly/boundary_test.rs` | 11 errors | **HIGH** |
| `tests/anomaly/crash_injection_test.rs` | — | **MEDIUM** — FileStorage方法签名变更 |
| `tests/unit/optimizer_cost_test.rs` | — | 待分析 |
| `tests/unit/optimizer_rules_test.rs` | — | 待分析 |
| `tests/integration/server_integration_test.rs` | — | 待分析 |

---

## 修复优先级建议

### P0 — 立即可做（去掉 cfg 即可）
批量处理 29 个编译通过文件：
```bash
for f in <list>; do
  sed -i '' 's/^#!\[cfg(any())\].*/ /' "$f"
  cargo test --test <name> || git checkout -- "$f"
done
```

### P1 — LOW 难度（< 1h 每个）
- `expr_single_engine_test.rs`: 去掉duplicate字段 + execute签名
- `savepoint_test.rs`: execute签名变更 (5处)

### P2 — MEDIUM 难度（1-3h 每个）
- `types_value_test.rs`: 删除Date/Timestamp测试 + 替换to_bool

### P3 — HIGH 难度（需要架构理解）
- `foreign_key_test.rs`（需要理解FK约束重构）
- `buffer_pool_test.rs`（需要理解新ClockReplacer API）
- `concurrency_stress_test.rs` / `stress_test.rs` / `snapshot_isolation_test.rs`
  （需要理解TransactionManager SSI重构）

---

## 相关 API 变更摘要（供修复参考）

```
1. ExecutionEngine::execute(Statement) → execute(&str)
2. Value::Date / Value::Timestamp 枚举变体已移除
3. Value::to_bool() 方法已移除
4. ColumnDefinition.references 字段已移除
5. ForeignKeyConstraint: singular → plural (referenced_column → referenced_columns)
6. TransactionManager::begin_transaction() → Result<TxId>
7. commit_transaction(tx_id) → commit_transaction() (不接受TxId)
8. BufferPoolWithClock / ClockProCache 已移除
9. Page::new(page_id, data) → Page::new(page_id)
```

---

## 相关 Issue

| Issue | 描述 | 状态 |
|-------|------|------|
| #3420 | SEM-4 Coverage ≥ 80% | 进行中 |
| #3421 | Re-enable disabled integration tests | P0 |
| #3136 | Remove MOCK storage backend | 影响测试 |
