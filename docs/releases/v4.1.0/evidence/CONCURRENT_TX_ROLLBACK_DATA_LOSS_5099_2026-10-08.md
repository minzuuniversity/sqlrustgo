# #5099 — 并发事务静默丢行：根因、部分修复与残留

- **Issue**: #5099（重构 P0：事务上下文从实例级状态改为 per-connection）
- **Date**: 2026-10-08（**2026-10-08 更正**：初版结论「已修复」不成立，见 §4.1）
- **Base**: `develop/v4.1.0` = `9083e82a10`
- **PR**: #5132（merged as `b06be91934`）
- **Severity**: P0 — 静默数据丢失，无任何错误信号
- **状态**: 🟢 **服务器卡死已修复**（2026-10-09，TLS 读路径空转，见
  `TLS_READ_SPIN_HANG_5099_2026-10-09.md`）—— 1h SOAK 跑满 3602s / 0 FATAL。
  丢行普查固定 seed 20 次重复全 0。⚠️ 仍遗留 `COUNT(*)` 与范围扫描返回 0
  的 P0 缺陷（见 §4.5），本次统计已改用逐 id 点查绕开。

---

## 1. 症状

1h SOAK（`SOAK_WORKLOAD=oltp_read_write`）在 **60 秒**内中断：

```
[22:47:54] prepare 完成（10000 rows ✓）
[22:48:00] FATAL: mysql_drv_query() returned error 1062 (Duplicate entry '5024' for key 'PRIMARY')
```

进一步实测：8000 并发事务下 **200 行的表只剩约 91 行**（55%），而**每一次
`ROLLBACK` 都返回成功**。

## 2. 根因：身份被「推断」而非「传递」

> **tx_id 是唯一的，但事务控制接口仍然通过共享的 `current_tx_id` 槽位携带它。**

该槽位位于**所有连接共享的单个 `FileStorage`** 上。因此任何问「我是哪个事务」
的调用，答案都是**最后写入的那个连接**。

### 2.1 唯一 id 本来就存在

`crates/transaction/src/transaction_manager.rs:73,125`：

```rust
static NEXT_TX_ID: AtomicU64 = AtomicU64::new(1);
let tx_id = TxId::new(NEXT_TX_ID.fetch_add(1, Ordering::Relaxed));
```

进程级全局分配，**全局唯一**。引擎也确实把它传到了存储层
（`begin_transaction_lockfree(tx_id)`、`set_current_tx_id(id)`）。

丢失发生在**回读**：事务控制方法不接受 id 参数，而是回头去读那个共享槽位。

### 2.2 那条撑起整个失效的错误注释

`ExecutionEngine::commit_transaction` 原本写着：

```rust
// Re-assert our tx id via the shared setter: no writer can
// interleave under this read guard, so the lockfree promote's
// capture inside sees OUR id rather than a stomped slot.
```

**这是错的。** `parking_lot::RwLock` 允许多个并发读者 —— 对端连接可以在那次
重断言与 `commit_transaction_lockfree` 回读之间覆盖那个原子槽。

**重断言只缩小了窗口，并没有关闭它。** 代码看起来是安全的，行为却不是。

### 2.3 三个受影响的判断点

| 位置 | 问题 |
|---|---|
| `MvccStorage::commit_transaction_lockfree` | `promote_pending_for(tx_id)` 的 id 来自回读共享槽 → 提交了错误的连接的行 |
| `commit_transaction_lockfree` / `rollback_transaction_lockfree`（全部后端） | 无 tx_id 参数，内部读共享槽 |
| `crates/executor/src/trigger.rs` | 用 `in_transaction()` 判断「是否有外层事务」，而它问的是「是否有**任何**事务打开」 |

第三条的独立后果：对端开着事务时，本连接会误以为有外层事务、跳过自己的事务，
导致**触发器 DML 失去原子性**。

## 3. 修复

### 存储 trait（`crates/storage/src/engine.rs`）

新增显式 id 接口，全部默认回落旧行为，**任何后端与测试都不被破坏**：

