# 迭代 5 · 批次 12 —— ROLLBACK 路径：进程级 SIGABRT 与「回滚后行仍可见」（#5480 / #5506）

> **基线**：`develop/v4.1.0` @ `42077e8e2eb34725152962033f0ba951cb4d7bb9`
> **日期**: 2026-10-22
> **分支**: `fix/rollback-path-5480-batch12`
> **基线**: `develop/v4.1.0` @ 批次 11 的 merge commit
> **范围**: 存储层事务回滚。2 个 issue，共 4 处根因：
> **1 个 trait-vs-inherent 解析陷阱导致的无限递归**（进程 SIGABRT），
> **1 个固有方法遮蔽 trait 方法导致回滚语义缺失**，
> **1 个无锁回滚路径的 undo 空实现**，
> **1 个 blanket 缓冲清空违反隔离契约**。

---

## 0. 一句话

#5480 把栈溢出定位在「WAL 崩溃恢复路径」，并推测是恢复期间的递归或析构递归。
**实测证明两者都不是**：栈溢出发生在**更早的位置** —— `rollback_transaction()`。
恢复路径根本没有执行到。

---

## 1. 实测：栈溢出的精确位置

在仓库外用一个独立 crate 复现（只依赖 `crates/storage`），逐步打点：

```text
[step] create wal storage
[step] seed table t6
[step] tx1 begin / insert / commit / committed
[step] tx2 begin
[step] tx2 insert
[step] tx2 rollback            ← 打印之后立即溢出
```

```text
thread 'main' (46522580) has overflowed its stack
fatal runtime error: stack overflow, aborting
```

⇒ 溢出发生在 **`rollback_transaction()` 内部**。
`run_recovery()` 在其后，**根本没有执行**。

---

## 2. 根因 1 —— trait 实现自我递归（无限递归 ⇒ 栈溢出 ⇒ SIGABRT）

`crates/storage/src/file_storage.rs` 同时有两个 `discard_all_buffers`：

```rust
// 固有方法（impl FileStorage）
pub fn discard_all_buffers(&self) {
    Self::with_write_lock(self, |s| { s.insert_buffer.clear(); });
}

// trait 实现（impl StorageEngine for FileStorage）
#[allow(unconditional_recursion)]
fn discard_all_buffers(&mut self) {
    self.discard_all_buffers();      // ← 调回自己
}
```

`self.discard_all_buffers()` **不会**解析到上面的固有 `&self` 方法：
接收者候选 `&mut FileStorage` 在**按值**匹配阶段就命中了 trait 方法
（trait 方法的 `self` 类型正是 `&mut FileStorage`），
固有的 `&self` 实现根本没有机会被选中。⇒ **无限递归**。

原注释的**意图**是对的（「Delegate to the inherent `&self` implementation」），
但**调用形式**用错了；`#[allow(unconditional_recursion)]` 把编译器本来会报的警告盖掉了。

**同一个文件里的 `drop_table_in_db` 踩过完全一样的坑，并且已经用全限定路径修好了**：

```rust
// crates/storage/src/file_storage.rs:6546（既有修法）
FileStorage::drop_table_in(self, db, table)
```

⇒ `discard_all_buffers` 是同一个坑的**未修版本**。

---

## 3. 根因 2 —— 泛型包装层把调用导回 trait 方法（⇒ 生产形态同样崩溃）

生产服务端的存储栈是
`WalStorage<MvccStorage<FileStorage>, FileBackedWalManager>`
（`src/engine_builder.rs:294` / `:353` / `:411`）。

`MvccStorage` 是**泛型**的：

```rust
// crates/storage/src/mvcc_storage.rs:1122
fn discard_all_buffers(&mut self) {
    self.inner.discard_all_buffers()      // S 是泛型 ⇒ 只能解析到 StorageEngine 的 trait 方法
}
```

