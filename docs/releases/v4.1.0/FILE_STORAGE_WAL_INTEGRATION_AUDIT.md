# B-0：FileStorage / TxManager / WAL / Recovery 集成审计

- source_agent: mcode (MiniMax-M3.1-Flash-Preview)
- source_run: b0-wal-integration-audit-20261010
- 基线: `e849bc99f6`（PR #5244 合并后）
- 关联: #5177 Phase B 架构闸门、IAIA-410
- 性质: **架构核验报告**，结论全部来自代码证据，不依赖历史设计文档或模块名称

---

## 0. 本报告回答的问题

> SQL COMMIT 返回成功之后，如果立即杀死进程并重启，FileStorage 中的 UPDATE 究竟依靠
> 什么机制保证不丢失、不重复、不回滚已提交数据？

**答案：依靠 `WalStorage` 的 WAL + 每事务 `fsync`。但 `FileStorage` 自身不提供这个保证
—— 它只把条目 `append` 进 `BufWriter`，从不 `flush`/`sync`。**

这个区别决定了 Row Mutation Delta 应当**依附 WAL**，而不是自己造一套事务协议。

---

## 1. 八个检查点

| # | 检查点 | 代码证据 | 结论 |
|---|---|---|---|
| 1 | FileStorage 是否由正常 Server 启动路径实例化 | `crates/mysql-server/src/lib.rs:6863` `FileStorage::new_with_wal(...)` → `:6924` `MvccStorage::new(...)`；`WalStorage` 包装见 `:6900` 之后的 `sync_mode` 分支构造 | ✅ 是 |
| 2 | SQL UPDATE 是否最终落到该实例 | `src/engine_dml.rs:1344-1345` `delete_in_db` + `insert_in_db` | ✅ 是（且确为 delete+insert） |
| 3 | TxManager 是否管理该后端提交/回滚 | `file_storage.rs:5596` `commit_transaction_for`：丢弃 undo log + 追加 Commit 条目 | ⚠️ 见 §2 |
| 4 | 修改是否进入事务 WAL | `file_storage.rs:5618` `wal_append(vec![wal_tx_entry(Commit, tx_id, 0)])` | ✅ 是 |
| 5 | COMMIT 返回前哪些数据已持久化 | `wal_storage.rs:1019-1040` 按 `sync_mode` 分支；默认 `Every` → `:1033` `wal.lock().sync()` | ✅ WAL 已 fsync |
| 6 | 崩溃后靠什么恢复 | `crates/storage/src/recovery_engine.rs:612` `wal.recover()` + `apply_entry` | ✅ WAL 回放 |
| 7 | Flush 是事务一部分还是独立后台过程 | `file_storage.rs:1846` `flush()` 与 commit 无调用关系 | ❌ **完全独立** |
| 8 | Delta 重放后索引如何恢复 | 无 Delta 重放路径（Delta 只被写、不被读） | ⚠️ 见 §4 |

---

## 2. 检查点 3 的细节：FileStorage 的 COMMIT 本身不保证持久性

`FileStorage::commit_transaction_for`（`file_storage.rs:5596`）全文只做两件事：

```rust
s.tx_undo_log.retain(|e| e.tx_id != tx_id);      // 丢弃 undo
self.current_tx_id.store(0, ...);
if self.wal_enabled() {
    self.wal_append(vec![self.wal_tx_entry(WalEntryType::Commit, tx_id, 0)])?;
}
```

**没有 `flush()`，没有 `sync()`，也没有把 `insert_buffer` 落到 `t.json`。**

而 `FileStorage` 侧 `WalManager` 的三个层级是分开的（`file_backed_wal_manager.rs:141-170`）：

| 方法 | 行为 |
|---|---|
| `append` | 写入 `BufWriter`（用户态缓冲） |
| `flush` | `BufWriter::flush` → 内核页缓存 |
| `sync` | `flush` + `File::sync_data()` → **真正的 fsync** |

grep 全仓，`sync()` 的调用点只在 `wal_storage.rs:203/487/1028/1033` 与
`parallel_wal_storage.rs:268/277/287` —— **`FileStorage` 自己从不调用 `sync`**。

**结论：durability 由包装层 `WalStorage` 提供，不由 `FileStorage` 提供。** 这正是决策要求
区分的那一点。

---

## 3. 检查点 5：生产默认确实每事务 fsync

`WalStorage::commit_transaction`（trait 版，`wal_storage.rs:993`）在写入 Commit 条目后：

```rust
match self.sync_mode {
    WalSyncMode::Off      => { /* 无 sync —— 最快，但无持久性 */ }
    WalSyncMode::Batch(n) => { /* 每 n 个事务 sync 一次 */ }
    WalSyncMode::Every    => { self.wal.lock().sync()?; }      // ← 默认
    WalSyncMode::GroupCommit { .. } => { self.wal.lock().sync()?; }
}
```

`WalSyncMode` 的 `#[default]` 是 `Every`（`wal_storage.rs:36-39`，注释
"Sync after every transaction (default, full durability)"）。

**注意 `GroupCommit` 在这条路径上是"降级为每事务 fsync"** —— 代码注释自己写明
「WalStorage path doesn't install a coordinator; fall back to every-tx fsync for
safety」。也就是说 group commit 只在 `ParallelWalStorage` 路径生效。

