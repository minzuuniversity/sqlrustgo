# 不变量缺口提案（Task 10 · 逐用例重做映射的结果）

> **性质**：PROPOSED。本文件只登记「测试实际断言的内容在 `TRANSACTION_INVARIANTS.md` 的 18 条里找不到对应」这一事实，
> 并给出**待属主裁决**的新不变量草案。它**不是** ADR、不是批准、不改变任何门禁阈值。
>
> **作者**：`task10-executor/minimax-m3.1-flash`（Claude Code agent，模型 MiniMax-M3.1-Flash）
> **作业**：`V410-READINESS-IMPLEMENTATION-PLAN` / Task 10（逐用例重做 invariant 映射并修复 38 条不可判定断言）
> **工作区**：`/Volumes/workspace/dev/wt-v410-base`（分支 `base/v4.1.0-frozen`）
> **上游输入**：`docs/refactor/v4.1.0-c/takeover/ORACLE-REVIEW-LOG-T1.md` §5（38 条待修清单）与 §6（结构性根因）
> **日期**：2026-11-24（**修订 1**：撤下三条「靠标签贴出来」的映射，见 §0.4）

---

## 0. 这份清单是怎么来的

Task 5 的 `invariant_ids` 是**按 feature 分桶**贴上去的（`declared_invariant` 相同的整组用同一组 id），
Task 7 已经在 44 条 L1 上证明这批映射没有逐用例核对过（38 条不可判定，其中 25 条与「回滚后状态 == 事务前」毫无关系）。

Task 10 对**全部 75 条**逐条重做：读 `source.test_name` 指向的测试函数源码 → 判定它**实际断言**了什么 →
在 18 条不变量里找**真正对应**的 id。规则只有一条：

> **不得为了 `invariant_ids` 非空而贴一个不对应的 id。**
> 真的对不上就留空 `[]`，并在这里登记。
>
> **「标签」不是判据。** `source.declared_invariant`（声明目的）、feature 归属、探针标签，
> 都不能替代「读测试源码」。**源码读不到，就当作对不上**（§0.4）。

结果：**75 条里只有 12 条能落到已登记的不变量上，63 条对不上任何一条**。
这不是「映射做坏了」，而是**登记表本身的覆盖面小于试点用例集**——Task 5 的分桶映射把这一点掩盖了。

### 0.1 数字

| 项 | 值 |
|---|---|
| 矩阵场景总数 | 75 |
| 逐条重做后**有**对应不变量 | **12** 条用例 → **5** 条不变量（13 个 id-实例） |
| 逐条重做后**无**对应不变量（`invariant_ids == []`） | **63** |
| 其中 `invariant_ids` 相对 Task 5 **发生变化**的 | **65** |
| 其中 `invariant_ids` **未变**的 | 10（T1-003/004/006/007/102、T2-001、T4-101/102/103/104） |
| 18 条不变量中**有源码派生覆盖**的 | **5**（INV-01/02/04/07/14） |
| 18 条不变量中**零覆盖**的 | **13**（INV-03/05/06/08/09/10/11/12/13/15/16/17/18） |
| `source.declared_invariant` 的取值种数 | 13 → **75**（不再有任何分桶标签） |

### 0.2 零覆盖的不变量意味着什么

