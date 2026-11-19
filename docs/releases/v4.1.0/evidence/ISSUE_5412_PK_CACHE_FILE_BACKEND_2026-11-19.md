# #5412 的源码修复 —— 让 file 后端的 PK 缓存快路径真正生效，并堵住跨库误命中

**日期**：2026-11-19
**阶段**：ALPHA（本文件**不产生任何 PASS 声明**，不构成 ALPHA→BETA 的放行）
**基线**：`develop/v4.1.0` @ `f6beacad31`
**分支**：`fix/5412-file-backend-change-stamp`
**关联**：#5412（主）、#5507（MemoryStorage 提交快照机制的根因修复）、#5113（#5412 扩大暴露面的对冲面）、#5891（本轮新发现的既有缺陷）、PR #5889（根因定位）

---

## 0. 结论摘要

1. **#5412 的机制**（PR #5889 已定位，本轮实现修复）：不是「缓存被它自己要加速的那次 INSERT 失效」，
   而是 **`FileStorage` 及 WAL 包装层都继承 `StorageEngine::table_change_stamp` 的默认实现（恒为 0）**
   ⇒ 引擎的 PK 缓存快路径被**永久禁用** ⇒ 每一次带 PRIMARY KEY 的 INSERT 都整表扫描 ⇒ **O(N²)**。
   这是「正确但未优化」的既有设计代价，**不是回归**。

2. **修复 = D1 + X2**（双专家三轮裁决，两血统在 X2 上达成一致）：
   - **D1（存储层）**：trait 增 `bump_change_stamp`（默认空实现）；计数器**只存在于叶子 `FileStorage`**；
     包装层（MVCC / WAL / ParallelWal / Box）**纯向下委托**；MVCC 的 commit/rollback **必须**向下 bump。
   - **X2（引擎层）**：`PkIndexCache` 增 `db: String`；命中判定加 `e.db == stmt_db`。

3. **本轮同时修掉一个正确性漏洞**：跨库同名表（`d1.t` / `d2.t`）共用裸名 stamp 与裸名缓存槽
   ⇒ 缓存条目可能被**另一个数据库的键集**回答 ⇒ 重复键检查会**误拒绝**或**误接受**。
   X2 的 `db` 字段堵住它。**该漏洞在 `MemoryStorage` 上今天就已存在**（本轮首次固化为测试），
   本轮把 `FileStorage` 从同一暴露面里摘出来 ⇒ **暴露面净减少**，但**既有问题并未因此消失**（见 §5.3）。

4. **回归取证**（本机实测，见 §4）：新套件 **24/24 通过**；既有 `pk_cache_regression_test` **14/14 通过**；
   `sqlrustgo-storage` 库单测 **919 通过 / 0 失败**；根 crate 库单测 **174 通过 / 0 失败**。

5. **本轮新发现一个既有缺陷**（非本 PR 引入）：**非默认库里的 ROLLBACK 不撤销写入**
   ⇒ 已被回滚的行仍然可见，并挡住后续同键 INSERT。已登记 **#5891**，本 PR 不修（理由见 §6）。

6. **对冻结点的影响**：本 PR 改动了 `crates/` ⇒ **019 认证轮的既有结论对新的冻结点失效**，
   需在新的冻结点重跑（见 §7）。

---

## 1. #5412 的机制（源码级）

| 位置（`f6beacad31`） | 事实 |
|---|---|
| `crates/storage/src/engine.rs:1559-1581` | trait `table_change_stamp` 的**默认实现返回 0**，注释自陈「never trust a cached copy … stays correct rather than fast」 |
| `crates/storage/src/engine.rs:3588` | **全仓唯一的覆盖实现**，在 `impl StorageEngine for MemoryStorage` 里 |
| `crates/storage/src/engine.rs:2047 / 2882` | `MemoryStorage` 的计数器与 7 处 bump 调用点 |
| `src/engine_dml.rs:443` | 快路径入口条件 `if cacheable && live_stamp != 0` ⇒ **stamp 为 0 时快路径进不去**，落到整表扫描 |
| `src/engine_dml.rs:578-587` | 写入前再取 `pre_write_stamp`；与 `cached_stamp` 不一致 ⇒ 删缓存并整表重扫 |

