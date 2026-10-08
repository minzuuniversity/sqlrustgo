# ISSUE-RECOVERY-QUADRATIC-REPLAY (P0) — WAL 恢复逐条整表重写，启动不可用

> **Date**: 2026-10-08
> **Status**: `OPEN` — 实测复现，根因已定位到 file:line
> **Severity**: P0 — 阻断任何 release
> **Category**: 正确性/可用性 — 恢复路径复杂度缺陷
> **Found by**: v4.1.0 1h SOAK 验证（重启核对行数时暴露）
> **Source commit**: `b1e9a98ee6bac1e863ab8826c54a41500455b71d` (develop/v4.1.0)
> **file:line 均以该 commit 为准**
> **Evidence**: `docs/releases/v4.1.0/SOAK_V410_1H_2026-10-08/`
> （`recovery_stack_sample.txt` 为 `sample` 原始输出）

---

## 1. 现象

服务器重启后，**第一条全表扫描耗时 24 分 22 秒**，紧接着的同一条查询 **0.08 秒**：

```
$ mysql ... -e "SELECT COUNT(*) FROM sbtest1;"
68288
real  24m22.594s        <- 冷（重启后首次）

$ mysql ... -e "SELECT COUNT(*) FROM sbtest1;"
68288
real  0.08s             <- 热
```

第二次干净重启复现同样行为。

数据目录状态：

| 文件 | 大小 |
|---|---:|
| `sbtest1.json` | 13.6 MB |
| `sbtest1_idx_id.json` | 1.3 MB |
| `sqlrustgo.wal` | 8.0 MB |

---

## 2. 根因（已定位，非推测）

恢复不是"惰性首次访问"的良性开销，而是 **O(重放条目数 × 表大小)**，并且**阻塞启动**。

### 2.1 调用链

`sample` 抓取（pid 74061，恢复进行中第 28 分钟）：

```
run_server_with_listener_and_shutdown
  StatefulRecoveryEngine<FileStorage>::recover      recovery_engine.rs:99
    FileStorage::insert_direct                       file_storage.rs:3328
      FileStorage::save_table_full                   file_storage.rs:775
        serde_json::to_writer_pretty
        write_all_cold
```

恢复路径上每个 `WalEntryType::Insert` 经 `recovery_force_insert`
（`crates/storage/src/recovery_engine.rs:274`，由 `apply_wal_entry` 于 `:767` 调用）
进入 `FileStorage::insert_direct`，后者调用 `save_table_window`
（`crates/storage/src/file_storage.rs:723`）。

### 2.2 缺陷代码

恢复时 `last_saved_row_count == 0`，于是 `save_table_window` 每次都命中它的冷启动分支
（`file_storage.rs:742`）：

```rust
if total_rows == 0 || last_saved == 0 || total_rows <= last_saved {
    // Cold start, shrink, or a DELETE/UPDATE path: the caller
    // handed us a window, not a snapshot, so re-derive the
    // full table under the lock. Rare relative to inserts.
    if let Some(data) = st.tables.get(&scoped) {
        return self.save_table_full(db, table_name, data);   // 整表重写
    }
    return Ok(());
}
```

`save_table_full`（`:775`）用 `to_writer_pretty` 序列化**整张表**，写完后
`remove_file(delta_path)` 丢弃所有待合并的 delta。

**delta 追加的快路径在恢复期间一次都没走到** —— `last_saved == 0` 这个条件在首次
整表保存完成前始终成立，而每次整表保存又把状态保持在同一处。

### 2.3 文件系统层面的直接观测

恢复进行中 60 秒内的采样：

```
20:47:44  sbtest1.json  13,602,635 bytes
20:47:54  sbtest1.json   7,340,026 bytes
20:48:04  sbtest1.json           0 bytes
20:48:14  sbtest1.json  13,602,594 bytes
(任何采样点都不存在 .delta 文件)
```

13.6 MB 快照约**每 10 秒重写一次，反复数百次**，从不收敛到 delta 追加。
58,288 条 INSERT 对应一张增长到 68,288 行的表，累计是 58,288 × 平均表大小的
JSON 序列化量 —— 这就是 24 分钟的来源。

---

## 3. 影响

- **启动不可用**：恢复在 listener 接受连接之前完成，因此这是硬性可用性中断，
  不是"慢查询"。
- **代价随 soak 变长而恶化**：重放量正比于累积 WAL 条数，表越大每次重写越贵。
  这恰好是 168h SOAK 要覆盖的场景。
- **任何重启-再查询路径都受影响**：崩溃恢复、主从切换、备份还原、运维重启。
- **历史 SOAK 全部未覆盖**：`scripts/soak/v400_1h_soak.sh` 从不中途重启服务器，
  所以 v4.0.0 与 v4.1.0 的任何一次 SOAK 记录都没碰过这条路径。本次 1h run 之所以
  能发现，纯粹是因为验证行数时重启了服务器。

---

## 4. 可重现性