| 不变量 | 原文（登记表） | 试点里有没有能判定它的用例 |
|---|---|---|
| INV-03 | db/table/tx/epoch 显式封闭；跨库同名表不串写 | 无 |
| INV-05 | 一条语句多行/多表/trigger 所有可逆变化共同原子提交/撤销 | 无 |
| INV-06 | undo 失败不返回成功，不丢未完成 cursor，不允许继续 commit | 无 |
| INV-08 | 同步模式 ACK 前 durable_lsn >= commit_lsn，两 commit 入口语义相同 | 无 |
| INV-09 | durable 决定后发布失败不是「确定未提交」；OutcomeUnknown 需查询/幂等键 | 无 |
| INV-10 | checkpoint 证书基于 fsync 数据/目录的 generation，不是 flush 调用布尔 | 无（唯一沾边的 `(checkpoint-torn-write)` 探针源码不在仓内，**标签不能算覆盖**，见 §0.4） |
| INV-11 | reclaim 保留 active/replica/PITR 需要日志；segment 切换可恢复 | 无 |
| INV-12 | Recovery 失败不得进入 Recovered，重试不能空成功 | 无（唯一沾边的 `(crash-recovery-kill-9)` 探针源码不在仓内，**标签不能算覆盖**，见 §0.4） |
| INV-13 | approved 隔离语义由真实并发 + 版本/物理主账联合证明 | 无 |
| INV-15 | 索引/聚类/cache 与同一 committed RowStore 发布/撤销一致 | 无 |
| INV-16 | 旧格式迁移、重复值 redo 幂等、备份/复制/PITR 共享恢复身份 | 无 |
| INV-17 | 必需 case 的 FAIL/KNOWN_FAIL/NOT_RUN/未批准 UNSUPPORTED 均阻断认证 | 无（这是流程不变量，需由门禁而非单测证明） |
| INV-18 | 每 case/任务/调用回执可关联 SHA、原文、全 SHA256 和预算；无云 fallback | 无（同上） |

**INV-15 的零覆盖有一条硬证据**（不是「没找到用例」，是「用例钉住了反面」）：
`crates/storage/src/engine.rs` 的 `memory_storage_rollback_after_bufferpool_insert_does_not_change_pages_visited`
的注释明写「BufferPool 没有事务性排空」，即**回滚后排空缓存这一侧在基线上根本不存在**，
而 `crates/storage/src/buffer_pool.rs` 里没有任何 flush / 写回 / fsync 路径。
所以「cache 与 committed RowStore 的撤销一致」这条要求，基线**做不到**，试点也没有一条用例去钉它。

### 0.3 有**源码派生**覆盖的 5 条（12 条用例）

| 不变量 | 覆盖它的 case | 覆盖的形态 |
|---|---|---|
| INV-01 | PILOT-T1-003 / 004 / 006 / 007 / 102（5 条） | 逐行或按基数比较回滚前后的行多重集；T1-102 比较 undo_log_len |
| INV-02 | PILOT-T2-001（1 条） | 同一 key 的 5 个 row_id 全部存活 |
| INV-04 | PILOT-T1-101 / 102（2 条） | rollback_to 只弹出 cursor 之后的 undo |
| INV-07 | PILOT-T1-014（1 条） | PITR 重放不应用未提交事务的行 |
| INV-14 | PILOT-T4-101 / 102 / 103 / 104（4 条） | 静默成功检测（真生效）+ 非事务后端写前拒绝 |

> **口径**：这里的「覆盖」**只算源码派生**——每一条都是读测试源码之后判定的。
> 按标签贴出来的 id **不计入**（§0.4）。

### 0.4 修订 1：撤下三条「靠标签贴出来」的映射

Task 10 第一轮曾给三条 bespoke 探针贴了 id，依据是**探针标签与声明目的**：

| case_id | 曾经贴的 id | 撤下的理由 |
|---|---|---|
| `PILOT-T5-101` `(odku-rollback-noop)` | INV-01 | 探针源码不在本仓（未注册为 cargo test target）。唯一可读的是 `red_identity` 里登记的失败断言位置与归因文本，**不是测试源码** |
| `PILOT-T6-001B` `(crash-recovery-kill-9)` | INV-12、INV-07 | 探针源码不在本仓，`probe_registry` 未绑定该标签（执行状态 NOT_RUN） |
| `PILOT-T6-002B` `(checkpoint-torn-write)` | INV-10、INV-11 | 同上 |

**这三条正是本任务要消灭的缺陷类别**：Task 7 发现「`invariant_ids` 是按 feature 分桶贴的，从来没有按用例核对过」，
而按**标签**贴 id 是同一枚硬币的另一面——标签本身就是分桶的产物。
Step 2 要求「读该 case 的 `source.test_name` 指向的测试函数源码」；源码读不到，就**没有可核对的断言**，
按任务书自己的兜底条款（「若某个测试函数实际断言的内容不对应任何现有不变量，`invariant_ids` 留空 `[]`
并把该 case 记入待新建的不变量提案清单」）处置。