在**泛型上下文**里 `S` 上只有 trait 方法可选 ⇒ 单态化到 `S = FileStorage` 后
落到 `FileStorage` 的那个自我递归实现 ⇒ **同样的无限递归**。

**两种栈形态实测对比（修复前）**：

| 栈形态 | 结果 |
|---|---|
| `WalStorage<FileStorage, _>`（`tests/integration/stress/process_kill_crash_test.rs` 用的形态） | **SIGABRT** |
| `WalStorage<MvccStorage<FileStorage>, _>`（**生产形态**） | **SIGABRT** |

⇒ **生产服务端上，任何一条 `ROLLBACK` 都会让整个进程栈溢出并被 SIGABRT 杀掉。**
不是连接级失败，是**服务进程崩溃** ⇒ 崩溃循环。

---

## 4. 根因 3 —— 固有 `rollback_transaction` 遮蔽了已修好的 trait 实现

`crates/storage/src/wal_storage.rs` 有**两个** `rollback_transaction`：

| | 内容 |
|---|---|
| 固有方法（原 :563） | 写 WAL Rollback 条目 → `inner.discard_all_buffers()` → 从 `active_txs` 移除 |
| trait 实现（原 :1500） | 写 WAL Rollback 条目 → 从 `active_txs` 移除 → **`inner.rollback_transaction()`** → `inner.discard_all_buffers()` → `current_tx_id` 归零 |

trait 实现里有一段 #4974 的说明：

> Without it `MvccStorage::rollback_transaction`'s `discard_pending()` is never
> reached from the `&mut` rollback, so an aborted transaction's versions stay
> pending forever.

**这个修复从来没有生效过** —— 因为固有的 `&mut self` 方法在方法解析的同一阶段
**遮蔽**了 trait 方法，所有调用方（含引擎）到达的都是那个**不完整**的固有版本。

后果（修掉崩溃之后才能看到）：**ROLLBACK 之后，被回滚的行仍然可见**。

---

## 5. 根因 4 —— blanket 缓冲清空违反隔离契约

`crates/mysql-server/tests/verify_4957_rollback_buffer_test.rs` 明确钉住契约：

> `one_connections_rollback_must_not_discard_anothers_buffered_rows`
> —— "A's ROLLBACK must not discard B's pending rows (100..110)"

而 `FileStorage` 的两个事实让 **blanket 清空**必然违约：

1. `insert_buffer` 是**实例级**的，不是连接级的（#5060）—— 它装着**所有**打开的事务的行；
2. `FileStorage::insert` **总是缓冲**（"Now: always buffer inserts, caller
   explicitly calls flush() to persist"），所以缓冲里还装着**已提交但尚未 flush** 的行。

⇒ blanket `insert_buffer.clear()` 会同时毁掉
**别的连接的未提交行**与**已提交但未落盘的行**。

⇒ 正确的机制是**逐事务撤销**
（`FileStorage::rollback_transaction_for`：按值精确移除**本事务**的
`UndoOp::BufferedInsert` 条目 —— #5055 / #5060）。

---

## 6. 修法

| # | 位置 | 改动 |
|---|---|---|
| 1 | `crates/storage/src/file_storage.rs` | trait `discard_all_buffers` 改用**全限定路径** `FileStorage::discard_all_buffers(self)`（与同文件 `drop_table_in_db` 既有修法一致） |
| 2 | `crates/storage/src/file_storage.rs` | `rollback_transaction_lockfree_for` 补上**逐事务 undo**（当前是一个空占位：`s.insert_buffer.get_mut(&String::new())`）⇒ #5506 |
| 3 | `crates/storage/src/wal_storage.rs` | 固有 `rollback_transaction` 改为**委托给 trait 实现**（与同文件既有 `commit_transaction` 完全同构的 UFCS 写法）⇒ 消除遮蔽 |
| 4 | `crates/storage/src/wal_storage.rs` | trait 实现里**去掉 blanket 的 `inner.discard_all_buffers()`**，只保留 `inner.rollback_transaction()`（逐事务撤销）⇒ 保住隔离契约 |
| 5 | `crates/storage/src/file_storage.rs` | `discard_all_buffers_shared` **显式实现为 no-op**（原先落到 trait 默认实现），并在代码里写明「任何实现必须是**逐事务**的」⇒ 见 §5.5 |

