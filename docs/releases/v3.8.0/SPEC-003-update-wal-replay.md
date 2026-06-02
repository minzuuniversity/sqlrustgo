# SPEC-003 — Update WAL 重放正确性

> **PR Number**: SPEC-003 (扩展 PR-840)
> **PR Title**: 修复 Update WAL 重放 — Update 数据正确回放
> **Version**: v3.8.0
> **Branch**: `fix/v3.8.0-update-replay-recovery` (从 `origin/develop/v3.8.0` 切出)
> **Auditor**: Hermes Agent
> **Created**: 2026-06-02
> **Status**: DRAFT — 待执行

---

## 1. 概述

### 1.1 问题 (基于 LEGACY_FIXES_VERIFICATION_REPORT §2.4 + §4.1 G5)

PR-830E 实现了 WAL recovery lifecycle，但 **Update 重放在 Recovery 中跳过**:

```rust
// crates/storage/src/recovery_engine.rs:391-397
WalEntryType::Update => {
    log::warn!(
        "RecoveryEngine: UPDATE replay skipped for table {} (tx_id={})",
        table_name,
        entry.tx_id
    );
}
```

**影响**:
- Crash recovery 后 UPDATE 数据不一致
- PR-830E 文档明确记录为"已知限制" (Full semantic parsing requires a separate improvement pass)
- 22 个 WAL Recovery 测试中, 部分测试如 `test_wal_004` 实际测试 FileStorage 持久化而非 WAL replay

### 1.2 现状核实 (2026-06-02 develop/v3.8.0 @ 95951420e)

| 位置 | 状态 | 备注 |
|------|------|------|
| `recovery_engine.rs:391-397` | ❌ Update skip | `log::warn!` + no-op |
| `wal_storage.rs:334-352 update(table, filters, updates)` | ⚠️ data 字段 = old row bytes | 不含 updates 信息 |
| `wal_storage.rs:354-365 update_if(filter, mutation)` | ⚠️ data 字段 = `format!("{:?}", mutation)` | debug 格式, 不可逆 |
| `exp_g_wal_contracts_verified::test_wal_004` | ✅ PASS | 但用 `with_wal_file` (无 crash recovery), 测的是 FileStorage 持久化, 不是 WAL replay |

**关键发现**: EXP-G 测试 **不** 触发 WAL recovery 路径, 所以 Update skip 不会导致测试失败。但真实 crash recovery 场景下, Update 数据**会丢失**。

### 1.3 WAL Entry 编码现状

| 字段 | Insert | Delete | Update (current) |
|------|--------|--------|------------------|
| `key` | 行 key bytes (filter value encoded) | 行 key bytes | 行 key bytes |
| `data` | 完整 record bytes (i:/s:/b:/n:/f:/B: prefix) | None (用 key) | old row bytes (没用) or mutation debug string |

**问题**:
- `update()` 路径: data 存的是 old row, 还原后是 (1, 100), 但 **没存** `SET balance = 900`
- `update_if()` 路径: data 是 `format!("{:?}", mutation)`, 不是有效编码格式, 不可逆

---

## 2. 功能范围

### 2.1 必须做 (Must Do)

| 任务 | 实施方式 | 验证方法 |
|------|----------|----------|
| **改 `log_update` 签名** | 增加 `updates: &[(usize, Value)]` 参数 | build + 调用方更新 |
| **新编码格式** | data = `prefix("U:") + record_bytes + ";" + updates_count(u32 LE) + [col_idx(u32 LE) + value_bytes]...` | bytes_to_record_and_updates helper |
| **新 helper `bytes_to_record_and_updates`** | 解析 U: 编码 | unit test |
| **`update()` 路径** | 传入 `updates` 给 `log_update` | build PASS |
| **`update_if()` 路径** | 从 `mutation.assignments()` 提取 updates, 传入 | build PASS |
| **改 `apply_entry` Update 分支** | 调用 `storage.update(table, &filters, &updates)` | unit test |
| **新增测试** `test_update_wal_replay_survives_crash` | crash + recovery + balance 验证 | test PASS |
| **更新** LEGACY_FIXES_REPORT §2.4 状态 | 标记为 "✅ FIXED" | grep |