**后果（必须明说）**：

- 映射用例由 15 降为 **12**，空映射由 60 增为 **63**；
- **INV-10 与 INV-12 的唯一「覆盖」本来就是标签派生的**，撤下之后它们变成**零覆盖**；
  有源码派生覆盖的不变量由 7 条降为 **5 条**（INV-01/02/04/07/14）；
- 零覆盖的不变量由 11 条增为 **13 条**；
- **Task 9 的放行判据必须用「源码派生覆盖」这个数**：有源码派生覆盖的用例是 **12 条**，
  L1 的 44 条里是 **8 条**，L1 的 42 条 PASS 里是 **6 条**
  （`PILOT-T1-003/004/006/007/014`、`PILOT-T2-001`）。
  「L1 42 条 PASS」**不等于**「42 条不变量覆盖」。

三条探针连同它们的标签、声明目的与曾经的 id 一并登记在 §1 的 **GAP-14**，
**待探针源码迁入仓内后重新判定**。

---

## 1. 缺口提案

每条提案给出：覆盖的用例、它们实际断言的东西、**草案不变量文本**、以及为什么现有 18 条覆盖不到。
草案文本是给属主裁决用的**提案**，不是批准。

### GAP-01 · 事务标志与事务号的生命周期（10 条）

| case_id | 实际断言 |
|---|---|
| PILOT-T1-001 | begin 返回 tx id > 0 且 in_transaction() 为 true；commit/rollback 之后为 false；set_current_tx_id(42) 后回显 42 |
| PILOT-T1-008 | begin 返回 1；rollback 之后 in_transaction() 为 false |
| PILOT-T1-010 | 初始 !in_transaction() 且 tx id 0；begin 返回 1；commit 之后 !in_transaction() |
| PILOT-T1-011 | 新建实例 !in_transaction() 且 tx id 0 |
| PILOT-T1-012 | set_current_tx_id(42) 之后回显 42 且 in_transaction() 为 true |
| PILOT-T1-013 | begin/commit/rollback 三个调用都返回 Ok；tx id 原子初值 0，set_current_tx_id(99) 后回显 99 |
| PILOT-T1-017 | wrapper 的 begin 返回 tx id > 0，commit/rollback 返回 Ok |
| PILOT-T1-018 | tx id 0 → 空闲；tx id 7 → in_transaction() 为 true；置回 0 → 复位 |
| PILOT-T1-020 | guard 透传内层 tx 状态；flush() 返回 Ok |
| PILOT-T1-026 | set_current_tx_id(42) 之后回显 42 且 in_transaction() 为 true |

**草案**：`INV-19`（候选）事务标志与事务号必须一致：begin/commit/rollback 之后 `in_transaction()` 与 `current_tx_id()` 互相自洽，wrapper 不得改写内层的 tx 状态。

**为什么现有 18 条覆盖不到**：18 条讲的是**状态量**（行、版本、页、日志段、checkpoint 证书）在事务与恢复中的语义，
没有一条讲**事务标志本身**是否自洽。试点里这类用例有 10 条，占 T1 的三分之一。

### GAP-02 · WAL 记账与条目计数（7 条）

| case_id | 实际断言 |
|---|---|
| PILOT-T1-021 | 5 个已提交事务 × 3 条记录 ⇒ recover() 得 15 条 |
| PILOT-T1-022 | log_begin+log_insert+log_rollback ⇒ recover() 得 3 条且第 3 条类型为 Rollback |
| PILOT-T1-023 | 2 个已提交事务 × 3 条记录 ⇒ recover() 得 6 条 |
| PILOT-T1-024 | begin 后 is_tx_active(7) 为 true；rollback 后为 false |
| PILOT-T1-025 | 新建 WalStorage 的 active_tx_ids() 为空 |
| PILOT-T1-027 | 2 个已提交事务 ⇒ recover() 中 Commit 条目恰 2 条 |
| PILOT-T1-028 | begin+insert+rollback ⇒ recover() 中 Rollback 条目恰 1 条 |