改动 3 顺带带来两个此前缺失的行为：
`current_tx_id` 归零（事务真正结束）、`inner.rollback_transaction()` 被调用
（内层引擎的待定版本被丢弃）。

### 5.5 自审时抓到的隔离回归（本批次自己引入的，改掉）

第一次实现时，改动 2 把 `rollback_transaction_lockfree_for` 补上了逐事务 undo，
同时**顺手**给 `FileStorage` 加了 `discard_all_buffers_shared`，实现为
`FileStorage::discard_all_buffers(self)`（blanket 清空）——
直觉是「缺的方法就补上，语义对齐 `&mut` 版本」。

自审时逐条核对调用链，发现这条链是：

```text
WalStorage::rollback_transaction_lockfree_for(tx_id)
  ① WAL `Rollback` 条目 + sync
  ② inner().rollback_transaction_lockfree_for(tx_id)  → 逐事务 undo      ✅ 正确
  ③ inner().discard_all_buffers_shared()              → blanket 清空    ❌ 连坐
```

③ 落在**同一个实例**的 `insert_buffer` 上，而该缓冲是**实例级**的（#5060），
`FileStorage::insert` 又**总是缓冲** ⇒ ③ 会把
**别的连接的未提交行**和**已提交但未落盘的行**一起清掉。

既有钉子 `crates/mysql-server/tests/verify_4957_rollback_buffer_test.rs` 正面命中：

> `assert_eq!(count(&mut b), 10, "A's ROLLBACK must not discard B's pending rows (100..110)")`

而该测试的 `storage: None` **默认就是 `FileStorage` + WAL**
（`crates/mysql-server/src/lib.rs` `EphemeralConfig::storage` 文档），
⇒ 走的就是这条被污染的链路。

**修法**：改动 2 才是 #5506 的真正修复点（③ 从来不是修复点），
因此 `discard_all_buffers_shared` 必须是 **no-op**：
逐事务 undo 已经在 ② 做完了，而它是**唯一**能在不碰别人行的前提下完成撤销的机制。

**并补上缺失的钉子**：原有隔离测试的 4 个用例都走 `&mut self` 回滚
（`WalStorage::rollback_transaction`），**没有一个走 ③** ⇒ 覆盖缺口。
新增 `the_walstorage_lock_free_rollback_keeps_other_transactions_rows`
专门钉 ③ 所在的那条路径（见 §7）。

⇒ 这条记录留在证据里，是因为它正是「判别力」的价值所在：
**测试不是用来证明代码对的，是用来证明代码错得能被看见的。**


---

## 7. 判别力：每一处改动都有对应的**变异失败**

判别力不是「测试通过」，而是**「把改动撤掉，测试必须红」**。
本批次对 4 处改动逐一做了变异实验（在真实工作树上注入，不是推理）：

| 改动 | 变异方式 | 观察到的失败 |
|---|---|---|
| ① `FileStorage::discard_all_buffers` 递归 | 整体撤销修复 | **SIGABRT / 栈溢出**，测试二进制被中止（`has overflowed its stack`） |
| ③ `WalStorage::rollback_transaction` 委托 | 同上 | 同上；且修复后「回滚后行仍可见」也一并消失 |
| ④ 去掉 blanket `discard_all_buffers()` | **只**把 blanket 清空加回 | `a_rollback_removes_only_its_own_transaction_rows` **FAILED**：`left: []` / `right: [2]` —— B 的未提交行被 A 的回滚抹掉了 |
| ② 无锁回滚的逐事务 undo（#5506） | **只**撤掉无锁 undo | `the_lock_free_rollback_undoes_the_aborted_transaction_rows` **FAILED**：`left: [1]` / `right: []` |
| ⑤ `discard_all_buffers_shared` 保持 no-op | **只**把 blanket 清空加回 | `the_walstorage_lock_free_rollback_keeps_other_transactions_rows` **FAILED**：`left: []` / `right: [2]` —— B 的未提交行被 A 的无锁回滚抹掉了 |