### 2.2 禁止做 (Must NOT Do)

- ❌ 修改 `bytes_to_record` 行为 (保持现有 Insert 编码)
- ❌ 改变 `WalEntry` 结构体 (保持字段)
- ❌ 重写整个 recovery engine
- ❌ 修改 EXP-G 测试 (`test_wal_004` 等) — 它们测的是 FileStorage 持久化, 不在本 SPEC 范围
- ❌ 把 `key_to_filter_values` 改成更复杂解析 (保持简单)

### 2.3 不在范围内 (Out of Scope)

- mysql-server WAL 接入 → SPEC-004
- MERGE executor → SPEC-005/006/007
- MVCC 集成 → 单独 SPEC
- CheckpointManager 持久化 → 单独 SPEC
- FileStorage 自身的优化 → 单独 SPEC

---

## 3. 技术设计

### 3.1 新编码格式

**字段 `data` (WalEntry.data: Option<Vec<u8>>) 在 Update 时**:

```
┌─────┬──────────────┬─────┬──────────────┬──────────────┬─────┐
│"U:" │ record_bytes │";"  │updates_count │ col_idx  u32 │value│
│3B   │  变长        │1B   │   u32 LE     │  4B  + 变长  │... │
└─────┴──────────────┴─────┴──────────────┴──────────────┴─────┘
```

**示例**: `UPDATE accounts SET balance = 900 WHERE id = 1`
- 旧 record: (1, 100) → bytes_to_record: `i:8byte_LE(1) i:8byte_LE(100)`
- updates: `[(1, Value::Integer(900))]` (col_idx=1 for balance)
- 最终 data: `b"U:" + record_bytes + b";" + u32_LE(1) + u32_LE(1) + value_bytes(900)`

**value_bytes 格式**: 复用现有 `record_to_bytes` 但单值 (record 是 Vec<Value>, 单 value 同样编码)

### 3.2 `bytes_to_record_and_updates` 函数签名

```rust
fn bytes_to_record_and_updates(
    data: &[u8],
) -> Result<(Vec<Value>, Vec<(usize, Value)>), SqlError> {
    // 1. 检查 "U:" prefix
    // 2. 解析 record (复用 bytes_to_record 逻辑, 直到 ";")
    // 3. 解析 updates_count (u32 LE)
    // 4. 循环读取 col_idx + value (复用 bytes_to_record 的单值解析)
}
```

### 3.3 `log_update` 新签名

```rust
fn log_update(
    &mut self,
    table_id: u64,
    key: Vec<u8>,
    old_record: Vec<u8>,           // 旧 record bytes (for rollback info)
    updates: &[(usize, Value)],    // 新: 要应用的更新列表
) -> SqlResult<()> {
    let mut data = Vec::new();
    data.extend_from_slice(b"U:");
    data.extend_from_slice(&old_record);
    data.push(b';');
    data.extend_from_slice(&(updates.len() as u32).to_le_bytes());
    for (col_idx, value) in updates {
        data.extend_from_slice(&(*col_idx as u32).to_le_bytes());
        data.extend_from_slice(&Self::value_to_bytes(value));
    }
    // ... rest unchanged (entry_type=Update, key=Some(key), data=Some(data))
}
```

### 3.4 `apply_entry` Update 分支修复

```rust
WalEntryType::Update => {
    let key = entry.key.as_deref().ok_or_else(|| {
        SqlError::ExecutionError("Update entry missing key".to_string())
    })?;
    let data = entry.data.as_deref().ok_or_else(|| {
        SqlError::ExecutionError("Update entry missing data".to_string())
    })?;
    let (old_record, updates) = bytes_to_record_and_updates(data)?;
    let _ = old_record; // 暂未使用 (future: 用于冲突检测)
    let filter_values = key_to_filter_values(key)?;
    storage.update(&table_name, &filter_values, &updates)?;
}
```

### 3.5 `update()` / `update_if()` 调用方更新