```
commit_transaction_for(tx_id)              rollback_transaction_for(tx_id)
commit_transaction_lockfree_for(tx_id)     rollback_transaction_lockfree_for(tx_id)
is_transaction_active(tx_id)
```

### 后端

`WalStorage` / `MvccStorage` / `FileStorage` 改为**接收 id 参数**，不再回读共享槽。

### 引擎（`src/execution_engine_methods.rs`）

5 处事务控制调用点全部传 `TxSession.current_tx_id` —— 这是引擎本就拥有的
每连接状态。

### Trigger（`crates/executor/src/trigger.rs` + `src/engine_dml.rs`）

`TriggerExecutor` 新增 `outer_tx_id`，由引擎在 3 个 DML 入口设置，
并改用 `is_transaction_active(outer_tx_id)` 判断。

### 未动的字段

`dirty_tables` 保持存储全局 —— 它跟踪「哪些表待 flush」，本就是存储全局事实。
把它搬进 per-connection 是错的。

## 4. 实测

真实服务器二进制（`sqlrustgo-mysql-server`），pymysql 客户端走完整链路
`handle_connection → do_command_loop → ExecutionEngine → WalStorage →
MvccStorage → FileStorage`。

负载：8 线程 x 200 次 `BEGIN; DELETE id=N; INSERT id=N`，共 5 轮 8000 事务
（即 sysbench `oltp_read_write` / `execute_delete_inserts` 的确切形态）。

### 4.1 ⚠️ 更正：本文档初版结论「已修复」是错误的

初版曾记录：

| | ERROR 1062 | 丢失行数 | 每轮 |
|---|---:|---:|---|
| 修复前 | 28 | **323 (40.4%)** | 0.5% ~ 54% 波动 |
| 修复后（初版声称） | **0** | **0** | 5/5 轮均为 0 |

**该「修复后」结论不成立。** 同一二进制（`d7fbf3b475`，08:29 构建）、
同一负载复跑，得到不一致的结果：

| 轮次 | 结果 |
|---|---|
| run A | `errors=0 count=200` |
| run B | `errors=8 count=199` |
| run C（1000 行） | `errors=22 count=996` |

**且 1h SOAK 仍以相同症状失败**：

```
[08:36:18] === SOAK 开始 ===
FATAL: mysql_drv_query() returned error 1062 (Duplicate entry '4976' for key 'PRIMARY')
```

更严重的是，复跑过程中出现**连接丢失与服务器卡死**：

```
pymysql.err.OperationalError: (2013, 'Lost connection to MySQL server during query')
server: 850% CPU / 21 线程空转 / SELECT 1 超时（21s 无响应）
```

该卡死现象与本次 SOAK 排查**最初**的观察一致 —— 当时记为「未能稳定复现，
不作为结论」，现已可复现。

**结论更正：#5099 的修复是真实改进，但不是完整修复。** 丢行率从 40.4% 降到
1% 量级（200 行丢 1~8 行），但 `ERROR 1062` 与偶发服务器卡死仍在。

### 4.2 根因：为什么初版会误判

初版把「5/5 轮全 0」当成了充分证据。**但这个缺陷恰恰以高波动为特征**
（修复前实测丢行率在 0.5%~54% 间跳）。一个 5 轮全 0 的样本，
在如此高方差的现象面前不构成证据 —— 同一二进制随后就给出了非零结果。

本次排查中同类教训已出现两次：
- undo 分桶的首轮实现曾「2 passed」，连跑 5 次失败 4 次；
- 本文档初版曾报「5/5 轮全 0」，复跑即出现错误。

**方法论修正**：并发缺陷的验收需要**固定 seed 的多次重复**，
或直接以「长时间 SOAK 不出 FATAL」为准，而非若干轮并发脚本。

### 4.3 逐轮明细（修复前，原始基线）

| 轮次 | 1062 | 行数变化 | 丢失率 |
|---|---:|---|---:|
| rep0 | 4 | 200 → 182 | 9.0% |
| rep1 | 13 | 200 → 199 | 0.5% |
| rep2 | 4 | 200 → 110 | 45.0% |
| rep3 | 2 | 200 → 92 | 54.0% |
| rep4 | 5 | 200 → 94 | 53.0% |

