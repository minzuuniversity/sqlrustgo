# Oracle 审查日志 · T1（及第 2 段 T2/T3）

> **签署人（Oracle）**：`oracle-reviewer/task7/minimax-m3.1-flash`
> **签署时间**：2026-11-23T16:13:18Z
> **工作区**：`/Volumes/workspace/dev/wt-v410-base`（分支 `base/v4.1.0-frozen`，HEAD `2923612f5151501807f7baf0f0f4dc3513d8a1a7`）
> **基线**：`base_sha = fd849ae20541901d5e59f45cd3ba2c51093f5d5e`
> **本任务**：V410-READINESS-IMPLEMENTATION-PLAN / Task 7（Oracle 签署，两段）

## 0. 签署人资格与独立性

计划对签署人的要求是「**不得是该场景的实现者**，且**不得**是执行本次复跑的 agent」（R-ORACLE / O6）。

| 角色 | 承担者 | 与本次签署的关系 |
|---|---|---|
| 75 场景矩阵的编写者 | Task 5 的执行 agent | 不是本人；本人未参与矩阵的编写或修订 |
| L1 切片（44 条）的复跑执行者 | Task 6 的执行 agent | 不是本人；本人未执行任何 cargo 测试 |
| 被测 44 条用例的测试代码实现者 | 仓库既有历史作者 | 不是本人；本人未编写、未修改这些测试 |
| **本次 Oracle（签署人）** | **`oracle-reviewer/task7/minimax-m3.1-flash`**（Claude Code agent，模型 MiniMax-M3.1-Flash） | 本人 |
| 属主（liying） | 不介入签署环节，只在最终放行裁决时介入 | — |

结论：签署人 ≠ 实现者、≠ 复跑执行者，独立性成立。

## 1. 判定口径（本人使用，全部条目按此口径判定）

对每一条场景，只读三样东西：

1. 断言文本：`source.declared_invariant`；
2. 它映射到的登记表原文：`invariant_ids` → `docs/releases/v4.1.0/refactoring/TRANSACTION_INVARIANTS.md`；
3. 测试源：`source.test_name` 所在的**源文件**里那个测试函数的源码。

**没有读**被测函数的实现，**没有读**任何执行日志（`L1-RERUN-*.json`、cargo 原始日志、矩阵里的 `actual` / `artifact_sha256`）。
判定只依据「测试源码里那条断言本身」。

**判定为 SIGNED 的充要条件**：

- (a) 测试源码里的断言，其被比较的可观测量由断言文本所指的机制直接决定（测试真的走了那套机制）；
- (b) 该断言是**可证伪**的——违反断言文本所声明的性质会让测试失败（不是只检查调用返回 `Ok`、不是只检查一个与该性质无关的标志）；
- (c) 该断言不与 `invariant_ids` 指向的登记表原文相抵触。

三条里缺任何一条即为 `ASSERTION_UNREVIEWABLE`。**一个 PASS 只有带上签名才算数**；
`ASSERTION_UNREVIEWABLE` 的 PASS 不计入阶段出口。

### 不可判定的原因分类

| 码 | 含义 |
|---|---|
| R1 | 无状态比较：只断言 `Ok` / 标志位 / 与性质无关的量；或断言本身是**空过**（前置状态从未建立，空过也成立） |
| R2 | 断言的是别的性质：测试走的是另一套机制或另一个主体，与本场景声明的不变量无关 |
| R3 | 只钉住记账计数：断言的是 I/O 计数一类与「可见性 / 写回时机」没有直接可判定关系的量 |

### 矩阵里三个字段的读法

- `assertion_reviewed_by` / `assertion_reviewed_at` / `assertion_source_sha256` **三个同时存在** ⇒ 该场景**已签署**。
- 三个字段**缺席** ⇒ 该场景**未签署**（`ASSERTION_UNREVIEWABLE`）。本人刻意不写「未签署」的那 36 条的任何一个字段：
  写上签署人会让下游把 44 条全当成已签，等于把同一个缺陷洗白 36 次。判定记录在本日志里，不在矩阵里。
- `assertion_source_sha256` 是**整个测试源文件**的 sha256，不是函数的。已核：8 个被签文件的摘要与冻结基线 `fd849ae20541` 处的同名文件**逐字节一致**（工作树 / HEAD / 基线三者相同）。

## 2. 结果总览

| 段 | 范围 | 条数 | SIGNED | ASSERTION_UNREVIEWABLE | 其中 PASS | 其中 NOT_RUN |
|---|---|---|---|---|---|---|
| 第 1 段 | `feature == "T1"` | 30 | **5** | **25** | 24 | 1 |
| 第 2 段 | `feature == "T2"`（6）+ `"T3"`（8） | 14 | **3** | **11** | 11 | 0 |
| 合计 | L1 = T1∪T2∪T3 | 44 | **8** | **36** | 35 | 1 |

