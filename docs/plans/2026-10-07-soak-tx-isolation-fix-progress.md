# P0-1 修复进度 — 部分修复，被更深层缺陷阻塞

> **日期**: 2026-10-07
> **分支/worktree**: `origin/develop/v4.1.0` @ `ed181dfc0b` + 本次改动
> **结论**: ⚠️ **未修复** — 完成了根因定位与部分加固，暴露了更深层的架构缺陷

---

## 1. 交付物

| 产物 | 路径 |
|---|---|
| 回归测试（先失败） | `tests/integration/transaction/concurrent_rollback_isolation_test.rs` |
| 存储层改动 | `crates/storage/src/file_storage.rs` |
| 测试注册 | `Cargo.toml` 新增 `[[test]] concurrent_rollback_isolation_test` |

---

## 2. 根本原因（三层，逐层剥开）

`FileStorage` 是**全服务器共享的单实例**（`do_command_loop` 接收
`Arc<RwLock<BoxStorageEngine>>`，每个连接 `storage.clone()`），
而事务状态全部是**实例级字段**，不是 per-connection：

| 字段 | 位置 | 问题 |
|---|---|---|
| `current_tx_id: AtomicU64` | `file_storage.rs:206` | **单个共享单元** |
| `tx_undo_log: Vec<UndoOp>` | `WriteState` | 单一共享向量 |
| `insert_buffer` | `WriteState` | 单一共享 map |

### 第 1 层（已修）：回滚重放了他人的 undo

`rollback_transaction` 把整个共享向量排空重放。已修：
- 引入 `TxUndoEntry { tx_id, op }` 包装，9 个 push 点全部带上 tx id
- 回滚时 `if entry.tx_id != rolling_back_tx { continue }`
- `commit_transaction` / `begin_transaction` 的 `clear()` 改为
  `retain(|e| e.tx_id != ...)`，不再清空他人在途事务的 undo

### 第 2 层（**阻塞点，已诊断确认**）：并发事务拿到同一个 tx id

诊断输出（30 次试验，逐次打印 tx id）：

```
trial 15: LOST. commit_tx=1 rollback_tx=1 total=49
trial 24: LOST. commit_tx=1 rollback_tx=1 total=49
```

**两个并发事务的 `begin_transaction()` 都返回 tx id = 1。**

因为 `begin_transaction` 读的是共享的 `self.current_tx_id`：
第一个 `BEGIN` 写入 1，第二个 `BEGIN` 看到 `existing != 0`，
按 MySQL nested-BEGIN 语义 **返回已存在的 id**（`file_storage.rs:5382-5386`）。

于是第 1 层的 `tx_id != rolling_back_tx` 过滤**无法区分这两个事务** ——
它们本来就"是"同一个 tx id，修复在语义上失效。

---

## 3. 测试结果

### 修复前（基线）

```
test concurrent_delete_insert_txns_lose_no_rows ... FAILED
  row count drifted from 200 to 202
test committed_delete_survives_peer_rollback ... FAILED
  erased in 1/30 trials
```

### 修复后（3 次连续运行）

| 运行 | SOAK 形态测试 | 同 id 删除测试 |
|---|---|---|
| 1 | PASS | FAIL 2/30 |
| 2 | FAIL 200→202 | FAIL 1/30 |
| 3 | PASS | FAIL 4/30 |

**仍未修复**，且 SOAK 形态测试出现**双向漂移**（既丢行也增行，
`200 → 202` 说明存在重复插入路径）。

诚实结论：第 1 层改动**没有把指标推向零**。SOAK 形态测试偶发通过
属噪声，不是修复生效的证据。

---

## 4. 为什么不能继续修

真正的修复需要把 `current_tx_id` 从**实例级共享单元**改为
**per-connection 上下文**。这不是局部改动：

- `StorageEngine` trait 的 `set_current_tx_id(&mut self, id)` /
  `set_current_tx_id_shared(&self, id)` 签名假定实例级状态
- 全仓 45+ 处调用点，跨 `engine` / `executor` / `storage` / `mysql-server`
- `current_db`、`insert_buffer`、`dirty_tables` 存在同样的共享问题
- 正确的方向是 `thread_local` 或把 tx 上下文作为显式参数贯穿 DML 路径

这是一次**存储层事务模型重构**，不是 bug fix。风险面远超"修一个
P0"，且会触及所有 DML 路径 —— 应当独立立项、独立评审，
在 SOAK 已有的 45% 丢行证据驱动下推进。

强行在本轮继续改，只会在未验证的状态下堆叠更多共享状态改动，
风险高于收益。

---

## 5. 建议的处置

| 选项 | 说明 |
|---|---|
| **A（推荐）** | 保留本次诊断成果 + 回归测试作为重构的验收基准，另立 issue 做事务模型重构 |
| B | 回退 `file_storage.rs` 的部分改动，只留回归测试（它当前稳定复现缺陷） |
| C | 继续在本 worktree 上做 per-connection 重构 —— 需要完整回归测试套件兜底 |

无论选哪个，**SOAK 在当前 v4.1.0 上都无法出具可信基线**：
45-55% 丢行率 + 静默失败，任何压测数字都不可信。

---

## 6. 回归测试的价值

`concurrent_rollback_isolation_test.rs` 稳定复现缺陷（多轮 1-4/30），
且断言的是**数据结果**（行是否存在、行数是否守恒），不是"调用是否返回 Ok"——
后者在整个缺陷期间一直是绿的，而数据在消失。

重构完成后，这个测试应作为验收标准：复现率必须降到 0/30。