### 4.4 残留（2026-10-09 更新）

1. **残留 `ERROR 1062`** —— 2026-10-09 复核：固定 seed 重复 20 次
   （200 行与 1000 行两档，后者 80,000 事务）**均为 0**，未复现。
   本次复核的行数统计改用**逐 id 点查**（见 §4.5），口径比原 `COUNT(*)` 严格。

2. **服务器卡死** —— ✅ **根因已定位并修复**（2026-10-09）：
   `impl Read for TlsStream` 的 `while wants_read()` 循环在 socket 无数据时
   `break`，但 `wants_read()` 在该状态下仍为 true，导致零进展忙循环、
   每 worker 100% CPU（实测 1334%）。**1h SOAK 跑满 3602s，0 FATAL。**
   详见 `TLS_READ_SPIN_HANG_5099_2026-10-09.md`。

### 4.5 ⚠️ 复核口径：不能用 `COUNT(*)` 数行（2026-10-09）

复核时发现本二进制 `SELECT COUNT(*)` / 范围扫描 / `SUM` 聚合**一律返回
0 或 NULL**，点查却正常：

| 查询 | 返回 |
|---|---|
| `SELECT COUNT(*) FROM t` | 0 |
| `SELECT COUNT(*) FROM t WHERE id>=1 AND id<=10` | 0 |
| `SELECT COUNT(*) FROM t WHERE id=2`（点查） | 1 |

**这不是丢行**：10000 行的表，13 个抽样 id 全部命中，差额在 `.delta`
insert buffer 中。用 pristine 二进制复现**结果完全相同**，与 TLS 修复无关。

因此本节所有行数统计一律改用**逐 id 点查**。一个报 0 的 `COUNT(*)`
会让普查脚本永远「通过」——它会像隐藏丢行一样隐藏本缺陷。
oracle 已做反向对照：主动删 37 行，精确报出 37/37。

---

## 5. 现有套件

```
sqlrustgo-storage                       895 passed; 0 failed
blk2_walstorage_no_escape_hatch_test      5 passed
blk2_shared_tx_path_test                  4 passed
integration_savepoint_test                9 passed
mvcc_transaction_test                    27 passed
server_thread_model_guard_test            4 passed
```

两个 BLK-2 门禁的源码文本断言仍然成立：`{*commit,rollback}_transaction_lockfree`
保持 `&self` 签名（该签名是为规避一次真实死锁而存在，见 `engine.rs:1726-1741`
记录的「8 并发读写下整个服务器锁死」），并继续调用 `set_current_tx_id_shared`。

## 6. 已知遗留

`tests/integration/transaction/concurrent_rollback_isolation_test.rs` **仍失败**。

它直接驱动 `FileStorage`，因此从不经过承载 id 的引擎与 trigger 接线 ——
修的是它没走的那条路。该测试的 oracle 已在 PR #5125 修正：原断言让 A、B 都删除
同一个 `id=42` 并要求该行存活，这在逻辑上不可满足（A 提交删除 42、B 回滚删除同一
42 时，「42 存在」与「A 的提交是永久的」互斥；MySQL 靠行锁解决，不保证复活）。

修正后改用不同 key 的无歧义场景，在**未修复**代码上仍 **30/30 失败**，
且两半各自独立失败（已提交 DELETE 被撤销 21/30、已回滚 DELETE 未恢复 9/30）——
证明它测的是真实缺陷而非不可能的期望。

后续项：让该测试复现引擎的调用序列（`set_current_tx_id` + 显式 id 的
commit/rollback），使其真正覆盖本次修复的路径。

## 7. 关联

- Issue #5099 —— 本 issue
- PR #5125 —— 修正回滚隔离测试的 oracle
- PR #5107 —— lockfree undo 按事务作用域（#5098 的遗漏）
- PR #5104 —— 线程模型守卫测试
- `docs/plans/2026-10-07-soak-blocker-report.md` —— 排查记录
- `docs/plans/2026-10-08-issue-5099-tx-context-refactor-design.md` —— 设计（修订版）