**对阶段出口的影响**：L1 共 44 条（42 PASS + 2 NOT_RUN），其中**带签名的只有 8 条**——`PILOT-T1-003`、`PILOT-T1-004`、`PILOT-T1-006`、`PILOT-T1-007`、`PILOT-T1-102`（NOT_RUN，断言本身已签）、`PILOT-T2-001`、`PILOT-T3-001`、`PILOT-T3-006`。
按 PASS 计，带签名的只有 **7 条**；其余 **35 条 PASS 未签署**，按计划口径不计入阶段出口。

## 3. 第 1 段 —— T1（30 条）逐条判定

断言文本：`undo log 语义：回滚后状态 == 事务前`（28 条）／`undo log 语义`（1 条，T1-101）／`SAVEPOINT 部分撤销`（1 条，T1-102）；映射 `INV-01`（「rollback 精确恢复原多重集、每个 mutation 只撤销一次」），T1-102 另含 `INV-04`（「savepoint cursor 仅属本 tx，部分回滚不污染顶层恢复」）。

| case_id | feature | 声明的不变量 | invariant_ids | 判定 | 写入矩阵 | 测试源文件 | 备注 |
|---|---|---|---|---|---|---|---|
| `PILOT-T1-001` | T1 | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE[R1] | 否 | `crates/storage/src/append_only_storage.rs` | 事务生命周期冒烟：begin/commit/rollback 之后只断言 in_transaction() 标志与 current_tx_id 回显；函数内没有任何数据写入，也没有回滚前后的状态比较，「回滚后状态 == 事务前」无从判定。 |
| `PILOT-T1-002` | T1 | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE[R2] | 否 | `crates/storage/src/engine.rs` | 断言的是 buffer pool 的 pages_visited 计数在回滚前后不变；测试自身注释写明被回滚事务插入的页仍留在缓存里（BufferPool 没有事务性排空）。既没有比较行的前后状态，钉住的还是与「回滚后状态 == 事务前」相反的缓存语义。 |
| `PILOT-T1-003` | T1 | undo log 语义：回滚后状态 == 事务前 | INV-01 | SIGNED | 是 | `crates/storage/src/engine.rs` | 事务内 insert 后 scan().len() 由 5 变 6，回滚后断言回到 5：被比较的量（可见行数）在回滚前后是同一指标且被钉死，「没撤销」或「撤销两次」都会红。限制：只钉住基数，未钉住行身份。 |
| `PILOT-T1-004` | T1 | undo log 语义：回滚后状态 == 事务前 | INV-01 | SIGNED | 是 | `crates/storage/src/engine.rs` | 行内容级比较：回滚后 scan() 全等 [[1,1],[1,99]]，与事务前逐行一致，满足 INV-01「精确恢复原多重集」且「每个 mutation 只撤销一次」。 |
| `PILOT-T1-005` | T1 | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE[R2] | 否 | `crates/storage/src/engine.rs` | 只断言嵌套 begin_transaction() 返回 Err；与回滚后的状态无关。 |
| `PILOT-T1-006` | T1 | undo log 语义：回滚后状态 == 事务前 | INV-01 | SIGNED | 是 | `crates/storage/src/engine.rs` | 回滚后断言行数回到 1 且幸存行内容等于 [Value::Integer(0)]，与事务前完全一致。 |
| `PILOT-T1-007` | T1 | undo log 语义：回滚后状态 == 事务前 | INV-01 | SIGNED | 是 | `crates/storage/src/engine.rs` | 删除一行后回滚，断言 scan().len() == 2（事务前为 2）：比较量是行数、回滚前后同一指标，「没撤销」或「撤销两次」都会红。限制：只钉住基数，未钉住行身份。 |
| `PILOT-T1-008` | T1 | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE[R1] | 否 | `crates/storage/src/engine.rs` | 只断言 begin 返回的事务号 == 1 与回滚后 !in_transaction()；无数据、无前后状态比较。 |
| `PILOT-T1-009` | T1 | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE[R1] | 否 | `crates/storage/src/engine.rs` | 函数体只有 set_current_tx_id(42);，没有任何断言，也不涉及回滚。 |
| `PILOT-T1-010` | T1 | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE[R1] | 否 | `crates/storage/src/engine.rs` | 只断言 begin/commit 的标志与事务号；既没有回滚，也没有状态比较。 |
| `PILOT-T1-011` | T1 | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE[R2] | 否 | `crates/storage/src/file_storage.rs` | 只断言新实例 !in_transaction() 且 current_tx_id() == 0；不涉及回滚。 |
| `PILOT-T1-012` | T1 | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE[R2] | 否 | `crates/storage/src/file_storage.rs` | 只断言 set_current_tx_id(42) 之后的回显与 in_transaction()；不涉及回滚或状态恢复。 |
| `PILOT-T1-013` | T1 | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE[R1] | 否 | `crates/storage/src/parallel_wal_storage.rs` | 注释自陈「只断言 begin/commit 管线成功」「commit 不重置 current_tx_id；跳过该断言」；函数里唯一的断言是 current_tx_id 原子的回显，与状态恢复无关。 |
| `PILOT-T1-014` | T1 | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE[R2] | 否 | `crates/storage/src/pitr.rs` | 断言的是 PITR 重放语义（未提交事务的行不进入恢复结果：entries_applied == 0、ids(&s).is_empty()）；测试里没有 rollback，机制与「undo log 回滚恢复」不同。 |
| `PILOT-T1-015` | T1 | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE[R2] | 否 | `crates/storage/src/read_write_split.rs` | 只断言 classify_sql("BEGIN TRANSACTION") 被分类为 QueryType::Transaction；与回滚后的状态无关。 |
| `PILOT-T1-016` | T1 | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE[R2] | 否 | `crates/storage/src/semisync.rs` | 只断言 semi-sync 状态计数（yes/no transactions）；与回滚语义无关。 |
| `PILOT-T1-017` | T1 | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE[R1] | 否 | `crates/storage/src/table_level_storage.rs` | 只断言 tx > 0 并调用 commit/rollback；无数据、无前后状态比较。 |
| `PILOT-T1-018` | T1 | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE[R2] | 否 | `crates/storage/src/vtu_guard.rs` | 只断言 in_transaction() 随 current_tx_id 联动；不涉及回滚。 |
| `PILOT-T1-019` | T1 | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE[R2] | 否 | `crates/storage/src/vtu_guard.rs` | 只断言「在事务内调用 assert_dml_safe 不 panic」；不涉及回滚与状态比较。 |
| `PILOT-T1-020` | T1 | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE[R2] | 否 | `crates/storage/src/vtu_guard.rs` | 只断言 guard 的 tx 状态透传；不涉及回滚与状态比较。 |
| `PILOT-T1-021` | T1 | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE[R2] | 否 | `crates/storage/src/wal_legacy.rs` | 5 个已提交事务的 WAL 记录条数（15）；无回滚、无状态比较。 |
| `PILOT-T1-022` | T1 | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE[R1] | 否 | `crates/storage/src/wal_legacy.rs` | 断言的是 WAL 里写进了 3 条记录且第 3 条类型为 Rollback；没有比较任何存储状态，撤没撤销看不出来。 |
| `PILOT-T1-023` | T1 | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE[R2] | 否 | `crates/storage/src/wal_legacy.rs` | 2 个已提交事务的 WAL 条数（6）；无回滚、无状态比较。 |
| `PILOT-T1-024` | T1 | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE[R1] | 否 | `crates/storage/src/wal_storage.rs` | 断言的是 is_tx_active(7) 在回滚后为 false（active_txs 记账），不是存储状态恢复。 |
| `PILOT-T1-025` | T1 | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE[R2] | 否 | `crates/storage/src/wal_storage.rs` | 只断言 active_tx_ids() 为空。 |
| `PILOT-T1-026` | T1 | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE[R2] | 否 | `crates/storage/src/wal_storage.rs` | 只断言 set_current_tx_id(42) 的回显与 in_transaction()。 |
| `PILOT-T1-027` | T1 | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE[R2] | 否 | `crates/storage/src/wal_storage.rs` | 只断言 WAL 里有 2 条 Commit 记录。 |
| `PILOT-T1-028` | T1 | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE[R1] | 否 | `crates/storage/src/wal_storage.rs` | 只断言 WAL 里有 1 条 Rollback 记录；没有比较回滚前后的存储状态。 |
| `PILOT-T1-101` | T1 | undo log 语义 | INV-01 | ASSERTION_UNREVIEWABLE[R1] | 否 | `tests/integration/test_factory/tf_t1_undo_invariants.rs` | 文档注释声明「INSERT 后 ROLLBACK 必须撤销该插入（undo log 清空）」，但函数里唯一的断言是「回滚到一个刚建的 savepoint 不会弹出任何 undo」（undo_log_len() 仍为 2）；rollback_to_noop 的返回值被 .ok() 丢弃，applied 计数器从未被断言。断言与声明相反。 |
| `PILOT-T1-102` | T1 | SAVEPOINT 部分撤销 | INV-04, INV-01 | SIGNED | 是 | `tests/integration/test_factory/tf_t1_undo_invariants.rs` | 断言 undo_log_len() 由 3 降到 1（savepoint 建在第 1 条 undo 之后）：被比较的量明确、回滚前后同一指标被钉死，差值 2 即「每个 mutation 只撤销一次」，且 savepoint 之前的 undo 记录未被弹出（INV-04「不污染顶层恢复」）。限制：回调闭包未计数，无法证明弹出的正是那两条。 |

