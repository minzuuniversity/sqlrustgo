# MVCC 回归根因分析报告

> Issue: #1394  
> 日期: 2026-05-26  
> 状态: **根因已定位**

---

## 执行摘要

**根因 commit**: `9b390144` (May 14, 2026)  
**变更内容**: `feat(transaction): implement LockTarget abstraction and MVCC REPEATABLE READ`

**性能影响**:
- UPDATE QPS: ~75K (baseline 85K) — **下降 ~12%**
- Transaction abort rate: 5% (目标 < 2%)
- System Risk: 0.38

---

## 根因分析

### 问题 1: Range Lock O(n) 冲突检测

**文件**: `crates/transaction/src/lock.rs:348-368`

```rust
for existing_lock in self.range_locks.values() {  // BTreeMap 遍历
    if existing_lock.holders.contains(&tx_id) {
        continue;
    }
    if matches!(&existing_lock.target, LockTarget::Gap { .. })
        && existing_lock.target.overlaps(&target)
    {
        // Gap vs NextKey 冲突处理...
    }
}
```

**问题**: 每次获取 NextKey 锁时遍历所有 range_locks，在高并发 UPDATE 场景下成为瓶颈。

**复杂度**: O(n) 其中 n = range_locks 数量

---

### 问题 2: REPEATABLE READ 每次读更新 snapshot_timestamp

**文件**: `crates/transaction/src/transaction_manager.rs:261-264`

```rust
if isolation == IsolationLevel::RepeatableRead {
    let read_ts = self.global_timestamp;  // 每次读都消耗时间戳
    self.global_timestamp += 1;
    active_tx.snapshot.snapshot_timestamp = read_ts;
}
```

**问题**: UPDATE 操作包含读（检查当前值），每次读都消耗全局时间戳并可能触发快照刷新。

---

### 问题 3: BTreeMap vs HashMap

**文件**: `crates/transaction/src/lock.rs`

`range_locks` 使用 `BTreeMap` 而非 `HashMap`，区间查找复杂度更高。

---

## 关键文件

| 文件 | 行数 | 说明 |
|------|------|------|
| `crates/transaction/src/lock.rs` | 1348 | Range lock 实现 |
| `crates/transaction/src/transaction_manager.rs` | ~500 | 事务管理 + snapshot_timestamp 更新 |
| `crates/transaction/src/mvcc.rs` | ~800 | MVCC snapshot |

---

## 验证步骤

1. 检查 `begin_transaction` 默认 isolation level 是否意外变为 REPEATABLE READ
2. 对比 `git diff HEAD~20` 前后 UPDATE QPS
3. 测量 range_lock 操作的 CPU 占比

---

## 修复建议

### 方案 1: 优化 Range Lock 冲突检测

将 BTreeMap 遍历优化为更高效的冲突检测算法：
- 使用区间树 (Interval Tree) 替代线性遍历
- 或使用更细粒度的锁分区

### 方案 2: 降低 REPEATABLE READ 频率

- 在 UPDATE 操作中合并多次读为单次读
- 延迟 snapshot_timestamp 更新到事务真正需要时

### 方案 3: 降级默认 Isolation Level

- 将默认 isolation level 从 REPEATABLE READ 降级为 READ COMMITTED
- 让用户显式选择 REPEATABLE READ

---

## 相关文档

- ADR-0005: MVCC 实现规范
- Engineering Control Plane: `~/wiki/knowledge/services/control_plane.py`
- Optimization Generator: `~/wiki/knowledge/services/optimization_generator.py`