⇒ 5 处改动，5 组独立的红灯。**没有一处是「改了但看不出差别」的。**

### 变异实验里被推翻的一个假设（记录在此，避免后人重走）

隔离契约的第一版测试**直接用 `FileStorage`**，结果它在**未修复的代码上也全绿** ——
因为 blanket 清空在 `WalStorage` 里，不在 `FileStorage` 里。
⇒ **驱动层级选错，测试就没有判别力。**

同样地，#5506 的无锁 undo 第一版测试**走 `WalStorage`**，
结果它在撤掉该改动后**仍然全绿** —— `WalStorage` 的包装层用自己的机制掩盖了它。
把测试降到**裸 `FileStorage`** 才看到真相：

| 裸 `FileStorage`，锁自由回滚 | 回滚后 `scan` | `flush()` 后 | 磁盘上的 `t.json` |
|---|---|---|---|
| 修复前 | 行**仍然可见** | 仍然可见 | **含该行** |
| 修复后 | 空 | 空 | `"rows": []` |

⇒ #5506 的前提**成立**：被回滚的行确实会落盘。

### 测试钉住的行为

#### `crates/storage/tests/rollback_no_stack_overflow_5480.rs`（3 个用例）

| 用例 | 钉住的行为 |
|---|---|
| `rollback_over_a_directly_wrapped_file_storage_does_not_overflow_the_stack` | 直接形态的回滚**不崩进程** |
| `rollback_over_the_production_storage_stack_does_not_overflow_the_stack` | **生产形态**（`WalStorage<MvccStorage<FileStorage>, _>`）的回滚**不崩进程** |
| `rollback_removes_the_aborted_transaction_rows_from_the_production_stack` | 回滚**真的删掉了行**（不只是不崩） |

> 单独成文件的原因：栈溢出会 **abort 整个测试二进制**，
> 必须与其它断言隔离，否则一次回归会同时带走别的用例。

#### `crates/storage/tests/rollback_isolation_5480.rs`（5 个用例）

| 用例 | 钉住的行为 | 驱动层级 |
|---|---|---|
| `a_rollback_removes_only_its_own_transaction_rows` | A 的回滚**只**删 A 的行；B 的未提交行**必须留下** | `WalStorage` |
| `a_rollback_does_not_discard_another_transaction_committed_rows` | B **已提交**（但未 flush）的行**必须留下** | `WalStorage` |
| `a_rolled_back_row_is_not_persisted_to_disk` | 被回滚的行**绝不落盘**（重开数据目录后仍不存在） | `WalStorage` |
| `the_lock_free_rollback_undoes_the_aborted_transaction_rows` | **锁自由**回滚同样撤销该事务的行，且不落盘 | **裸 `FileStorage`** |
| `the_walstorage_lock_free_rollback_keeps_other_transactions_rows` | **锁自由**回滚走 `WalStorage` 包装层时，B 的未提交行**必须留下**（专门钉 ③ 所在的那条路径） | `WalStorage` |

> 前 4 个用例**都走 `&mut self` 回滚**，一个都没有经过
> `discard_all_buffers_shared()` ⇒ 覆盖缺口。
> 第 5 个用例就是为补这个缺口而加的（§5.5）。

---

## 8. 回归