```rust
// update() 路径 (wal_storage.rs:334)
fn update(&mut self, table: &str, filters: &[Value], updates: &[(usize, Value)]) -> SqlResult<usize> {
    let table_id = Self::table_name_to_id(table);
    let rows = self.inner.scan(table)?;
    for row in &rows {
        if Self::row_matches_filter(row, filters) {
            let key = Self::record_key(row);
            let old_data = Self::record_to_bytes(row);
            self.log_update(table_id, key, old_data, updates)?;  // 新增 updates 参数
        }
    }
    self.inner.update(table, filters, updates)
}

// update_if() 路径 (wal_storage.rs:354)
fn update_if(&mut self, table: &str, filter: &RowFilter, mutation: &RowMutation) -> SqlResult<usize> {
    let table_id = Self::table_name_to_id(table);
    let key = format!("RowFilter-{:p}", filter).into_bytes();
    let assignments = mutation.assignments();
    // 从 rows 找到匹配的 row, 用作 old_record
    // ... or use empty old_record for update_if (RowFilter 难还原)
    // 简化: 对 update_if 使用 "U:" + empty + ";" + updates_count + assignments
    let mut data = Vec::new();
    data.extend_from_slice(b"U:;");
    data.extend_from_slice(&(assignments.len() as u32).to_le_bytes());
    for (col_idx, value) in assignments {
        data.extend_from_slice(&(*col_idx as u32).to_le_bytes());
        data.extend_from_slice(&Self::value_to_bytes(value));
    }
    // ... entry_type=Update, key=Some(key), data=Some(data)
}
```

### 3.6 新测试

**`test_update_wal_replay_survives_crash`** (recovery_engine.rs mod tests):
1. 创建 WalStorage + CheckpointManager
2. INSERT + COMMIT (1, 100)
3. UPDATE balance = 900 + COMMIT
4. **模拟 crash**: 不 flush storage, 直接丢弃内存
5. 重新创建 WalStorage (从磁盘加载)
6. 调用 `recover_wal` (RECOVERY-002 风格)
7. 验证 balance == 900 ✅

### 3.7 验证矩阵

| 检查项 | 命令 | 通过条件 |
|--------|------|----------|
| Format check | `cargo fmt --all -- --check` | EXIT 0 |
| 编译 | `cargo build -p sqlrustgo-storage --all-features` | Finished |
| storage lib tests | `cargo test -p sqlrustgo-storage --lib` | ≥ 287/287 PASS (286+1) |
| executor lib tests | `cargo test -p sqlrustgo-executor --lib` | 327/327 PASS (无回归) |
| 全 workspace build | `cargo build --all-features` | Finished |
| clippy 警告 | `cargo clippy -p sqlrustgo-storage` | 警告数不增加 |

### 3.8 提交规范

```bash
git commit -m "fix(storage): Update WAL replay — encode updates + replay correctly

PR-830E 已实现 WAL recovery lifecycle, 但 Update 重放被 skip
(recovery_engine.rs:391-397)。导致 crash recovery 后 UPDATE 数据丢失。

本修复:
1. log_update 新签名: log_update(table_id, key, old_record, updates)
2. 新编码 'U:record;updates_count[(col_idx,value_bytes)]...'
3. apply_entry Update 分支真正调用 storage.update(table, &filters, &updates)
4. update() / update_if() 调用方更新, 传入 updates
5. 新测试 test_update_wal_replay_survives_crash 验证

验证: storage lib tests +1 PASS, 无回归.

SPEC-003: Update WAL Replay Correctness
源: docs/releases/v3.8.0/LEGACY_FIXES_VERIFICATION_REPORT.md §2.4 G5"
```

---

## 4. 实施计划

### 4.1 执行步骤