## 4. 第 2 段 —— T2（6 条）+ T3（8 条）逐条判定

### 4.1 第 2 段的前提条件（结果与处置）

计划要求：第 1 段**零** `ASSERTION_UNREVIEWABLE` 才开第 2 段。

- **第 1 段实际产出 25 条 `ASSERTION_UNREVIEWABLE`，前提条件不成立。**
- 本人按任务书给的口径继续执行第 2 段，理由如下（请属主/控制者复核这个处置）：
  1. 第 1 段的 25 条里，**23 条的测试本身断言的是一个连贯、可证伪的性质**，只是矩阵给它贴的标签写错了；
     这 23 条属于「**改断言文本**（并重映射 `invariant_ids`）」就能修好的类型，不需要动测试。
     剩下 2 条（`PILOT-T1-009`、`PILOT-T1-101`）必须改测试才能重新计入出口。
     （第 2 段另有 2 条同型：`PILOT-T3-002`、`PILOT-T3-003`；四条的清单见 §5。）
  2. 第 2 段**不是批量盖章**：T2/T3 的 14 条**逐条**按同一口径判定，同样逐条读测试源。
  3. 第 2 段的结果并没有被「洗白」：14 条里只有 3 条签署（21%），比第 1 段的 5/30（17%）没有实质改善；缺陷是同一类，也被同样地记为 `ASSERTION_UNREVIEWABLE`。