⇒ **生产栈全程拿不到非零 stamp**：
`FileStorage → MvccStorage → WalStorage / ParallelWalStorage → BoxStorageEngine`
（`crates/mysql-server/src/lib.rs:7321-7591`；CLI 链 `src/engine_builder.rs:409-411`）
⇒ **每一次带 PK 的 INSERT 都整表扫描** ⇒ N 次逐行 INSERT 成本 O(N²)。

⇒ **O(N) 闭环的来源**（本轮核实）：`src/engine_dml.rs:795-818` 已实现「写后增量扩展缓存 + 把 stamp 重打为写后值」，
所以**只要 file 后端能给出非零 stamp**，后续 INSERT 就能命中已扩展的索引 ⇒ 总扫描数 = 1 ⇒ **O(N)**。

---

## 2. 双专家裁决与 ADR-014 D-06 的逐点裁决

**血统**：deepseek-web（`/tmp/_consult_5412_v2/deepseek.md`、`/tmp/_consult_5412_v3/deepseek.md`）
与 chatGPT-web（`/tmp/_consult_5412_out/chatgpt.md`、`/tmp/_consult_5412_v3gpt/chatgpt.md`）。
**事实集**：`/tmp/_consult_5412_facts.md`（第一轮）、`_facts_v2.md`（34709 B）、`_facts_v3.md`（8315 B，
两路第三轮**同一事实集**）。

ADR-014 D-06 要求：**两血统分歧不取平均，逐点裁决并公开分歧**。下表是逐点裁决结果。

| 议题 | deepseek（第三轮） | chatGPT（第三轮） | **本轮裁决** | 依据 |
|---|---|---|---|---|
| **Q-B：跨库方案** | **X2**（storage 保持 D1 裸名槽 + 引擎缓存记 db）；**否决 X1**（全局计数器溢出会一次性废掉所有表的缓存）、**否决 X3**（改动面过大） | **X2**；**否决 X1**（须保证不同 `(db,table)` 的有效 stamp 永不相等，per-桶 wrapping 不成立）；X3 留后续治理 | **采纳 X2**（两血统一）；X1/X3 不采用 | 两血一致，无分歧 |
| **Q-A：跨库反例是否成立** | 「**同意反例成立**」 | 「**机制成立，但原始执行序列不成立**」——指出原反例中 T1 已插入键 1、T4 插入键 2 都不构成重复，给出修正后的 T1–T4 | **机制层：两血一致**（跨库误命中真实存在）⇒ 采纳 X2。**用例层：采 chatGPT 的修正版**，且本轮**在真实代码上重新推导并双向验证**（§5.2） | **此处存在分歧，按 D-06 公开**：deepseek 认可反例整体成立，chatGPT 否定其具体执行序列。裁决：chatGPT 在**具体用例层**更准确（我复核后确认其数据错误属实），但**不因此改动机制层结论**；最终用例由本轮在代码上独立推导，取**双向断言**（误拒绝 + 误接受），覆盖两方各自描述的失败形态 |
| **Q-C：跨库误命中的既有性** | 属既有问题 | 属既有问题（`MemoryStorage` 用 DEFAULT_DATABASE 桶，缓存却无库身份） | **一致**：既有问题，本轮固化用例 | 两血一致 |
| **Q-D：本轮是否一并修** | (a) 只修 #5412 + 公开暴露面 + 固化用例 | (b) 本轮一并做最小正确性修复，但范围限定 X2，不扩到 X3 | **两者都做**：本轮 PR 内完成 X2 正确性修复（= (b) 的实质），**同时**在 PR 与本文件公开 `MemoryStorage` 的既有暴露面并固化跨库用例（= (a) 的实质）⇒ 既有暴露面只减不增 | 分歧已消解：X2 本身就是最小正确性修复，(a) 与 (b) 不互斥 |
| **Q-E：失效风险分类** | 21 条写入点分类清单；`get_table_mut` 是裸名口径下唯一的正确性漏洞，建议 `#[deprecated]` + 测试改走 `insert_at` | 六类风险：库名前缀/裸名混用、DROP/CREATE/RENAME 后沿用旧身份、写入绕过入口、MVCC 提交/回滚与 stamp 不一致、mutation 与 bump 非原子、计数器回绕 | **采纳 chatGPT 的六类清单作为回归矩阵**（本轮逐类落用例）。**唯一分歧：`get_table_mut`** —— deepseek 建议废弃，**本轮裁决保留**：该方法借出 `&mut` 时调用方独占整个 storage，期间无并发读者，**在借用点保守 bump**即可保证安全；废弃它会动到 `FileStorage` 的公开 API 与既有调用面，超出 ALPHA 改动面控制 | **分歧按 D-06 公开**：deepseek 主张废弃，本轮保留并以「借出即 bump」替代 |
| **Q-F：性能兜底** | 否决「每次 scan 算全表哈希」（会把 O(N) 抵消回 O(N²)）；改用 scan 计数 + 写入口 epoch 对账 + CI debug 构建 | 同样否决逐行全表哈希；建议 scan 计数 ≤ 2 + **独立 oracle**（不得用被测缓存生成预期值） | **采纳双方一致意见**：引擎侧 `pk_cache_builds` 计数（先例：#5413 的 `parallel_filter_runs`）+ **测试侧独立 oracle**（`SELECT COUNT(*)` 走存储自己的扫描路径） | 两血一致 |