| Step | 操作 | 工具 |
|------|------|------|
| 1 | 添加 `bytes_to_record_and_updates` + `value_to_bytes` helpers | patch |
| 2 | 改 `log_update` 签名 (4 参数) | patch |
| 3 | 改 `update()` 路径 (传入 updates) | patch |
| 4 | 改 `update_if()` 路径 (从 mutation.assignments() 提取) | patch |
| 5 | 改 `apply_entry` Update 分支 (真实重放) | patch |
| 6 | 添加 `test_update_wal_replay_survives_crash` | patch |
| 7 | `cargo build -p sqlrustgo-storage --all-features` 验证编译 | terminal |
| 8 | `cargo test -p sqlrustgo-storage --lib` 验证测试 | terminal |
| 9 | `cargo fmt --all && cargo fmt --all -- --check` | terminal |
| 10 | 更新 LEGACY_FIXES_REPORT §2.4 G5 状态 | patch |
| 11 | `git commit` | terminal |
| 12 | 推送 + 创建 Gitea PR | curl |

### 4.2 时间表

| 阶段 | 预计耗时 |
|------|----------|
| Helpers + log_update 改签名 | 20 min |
| 调用方更新 | 10 min |
| apply_entry Update 重放 | 15 min |
| 测试 + 验证 | 15 min |
| 提交 + 推送 + PR | 5 min |
| **总计** | **< 70 min** |

---

## 5. 验收标准 (Acceptance Criteria)

### 5.1 必须全部满足

- [ ] **AC-1**: `log_update` 新签名 4 参数 (table_id, key, old_record, updates)
- [ ] **AC-2**: `apply_entry` Update 分支调用 `storage.update`, 不再 skip
- [ ] **AC-3**: 新测试 `test_update_wal_replay_survives_crash` PASS
- [ ] **AC-4**: `cargo test -p sqlrustgo-storage --lib` ≥ 287/287 PASS (286+1)
- [ ] **AC-5**: `cargo test -p sqlrustgo-executor --lib` 仍 327/327 PASS
- [ ] **AC-6**: `cargo fmt --all -- --check` EXIT 0
- [ ] **AC-7**: clippy 警告数不增加
- [ ] **AC-8**: LEGACY_FIXES_REPORT §2.4 G5 状态更新为 "✅ FIXED"
- [ ] **AC-9**: PR base = `develop/v3.8.0`
- [ ] **AC-10**: 3 平台分支一致

---

## 6. 风险与缓解

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|----------|
| 新编码格式与历史 WAL 文件不兼容 | 中 | 中 | 旧 WAL 用旧格式, decode 失败时 graceful skip |
| `value_to_bytes` 实现差异 | 低 | 中 | 复用 `record_to_bytes` 现有逻辑 |
| apply_entry 真实重放破坏 crash recovery 一致性 | 低 | 高 | 全量 storage 测试 + crash simulation test |
| 旧 EXP-G 测试因 Update 数据变化失败 | 低 | 低 | EXP-G 用 with_wal_file, 不调 recovery, 不影响 |

---

## 7. 关联

- **上游**:
  - `docs/releases/v3.8.0/LEGACY_FIXES_VERIFICATION_REPORT.md` §2.4 G5
  - `docs/releases/v3.8.0/PR-830E_SPEC.md`
  - PR #2707 `fix(storage): WAL replay correctness for DELETE/UPDATE` (PR-840)
- **下游**:
  - `docs/releases/v3.8.0/BETA_GATE_REPORT.md` (后续更新 RECOVERY 状态)
- **Gitea Issue**: 无单独 Issue
- **后续**: SPEC-004 (mysql-server WAL) + SPEC-005/006/007 (MERGE)

---

## 8. Truthfulness 声明

**本 SPEC 基于 2026-06-02 实际代码审查 (develop/v3.8.0 @ 95951420e)**:
- EXP-G 测试 (`test_wal_004_update_survives`) 用 `with_wal_file` (无 crash recovery), 不触发 Update skip
- 真实 crash recovery 场景下 Update 数据丢失 (PR-830E 已知限制)
- 新测试 `test_update_wal_replay_survives_crash` 才是真正验证 Update 重放
- 新编码格式为 U: prefix 区分旧格式 (decode 失败时 fallback skip)

---

*本 SPEC 依据 ADR-001 Truthfulness Framework 编写, 所有状态变更基于实际执行证据。*