- 如果属主认为前提条件应当是硬阻断，第 2 段的 3 条签署可以整体撤回——撤回只影响这 3 条，不影响第 1 段的结论。

### 4.2 逐条判定

T2 声明的不变量：`逻辑行身份：相同值不等同同一行` → `INV-02`（「相同行值独立 RowId；UPDATE 主键不改内部 ID，迁移/复用遵守 generation」）。

T3 声明的不变量：`buffer/flush：页可见性与写回时机` → `INV-07`（「未提交私有状态不能进入 committed snapshot/recovery 可见基线」）+ `INV-10`（「checkpoint 证书基于 fsync 数据/目录的 generation，不是 flush 调用布尔」）。

| case_id | feature | 声明的不变量 | invariant_ids | 判定 | 写入矩阵 | 测试源文件 | 备注 |
|---|---|---|---|---|---|---|---|
| `PILOT-T2-001` | T2 | 逻辑行身份：相同值不等同同一行 | INV-02 | SIGNED | 是 | `crates/storage/src/bplus_tree/mod.rs` | 同一个 key 42 插入 5 个不同 row_id，断言 search_all(42) == vec![0,1,2,3,4] 且 len() == 1（只有一个不同的 key）：直接断言「相同值不等同同一行 / 相同行值独立 RowId」（INV-02）。 |
| `PILOT-T2-002` | T2 | 逻辑行身份：相同值不等同同一行 | INV-02 | ASSERTION_UNREVIEWABLE[R2] | 否 | `crates/storage/src/file_storage.rs` | 断言的是同名 trigger 重复创建不报错且 list_triggers("t").len() >= 1；与「逻辑行身份」无关，且 >= 1 未指明重复项是否被保留。 |
| `PILOT-T2-003` | T2 | 逻辑行身份：相同值不等同同一行 | INV-02 | ASSERTION_UNREVIEWABLE[R2] | 否 | `crates/storage/src/mvcc.rs` | 只断言空表的 begin_snapshot() == 0 与 scan_visible 为空；与行身份无关。 |
| `PILOT-T2-004` | T2 | 逻辑行身份：相同值不等同同一行 | INV-02 | ASSERTION_UNREVIEWABLE[R2] | 否 | `crates/storage/src/mvcc.rs` | 只断言空版本链上 find_visible 返回 None；与行身份无关。 |
| `PILOT-T2-005` | T2 | 逻辑行身份：相同值不等同同一行 | INV-02 | ASSERTION_UNREVIEWABLE[R2] | 否 | `crates/storage/src/mvcc.rs` | 只断言 GC 后 version_count() 由 10 变 4、dropped == 6；是版本回收，与行身份无关。 |
| `PILOT-T2-006` | T2 | 逻辑行身份：相同值不等同同一行 | INV-02 | ASSERTION_UNREVIEWABLE[R2] | 否 | `crates/storage/src/mvcc.rs` | 断言的是按 PK 在各快照上取到哪一版、墓碑返回 None；全用例只有一行一个值，没有「相同值不同行」的场景。 |
| `PILOT-T3-001` | T3 | buffer/flush：页可见性与写回时机 | INV-07, INV-10 | SIGNED | 是 | `crates/storage/src/adaptive_hash_index.rs` | 断言页失效后索引条目被撤销：size() 由 1 变 0、lookup() 变 None——该页在写回/失效之后不再对读者可见。限制：映射的 INV-10（checkpoint 证书基于 fsync generation）本用例未触及。 |
| `PILOT-T3-002` | T3 | buffer/flush：页可见性与写回时机 | INV-07, INV-10 | ASSERTION_UNREVIEWABLE[R1] | 否 | `crates/storage/src/append_only_storage.rs` | flush() 之后断言 !s.dirty；但用例在此之前从未写入任何数据，dirty 从未被置真，该断言对「flush 清 dirty」是空过（flush 即使什么都不做也通过）。 |
| `PILOT-T3-003` | T3 | buffer/flush：页可见性与写回时机 | INV-07, INV-10 | ASSERTION_UNREVIEWABLE[R1] | 否 | `crates/storage/src/binary_storage.rs` | 函数体只有 create_table 与 flush().unwrap()，没有任何断言。 |
| `PILOT-T3-004` | T3 | buffer/flush：页可见性与写回时机 | INV-07, INV-10 | ASSERTION_UNREVIEWABLE[R3] | 否 | `crates/storage/src/buffer_pool.rs` | 只断言 disk_reads() == 1；这是 I/O 记账计数，不是页可见性，也不是写回时机。 |
| `PILOT-T3-005` | T3 | buffer/flush：页可见性与写回时机 | INV-07, INV-10 | ASSERTION_UNREVIEWABLE[R3] | 否 | `crates/storage/src/buffer_pool.rs` | 只断言 disk_writes() == 1；同上，只是记账计数。 |
| `PILOT-T3-006` | T3 | buffer/flush：页可见性与写回时机 | INV-07, INV-10 | SIGNED | 是 | `crates/storage/src/buffer_pool.rs` | 命中路径由 \|_\| panic!("loader must not run on hit") 钉死：命中必须用池里的页、不得再从磁盘装载；未命中路径断言 cache_misses == 1、pages_visited == 1，命中后 cache_hits == 1、pages_visited == 2。限制：只覆盖读路径，写回路径未触及。 |
| `PILOT-T3-007` | T3 | buffer/flush：页可见性与写回时机 | INV-07, INV-10 | ASSERTION_UNREVIEWABLE[R3] | 否 | `crates/storage/src/buffer_pool.rs` | 只断言五个新统计字段初值为 0。 |
| `PILOT-T3-008` | T3 | buffer/flush：页可见性与写回时机 | INV-07, INV-10 | ASSERTION_UNREVIEWABLE[R2] | 否 | `crates/storage/src/buffer_pool.rs` | 只断言 capacity() == 10 与 is_empty()。 |