**草案**：`INV-20`（候选）WAL 必须记录每一次事务边界事件：每个已提交事务产生恰好一组 Begin/Commit 记录，每个回滚产生恰好一条 Rollback 记录，且回滚后该事务不得留在活动事务表里。

**为什么现有 18 条覆盖不到**：INV-08 讲的是**同步模式下的 ACK 前 durable_lsn**（要比较 lsn 与提交边界），
INV-16 讲的是**迁移/redo 幂等/恢复身份**。这些用例只数条目，不比较 lsn、不比较 durable 状态，
所以既不是 INV-08 也不是 INV-16；而「撤销了几条」本身在 18 条里没有任何一条覆盖（INV-01 讲的是**存储状态**的恢复）。

### GAP-03 · 事务准入、SQL 分类与守卫放行（3 条）

| case_id | 实际断言 |
|---|---|
| PILOT-T1-005 | 嵌套 begin_transaction() 返回 Err |
| PILOT-T1-015 | classify_sql("BEGIN TRANSACTION") == QueryType::Transaction |
| PILOT-T1-019 | 事务内 assert_dml_safe("insert", "users") 不 panic |

**草案**：`INV-21`（候选）事务准入必须显式：嵌套事务必须被拒绝而不是静默成功；事务控制语句必须被识别为事务语句；事务内的 DML 安全检查必须放行且不得 panic。

**为什么现有 18 条覆盖不到**：INV-14 是**能力**不足时的写前拒绝（后端能力），这三条讲的是**事务语义**层面的准入，
主体不同。

### GAP-04 · semi-sync 主从事务计数（1 条）

| case_id | 实际断言 |
|---|---|
| PILOT-T1-016 | 2 次 yes + 1 次 no 之后，yes_transactions == 2、no_transactions == 1 |

**草案**：`INV-22`（候选）semi-sync 主库必须准确记录同步/异步事务条数（计数本身可核对）。

**为什么现有 18 条覆盖不到**：INV-08 讲的是 ACK 前的 lsn 序，INV-09 讲的是 durable 之后的发布失败语义。
这两条用例只数计数器，不涉及 lsn、不涉及发布结果。

### GAP-05 · 无断言 / 空过 / 恒真断言（4 条）——**这一组不是「缺一条不变量」，是「缺一条真断言」**

| case_id | 实际断言 | 问题 | 在 Task 7 的 38 条内？ |
|---|---|---|---|
| PILOT-T1-009 | **没有任何断言**（函数体只有 `set_current_tx_id(42);`） | 空过也成立，无法证伪 | 是 |
| PILOT-T3-002 | `assert!(!s.dirty)`，但从未写入过数据，dirty 从未被置真 | 对「flush 清 dirty」是空过 | 是 |
| PILOT-T3-003 | **没有任何断言**（只有 create_table 与 flush() 两个 unwrap） | 空过也成立 | 是 |
| PILOT-T5-008 | `assert!(result.is_none() || result.is_some())` | **恒真式**，任何返回值都通过 | **否**（Task 7 只判了 L1 的 44 条；本轮全 75 条重做时新发现） |

**处置建议（需属主裁决）**：这 4 条**不应该靠新增不变量来覆盖**，而应该改测试本体：

1. `PILOT-T1-009` —— §5 已建议补一条可证伪断言（例如 `current_tx_id() == 42`）；本任务只允许改 R3 那两条测试体，故未改。
2. `PILOT-T3-002` —— §5 已建议先写入数据把 dirty 置真再 flush；同上，未改。
3. `PILOT-T3-003` —— §5 已建议在 flush 后断言可观测结果；同上，未改。
4. `PILOT-T5-008` —— 恒真断言，建议换成对 `insert_key_value` 返回值的具体断言。

**在改掉之前，这 4 条对任何不变量都不构成证据，`invariant_ids` 留空。**
其中 3 条在 Task 7 的 38 条清单内（所以「38 条里 35 条修到可判定、3 条仍不可判定」这个统计只算这 3 条），
`PILOT-T5-008` 不在那 38 条内。