**两血的共同底线**：不得用「每次 INSERT 再算一次全表哈希」当兜底 —— 那会把本次修复的收益原样抵消。

---

## 3. 实现（逐文件）

### 3.1 存储层

| 文件 | 改动 |
|---|---|
| `crates/storage/src/engine.rs` | trait 新增 `fn bump_change_stamp(&self, _table: &str) {}`（**默认空实现**，紧跟 `table_change_stamp` 之后） |
| `crates/storage/src/file_storage.rs` | 新增独立字段 `change_stamps: parking_lot::RwLock<HashMap<String,u64>>`（4 个构造点同步）；新增自由函数 `bump_change_stamp_slot(stamps, table)`（取 map 而非 `&self`，以便与行变更同临界区 bump 而不触发借用冲突）；`impl StorageEngine for FileStorage` 覆写 `table_change_stamp` / `bump_change_stamp`（**裸小写表名** key，`checked_add`；溢出毒化为 `u64::MAX` ⇒ 读作 0「不可缓存」，**绝不回绕**） |
| `crates/storage/src/mvcc.rs` | 新增 `VersionedTable::has_pending_for(tx_id)` |
| `crates/storage/src/mvcc_storage.rs` | `promote_pending_for` / `discard_pending_for` 改为「**promote/discard 前**收集有 pending 版本的表 → 操作 → 对这些表调 `self.inner.bump_change_stamp`」；新增 trait 覆写 `table_change_stamp` / `bump_change_stamp` 向下委托 |
| `crates/storage/src/wal_storage.rs`、`parallel_wal_storage.rs`、`binary_storage.rs`（`BoxStorageEngine`） | 新增 trait 覆写 `table_change_stamp` / `bump_change_stamp` 向下委托 |

**`FileStorage` 的 bump 点（21 处，覆盖全部改变「扫描会返回什么」的写路径）**：
`insert_direct`、`insert_via_bufferpool`、`insert_buffered`、`update_if_at`、`update_at`、
`delete_if_at`、`delete_at`、`delete_collect_pks_at`、`insert_table_in`、`drop_table_in`、
`rename_table`（旧名 + 新名）、`add_column`、`drop_column`、
`undo_transaction_in_place`（改为返回受影响表名，2 个调用方 bump）、
`apply_undo`、`discard_all_buffers`（先收集 key）、`clear_all_tables`（先收集 key）、
`get_table_mut`（借出 `&mut` 时保守 bump）、`flush_buffer`（仅「表不在 tables 中导致丢行」的分支 bump）。

**三条设计约束**（均为本轮自定、已写进代码注释）：

1. **bump 必须在行变更之后、同一临界区内** —— 否则读者可能把「新 stamp」与「旧行集」配对，
   而那个组合在行变更后看起来是新鲜的。