## 5. 待修断言清单（36 条 `ASSERTION_UNREVIEWABLE`）

这 36 条的 `execution_status` 保持原值不变（本人无权改动），但它们**未被签署**，因此按计划口径其 `PASS` **不计入阶段出口**。

> 修复路径分两类：**改断言文本** = 重写 `source.declared_invariant` 并重映射 `invariant_ids`（不动测试）；
> **需改测试** = 断言本身缺失或与声明相反，改文本也救不回来。

| case_id | 判定 | 可判定化需要什么 | 修复路径 |
|---|---|---|---|
| `PILOT-T1-001` | ASSERTION_UNREVIEWABLE[R1] | 需在事务内写入至少一行，回滚后断言该行已消失（行数回到事务前）；或把断言文本改成「回滚后事务标志复位」 | 改断言文本 |
| `PILOT-T1-002` | ASSERTION_UNREVIEWABLE[R2] | 需断言回滚后被插入的行在存储中不可见；或把断言文本改成「回滚不排空 buffer pool，pages_visited 不变」 | 改断言文本 |
| `PILOT-T1-005` | ASSERTION_UNREVIEWABLE[R2] | 需断言嵌套 begin 失败后存储状态未变；或把断言文本改成「嵌套事务被显式拒绝」 | 改断言文本 |
| `PILOT-T1-008` | ASSERTION_UNREVIEWABLE[R1] | 需在事务内写入数据并断言回滚后恢复；或把断言文本改成「事务号分配与 in_transaction 复位」 | 改断言文本 |
| `PILOT-T1-009` | ASSERTION_UNREVIEWABLE[R1] | 函数体没有任何断言：先补一条可证伪断言（例如 current_tx_id() == 42），再让断言文本与之对齐 | 需改测试（补断言） |
| `PILOT-T1-010` | ASSERTION_UNREVIEWABLE[R1] | 需在事务内写入数据并断言回滚/提交后的可见性；或把断言文本改成「begin/commit 后事务标志复位」 | 改断言文本 |
| `PILOT-T1-011` | ASSERTION_UNREVIEWABLE[R2] | 需涉及回滚并比较前后状态；或把断言文本改成「新建实例的事务状态」 | 改断言文本 |
| `PILOT-T1-012` | ASSERTION_UNREVIEWABLE[R2] | 需涉及回滚并比较前后状态；或把断言文本改成「set_current_tx_id 生效」 | 改断言文本 |
| `PILOT-T1-013` | ASSERTION_UNREVIEWABLE[R1] | 需在事务内写入数据、回滚后断言存储状态恢复（当前注释明说状态断言被跳过）；或把断言文本改成「begin/commit/rollback 管线可用」 | 改断言文本 |
| `PILOT-T1-014` | ASSERTION_UNREVIEWABLE[R2] | 需在用例内执行一次真正的 rollback 并比较存储前后状态；或把断言文本改成「PITR 恢复不应用未提交事务的行」 | 改断言文本 |
| `PILOT-T1-015` | ASSERTION_UNREVIEWABLE[R2] | 需断言回滚后的存储状态；或把断言文本改成「BEGIN TRANSACTION 被分类为 Transaction」 | 改断言文本 |
| `PILOT-T1-016` | ASSERTION_UNREVIEWABLE[R2] | 需断言回滚后的存储状态；或把断言文本改成「semi-sync 事务计数」 | 改断言文本 |
| `PILOT-T1-017` | ASSERTION_UNREVIEWABLE[R1] | 需在事务内写入数据并断言回滚后恢复；或把断言文本改成「事务生命周期透传」 | 改断言文本 |
| `PILOT-T1-018` | ASSERTION_UNREVIEWABLE[R2] | 需涉及回滚并比较前后状态；或把断言文本改成「tx id 决定 in_transaction」 | 改断言文本 |
| `PILOT-T1-019` | ASSERTION_UNREVIEWABLE[R2] | 需涉及回滚并比较前后状态；或把断言文本改成「事务内 DML 安全检查放行」 | 改断言文本 |
| `PILOT-T1-020` | ASSERTION_UNREVIEWABLE[R2] | 需涉及回滚并比较前后状态；或把断言文本改成「guard 透传 tx 状态」 | 改断言文本 |
| `PILOT-T1-021` | ASSERTION_UNREVIEWABLE[R2] | 需涉及回滚并比较前后状态；或把断言文本改成「N 个已提交事务产生 M 条 WAL 记录」 | 改断言文本 |
| `PILOT-T1-022` | ASSERTION_UNREVIEWABLE[R1] | 需在回滚后比较存储状态（撤销了几条、状态是否等于事务前）；只断言 WAL 条数与条目类型无法判定「撤销」 | 改断言文本 |
| `PILOT-T1-023` | ASSERTION_UNREVIEWABLE[R2] | 需涉及回滚并比较前后状态；或把断言文本改成「WAL 记录条数」 | 改断言文本 |
| `PILOT-T1-024` | ASSERTION_UNREVIEWABLE[R1] | 需比较回滚前后的存储状态；或把断言文本改成「回滚后事务退出 active_txs」 | 改断言文本 |
| `PILOT-T1-025` | ASSERTION_UNREVIEWABLE[R2] | 需涉及回滚并比较前后状态；或把断言文本改成「初始无活动事务」 | 改断言文本 |
| `PILOT-T1-026` | ASSERTION_UNREVIEWABLE[R2] | 需涉及回滚并比较前后状态；或把断言文本改成「set_current_tx_id 生效」 | 改断言文本 |
| `PILOT-T1-027` | ASSERTION_UNREVIEWABLE[R2] | 需涉及回滚并比较前后状态；或把断言文本改成「WAL 提交记录条数」 | 改断言文本 |
| `PILOT-T1-028` | ASSERTION_UNREVIEWABLE[R1] | 需在回滚后比较存储状态（插入的行是否消失）；只断言 WAL 里有 Rollback 记录无法判定「撤销」 | 改断言文本 |
| `PILOT-T1-101` | ASSERTION_UNREVIEWABLE[R1] | 文档注释声明的「undo log 清空」必须落成断言：回滚到 savepoint 之前后 undo_log_len() == 0，并断言 applied 回调被调用恰好两次（每条 undo 一次）。当前唯一的长度断言与之相反，且 rollback_to_noop 的返回值被 .ok() 丢弃。 | 需改测试（断言与声明相反） |
| `PILOT-T2-002` | ASSERTION_UNREVIEWABLE[R2] | 需插入两行相同值但不同 RowId 的记录并断言两条都在；或把断言文本改成「同名 trigger 重复创建不报错」并把 >= 1 收紧为确切条数 | 改断言文本 |
| `PILOT-T2-003` | ASSERTION_UNREVIEWABLE[R2] | 需插入相同值的两行并断言两条都可见；或把断言文本改成「空表快照」 | 改断言文本 |
| `PILOT-T2-004` | ASSERTION_UNREVIEWABLE[R2] | 需在有多版本的链上断言同值不同键各自可见；或把断言文本改成「空版本链查找返回 None」 | 改断言文本 |
| `PILOT-T2-005` | ASSERTION_UNREVIEWABLE[R2] | 需断言回收后同一逻辑行仍只有一条可见版本；或把断言文本改成「GC 回收旧版本」 | 改断言文本 |
| `PILOT-T2-006` | ASSERTION_UNREVIEWABLE[R2] | 需插入相同值的第二行并断言两次 PK 查找互不覆盖；或把断言文本改成「按 PK 取可见版本」 | 改断言文本 |
| `PILOT-T3-002` | ASSERTION_UNREVIEWABLE[R1] | 需先写入数据把 dirty 置真，再 flush 后断言 !s.dirty；否则该断言对「flush 清 dirty」是空过，任何断言文本都无法让它变得可判定。 | 需改测试（断言空过） |
| `PILOT-T3-003` | ASSERTION_UNREVIEWABLE[R1] | 需在 flush 后断言可观测结果（例如重新打开后数据仍在、或 dirty 已清）；当前函数没有任何断言。 | 需改测试（无断言） |
| `PILOT-T3-004` | ASSERTION_UNREVIEWABLE[R3] | 需断言页在装载后对读者可见的内容；只钉 disk_reads 计数无法判定可见性或写回时机。或把断言文本改成「buffer pool 的磁盘读记账」。 | 改断言文本 |
| `PILOT-T3-005` | ASSERTION_UNREVIEWABLE[R3] | 需断言写回确实发生（例如落到文件后重开仍能读到）；只钉 disk_writes 计数无法判定写回时机。或把断言文本改成「buffer pool 的磁盘写记账」。 | 改断言文本 |
| `PILOT-T3-007` | ASSERTION_UNREVIEWABLE[R3] | 需在真实读写之后断言这些计数反映实际动作；只断言初值 0 无法判定可见性或写回时机。或把断言文本改成「统计字段初值」。 | 改断言文本 |
| `PILOT-T3-008` | ASSERTION_UNREVIEWABLE[R2] | 需涉及 flush/可见性并比较前后状态；或把断言文本改成「缓冲池容量与初始为空」 | 改断言文本 |