### GAP-06 · MVCC 快照可见性与版本回收（4 条）

| case_id | 实际断言 |
|---|---|
| PILOT-T2-003 | 空表 begin_snapshot() == 0，scan_visible(0,0) 为空 |
| PILOT-T2-004 | find_visible(&[], 0, 0) 与 find_visible(&[], u64::MAX, 0) 都返回 None |
| PILOT-T2-005 | 10 个版本 ⇒ version_count()==10；gc(latest, lag=3) ⇒ dropped==6、version_count()==4 |
| PILOT-T2-006 | 缺失 PK → None；PK=1 在 ts1/ts2 取到 [10,20]/[99]；ts3 墓碑 → None |

**草案**：`INV-23`（候选）快照可见性必须与版本链一致：在某个快照上只能看到该快照时刻已提交且未被墓碑遮蔽的版本；GC 只能回收不再被任何可见快照需要的版本。

**为什么现有 18 条覆盖不到**：INV-07 是「未提交私有状态不得进入 committed 可见基线」，
而这 4 条**全部只用已提交版本**（`commit_tx` 都被调用），没有未提交态可对照；INV-13 要求真实并发 + 物理主账联合证明，这 4 条是单线程单元测试。

### GAP-07 · DDL 元数据与触发器（7 条）

| case_id | 实际断言 |
|---|---|
| PILOT-T2-002 | 同名 trigger 重复 create 均返回 Ok；list_triggers("t").len() >= 1 |
| PILOT-T4-001 | compact_table("t") 返回 Ok（返回值被丢弃，未比较墓碑是否清除） |
| PILOT-T4-004 | has_table / list_tables / get_table_info / drop_table 的前后状态 |
| PILOT-T4-005 | 向不存在的表 insert 返回 Err |
| PILOT-T4-006 | load_table("nope") 返回 Ok（no-op） |
| PILOT-T4-007 | load_all() 返回 Ok 且 list_tables() 为空 |
| PILOT-T4-008 | 桩方法返回 Ok 或空值（update/update_if == 0、索引与 trigger 列表为空、has_view 为假、add_column/rename_table 返回 Ok） |

**草案**：`INV-24`（候选）DDL 必须诚实：定义类操作要么真实生效并可被读回，要么显式报错；对不存在的对象的操作语义必须明确（报错或 no-op），不得含糊。

**为什么现有 18 条覆盖不到**：INV-05 讲的是 trigger **触发**产生的可逆变化必须与语句一起原子提交或撤销，
这 7 条都不触发 trigger、也不涉及提交/撤销；INV-14 是**后端能力**不足时的写前拒绝，
`PILOT-T4-005` 拒绝的是**模式错误**（表不存在），主体不同。

### GAP-08 · 存储后端的读写生效（2 条）

| case_id | 实际断言 |
|---|---|
| PILOT-T4-002 | create_table + 2 次 insert ⇒ scan 返回 2 行 |
| PILOT-T4-003 | insert 1 行 ⇒ delete 返回 1 且 scan 为空 |

**草案**：`INV-25`（候选）接受写入的后端必须真的写进去：insert 之后 scan 必须看得到，delete/update 必须返回真实受影响行数。

**为什么现有 18 条覆盖不到**：这 2 条的主体是**能力充足**的后端的 DML 结果；
INV-14 讲的是**能力不足**时必须写前拒绝。已登记的 4 条 INV-14 用例（T4-101..104）覆盖的是**静默成功**那一族
（BinaryTableStorageV2 是已知桩），所以「非桩后端真的写进去了」这一族目前无人认领。

### GAP-09 · 二进制格式与编解码（2 条）

| case_id | 实际断言 |
|---|---|
| PILOT-T4-009 | 三种 BinaryFormatError 的 Display 文本逐字比对 |
| PILOT-T4-010 | write_string / read_string 往返等值 |

**草案**：`INV-26`（候选）二进制格式的错误呈现与字符串编解码必须可往返、错误信息必须可读（人可诊断）。