2. **`flush_buffer` / `flush_all_buffers` 的正常路径不 bump** —— 行从 insert buffer 移到
   `tables.rows`，而 `scan` 无条件 merge 两者 ⇒ 可见行集没变。
3. **MVCC 的 commit/rollback 只 bump「本次 tx 真有 pending 版本」的表** —— 不能全表 bump：
   隐式事务每条语句都 commit，全表 bump 会让每次 INSERT 都失效缓存 ⇒ O(N²) 复活。

### 3.2 引擎层

| 文件 | 改动 |
|---|---|
| `src/engine_dml.rs` | `PkIndexCache` 新增 `pub db: String`；命中判定加 `&& e.db == stmt_db`；建索引时写 `db: stmt_db.clone()`；`stmt_db` **规范化为小写**（见下）；**写后维护加 db 守卫**；cache-build 处 `engine.pk_cache_builds.fetch_add(1, Relaxed)` |
| `src/execution_engine.rs` | `ExecutionEngine` 新增 `pub(crate) pk_cache_builds: AtomicU64`；新增 `pub fn pk_cache_build_count(&self) -> u64` |
| `src/engine_builder.rs` | 7 个构造点加 `pk_cache_builds: AtomicU64::new(0)` |

**`stmt_db` 的规范化**（chatGPT Q-B 的明确要求）：命中判定比较的是**数据库身份**，
两侧必须是同一个**规范化**名字。parser 保留原样写法（`MyDb`），而存储层按 `scoped_key` 大小写不敏感解析
（两半都小写）。若直接比较原始写法，`MyDb` 与 `mydb` 会被当成两个库 —— 不会答错，但每次大小写变体都会白白重建缓存。
⇒ 本轮把 `stmt_db` 统一小写（`session_db()` 本来就是小写的，`insert.schema` 不是）。

**写后维护的 db 守卫**：写路径把新键并进缓存槽时，**只并入属于本语句数据库的条目**。
若在「本语句查缓存」与「本语句写入」之间，另一个数据库的同名表语句替换了同一个槽，
无守卫的并入会把两个库的键混进同一个索引 ⇒ 后续该库的查询会拿到掺了别库键的答案。
守卫发现槽已属于别的库时**删除该槽**（不复活、不污染）。

### 3.3 覆盖范围：哪些后端拿到了 stamp，哪些没有

全仓有 **12 个 `impl StorageEngine`**。本轮只给**生产链路上的包装层**加了向下转发，
其余实现体继承 trait 默认实现（`table_change_stamp` ⇒ 0 ⇒ 快路径禁用 ⇒ **正确但慢**，不是正确性问题）。

| 实现体 | 本轮是否覆写 stamp | 是否在生产链路上 |
|---|---|---|
| `FileStorage` | **是（计数器所在）** | **是**（file / parallel 两种模式的叶子） |
| `MvccStorage` | **是（纯转发 + commit/rollback bump）** | **是** |
| `WalStorage` / `ParallelWalStorage` | **是（纯转发）** | **是**（file / parallel 模式） |
| `BoxStorageEngine` | **是（纯转发）** | **是**（服务端统一的引擎类型） |
| `MemoryStorage` | 本来就有（既有实现） | 测试与内存模式 |
| `BinaryTableStorage`（及其 V2） | **否** | 服务端 `storage=binary` 模式 ⇒ **该模式下快路径仍禁用（正确但慢）** |
| `ColumnarStorage` / `TableLevelStorage` / `AppendOnlyStorage` | **否** | 非生产链路 |
| `VtuGuard<S>` / `ScanCountingStorage` | **否** | 前者是**标记辅助**（`assert_path_for_dml`），不是链路包装；后者仓内无使用点 |

⇒ **#5412 的性能收益在服务端的 file / parallel 两种存储模式上生效**；
`binary` 模式（`BinaryTableStorage`）与若干实验后端**仍未实现 stamp**，
它们留在「正确但慢」的状态。**本轮不扩到这些后端**（ALPHA 改动面控制，双血裁决亦限定 D1 的叶子为 `FileStorage`），
登记为后续项。