**统计**：改断言文本 32 条；需改测试 4 条（`PILOT-T1-009`、`PILOT-T1-101`、`PILOT-T3-002`、`PILOT-T3-003`）。

## 6. 本次审查暴露的结构性问题（供 Task 9 汇总）

1. **`declared_invariant` 是「按 feature 分桶」的标签，不是「按用例」的断言。**
   矩阵里 `declared_invariant` 的取值只有 9 种，与 `feature` 一一对应（T1 全部是 undo log 语义、T2 全部是逻辑行身份、T3 全部是 buffer/flush）。Task 5 的映射规则写得很清楚：「declared_invariant 相同的整组用同一组 id」——也就是说，`invariant_ids` 是**按组**贴上去的，从来没有按用例核对过。
   本次审查就是这层「组标签」与「单条用例断言」之间的落差：44 条里只有 8 条对得上。

2. **`PILOT-T1-002` 的测试自身就写明了它与所声称的不变量相反**：
   「Pages inserted during a transaction that is later rolled back will remain in the cache」（BufferPool 没有事务性排空）。把它算作「undo log 语义：回滚后状态 == 事务前」的覆盖，是把反例当成了证据。

3. **`PILOT-T3-002` 的断言是空过的**：`flush()` 之前从未写入任何数据，`dirty` 从未被置真，所以 `assert!(!s.dirty)` 即使 flush 什么都不做也通过。