| 检查 | 结果 |
|---|---|
| `cargo test -p sqlrustgo-storage --test rollback_no_stack_overflow_5480` | **3 passed / 0 failed** |
| `cargo test -p sqlrustgo-storage --test rollback_isolation_5480` | **5 passed / 0 failed** |
| `cargo test -p sqlrustgo-mysql-server --test verify_4957_rollback_buffer_test` | 既有隔离契约（生产形态：file-backed server 双连接）**2 passed / 0 failed** |
| 变异实验（5 组，见 §7） | **5 / 5 变异各自产生独立的红灯** |
| `cargo test -p sqlrustgo-storage --all-features` | 存储层全量：**91 个目标全绿**；2 个目标因**缺 server 二进制**未跑（见下） |
| `cargo fmt --check --all` | PASS |
| `cargo clippy -p sqlrustgo-storage --all-features --lib -- -D warnings` | exit 0 |
| `cargo clippy -p sqlrustgo-storage --all-features --test rollback_no_stack_overflow_5480 --test rollback_isolation_5480 -- -D warnings` | exit 0 |
| `cargo clippy -p sqlrustgo-storage --all-features --all-targets -- -D warnings` | **exit 101 —— 基线既存红**，见下 |
| 本批次改动面 | `crates/storage/` 两个源文件 + 两个新测试文件 |

### clippy 的 `--all-targets` 基线既存红（不是本批次引入）

`cargo clippy -p sqlrustgo-storage --all-features --all-targets -- -D warnings`
在本批次代码上 **exit 101**，4 个错误全部落在
`crates/storage/tests/file_storage_concurrent_same_row_5059.rs`：

| 行 | lint |
|---|---|
| 114 / 172 | `unused_mut` —— `let mut g = storage.write().unwrap();` |
| 179 | `clippy::get_first` —— `r.get(0)` |
| 183 | `clippy::len_zero` —— `rows.len() >= 1` |

**归属判定**：该文件**不在本批次的改动面里**
（`git diff --name-only` 只有 `crates/storage/src/file_storage.rs`
与 `crates/storage/src/wal_storage.rs` 两个源文件），
lint 结果不依赖本批次 ⇒ **基线既存红**，非本批次引入。

**但它暴露了一个真实的门禁覆盖缺口**：
CI 的 clippy 门禁是

| workflow | 命令 | 覆盖范围 |
|---|---|---|
| `.github/workflows/ci-pr.yml:63` | `cargo clippy --all-features -- -D warnings` | 无 `--all-targets` ⇒ **不检查任何测试代码** |
| `.github/workflows/regression.yml:346` | `cargo clippy --all-targets -- -D warnings` | 根 `Cargo.toml` 既是 `[package]` 又是 `[workspace]` 根，且**没有 `default-members`** ⇒ 默认成员**只有根包** ⇒ `crates/storage/tests/*` **不被检查** |

⇒ 存储层测试代码里的 lint 错误**永远不会被 CI 拦住**。
已单独立 issue 跟踪（见 §10）。

⇒ 本批次因此把 clippy 门禁**收窄到自己的改动面**
（`--lib` + 两个新测试目标），并把 `--all-targets` 的基线红**单独记账**，
而不是把它算成「本批次通过」。

### 存储层全量里那两个未跑的目标

`crash_commit_fsync_b1` 与 `crash_wal_index_b1` 需要 **server 二进制**：
测试从 `<workspace>/target/debug/sqlrustgo-mysql-server` 解析路径（#5500 的修复），
而本批次的编译用的是独立的 `CARGO_TARGET_DIR`，那个路径下没有该二进制 ⇒
两个目标报 `server binary not found` ⇒ **NOT-RUN（环境阻塞，受控原因码：server-binary-missing）**，
**不是代码红**。

补跑（用 `SQLRUSTGO_SERVER_BIN` 指向已构建的二进制）：

| 目标 | 结果 |
|---|---|
| `crash_commit_fsync_b1` | **2 passed / 0 failed** |
| `crash_wal_index_b1` | `index_update_crash_does_not_serve_dangling_rows` **FAILED** —— **基线既存红**，已用基线对照判定，见 §9 |