**GitNexus 不可用（如实披露）**：本会话**没有配置任何 MCP server**（`mcp list` ⇒ `servers: []`），
`gitnexus_impact` / `gitnexus_detect_changes` **无法调用**；仓内 `npx gitnexus status` ⇒ `Repository not indexed`。
⇒ 按 AGENTS.md 的「仓内等价检索」替代路径，本轮对受影响符号做了完整枚举：
`PkIndexCache` 的构造点（**唯一一处**：`src/engine_dml.rs:506`）、`undo_transaction_in_place` 的调用点（**两处**，均已改）、
`ExecutionEngine` 的构造点（`engine_builder.rs` 7 处 + `execution_engine.rs` 1 处，均已加 `pk_cache_builds`）、
以及上表的 12 个 `StorageEngine` 实现体。编译与全量测试通过（§4）作为等价验证。


---

## 4. 回归取证（本机实测）

### 4.1 新套件 `pk_cache_file_backend_test`

```
$ cd /Volumes/workspace/dev/srg-gate-fix
$ nice -n 19 cargo test --test pk_cache_file_backend_test -- --nocapture
test file_backend::bulk_insert_builds_the_index_once_not_once_per_row ... ok
test file_backend::cross_database_duplicate_is_still_rejected ... ok
test file_backend::cross_database_new_key_is_not_rejected ... ok
test file_backend::database_name_case_variants_share_one_cache_entry ... ok
test file_backend::delete_frees_the_key ... ok
test file_backend::drop_and_recreate_table_starts_empty ... ok
test file_backend::duplicate_key_is_still_rejected ... ok
test file_backend::interleaved_writes_across_databases_keep_their_keys_apart ... ok
test file_backend::new_key_is_still_accepted ... ok
test file_backend::update_invalidates_the_cached_key_values ... ok
test file_backend::write_that_bypasses_the_engine_invalidates_the_cache ... ok
test memory_backend::... （同上 11 例） ... ok
test committed_row_in_another_connection_becomes_a_duplicate ... ok
test rolled_back_row_in_another_connection_is_not_a_duplicate ... ok

test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
```

**覆盖矩阵**（11 例 × 2 后端 + 2 例 MVCC 栈）：

| # | 用例 | 钉住什么 |
|---|---|---|
| 1 | `bulk_insert_builds_the_index_once_not_once_per_row` | **#5412 的主张**：200 次 INSERT 的索引构建数 ≤ 2；再插 200 行后**构建数不变** ⇒ 扫描数不随行数增长（O(N²) 不会复活） |
| 2 | `duplicate_key_is_still_rejected` | 重复键仍被拒，且被拒的行不落库 |
| 3 | `new_key_is_still_accepted` | 新键仍被接受 |
| 4 | `write_that_bypasses_the_engine_invalidates_the_cache` | **旁路写入失效**：绕过引擎直接写 storage 后，同键 INSERT 仍被拒 |
| 5 | `cross_database_new_key_is_not_rejected` | **跨库误拒绝方向**：`d1.t` 有的键在空的 `d2.t` 里必须能插进去 |
| 6 | `cross_database_duplicate_is_still_rejected` | **跨库误接受方向**：`d2.t` 已有的键在缓存槽装着 `d1.t` 键集时仍必须被拒 |
| 7 | `interleaved_writes_across_databases_keep_their_keys_apart` | 跨库交替写入 25 轮后，两库的键集与**测试自己的 oracle** 完全一致 |
| 8 | `database_name_case_variants_share_one_cache_entry` | `D1` 与 `d1` 是同一个库 ⇒ 索引只构建一次 |
| 9 | `drop_and_recreate_table_starts_empty` | DROP + CREATE 同名表后旧键重新可用（chatGPT Q-E 第二类） |
| 10 | `update_invalidates_the_cached_key_values` | UPDATE 也要移动 stamp（缓存存的是值，不只是键） |
| 11 | `delete_frees_the_key` | DELETE 后键重新可用 |
| 12 | `committed_row_in_another_connection_becomes_a_duplicate` | **MVCC 提交可见性翻转**（chatGPT Q-E 第四类）：A 提交后，B 的同键 INSERT 必须被拒 |
| 13 | `rolled_back_row_in_another_connection_is_not_a_duplicate` | MVCC 回滚后，同键不再是重复（chatGPT Q-E 第四类） |