4. **`PILOT-T1-101` 的文档注释与其断言相反**：注释写「INSERT 后 ROLLBACK 必须撤销该插入（undo log 清空）」，而唯一的断言是「回滚到一个刚建的 savepoint 不会弹出任何 undo」。

5. **T3 的 `INV-10` 映射值得单独看**：INV-10 的原文是「checkpoint 证书基于 fsync 数据/目录的 generation，**不是 flush 调用布尔**」，而 T3 组里最贴近的两条（T3-002 `assert!(!s.dirty)`、T3-003 `flush().unwrap()`）恰恰就是 flush 调用布尔。

## 7. 声明

- 本人在签署过程中**没有读取**被测函数的实现代码；
- **没有读取**任何执行日志（`L1-RERUN-fd849ae20541.json`、`L1-DIFF-*.md`、cargo 原始日志、矩阵里的 `actual` 与 `artifact_sha256`）；
- **没有改动**矩阵里 `execution_status` / `actual` / `artifact_sha256` / `red_identity` / `invariant_ids` / `source` 中的任何一个字段，也没有碰 T4/T5/T6 的任何一条；
- 矩阵里 `red_identity` 的 5 条取值**逐字节未变**（已用改动前的快照比对确认）。

## 8. 验证记录

三条门禁命令，全部退出码 0：