**为什么现有 18 条覆盖不到**：INV-16 讲的是旧格式**迁移**、重复值 **redo 幂等**与备份/复制/PITR 的**共享恢复身份**，
单个字段的编解码往返不在其中。

### GAP-10 · 缓冲池的记账与初始状态（4 条）

| case_id | 实际断言 |
|---|---|
| PILOT-T3-004 | 一次 get_or_load 之后 disk_reads() == 1 |
| PILOT-T3-005 | insert 一个页之后 disk_writes() == 1 |
| PILOT-T3-007 | BufferPoolStats::new() 的 7 个计数器全为 0 |
| PILOT-T3-008 | BufferPool::new(10) 的 capacity()==10 且 is_empty() |

**草案**：`INV-27`（候选）缓冲池的 I/O 记账必须与实际发生的装载/写回一一对应（不是估计值）。

**为什么现有 18 条覆盖不到**：INV-10 讲的是 **checkpoint 证书**依据 fsync 的 generation 而不是 flush 布尔；
这些用例既不产生证书、也不比较 generation，只数计数器。计数器**本身**在 18 条里无人认领。

### GAP-11 · 索引数据结构自身的 CRUD（7 条）

| case_id | 实际断言 |
|---|---|
| PILOT-T5-001 | 17 次 record_access 后 promoted_count()==1、size()==1 |
| PILOT-T5-002 | 复合键 insert 后 search 返回 Some |
| PILOT-T5-003 | delete(1) 返回 true 且 len()==0 |
| PILOT-T5-004 | 逐条 delete 后 search 为 None、len 依次 2/1/0 |
| PILOT-T5-005 | 两次 insert 后 len()==2 且 !is_empty() |
| PILOT-T5-006 | 100 次 insert 后 len()==100 |
| PILOT-T5-007 | search 返回对应值，未插入的键返回 None |

**草案**：`INV-28`（候选）索引数据结构自身必须正确：插入的每个键值对都可被找回、删除后不可找回、计数与内容一致。

**为什么现有 18 条覆盖不到**：INV-15 讲的是索引/聚类/cache 与**同一 committed RowStore** 的发布/撤销一致——
这 7 条里根本没有 RowStore，也没有发布/撤销，只有索引自己的容器语义。
**这是一个覆盖面缺口，不是不变量写错了。**

### GAP-12 · checkpoint 元数据与保留策略（6 条）

| case_id | 实际断言 |
|---|---|
| PILOT-T6-001 | 5 个 checkpoint 文件 ⇒ cleanup_old_checkpoints(2) 删除 3 个 |
| PILOT-T6-002 | 新建 CheckpointManager 的 last_checkpoint() 为 None |
| PILOT-T6-003 | from_json(to_json(meta)) 后 lsn/timestamp/tx_count 相等 |
| PILOT-T6-004 | interval=3600s、max_wal_size=100MB 下 needs_checkpoint() 为 false |
| PILOT-T6-005 | record_checkpoint(meta) 之后 last_checkpoint().lsn == 1000 |
| PILOT-T6-006 | save_metadata(meta) 之后 load_metadata() 得到 lsn==2000、tx_count==100 |

**草案**：`INV-29`（候选）checkpoint 元数据必须可无损往返；保留策略必须按份数生效且可核对；触发条件必须由配置显式决定。

**为什么现有 18 条覆盖不到**：INV-10 讲的是**证书依据**（fsync 的 generation 而非 flush 布尔），
INV-11 讲的是 **WAL 日志段**的 reclaim 与 segment 切换可恢复。
这 6 条讲的是 checkpoint **元数据对象**本身，既不产生证书、也不涉及日志段。

### GAP-13 · 缓存与页失效 / 回滚的语义（3 条）——**与 INV-15 直接冲突的一条**