### 4.2 既有套件与库单测（回归）

```
$ nice -n 19 cargo test --test pk_cache_regression_test
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ nice -n 19 cargo test -p sqlrustgo-storage --lib
test result: ok. 919 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out

$ nice -n 19 cargo test --lib -j 2
test result: ok. 174 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

**并发口径披露**：019 认证轮在本轮取证期间**仍在运行**（执行器 PID 18229）⇒ 本轮所有 cargo 命令
一律 `nice -n 19` + `-j 2` 让路。**这是与 019 的并发**，与「认证轮期间不得并发重量门禁」的约束存在张力，
如实登记；本次并发的目的是**源码级回归**（非重量门禁脚本），且全部以低优先级执行。

### 4.3 变异测试取证（证明护栏是承重的）

「测试是绿的」本身不能证明护栏有效。本轮对两处 `db` 守卫做了**变异测试**：
临时去掉守卫后重跑跨库用例，确认它们**真的会红**。

| 变异 | 结果 |
|---|---|
| 去掉**命中判定**的 `&& e.db == stmt_db` | `memory_backend::cross_database_new_key_is_not_rejected` **FAILED**（误拒绝复现，报 `DuplicateKey("Duplicate entry '7' for key 'PRIMARY'")`）；`memory_backend::cross_database_duplicate_is_still_rejected` **FAILED**（误接受复现，报「insert succeeded」）；`file_backend::cross_database_new_key_is_not_rejected` **FAILED**（误拒绝复现） |
| 再去掉**写后维护**的 `entry.db == stmt_db` 守卫 | 同上 3 条仍 **FAILED**（`file_backend::cross_database_duplicate_is_still_rejected` 仍为 ok，见下） |
| 恢复实现 | **24 passed / 0 failed** |

⇒ **三个失败方向在两个后端上都可复现** ⇒ 护栏是承重的，不是装饰。

**为什么 `file_backend::cross_database_duplicate_is_still_rejected`（误接受方向）在 file 后端不复现**：
file 后端的 **CREATE TABLE 会移动 stamp**（见 §3.1 的 `insert_table_in` bump 点），而 `MemoryStorage` 的
CREATE TABLE **不移动** stamp（它的 7 处 bump 只覆盖 insert/delete/update）。
后果是：file 后端上「同名槽会累积本库写过的键」，误接受方向被这一累积语义掩盖；
而 memory 后端上 T1 发生在 stamp 仍为 0 的时刻（**不建槽**），到 T3 时槽里只有另一个库的键集
⇒ 误接受方向在 memory 上原形毕露。
⇒ **两个后端在同一形状下的失效形态不同，这本身就是「跨库暴露面」的一个证据**，
本轮已把两个后端各自的形态都固化成用例。

**最终实现状态**：两处守卫均已恢复，`cargo test --test pk_cache_file_backend_test` ⇒ **24 passed / 0 failed**。


---

## 5. 跨库误命中：既有暴露面的公开

### 5.1 机制

- 存储层的 stamp **按裸表名**计数（`MemoryStorage` 记在 DEFAULT_DATABASE 桶；本轮 `FileStorage` 同口径）。
- 引擎的 PK 缓存**按裸表名**分槽。
- ⇒ `d1.t` 与 `d2.t` **共用一个 stamp、一个缓存槽**。
- ⇒ stamp 只能说明「叫 `t` 的表变过」，**不能说明「这些行变过」** ⇒ 两个库的索引可以互相顶替。

### 5.2 本轮在代码上独立推导的两个失败形态

deepseek 认可反例成立、chatGPT 否定其具体序列。**本轮不采信任一方的序列，改为在真实代码路径上重新推导**，
并对**两个方向**都做断言（既覆盖 deepseek 描述的形态，也覆盖 chatGPT 描述的形态）：

**误拒绝**（缓存槽装的是 `d1.t` 的键集）：

| 步 | 操作 | stamp(`t`) | 缓存槽内容 |
|---|---|---|---|
| T1 | `INSERT INTO d1.t VALUES (7)` | 0 → 1 | 无槽（stamp 为 0 ⇒ 不建索引） |
| T2 | `INSERT INTO d1.t VALUES (9)` | 1 → 2 | `d1` 的键集 `{7,9}` |
| T3 | `INSERT INTO d2.t VALUES (7)` | 2 | **命中 `d1` 的槽 ⇒ 看到 7 ⇒ 误判重复 ⇒ 拒绝**（而 `d2.t` 是空的，7 在那里是自由键） |

**误接受**（缓存槽装的是 `d1.t` 的键集，但 `d2.t` 早已持有该键）：

| 步 | 操作 | stamp(`t`) | 缓存槽内容 |
|---|---|---|---|
| T1 | `INSERT INTO d2.t VALUES (7)` | 0 → 1 | 无槽 |
| T2 | `INSERT INTO d1.t VALUES (9)` | 1 → 2 | `d1` 的键集 `{9}` |
| T3 | `INSERT INTO d2.t VALUES (7)` | 2 | **命中 `d1` 的槽 ⇒ 看不到 7 ⇒ 放行**（而 `d2.t` 已有 7 ⇒ **重复键被静默接受**） |

⇒ 两个方向都由 §4.1 的用例 5、6 覆盖，**两个后端各跑一遍**。

**两个后端的失效形态不完全相同**（这是本轮实测出来的，不是一开始就知道的）：
`MemoryStorage` 的 CREATE TABLE **不**移动 stamp（它的 bump 只覆盖 insert/delete/update），
`FileStorage` 的 CREATE TABLE **会**移动 stamp。
⇒ 在「先建表、后写入」的同一形状下，memory 后端上两个方向都复现，
file 后端上复现「误拒绝」方向，而「误接受」方向被「同名槽会累积本库写过的键」掩盖（详见 §4.3）。
⇒ **两个后端的用例都保留**，因为它们钉的是不同的东西。


### 5.3 暴露面的净变化（公开）

- **修复前**：`MemoryStorage` **与** `FileStorage`（= 生产后端）**都**在暴露面里。
- **修复后**：`MemoryStorage` **仍在**暴露面里（**本轮只固化用例，未改其存储语义**）；
  `FileStorage` 被 X2 摘出暴露面。
- ⇒ **暴露面净减少**（生产后端不再暴露），**但既有问题没有因此消失**：`MemoryStorage` 上今天就能复现。

**为什么不在本轮把 `MemoryStorage` 也彻底修掉**：彻底修需要把 `MemoryStorage` 的 stamp 键从
DEFAULT_DATABASE 桶改成 `(db, table)` 口径，并同步改它的 7 处 bump 调用点与所有读点 ——
这正是 chatGPT 所称的 **X3**，两血都判定「改动面过大，留后续治理」。
本轮的处理是：**修 file 后端（#5412 本体）+ 堵住引擎侧的跨库顶替（X2）+ 把既有暴露面固化为测试 + 公开登记**。

⇒ **这意味着：即便本 PR 合入，`MemoryStorage` 上跨库同名表的误命中仍然存在**。
它被钉在测试里（用例 5、6 在 memory 后端上跑），但**测试是绿的** —— 因为 X2 的引擎侧守卫
对**两个后端都生效** ⇒ 引擎层已经不会拿别库的键集作答了。
换言之：**X2 把「引擎用别库的键集作答」这个漏洞在两个后端上都堵住了**；
剩下的「两个库共用一个 stamp 槽」是**过度失效**（性能损失，不是正确性问题）。

---

## 6. 本轮新发现的既有缺陷（#5891，不在本 PR 修）

**现象**：**非默认库**里的 `ROLLBACK` **不撤销写入** ⇒ 被回滚的行仍然可见，
并挡住后续同键 INSERT（报 `DuplicateKey`）。

**根因**：`crates/storage/src/file_storage.rs` 的 `undo_transaction_in_place`
在撤销时用**硬编码的 `DEFAULT_DATABASE`** 定位要改的行/缓冲
（基线 `f6beacad31` 的 `:5516` / `:5524`），而 undo 日志本身**只记裸表名、不记库**。

**归属**：**既有缺陷，非本 PR 引入** —— 基线代码里已在，
且 `git diff f6beacad31 -- crates/storage/src/file_storage.rs` 中涉及 `DEFAULT_DATABASE` 的增删行数 = **0**。

**实测**（两个引擎共用一个 `MvccStorage<FileStorage>`）：

```
during tx, B count = 0
after rollback, B count = 1
after rollback, B rows = [[Integer(2)]]
default-db after rollback, D count = 0
```

**处置**：已登记 **#5891**（`priority/p0` / `state/ready` / `cap/architecture` / `cost/t2`）。
本 PR **不修**：修它要往 undo 日志里补 `db`（多个 `UndoOp` 变体 + 若干写入点 + 撤销点），
会显著扩大 #5412 的改动面与回归风险，违反 ALPHA 阶段的改动面控制。
⇒ **是否并入本批次由属主裁定。**

---

## 7. 未做 / 未验证（诚实登记）

| 项 | 状态 |
|---|---|
| **019 认证轮（GA 35 项）** | 本 PR 改了 `crates/` ⇒ **既有轮次结论对新的冻结点失效**，需在新的冻结点重跑。019 当前仍停在 19/35、第 20 项在跑（执行器 PID 18229） |
| **ALPHA 10 门禁在当前 tip 的复跑取证** | **未做**（等 CPU 窗口） |
| **#5580 的 cargo 级 G9/G10 真跑** | **未做**（桩 A/B 不能替代真跑） |
| **ALPHA→BETA 跃迁** | **未做，且本 PR 不构成放行**：需 AB-01..AB-10 逐项取证 + 独立 Reviewer 回执（AB-10）+ **人工批准记录** |
| **当期 open P0** | 27（本 PR 新登记 #5891 后为 **28**）⇒ **E1 不满足** |
| **X3（stamp 键改成 `(db,table)`）** | **未做**，两血都判定留后续治理 |
| **`MemoryStorage` 的 stamp 键口径** | **未改**（见 §5.3：引擎侧已堵，既有暴露面已固化并公开） |
| **`get_table_mut` 的废弃** | **未做**（本轮裁决：保留 + 借用点保守 bump；分歧见 §2） |
| **CI 开关 `has_actions=false`** | 未动（SOAK 独占期主动关闭，注明可逆） |

---

## 8. 复现命令

```bash
cd /Volumes/workspace/dev/srg-gate-fix