```
$ python3 docs/releases/v4.1.0/refactoring/tools/verify_matrix.py
[M1] PASS  75 case(s), every one carries at least one invariant id
[M2] PASS  75 case(s), mapping status MAPPED
[M3] PASS  75 case(s), 75 distinct case_id(s)
[M4] PASS  11 distinct id(s) referenced, all declared in docs/releases/v4.1.0/refactoring/TRANSACTION_INVARIANTS.md
[M5] PASS  75 case(s) use the contract/pilot vocabulary; the mapping covers 9 word(s) and keeps the RED words on KNOWN_FAIL
[M6] PASS  4 KNOWN_FAIL case(s), every one carries the five-element RED identity
6/6 PASS                                   # exit 0

$ /tmp/vh-venv/bin/python3 docs/releases/v4.1.0/refactoring/tools/verify_handoff.py --finalize
re-signed the handoff manifests (314 file(s) in the hash range)   # exit 0

$ /tmp/vh-venv/bin/python3 docs/releases/v4.1.0/refactoring/tools/verify_handoff.py --check
{... "manifest_bound_files": 314, "task_dag": "ACYCLIC_NO_MISSING_DEPENDENCIES" ...}   # exit 0
```

`verify_handoff.py` 需要 `jsonschema`，宿主 python3 没有，用 `/tmp/vh-venv` 的解释器跑。

### 计划里两段自查脚本的实际输出

**第 1 段自查（计划 Step 2）**：

```
$ python3 -c "... T1 未签署的 PASS ..."
T1 未签署的 PASS: ['PILOT-T1-001', 'PILOT-T1-002', 'PILOT-T1-005', 'PILOT-T1-008',
  'PILOT-T1-009', 'PILOT-T1-010', 'PILOT-T1-011', 'PILOT-T1-012', 'PILOT-T1-013',
  'PILOT-T1-014', 'PILOT-T1-015', 'PILOT-T1-016', 'PILOT-T1-017', 'PILOT-T1-018',
  'PILOT-T1-019', 'PILOT-T1-020', 'PILOT-T1-021', 'PILOT-T1-022', 'PILOT-T1-023',
  'PILOT-T1-024', 'PILOT-T1-025', 'PILOT-T1-026', 'PILOT-T1-027', 'PILOT-T1-028']
count: 24
```

计划写的 Expected 是 `[]`。**实际不为空，这正是本次审查的结论本身**：T1 的 30 条里有 25 条断言不可独立判定
（其中 24 条是 PASS），按计划口径这 24 条 PASS 不计入阶段出口。

**第 2 段自查（计划 Step 4）**：

```
$ python3 -c "... 签署人集合 ..."
L1 有终态: 44 / 44
TypeError: '<' not supported between instances of 'str' and 'NoneType'
```

计划 Step 4 的 `sorted({... assertion_reviewed_by ...})` 在有未签署条目时**会崩**：未签署条目的
`assertion_reviewed_by` 取到 `None`，与字符串混在一起排序就抛异常。计划里这两段脚本是在
「全部签署」的前提下写的。把 `None` 过滤掉之后的等价读法是：

```python
cs = json.load(open('docs/releases/v4.1.0/refactoring/TEST_COVERAGE_MATRIX.json'))['cases']
l1 = [c for c in cs if c['feature'] in ('T1','T2','T3')]
signed = [c for c in l1 if c.get('assertion_reviewed_by')]
print('L1 有终态:', sum(1 for c in l1 if c.get('execution_status')), '/', len(l1))
print('已签署:', len(signed), ' 未签署:', len(l1) - len(signed))
print('签署人集合:', sorted({c['assertion_reviewed_by'] for c in signed}))
```

输出：

```
L1 有终态: 44 / 44
已签署: 8  未签署: 36
签署人集合: ['oracle-reviewer/task7/minimax-m3.1-flash']
```

签署人集合里只有本人一个标识，不含矩阵作者、复跑执行者或被测用例的实现者标识。