**但 `Off` 与 `Batch(n)` 存在。** 若配置为这两者，`COMMIT` 返回成功时数据可能只在页缓存，
进程被 kill 会丢失。**这是配置面，不是默认行为** —— 但任何把 `Off` 当性能开关使用的部署，
都会得到「COMMIT 成功但丢数据」。

---

## 4. 检查点 7 与 8：Flush 与 Delta 的真实地位

### Flush 完全独立于事务

`FileStorage::flush()`（`file_storage.rs:1846`）与 `commit_transaction_for` 之间**没有任何
调用关系**。COMMIT 不触发 flush，flush 也不需要 COMMIT。

推论：**`COMMIT` 成功 ≠ 数据已落 `t.json`。** 已落盘的只有 WAL。这对 B 阶段的设计是决定性的
—— Delta/snapshot 是**物化状态**，不是事务边界。

### Delta 目前只写不读

实测（Phase A）：

| 场景 | flush 后文件 |
|---|---|
| 纯 INSERT（行数增长） | `t.json`, `t_idx_id.json`, **`t.delta`** |
| UPDATE（行数不变） | `t.json`, `t_idx_id.json`（全量重写） |

`save_table_window`（`file_storage.rs:1570` 附近）的增量分支判据是 `total_rows > last_saved`。
增量路径本身是活的，**只是只能表达追加，无法表达改写**。

检查点 8 因此部分不适用：Delta 从不参与回放，所以「Delta 重放后索引如何恢复」这个问题
目前不存在；但一旦让 Delta 承载 UPDATE/DELETE，**它就必须能被回放，索引一致性就成为新问题**。

---

## 5. 对两条架构路径的判定

### 路径 A（WAL 为权威恢复日志）—— **成立**

代码证据支持：

1. COMMIT 时 WAL 已 fsync（`wal_storage.rs:1033`，默认 `Every`）
2. 启动时 `RecoveryEngine` 从 WAL 回放（`recovery_engine.rs:612`）
3. 回放按 `database_name` 定位库（#5192 已修复并验证）

**Delta 在这个架构里是可重新生成的物化状态** —— 崩溃后 WAL 回放会重建行集，
snapshot/delta 只是加速冷启动。**因此 Row Mutation Delta 不需要自己实现事务提交、
未提交数据隔离或原子快照协议**，它只需能被 WAL 回放重建即可。

这直接回答了决策中的核心疑问：

> 存在 `sqlrustgo.wal` 文件，不等于已经实现事务级 WAL 保证。

——**本报告的结论是：在默认配置下，事务级 WAL 保证确实存在**（每事务 fsync + 启动回放），
不是仅有一个文件。**但在 `sync_mode = Off/Batch` 下不成立**，这是一个未被守护的配置面。

### 路径 B（FileStorage 独立承担持久化）—— **不必要且危险**

若让 Delta 自带事务协议，就会出现两条恢复路径（WAL 与 Delta）竞争同一份数据。
代码里 `RecoveryEngine` 只认 WAL，Delta 从不被读 —— **路径 B 等于新增一条无人验证的恢复路径**。

决策中「不要以两个日志文件同时存在为由，构造未经证明的双重恢复协议」的顾虑，
在代码层面得到确认：双重协议的风险是真实的，因为 WAL 回放路径已经存在且是唯一被测试的。

---

## 6. 遗留风险（不阻塞 B 阶段，但需登记）

| 风险 | 证据 | 影响 |
|---|---|---|
| `sync_mode = Off` 可被配置 | `wal_storage.rs:36-43` | COMMIT 成功但崩溃丢数据。需门禁或文档守护 |
| `WalStorage` 的 `GroupCommit` 是降级实现 | `wal_storage.rs:1035-1039` 注释 | 以为在 group commit，实际每事务 fsync（性能低于预期，方向安全） |
| Delta 无回放路径 | Phase A 实测 | 承载 UPDATE 后必须补回放 + 索引一致性 |
| `commit_transaction_for` 不落 snapshot | `file_storage.rs:5596` | 数据只在 WAL；snapshot 滞后是设计而非缺陷，但必须写进文档 |

---

## 7. 对 B 阶段的直接结论

1. **Row Mutation Delta 应当依附 WAL**（路径 A），不自行实现事务协议
2. **Delta 必须可被回放**，且回放后需保证索引一致 —— 这是新增要求
3. **不得弱化事务承诺**：Delta 只能是 WAL 的物化，不得成为权威
4. B 阶段首批四项工作（持久化兼容测试、UPDATE/DELETE 重启测试、故障注入框架、
   本报告）现已交付三项，第四项即本文件

---

## 8. 引用

- `crates/storage/src/file_storage.rs:5596` `commit_transaction_for`
- `crates/storage/src/file_storage.rs:1846` `flush`
- `crates/storage/src/file_storage.rs:1570` `save_table_window`
- `crates/storage/src/wal/file_backed_wal_manager.rs:141-170` `append` / `flush` / `sync`
- `crates/storage/src/wal_storage.rs:36-46` `WalSyncMode`
- `crates/storage/src/wal_storage.rs:993-1040` `commit_transaction`（`:1033` 默认 fsync）
- `crates/storage/src/recovery_engine.rs:612` WAL 回放入口
- `crates/mysql-server/src/lib.rs:6863` 生产实例化（`:6924` 包 `MvccStorage`）
- `src/engine_dml.rs:1344-1345` UPDATE 的存储层调用序列