```bash
# 1. 起一个 file storage 服务，跑一轮写入
target/release/sqlrustgo-mysql-server serve --port 3431 \
  --data-dir /tmp/repro-dir --storage file --wal-sync every &

bash scripts/soak/v400_prepare_soak.sh 3431 10000
# 灌入大量行（例如 58,288 条 INSERT）

# 2. 停服，再冷启动
kill <pid>; target/release/sqlrustgo-mysql-server serve --port 3431 \
  --data-dir /tmp/repro-dir --storage file --wal-sync every &

# 3. 计时首条全表查询
time mysql -h 127.0.0.1 -P 3431 -u root -e "SELECT COUNT(*) FROM sbtest1;"
#   -> 冷：分钟级；紧接着再跑一次 -> 毫秒级
```

并观察 `<data-dir>/<table>.json` 在恢复期间被反复整体重写。

---

## 5. 修复方向

按投入/收益排序：

1. **恢复期按表批量重放**：先把全部已提交条目按表聚合，再对每张表走一次
   `insert_direct`/delta 追加，而不是逐条。当前 `RecoveryEngineImpl::recover()`
   （`recovery_engine.rs:679`）已是 `for entry in &dml_entries { … apply_entry(…) }`
   的逐条循环（`:708`）。
2. **预置 `last_saved_row_count`**：进入恢复前把已落盘的行数写进
   `last_saved_row_count`，使 `save_table_window` 走 delta 分支而非冷启动分支。
   改动面最小，可直接消除重复整表重写。
3. **关闭 pretty 打印**：`save_table_full` 改用 `to_writer`（紧凑格式）。
   对本例数据形态预计可把文件缩小 2–3 倍，同步减少每次重写的 I/O。
4. **周期性 compaction 的触发条件**：`save_table_window` 的 delta 阈值
   (10 MB) 在恢复路径上从未生效，因为根本没进过 delta 分支；修好 1/2 之后需要
   重新验证该阈值不会引入新的重写尖峰。

修复必须附带回归测试（见 §6），且需覆盖 **FileStorage** 而非 MemoryStorage
—— `docs/audit/issues/ISSUE-2740_crash_recovery_unverified.md` 已指出既有恢复测试
全部使用 `MemoryStorage`，其生命周期是"进程退出即数据消失"，无法验证持久化要求。

---

## 6. 关闭条件（每行须有实测证据）

| 要求 | 可观测行为 | 测试设计 | 证据 |
|---|---|---|---|
| 恢复不再整表重写 | 恢复期间 `<table>.json` 不被重复整体重写 | 灌 N 行 → 重启 → 采样文件 mtime/大小 | 采样日志显示写入次数 ≤ 常数 |
| 恢复成本次线性 | N 行恢复耗时随 N 线性增长 | N ∈ {1k, 10k, 50k} 三点计时 | 三个耗时点，斜率 ≈ 1 |
| 重启后可服务 | 恢复完成后 listener 接受连接 | 重启 → 轮询端口直到 accept | 实测等待秒数 |
| 数据不丢 | 重启后行数与退出前一致 | INSERT N → 重启 → `COUNT(*)` | `COUNT(*) == N` |
| 增量路径被走到 | 恢复产生 `.delta` 文件或走追加分支 | 同上 + 检查 delta 路径命中 | 文件/日志证据 |

**这五项全部实测通过前，verdict 保持 `OPEN`。**

---

## 7. 与既有条目的关系

- `docs/audit/issues/ISSUE-2740_crash_recovery_unverified.md`（P0，Status `UNKNOWN`）：
  指出恢复正确性从未被 FileStorage 端到端验证。本条是**性能/可用性**维度，
  与之互补，不重复计数；但其"测试必须用 FileStorage"的结论对本条修复同样适用。
- `PERFORMANCE_OPTIMIZATION_PLAN.md §10.13` 提到一处**未分配 issue 编号**的
  "flush() 持久化缺口"（无周期性/事务性 flush，重启丢行）。本条位于同一代码区域
  （`file_storage.rs` 持久化路径），建议合并考虑，但两者根因不同：本条是恢复期
  的重复重写，不是持久化时机。
- 与 `SOAK_V410_1H_2026-10-08/STABILITY_REPORT.md` 的结论直接对应：同一次 run 中
  记录的 "RSS 无收敛" 与本条共同构成 **v4.1.0 尚不具备 168h SOAK 资格**的证据。

---

## 8. 本条不做的事（明确不主张）

- 不主张"WAL 本身有缺陷" —— WAL 内容正确，重放出的行数 68,288 与预期完全一致。
- 不主张"数据损坏" —— 冷/热两次查询返回相同结果。
- 不主张"recover() 每次都跑" —— `StatefulRecoveryEngine` 有 `RecoveryState` 幂等
  保护，重复调用返回空报告（`recovery_engine.rs:105`）；本条说的是**单次恢复内部**
  的复杂度，不是重复恢复。
