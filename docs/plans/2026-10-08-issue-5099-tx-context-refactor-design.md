# #5099 B 方案设计 —— 修订版（2026-10-08）

> **状态**: 设计稿，**未实施**。
> **修订原因**: 第一次实施尝试（undo 日志分桶）**实测无效**，本文档据此重写。
> **取代**: `B_PLAN_DESIGN_5099.md` 初版中「引入 TxContext 贯穿 DML 路径」的分期。

---

## 1. 修订摘要

初版方案的 Phase 1 是「让 TxContext 成为权威来源」，Phase 2 是「让 DML 路径显式携带 TxContext」。

**实施后的事实推翻了这个分期**：

| 初版假设 | 实测结果 |
|---|---|
| undo 按 tx_id 分桶即可解决并发回滚 | ❌ **无效**。测试 5 次跑出 4 次失败 |
| 需要引入新的 `TxContext` 类型并贯穿 DML | ❌ **不必要**。唯一 tx id 已经存在，只是被丢弃 |

**范围因此大幅收缩** —— 见下。

---

## 2. 关键发现：唯一 tx id 已经存在，只是被丢弃

### 2.1 引擎已经发唯一 id

`crates/transaction/src/mvcc.rs:271-282`：

```rust
pub fn begin_transaction(&mut self) -> TxId {
    let tx_id = TxId::new(self.next_tx_id);
    self.next_tx_id += 1;      // ← 每连接单调递增
    ...
}
```

`TransactionManager` 为**每个连接**持有自己的 `mvcc` 实例，因此每个连接拿到的 tx id 天然唯一。

### 2.2 引擎也把它传到了存储层

`src/execution_engine_methods.rs` 的 `begin_transaction`：

```rust
let lockfree_ok = {
    let storage = self.storage.read();
    storage.begin_transaction_lockfree(tx_id.as_u64()).is_ok()   // ← 显式传入
};
```

`WalStorage::begin_transaction_lockfree(tx_id)`（`wal_storage.rs:1034-1056`）原样向下传播：

```rust
self.current_tx_id.store(tx_id, Ordering::Relaxed);
self.inner().set_current_tx_id_shared(tx_id);
```

**且 DML 路径在同一把写锁内重断言该 id**（`engine_dml.rs:492-498`）：

```rust
let mut storage = engine.storage.write();
if let Some(id) = engine.tx_session.lock().current_tx_id {
    storage.set_current_tx_id(id.as_u64());   // ← 与语句同一临界区
}
```

**所以真实服务器路径下 tx 归属是正确的。**

### 2.3 FileStorage 在 `begin_transaction` 丢弃了它

`crates/storage/src/file_storage.rs:5405-5417`：

```rust
let id = self.next_tx_id();                        // ← 分配了
let (tx_id, is_new) = Self::with_write_lock(self, |s| {
    let existing = self.current_tx_id.load(O::Acquire);
    if existing != 0 {
        return Ok((existing, false));              // ← 返回共享 slot，不是 id
    }
    self.current_tx_id.store(id, O::Release);
    ...
});
```

`next_tx_id()` 的返回值被完全忽略。第二个 `BEGIN` 读到 `existing != 0` 便走 nested-BEGIN 分支，返回**别人**的 id。

---

## 3. 为什么 undo 分桶无效（已实测）

初版 Phase 1 的思路是「按 tx_id 分桶，回滚只取自己的桶」。

**实施结果：测试 5 次跑出 4 次失败**（`1 passed; 1 failed` ×3、`0 passed; 2 failed` ×2）。

原因直接来自 §2.3：两个并发事务**拿到的是同一个 id**，于是它们被写进**同一个桶**。桶化对它们没有任何隔离作用 —— 它只能隔离 id 不同的情形，而 id 不同这个前提本身正是被破坏的那一环。

**这解释了为什么「给 undo 加 tx_id 过滤」和「按 tx_id 分桶」两类方案都无效**：它们都把 tx_id 当作可靠的归属标识，而 tx_id 恰恰不可靠。

> 已回退该改动（157 行 diff），未提交。保留此结论以免重复尝试。

---

## 4. 修订后的方案

### 核心：让 `begin_transaction` 尊重调用方的 id

不再需要新类型、不需要贯穿 DML。需要的是：

**步骤 1 — `begin_transaction` 使用自己分配的 id**

移除 `existing != 0 → return existing` 的早退分支对共享 slot 的依赖。nested-BEGIN 语义应由**调用方**（引擎，`TxSession.current_tx_id`）判定，而不是存储层读共享状态。

> ⚠️ 需要保留 MySQL 的 nested-BEGIN 语义（第二个 `BEGIN` 不应新建事务）。但该判定已经在引擎层做过一次（`execution_engine_methods.rs:1596-1621`，`is_explicit_transaction`）。存储层的早退分支是**重复且有害**的 —— 它把并发连接误判为 nested。

**步骤 2 — undo 按该 id 归属**

`tx_undo_log` 改为 `HashMap<u64, Vec<UndoOp>>`（这一步初版已实现，本次复用）。步骤 1 完成后，分桶才真正生效。