| case_id | 实际断言 |
|---|---|
| PILOT-T1-002 | 回滚**不**排空 buffer pool：pages_visited 在回滚前后不变（测试注释明写被回滚事务插入的页仍留在缓存里） |
| PILOT-T3-001 | AHI 页失效后该页条目从索引缓存中消失（size 1→0、lookup 落空） |
| PILOT-T3-006 | 缓冲池读路径记账与命中守卫（pages_visited / cache_misses / cache_hits + 命中时不得调 loader） |

**草案**：`INV-30`（候选，**需属主裁决方向**）缓存与页生命周期必须显式规定：页失效后索引缓存不得再返回该页的 location；事务回滚对缓冲池的排空语义必须被明确规定并被测试钉住。

**为什么现有 18 条覆盖不到，且这里有一条冲突**：
INV-15 要求「索引/聚类/cache 与同一 committed RowStore 发布/撤销一致」。
基线上 `PILOT-T1-002` 钉住的恰恰是**反面**（回滚后缓存不排空），
而 `crates/storage/src/buffer_pool.rs` 里**没有任何 flush / 写回 / fsync 路径**，
`adaptive_hash_index.rs` 里**没有 committed 概念**。
因此在当前基线上，INV-15 这一族要么**不可满足**，要么必须先改生产语义——两者都不是本任务能做的（硬边界：不得写内核逻辑）。
**请属主裁决**：是修改 INV-15 的措辞、还是把缓存的撤销一致性排进后续 C01-10/11 的范围。

### GAP-14 · 探针源码不在仓内，无法从源码判定（3 条）——**「标签不是判据」的代价**

| case_id | 探针 | 状况 | 曾经（**已撤回**）贴过的 id |
|---|---|---|---|
| PILOT-T5-101 | `(odku-rollback-noop)` | 探针源未随基线迁入（未注册为 cargo test target），执行状态 NOT_RUN；`red_identity.source_sha256` 显式为 null（`identity_gap`） | INV-01（+ 原来的 INV-05） |
| PILOT-T6-001B | `(crash-recovery-kill-9)` | bespoke 探针，`probe_registry` 未绑定该标签，执行状态 NOT_RUN | INV-12 + INV-07 |
| PILOT-T6-002B | `(checkpoint-torn-write)` | bespoke 探针，`probe_registry` 未绑定该标签，执行状态 NOT_RUN | INV-10 + INV-11 |

**为什么对不上**：这三条**没有可读的测试源码**，因此无法判定它们「实际断言了什么」。
标签与声明目的不是判据（§0.4），所以 `invariant_ids` 留空。

**草案**：**不提议新不变量**——提议的是**处置**：
把这三条的**探针源码迁入仓内**（注册为 `[[test]]` 或落到 `crates/storage/tests/`），
迁入后按 Step 2 重新逐案判定；若届时仍对不上任何不变量，再进缺口提案。

**这是本轮对属主最需要决策的一条**：按 INV-17 的口径，
「必需 case 的 NOT_RUN 阻断认证」，而这三条正是「探针未迁入」造成的 NOT_RUN（L1-DIFF §五-3 记的同一类装配缺口）。

---

## 2. 63 条空映射用例的索引（逐条可查）

| 缺口 | 用例 |
|---|---|
| GAP-01 | PILOT-T1-001、PILOT-T1-008、PILOT-T1-010、PILOT-T1-011、PILOT-T1-012、PILOT-T1-013、PILOT-T1-017、PILOT-T1-018、PILOT-T1-020、PILOT-T1-026 |
| GAP-02 | PILOT-T1-021、PILOT-T1-022、PILOT-T1-023、PILOT-T1-024、PILOT-T1-025、PILOT-T1-027、PILOT-T1-028 |
| GAP-03 | PILOT-T1-005、PILOT-T1-015、PILOT-T1-019 |
| GAP-04 | PILOT-T1-016 |
| GAP-05 | PILOT-T1-009、PILOT-T3-002、PILOT-T3-003、PILOT-T5-008 |
| GAP-06 | PILOT-T2-003、PILOT-T2-004、PILOT-T2-005、PILOT-T2-006 |
| GAP-07 | PILOT-T2-002、PILOT-T4-001、PILOT-T4-004、PILOT-T4-005、PILOT-T4-006、PILOT-T4-007、PILOT-T4-008 |
| GAP-08 | PILOT-T4-002、PILOT-T4-003 |
| GAP-09 | PILOT-T4-009、PILOT-T4-010 |
| GAP-10 | PILOT-T3-004、PILOT-T3-005、PILOT-T3-007、PILOT-T3-008 |
| GAP-11 | PILOT-T5-001、PILOT-T5-002、PILOT-T5-003、PILOT-T5-004、PILOT-T5-005、PILOT-T5-006、PILOT-T5-007 |
| GAP-12 | PILOT-T6-001、PILOT-T6-002、PILOT-T6-003、PILOT-T6-004、PILOT-T6-005、PILOT-T6-006 |
| GAP-13 | PILOT-T1-002、PILOT-T3-001、PILOT-T3-006 |
| GAP-14 | PILOT-T5-101、PILOT-T6-001B、PILOT-T6-002B |