⇒ `crash_wal_index_b1` 的红灯**已与基线对照**；判定见 §9。


---

## 9. `crash_wal_index_b1` 红灯的归属判定（基线对照已完成）

该用例**不含任何 ROLLBACK**：它建库、建表、插行、建二级索引，
在 `AfterIndexUpdate` 故障点被 `SIGKILL`，重启后比较
「全表扫描的 SUM(id)」与「走索引的 SUM(id)」是否一致。

本批次改的是**回滚路径**，与该用例的路径无交集；但**基线对照仍是唯一硬证据**，
所以做了完整对照：还原基线源码 → 重建 server 二进制 → 跑 → 还原修复版 → 重建 → 再跑。

| 案例 | server 二进制来自 | 结果 |
|---|---|---|
| **BASE** | `0dea6393c821`（本批次之前） | `index_update_crash_does_not_serve_dangling_rows` **FAILED**，`crash_wal_index_b1.rs:164:25`，**481.09s** |
| **FIXED** | `42077e8e2e`（本批次修复后） | `index_update_crash_does_not_serve_dangling_rows` **FAILED**，`crash_wal_index_b1.rs:164:25`，**481.09s** |

⇒ **逐字相同**（同一断言行、同一耗时）⇒ 该红灯是**基线既存红**，
**不是本批次引入**。

### 红灯的真实原因（不是崩溃恢复逻辑）

失败信息不是「扫描与索引不一致」，而是**种子 SQL 本身就没跑成**：

```
seed sql: script failed: CREATE DATABASE d1; USE d1; CREATE TABLE t(...); ... CREATE INDEX idx_v ON t(v);
stderr=--------------
CREATE INDEX idx_v ON t(v)
--------------

ERROR 1146 (42000) at line 1: Table not found: t
```

⇒ `CREATE INDEX` 在**命名库**下找不到表 ⇒ **#5377**
（`create_index` 没有 `_in_db` 变体：命名库的索引既建不了也查不到）的症状。

⇒ 也就是说：**这个用例从来没走到它声称要验证的崩溃恢复断言**，
它在准备阶段就死了。这与 #5376 记录的
「崩溃测试 harness：SIGKILL 落在边界之后 —— 崩溃测试没有证明它声称证明的事」
是同一族问题。

> 附：该用例天然很慢（**481s**）。故障点 `reach()` 会**自旋 120 秒**
> 等外部 SIGKILL（`crates/storage/src/fault_injection.rs`），
> 加上重启与 60 秒启动超时，单用例 ≈ 8 分钟。
> 对照脚本因此给看门狗 **900 秒** —— 第一版给 300 秒时，
> 两个案例都被误判成「挂住」，差点把一个**可判定的对照**变成不可判定。
> 记录在此，是因为「工具的边界设窄了」和「代码有 bug」会长得一模一样。

---


1. **没有把 `discard_all_buffers_shared` 实现成 blanket 清空** ——
   那会直接违反 `verify_4957_rollback_buffer_test.rs` 钉住的隔离契约。
   本 PR 让它继续是 no-op，并在代码里写明「任何实现必须是**逐事务**的」。
2. **没有动空闲连接回收器**（#5464）—— 另一个批次。
3. **没有动 `MemoryStorage` 的提交快照机制**（#5507）—— 另一个批次。

---

## 10. 引用

- `crates/storage/src/file_storage.rs` — `discard_all_buffers` / `discard_all_buffers_shared` / `rollback_transaction_lockfree_for` / `undo_transaction_in_place`
- `crates/storage/src/wal_storage.rs` — `rollback_transaction`（固有 + trait）
- `crates/mysql-server/tests/verify_4957_rollback_buffer_test.rs` — 隔离契约的既有钉子
- `tests/integration/stress/process_kill_crash_test.rs` — #5480 原始用例
- **#5508**（本批次新立）— CI 的 clippy 门禁不覆盖 `crates/*` 的测试目标