**步骤 3 — 验证顺序不可颠倒**

必须**先**步骤 1 **后**步骤 2。若只做步骤 2，测试仍会失败（已验证）。

### 关于 `begin_transaction_lockfree`（原待确认项，已查清）

初版曾假设「生产走 `_lockfree` 路径，`begin_transaction` 只在测试里被调用」。
**该假设错误。** 裸 `begin_transaction()` 存在多个生产调用者：

| 调用点 | 是否 tx 相关 |
|---|---|
| `src/execution_engine_methods.rs:1680` | 是，且调用前已 `set_current_tx_id` |
| `crates/executor/src/trigger.rs:341` | **是** —— 触发器 DML 的自动事务边界 |
| `crates/executor/src/transactional_executor.rs:108` | 是 |
| `crates/server/src/openclaw_endpoints.rs:1978` | 是 |

其中 `trigger.rs:339-341` 尤其关键：

```rust
let in_outer_tx = storage.in_transaction();   // ← 读共享 slot
if !in_outer_tx {
    storage.begin_transaction()?;
}
```

触发器用自己的 DML 决定是否开事务，而判断依据 `in_transaction()` 读的正是
那个被并发覆盖的共享 slot。**并发下这里会误判** —— 一个连接以为自己在事务里
（因而不开新事务），实际它的 id 已被别人抢走。

**结论：修复必须落在 `begin_transaction` 与 `in_transaction` 的语义上，
不能只改 `_lockfree` 变体。** 这也意味着范围比初版设想的 §2 更靠近存储层入口。

---

## 5. 必须尊重的既有约束

### 5.1 BLK-2 门禁（`tests/blk2_walstorage_no_escape_hatch_test.rs`）

```rust
assert!(shared_tx_calls >= 3,
    "expected all three {{begin,commit,rollback}}_transaction_lockfree to call
     set_current_tx_id_shared, found {shared_tx_calls}");
```

这是**源码文本断言** —— 扫方法体找字符串。若修复涉及改写这些方法体会打破它，且失败信息无诊断价值。

### 5.2 `&self` 签名背后的死锁教训

`crates/storage/src/engine.rs:1726-1741` 记录：`*_transaction_lockfree` 声明为 `&self` 是为了避开全局 `Arc<RwLock<Storage>>` 写锁；从读守卫里伪造 `&mut` 曾导致**整个服务器在 8 并发读写下死锁**。

**含义**：`&self` 上拿不到"这是哪个连接"。若修复需要引入连接身份，必须走显式参数（`set_current_tx_id_shared` 的模式），**不能**改为 `&mut`。

### 5.3 字段处置（承接初版，仍然成立）

| 字段 | 处置 |
|---|---|
| `current_tx_id` | 修复目标是让它不再被并发覆盖 |
| `current_db` | 已由 #5025 修好，作为范式 |
| `tx_undo_log` | 按 tx_id 归属 |
| `dirty_tables` | **保持实例级** —— 存储全局事实，搬进 per-connection 是错的 |
| `insert_buffer` | 可留实例级，但回滚须只撤销本事务的缓冲行 |

---

## 6. 验收（承接 #5099，基线见 issue）

- [ ] `concurrent_rollback_isolation_test` 达 **0/30**，且**连续 5 次运行全绿**
- [ ] 真实服务器 8000 事务复跑：**0 丢行、0 个 1062**（基线 323 丢行 / 28 个 1062）
- [ ] 复跑**每轮均为 0** —— 基线丢行率波动 0.5%~54%，单轮不作数
- [ ] storage 894 + savepoint 9 + mvcc 27 + 两个 BLK-2 门禁 + 线程模型守卫 4 全绿
- [ ] 不得以 thread_local 结项

### 方法论约束（本次教训）

- **单次绿色结果不是证据**。本次分桶实现首次运行 2 passed，连跑 5 次却失败 4 次。
- **必须用真实服务器复跑**。单元测试直接驱动 `FileStorage`，比生产更严苛，不能单独作准。
- **改动无效时回退，不要提交**。分桶改动 157 行、零改善，提交只会增加 review 负担。

---

## 7. 待确认

1. ~~`begin_transaction` 在生产中是否仍有实际调用者？~~ **已查清：有 4 处**，
   含 `trigger.rs` 的事务边界判断。见 §4「关于 `begin_transaction_lockfree`」。
   → 修复落点确定为存储层入口（`begin_transaction` + `in_transaction`），
   而非仅 `_lockfree` 变体。
2. **BLK-2 门禁是否随修复调整？** 若修复触及 `_lockfree` 三个方法体，需决定
   是保留文本断言（加兼容调用点）还是改为行为断言（推荐，但需单独 PR 说明）。
3. **`in_transaction()` 的语义**：它当前读共享 slot，因此天然无法表达"我这个
   连接在事务里"。修复时需明确它应表达**调用方**的事务状态 —— 这决定了
   `trigger.rs` 的判断是否会一并被修正。