合计 63 条（10 + 7 + 3 + 1 + 4 + 4 + 7 + 2 + 2 + 4 + 7 + 6 + 3 + 3 = 63）。

矩阵里每条空映射的 `invariant_mapping_basis` 都写明了「为什么对不上」，
`invariant_mapping_status` 一律为 `NO_INVARIANT_MATCH`；
`verify_matrix.py` 的 M7 会把这份索引表与矩阵的空映射**双向**对齐：
表里有而矩阵里不是空映射 → 判红；矩阵里是空映射而表里没有 → 判红。

---

## 3. 三条探针的处置记录（供属主复核）

| case_id | 标签 | 声明目的 | 本轮处置 | 撤回的 id |
|---|---|---|---|---|
| PILOT-T5-101 | `(odku-rollback-noop)` | ODKU 回滚不空转 | `invariant_ids = []`，登记 GAP-14 | INV-01、INV-05 |
| PILOT-T6-001B | `(crash-recovery-kill-9)` | 崩溃恢复 | `invariant_ids = []`，登记 GAP-14 | INV-12、INV-07 |
| PILOT-T6-002B | `(checkpoint-torn-write)` | checkpoint 安全性 | `invariant_ids = []`，登记 GAP-14 | INV-10、INV-11 |

**撤回理由**（逐条）：这三条的测试源码都不在本仓，因此**无法按 Step 2 判定它们实际断言了什么**；
Task 5 的分桶映射与本轮第一轮的「按标签贴 id」是同一类错误的不同表现。

**它们不是「没被覆盖」，是「无法判定」**——这两件事在放行口径上必须分开：

- **无法判定** → 该 case 不得计入不变量覆盖（§0.3 的 5 条 / 12 条用例口径已经排除它们）；
- **没被覆盖** → 有一条可判定的用例存在，但它断言的东西不对应任何已登记不变量。

**待办**：把探针源码迁入仓内 → 重新判定 → 若对不上任何不变量，再进缺口提案。
迁入之前，这三条在 Task 9 的放行裁决里应按 `NOT_RUN`（装配缺口）处理。

---

## 4. 给属主的三个问题

1. **GAP-05 的 4 条**（T1-009 / T3-002 / T3-003 / T5-008）应该改测试本体（补断言 / 换成具体断言），
   而不是补不变量。是否授权本任务之外的改动？
2. **GAP-13 与 INV-15 的冲突**：基线的缓冲池没有事务性排空、没有写回路径，
   「cache 与 committed RowStore 撤销一致」在当前基线上不可满足。改 INV-15 的措辞，还是排进 C01-10/11？
3. **18 条不变量里有 13 条零覆盖**（其中 INV-17/INV-18 天然是流程不变量，应由门禁而非单测证明；
   INV-03/05/06/08/09/10/11/12/13/15/16 需要的用例在试点集里根本不存在或无法判定）。
   零覆盖的不变量是否应当阻断阶段出口（按 INV-17 的口径应当阻断）？
   **另外**：GAP-14 的三条探针源码要不要迁入仓内？这决定它们是「NOT_RUN（装配缺口）」还是「可判定」。