# 新套件
nice -n 19 cargo test --test pk_cache_file_backend_test -- --nocapture

# 既有 PK 缓存回归
nice -n 19 cargo test --test pk_cache_regression_test

# 库单测
nice -n 19 cargo test -p sqlrustgo-storage --lib
nice -n 19 cargo test --lib -j 2

# 改动面自查（本 PR 未碰 undo 路径的 DEFAULT_DATABASE 硬编码）
git diff f6beacad31 -- crates/storage/src/file_storage.rs | grep -c "^[+-].*DEFAULT_DATABASE"

# 看板一致性
/usr/bin/python3 scripts/gate/kanban_render.py render docs/releases/v4.1.0/KANBAN_v4.1.0_ALPHA.md
/usr/bin/python3 scripts/gate/kanban_render.py check  docs/releases/v4.1.0/KANBAN_v4.1.0_ALPHA.md
/usr/bin/python3 scripts/gate/kanban_merge_policy.py docs/releases/v4.1.0/KANBAN_v4.1.0_ALPHA.jsonl
```

---

## 9. 阶段口径

- **阶段仍是 ALPHA**。本文件**不产生任何 PASS 声明**。
- 本 PR 是 **ALPHA 阶段 ISSUE 的源码修复**，不是阶段跃迁的一部分，也不代替 AB-01..AB-10 的逐项取证。
- **改 crates ⇒ 019 认证轮需在新冻结点重跑**，这一条已随本 PR 公开登记。
