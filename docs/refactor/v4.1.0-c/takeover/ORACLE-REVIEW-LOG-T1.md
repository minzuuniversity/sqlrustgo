# Oracle 审查日志 · T1（及第 2 段 T2/T3）

> **本记录的作者（判定人，非签署人）**：`task7-executor/minimax-m3.1-flash`
> **审查会话 id**：`T7-ORACLE-JUDGEMENT-20261123T161318Z`
> **判定时间**：2026-11-23T16:13:18Z
> **工作区**：`/Volumes/workspace/dev/wt-v410-base`（分支 `base/v4.1.0-frozen`）
> **实读时的工作树 HEAD**：`2923612f5151501807f7baf0f0f4dc3513d8a1a7`
> **冻结基线 `base_sha`**：`fd849ae20541901d5e59f45cd3ba2c51093f5d5e`（矩阵每条 case 的 `base_sha` 同值）
> **本任务**：V410-READINESS-IMPLEMENTATION-PLAN / Task 7（Oracle 签署，两段）
>
> **⚠ Oracle 签署步骤的状态：BLOCKED。** 本记录里**没有任何一条是已完成的 Oracle 签名**。
> 矩阵的 `assertion_reviewed_by` 在 44 条上**全部缺席**——该字段保留给属主授权的独立 Reviewer 副签。
> 详见 §0 与 §9（Task 9 残余登记）。

## 0. 判定人身份、Oracle 签署步骤的 BLOCKED 状态

### 0.1 治理要求

`docs/refactor/v4.1.0-c/takeover/V410-READINESS-IMPLEMENTATION-PLAN.md` L767-772（Task 7 Step 1）：

> 签署人：**属主授权的独立 Reviewer**（Codex 或 DSH Reviewer 池），**不得**是该场景的实现者，
> **且不得**是执行本次复跑的 agent。签署人标识须具名并写入 `assertion_reviewed_by`。
>
> **属主不介入签署环节**（属主已明确仅在最终放行裁决时介入）。
> 若独立 Reviewer 不可得，本任务**不得**由实施者自签——那会使 R-ORACLE 失效。
> 此时应把该任务标记为 BLOCKED 并在 Task 9 的放行裁决记录中列为残余项。

`docs/refactor/v4.1.0-c/takeover/V410-REFACTOR-READINESS-DESIGN.md` L490-497（A3.4）同样规定：
签署人统一为**属主授权的独立 Reviewer（Codex 或 DSH Reviewer 池）**；契约字段是
`oracle_owner: "Owner/Independent Reviewer not implementer"`。

### 0.2 实际状况

| 角色 | 承担者 | 与本记录的关系 |
|---|---|---|
| 75 场景矩阵的编写者 | Task 5 的执行 agent | 不是本人 |
| L1 切片（44 条）的复跑执行者 | Task 6 的执行 agent | 不是本人（本人未执行任何 cargo 测试） |
| 被测 44 条用例的测试代码实现者 | 仓库既有历史作者 | 不是本人（本人未编写、未修改这些测试） |
| **本次判定人** | **`task7-executor/minimax-m3.1-flash`**（Claude Code agent，模型 MiniMax-M3.1-Flash），即 **Task 7 本身的执行者** | 本人 |
| 属主授权的独立 Reviewer（Codex 或 DSH Reviewer 池） | **不存在** | 无从取得 |

### 0.3 结论：Oracle 签署步骤 BLOCKED

本人**不在**「Codex 或 DSH Reviewer 池」里，也**没有**属主授权；而本人是 Task 7 本身的执行者，
按计划 L770-772 属于「本任务不得由实施者自签——那会使 R-ORACLE 失效」的情形。
**故 Oracle 签署步骤标记为 BLOCKED。**

Round 0 的做法有误：本人当时以 `oracle-reviewer/task7/minimax-m3.1-flash` 写入 `assertion_reviewed_by`，
等于以实施者身份自签。该 8 条签名已在 Round 1 全部**撤回**（见 §1.2）。

**保留下来的**：本人对 44 条断言的**判定**（哪些可独立判定、哪些不可、理由是什么）。
这些判定是技术性的、可被独立复现的，但不构成签署。副签需要属主授权的独立 Reviewer 完成。

### 0.4 判定内容的绑定

| 绑定项 | 值 | 记录位置 |
|---|---|---|
| 审查会话 id | `T7-ORACLE-JUDGEMENT-20261123T161318Z` | 日志 §0 头 + 矩阵 `assertion_review_session` |
| 实读时的工作树 HEAD | `2923612f5151501807f7baf0f0f4dc3513d8a1a7` | 日志 §0 头 |
| 冻结基线 `base_sha` | `fd849ae20541901d5e59f45cd3ba2c51093f5d5e` | 日志 §0 头 + 矩阵 `assertion_reviewed_commit` + 矩阵原有 `base_sha` |
| 测试源文件摘要 | 每个 case 的 `assertion_source_sha256` | 矩阵 |

已核：44 条涉及的 **17 个**测试源文件，在 `2923612f5151501807f7baf0f0f4dc3513d8a1a7` 与 `fd849ae20541901d5e59f45cd3ba2c51093f5d5e` 两处**逐字节相同**，
所以 `assertion_source_sha256` 对两处都成立。

## 1. 判定口径与记录字段

### 1.1 判定口径

对每一条场景，只读三样东西：

1. **断言文本**：`source.declared_invariant`；
2. **它映射到的登记表原文**：`invariant_ids` → `docs/releases/v4.1.0/refactoring/TRANSACTION_INVARIANTS.md`；
3. **测试源**：`source.test_name` 所在的**源文件**里那个测试函数的源码。

**没有读**被测函数的实现，**没有读**任何执行日志（见 §7 的完整声明与其中两处如实披露的边界）。
判定只依据「测试源码里那条断言本身」。

**判定为可独立判定（JUDGED）的充要条件**：

- (a) 测试源码里的断言，其被比较的可观测量由断言文本所指的机制直接决定（测试真的走了那套机制）；
- (b) 该断言是**可证伪**的——违反断言文本所声明的性质会让测试失败（不是只检查调用返回 `Ok`、不是只检查一个与该性质无关的标志）；
- (c) 该断言不与 `invariant_ids` 指向的登记表原文相抵触；
- (d) 该断言所覆盖的主体，与 `invariant_ids` 指向的登记表原文是**同一个主体**——组标签不算数。

四条里缺任何一条即为 `ASSERTION_UNREVIEWABLE`。

（(d) 是 Round 1 补上的：Round 0 只查了 (a)(b)(c)，对 `PILOT-T3-001` / `PILOT-T3-006` 用了组标签当断言，
已被 Round 1 改判，见 §3、§4。）

### 1.2 不可判定的原因分类

| 码 | 含义 |
|---|---|
| R1 | 无状态比较：只断言 `Ok` / 标志位 / 与性质无关的量；或断言本身是**空过**（前置状态从未建立，空过也成立） |
| R2 | 断言的是别的性质：测试走的是另一套机制或另一个主体，与本场景声明的不变量无关 |
| R3 | 只钉住记账计数：断言的是 I/O 计数一类与「可见性 / 写回时机」没有直接可判定关系的量 |

### 1.3 矩阵字段的读法（Round 1 改写）

| 字段 | 含义 | 取值 |
|---|---|---|
| `assertion_reviewed_by` | **签署人**（属主授权的独立 Reviewer 副签人） | **44 条上全部缺席** —— 副签尚未发生 |
| `assertion_review_verdict` | 本次判定的结论 | `JUDGED_PENDING_COUNTERSIGNATURE`（6）/ `ASSERTION_UNREVIEWABLE`（38） |
| `assertion_review_reason` | 不可判定的原因码 | `R1` / `R2` / `R3`（只在 `ASSERTION_UNREVIEWABLE` 的 38 条上出现） |
| `assertion_review_performed_by` | **判定人**（不是签署人） | `task7-executor/minimax-m3.1-flash`（44 条一致） |
| `assertion_review_session` | 审查会话 id | `T7-ORACLE-JUDGEMENT-20261123T161318Z`（44 条一致） |
| `assertion_reviewed_at` | 判定时间 | `2026-11-23T16:13:18Z`（44 条一致） |
| `assertion_reviewed_commit` | 判定所依据内容的 commit | `fd849ae20541901d5e59f45cd3ba2c51093f5d5e`（44 条一致） |
| `assertion_source_sha256` | 判定所读的**测试源文件**的 sha256 | 44 条各自的值（**17 个不同文件**） |

**下游读法**：

- 「已审查」= 有 `assertion_review_verdict` ⇒ **44 条**；
- 「已副签」= 有 `assertion_reviewed_by` ⇒ **0 条**；
- 「判定为不可独立判定」= `assertion_review_verdict == "ASSERTION_UNREVIEWABLE"` ⇒ **38 条**；
- 「判定为可独立判定、待副签」= `JUDGED_PENDING_COUNTERSIGNATURE` ⇒ **6 条**。

Round 0 曾用「三个 `assertion_*` 字段同时存在 = 已签」来编码，并让未签署的 38 条**一个字段都不写**；
那样矩阵读起来像「只审查了 8 条」。Round 1 改成：**44 条都带判定记录，只有副签人字段缺席**。

## 2. 结果总览

| 量 | 值 | 记录在哪 |
|---|---|---|
| L1 场景总数（T1+T2+T3） | 44 | 矩阵 |
| **已审查（判定完成）** | **44 / 44** | 矩阵 `assertion_review_verdict` 存在于 44 条 |
| **判定为可独立判定（待副签）** | **6** | `JUDGED_PENDING_COUNTERSIGNATURE` |
| **判定为不可独立判定** | **38** | `ASSERTION_UNREVIEWABLE`（R1 12 / R2 22 / R3 4） |
| **已副签（Oracle 签名完成）** | **0** | `assertion_reviewed_by` 在 44 条上全部缺席 |
| Oracle 签署步骤状态 | **BLOCKED** | 本日志 §0.3、§9 |

| 段 | 范围 | 条数 | 已审查 | 可独立判定 | 不可独立判定 | 已副签 |
|---|---|---|---|---|---|---|
| 第 1 段 | `feature == "T1"` | 30 | 30 | 5 | 25 | **0** |
| 第 2 段 | `feature == "T2"`（6）+ `"T3"`（8） | 14 | 14 | 1 | 13 | **0** |
| 合计 | L1 | 44 | 44 | 6 | 38 | **0** |

**对阶段出口的影响**：L1 的 42 条 PASS 里，**带完整 Oracle 签名的为 0 条**——签署步骤 BLOCKED。
其中 5 条 PASS 判定为「可独立判定、待副签」
（`PILOT-T1-003`、`PILOT-T1-004`、`PILOT-T1-006`、`PILOT-T1-007`、`PILOT-T2-001`），
另 1 条（`PILOT-T1-102`）是 NOT_RUN。**其余 37 条 PASS 判定为不可独立判定，不计入阶段出口。**

## 3. 第 1 段 —— T1（30 条）逐条判定

断言文本：`undo log 语义：回滚后状态 == 事务前`（28 条）／`undo log 语义`（1 条，T1-101）／
`SAVEPOINT 部分撤销`（1 条，T1-102）；映射 `INV-01`（「rollback 精确恢复原多重集、每个 mutation 只撤销一次」），
T1-102 另含 `INV-04`（「savepoint cursor 仅属本 tx，部分回滚不污染顶层恢复」）。

| case_id | feature | 测试源（source.test_name） | 声明的不变量 | invariant_ids | 判定 | 原因码 | 已副签 | 测试源文件 | 备注 |
|---|---|---|---|---|---|---|---|---|---|
| `PILOT-T1-001` | T1 | `append_only_storage::tests::transaction_lifecycle` | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE | R1 | 无 | `crates/storage/src/append_only_storage.rs` | 事务生命周期冒烟：begin/commit/rollback 之后只断言 in_transaction() 标志与 current_tx_id 回显；函数内没有任何数据写入，也没有回滚前后的状态比较，「回滚后状态 == 事务前」无从判定。 |
| `PILOT-T1-002` | T1 | `engine::memory_storage_rollback_after_bufferpool_insert_does_not_change_pages_visited` | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE | R2 | 无 | `crates/storage/src/engine.rs` | 断言的是 buffer pool 的 pages_visited 计数在回滚前后不变；测试自身注释写明被回滚事务插入的页仍留在缓存里（BufferPool 没有事务性排空）。既没有比较行的前后状态，钉住的还是与「回滚后状态 == 事务前」相反的缓存语义。 |
| `PILOT-T1-003` | T1 | `engine::tests::rollback_actually_undoes_an_insert_4948` | undo log 语义：回滚后状态 == 事务前 | INV-01 | JUDGED_PENDING_COUNTERSIGNATURE | — | 无 | `crates/storage/src/engine.rs` | 事务内 insert 后 scan().len() 由 5 变 6，回滚后断言回到 5：被比较的量（可见行数）在回滚前后是同一指标且被钉死，「没撤销」或「撤销两次」都会红。限制：只钉住基数，未钉住行身份。 |
| `PILOT-T1-004` | T1 | `engine::tests::rollback_of_update_restores_only_the_changed_row_5072` | undo log 语义：回滚后状态 == 事务前 | INV-01 | JUDGED_PENDING_COUNTERSIGNATURE | — | 无 | `crates/storage/src/engine.rs` | 行内容级比较：回滚后 scan() 全等 [[1,1],[1,99]]，与事务前逐行一致，满足 INV-01「精确恢复原多重集」且「每个 mutation 只撤销一次」。 |
| `PILOT-T1-005` | T1 | `engine::tests::test_nested_transaction_returns_error` | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE | R2 | 无 | `crates/storage/src/engine.rs` | 只断言嵌套 begin_transaction() 返回 Err；与回滚后的状态无关。 |
| `PILOT-T1-006` | T1 | `engine::tests::test_rollback_removes_inserted_rows` | undo log 语义：回滚后状态 == 事务前 | INV-01 | JUDGED_PENDING_COUNTERSIGNATURE | — | 无 | `crates/storage/src/engine.rs` | 回滚后断言行数回到 1 且幸存行内容等于 [Value::Integer(0)]，与事务前完全一致。 |
| `PILOT-T1-007` | T1 | `engine::tests::test_rollback_restores_deleted_rows` | undo log 语义：回滚后状态 == 事务前 | INV-01 | JUDGED_PENDING_COUNTERSIGNATURE | — | 无 | `crates/storage/src/engine.rs` | 删除一行后回滚，断言 scan().len() == 2（事务前为 2）：比较量是行数、回滚前后同一指标，「没撤销」或「撤销两次」都会红。限制：只钉住基数，未钉住行身份。 |
| `PILOT-T1-008` | T1 | `engine::tests::test_storage_engine_begin_rollback` | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE | R1 | 无 | `crates/storage/src/engine.rs` | 只断言 begin 返回的事务号 == 1 与回滚后 !in_transaction()；无数据、无前后状态比较。 |
| `PILOT-T1-009` | T1 | `engine::tests::test_storage_engine_set_current_tx_id` | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE | R1 | 无 | `crates/storage/src/engine.rs` | 函数体只有 set_current_tx_id(42);，没有任何断言，也不涉及回滚。 |
| `PILOT-T1-010` | T1 | `engine::tests::test_storage_engine_transaction` | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE | R1 | 无 | `crates/storage/src/engine.rs` | 只断言 begin/commit 的标志与事务号；既没有回滚，也没有状态比较。 |
| `PILOT-T1-011` | T1 | `file_storage::tests::test_in_transaction` | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE | R2 | 无 | `crates/storage/src/file_storage.rs` | 只断言新实例 !in_transaction() 且 current_tx_id() == 0；不涉及回滚。 |
| `PILOT-T1-012` | T1 | `file_storage::tests::test_set_current_tx_id` | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE | R2 | 无 | `crates/storage/src/file_storage.rs` | 只断言 set_current_tx_id(42) 之后的回显与 in_transaction()；不涉及回滚或状态恢复。 |
| `PILOT-T1-013` | T1 | `parallel_wal_storage::tests::transaction_lifecycle_via_wal` | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE | R1 | 无 | `crates/storage/src/parallel_wal_storage.rs` | 注释自陈「只断言 begin/commit 管线成功」「commit 不重置 current_tx_id；跳过该断言」；函数里唯一的断言是 current_tx_id 原子的回显，与状态恢复无关。 |
| `PILOT-T1-014` | T1 | `pitr::tests::open_transaction_at_target_is_not_applied` | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE | R2 | 无 | `crates/storage/src/pitr.rs` | 断言的是 PITR 重放语义（未提交事务的行不进入恢复结果：entries_applied == 0、ids(&s).is_empty()）；测试里没有 rollback，机制与「undo log 回滚恢复」不同。 |
| `PILOT-T1-015` | T1 | `read_write_split::tests::test_classify_sql_transaction` | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE | R2 | 无 | `crates/storage/src/read_write_split.rs` | 只断言 classify_sql("BEGIN TRANSACTION") 被分类为 QueryType::Transaction；与回滚后的状态无关。 |
| `PILOT-T1-016` | T1 | `semisync::tests::test_semi_sync_master_record_transactions` | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE | R2 | 无 | `crates/storage/src/semisync.rs` | 只断言 semi-sync 状态计数（yes/no transactions）；与回滚语义无关。 |
| `PILOT-T1-017` | T1 | `table_level_storage::tests::transaction_lifecycle_passthrough` | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE | R1 | 无 | `crates/storage/src/table_level_storage.rs` | 只断言 tx > 0 并调用 commit/rollback；无数据、无前后状态比较。 |
| `PILOT-T1-018` | T1 | `vtu_guard::tests::test_3129_memory_storage_in_transaction_reflects_tx_id` | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE | R2 | 无 | `crates/storage/src/vtu_guard.rs` | 只断言 in_transaction() 随 current_tx_id 联动；不涉及回滚。 |
| `PILOT-T1-019` | T1 | `vtu_guard::tests::test_3129_vtu_guard_passes_in_tx_with_memory_storage` | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE | R2 | 无 | `crates/storage/src/vtu_guard.rs` | 只断言「在事务内调用 assert_dml_safe 不 panic」；不涉及回滚与状态比较。 |
| `PILOT-T1-020` | T1 | `vtu_guard::tests::test_vtu_guard_tx_state_via_set_current_tx_id` | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE | R2 | 无 | `crates/storage/src/vtu_guard.rs` | 只断言 guard 的 tx 状态透传；不涉及回滚与状态比较。 |
| `PILOT-T1-021` | T1 | `wal_legacy::tests::test_wal_5_transactions` | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE | R2 | 无 | `crates/storage/src/wal_legacy.rs` | 5 个已提交事务的 WAL 记录条数（15）；无回滚、无状态比较。 |
| `PILOT-T1-022` | T1 | `wal_legacy::tests::test_wal_manager_log_rollback` | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE | R1 | 无 | `crates/storage/src/wal_legacy.rs` | 断言的是 WAL 里写进了 3 条记录且第 3 条类型为 Rollback；没有比较任何存储状态，撤没撤销看不出来。 |
| `PILOT-T1-023` | T1 | `wal_legacy::tests::test_wal_multiple_transactions` | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE | R2 | 无 | `crates/storage/src/wal_legacy.rs` | 2 个已提交事务的 WAL 条数（6）；无回滚、无状态比较。 |
| `PILOT-T1-024` | T1 | `wal_storage::tests::test_active_txs_lifecycle_begin_rollback` | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE | R1 | 无 | `crates/storage/src/wal_storage.rs` | 断言的是 is_tx_active(7) 在回滚后为 false（active_txs 记账），不是存储状态恢复。 |
| `PILOT-T1-025` | T1 | `wal_storage::tests::test_wal_active_tx_ids_empty` | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE | R2 | 无 | `crates/storage/src/wal_storage.rs` | 只断言 active_tx_ids() 为空。 |
| `PILOT-T1-026` | T1 | `wal_storage::tests::test_wal_set_current_tx_id` | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE | R2 | 无 | `crates/storage/src/wal_storage.rs` | 只断言 set_current_tx_id(42) 的回显与 in_transaction()。 |
| `PILOT-T1-027` | T1 | `wal_storage::tests::test_wal_storage_multiple_transactions` | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE | R2 | 无 | `crates/storage/src/wal_storage.rs` | 只断言 WAL 里有 2 条 Commit 记录。 |
| `PILOT-T1-028` | T1 | `wal_storage::tests::test_wal_storage_rollback` | undo log 语义：回滚后状态 == 事务前 | INV-01 | ASSERTION_UNREVIEWABLE | R1 | 无 | `crates/storage/src/wal_storage.rs` | 只断言 WAL 里有 1 条 Rollback 记录；没有比较回滚前后的存储状态。 |
| `PILOT-T1-101` | T1 | `tf_t1_01_insert_then_rollback` | undo log 语义 | INV-01 | ASSERTION_UNREVIEWABLE | R1 | 无 | `tests/integration/test_factory/tf_t1_undo_invariants.rs` | 文档注释声明「INSERT 后 ROLLBACK 必须撤销该插入（undo log 清空）」，但函数里唯一的断言是「回滚到一个刚建的 savepoint 不会弹出任何 undo」（undo_log_len() 仍为 2）；rollback_to_noop 的返回值被 .ok() 丢弃，applied 计数器从未被断言。断言与声明相反。 |
| `PILOT-T1-102` | T1 | `tf_t1_02_savepoint_partial_rollback` | SAVEPOINT 部分撤销 | INV-04, INV-01 | JUDGED_PENDING_COUNTERSIGNATURE | — | 无 | `tests/integration/test_factory/tf_t1_undo_invariants.rs` | 断言 undo_log_len() 由 3 降到 1（savepoint 建在第 1 条 undo 之后）：被比较的量明确、回滚前后同一指标被钉死，差值 2 即「每个 mutation 只撤销一次」，且 savepoint 之前的 undo 记录未被弹出（INV-04「不污染顶层恢复」）。限制：回调闭包未计数，无法证明弹出的正是那两条。 |

## 4. 第 2 段 —— T2（6 条）+ T3（8 条）逐条判定

### 4.1 第 2 段的前提条件：未成立，作业是在错误的指示下做的

**治理前提（绝对）**：

- `V410-READINESS-IMPLEMENTATION-PLAN.md` L798：**前提**：第 1 段流程验证通过（Step 1 无 `ASSERTION_UNREVIEWABLE`）。
- `V410-REFACTOR-READINESS-DESIGN.md` L502（第 2 段表格的「前置」列）：第 1 段流程验证通过**且**无 `ASSERTION_UNREVIEWABLE`。
- `task-7-brief.md` L44：**前提**：第 1 段流程验证通过（Step 1 无 `ASSERTION_UNREVIEWABLE`）。

**实际**：第 1 段产出 25 条 `ASSERTION_UNREVIEWABLE`，**前提不成立**。

**治理文件规定的处置**（`V410-REFACTOR-READINESS-DESIGN.md` L516-517）：

> 若第 1 段暴露流程缺陷（如「断言文本无法在不读实现的前提下独立判定」），
> 则 T1 的 `PASS` 不得计入阶段出口，**须先修正断言的可判定性**。

即：治理文件要求**停下来修断言**，而不是继续做第 2 段。

**实际发生的事**：本人依当时 dispatch 提示词里的一段话继续了第 2 段。那段话写的是
「若第 1 段产出的 `ASSERTION_UNREVIEWABLE` 判定为可通过澄清断言文本修复（而非改测试），
可以继续第 2 段并上报」。

**Round 1 更正**：

- 「澄清」二字在 `V410-READINESS-IMPLEMENTATION-PLAN.md` 与 `V410-REFACTOR-READINESS-DESIGN.md` 中
  **一次也没有出现**（已逐字检索）；`task-7-brief.md` L44 说的是相反的话（前提是绝对的）。
  那段话来自 **dispatch 提示词**，不是治理文件，也不是 brief——**dispatch 提示词当时是错的**。
  Round 0 的日志与报告把它记成「任务书给的口径」是错误引用，本轮更正。
- 因此：**第 2 段作业是在治理前提未成立、且指示本身有误的情况下做的。**
- **第 2 段的 3 条 Round 0 签名（`PILOT-T2-001`、`PILOT-T3-001`、`PILOT-T3-006`）已全部撤回**；
  第 2 段只保留**判定**，且明确标注为「未签署」。
- 治理文件要求的 BLOCKED 已按 §0.3 执行（Oracle 签署步骤 BLOCKED，残余登记见 §9）。

### 4.2 逐条判定

T2 声明的不变量：`逻辑行身份：相同值不等同同一行` → `INV-02`（「相同行值独立 RowId；UPDATE 主键不改内部 ID，迁移/复用遵守 generation」）。

T3 声明的不变量：`buffer/flush：页可见性与写回时机` → `INV-07`（「未提交私有状态不能进入 committed snapshot/recovery 可见基线」）
+ `INV-10`（「checkpoint 证书基于 fsync 数据/目录的 generation，不是 flush 调用布尔」）。

| case_id | feature | 测试源（source.test_name） | 声明的不变量 | invariant_ids | 判定 | 原因码 | 已副签 | 测试源文件 | 备注 |
|---|---|---|---|---|---|---|---|---|---|
| `PILOT-T2-001` | T2 | `bplus_tree::tests::duplicate_keys_keep_every_row_id` | 逻辑行身份：相同值不等同同一行 | INV-02 | JUDGED_PENDING_COUNTERSIGNATURE | — | 无 | `crates/storage/src/bplus_tree/mod.rs` | 【Round 1 保留判定，但撤签】同一个 key 42 插入 5 个不同 row_id，断言 search_all(42) == vec![0,1,2,3,4] 且 len() == 1（只有一个不同的 key）：直接断言 INV-02「相同行值独立 RowId」。本判定本身成立，但第 2 段作业是在治理前提未成立的情况下做的（见 §4.1），且 Oracle 签署步骤整体 BLOCKED（见 §0），故本条不产生任何签名。 |
| `PILOT-T2-002` | T2 | `file_storage::tests::test_save_trigger_load_trigger_duplicate` | 逻辑行身份：相同值不等同同一行 | INV-02 | ASSERTION_UNREVIEWABLE | R2 | 无 | `crates/storage/src/file_storage.rs` | 断言的是同名 trigger 重复创建不报错且 list_triggers("t").len() >= 1；与「逻辑行身份」无关，且 >= 1 未指明重复项是否被保留。 |
| `PILOT-T2-003` | T2 | `mvcc::tests::test_empty_table_snapshot` | 逻辑行身份：相同值不等同同一行 | INV-02 | ASSERTION_UNREVIEWABLE | R2 | 无 | `crates/storage/src/mvcc.rs` | 只断言空表的 begin_snapshot() == 0 与 scan_visible 为空；与行身份无关。 |
| `PILOT-T2-004` | T2 | `mvcc::tests::test_find_visible_at_snapshot_before_all` | 逻辑行身份：相同值不等同同一行 | INV-02 | ASSERTION_UNREVIEWABLE | R2 | 无 | `crates/storage/src/mvcc.rs` | 只断言空版本链上 find_visible 返回 None；与行身份无关。 |
| `PILOT-T2-005` | T2 | `mvcc::tests::test_gc_drops_old_versions` | 逻辑行身份：相同值不等同同一行 | INV-02 | ASSERTION_UNREVIEWABLE | R2 | 无 | `crates/storage/src/mvcc.rs` | 只断言 GC 后 version_count() 由 10 变 4、dropped == 6；是版本回收，与行身份无关。 |
| `PILOT-T2-006` | T2 | `mvcc::tests::test_get_visible_pk_lookup` | 逻辑行身份：相同值不等同同一行 | INV-02 | ASSERTION_UNREVIEWABLE | R2 | 无 | `crates/storage/src/mvcc.rs` | 断言的是按 PK 在各快照上取到哪一版、墓碑返回 None；全用例只有一行一个值，没有「相同值不同行」的场景。 |
| `PILOT-T3-001` | T3 | `adaptive_hash_index::tests::test_invalidate_page_removes_entries` | buffer/flush：页可见性与写回时机 | INV-07, INV-10 | ASSERTION_UNREVIEWABLE | R2 | 无 | `crates/storage/src/adaptive_hash_index.rs` | 【Round 1 改判】本用例断言的是 AHI 缓存失效（页失效后条目被撤销、lookup 变 None），与映射的 INV-07「未提交私有状态不能进入 committed snapshot/recovery 可见基线」和 INV-10「checkpoint 证书基于 fsync 数据/目录的 generation，不是 flush 调用布尔」都不是同一个主体：前者是「未提交态的可见基线」，后者是「checkpoint 证书的根据」，本用例两处都没触及。Round 0 曾把它判为可判定（并附「INV-10 未触及」的限制说明），那是把组标签当成了断言——已改判为 R2。 |
| `PILOT-T3-002` | T3 | `append_only_storage::tests::flush_clears_dirty` | buffer/flush：页可见性与写回时机 | INV-07, INV-10 | ASSERTION_UNREVIEWABLE | R1 | 无 | `crates/storage/src/append_only_storage.rs` | flush() 之后断言 !s.dirty；但用例在此之前从未写入任何数据，dirty 从未被置真，该断言对「flush 清 dirty」是空过（flush 即使什么都不做也通过）。 |
| `PILOT-T3-003` | T3 | `binary_storage::tests::test_bin_flush` | buffer/flush：页可见性与写回时机 | INV-07, INV-10 | ASSERTION_UNREVIEWABLE | R1 | 无 | `crates/storage/src/binary_storage.rs` | 函数体只有 create_table 与 flush().unwrap()，没有任何断言。 |
| `PILOT-T3-004` | T3 | `buffer_pool::tests::buffer_pool_bumps_disk_reads_on_loader_invocation` | buffer/flush：页可见性与写回时机 | INV-07, INV-10 | ASSERTION_UNREVIEWABLE | R3 | 无 | `crates/storage/src/buffer_pool.rs` | 只断言 disk_reads() == 1；这是 I/O 记账计数，不是页可见性，也不是写回时机。 |
| `PILOT-T3-005` | T3 | `buffer_pool::tests::buffer_pool_bumps_disk_writes_on_insert` | buffer/flush：页可见性与写回时机 | INV-07, INV-10 | ASSERTION_UNREVIEWABLE | R3 | 无 | `crates/storage/src/buffer_pool.rs` | 只断言 disk_writes() == 1；同上，只是记账计数。 |
| `PILOT-T3-006` | T3 | `buffer_pool::tests::buffer_pool_bumps_pages_visited_on_get_or_load` | buffer/flush：页可见性与写回时机 | INV-07, INV-10 | ASSERTION_UNREVIEWABLE | R3 | 无 | `crates/storage/src/buffer_pool.rs` | 【Round 1 改判】本用例的断言是三个记账计数（pages_visited / cache_misses / cache_hits）加上「命中时不得调用 loader」的守卫；声明文本的另一半「写回时机」完全没触及，而那几个计数与同文件里被判 R3 的 PILOT-T3-004 / T3-005 / T3-007 是同一类。Round 0 曾把它判为可判定（并附「只覆盖读路径」的限制说明），那是把组标签当成了断言——已改判为 R3。 |
| `PILOT-T3-007` | T3 | `buffer_pool::tests::buffer_pool_stats_exposes_five_new_fields_with_zero_init` | buffer/flush：页可见性与写回时机 | INV-07, INV-10 | ASSERTION_UNREVIEWABLE | R3 | 无 | `crates/storage/src/buffer_pool.rs` | 只断言五个新统计字段初值为 0。 |
| `PILOT-T3-008` | T3 | `buffer_pool::tests::test_buffer_pool_basic` | buffer/flush：页可见性与写回时机 | INV-07, INV-10 | ASSERTION_UNREVIEWABLE | R2 | 无 | `crates/storage/src/buffer_pool.rs` | 只断言 capacity() == 10 与 is_empty()。 |

## 5. 待修断言清单（38 条 `ASSERTION_UNREVIEWABLE`）

这 38 条的 `execution_status` 保持原值不变（本人无权改动），但它们**未经副签**，
按治理口径其 `PASS` **不计入阶段出口**。

> 修复路径分三类：**改断言文本** = 重写 `source.declared_invariant` 并重映射 `invariant_ids`（不动测试）；
> **需改测试** = 断言本身缺失或与声明相反，改文本也救不回来；
> **需改映射 + 改断言文本** = 组标签的 `invariant_ids` 与用例主体对不上，先要给它找到真正对应的不变量。

| case_id | 原因码 | 可判定化需要什么 | 修复路径 |
|---|---|---|---|
| `PILOT-T1-001` | R1 | 需在事务内写入至少一行，回滚后断言该行已消失（行数回到事务前）；或把断言文本改成「回滚后事务标志复位」 | 改断言文本 |
| `PILOT-T1-002` | R2 | 需断言回滚后被插入的行在存储中不可见；或把断言文本改成「回滚不排空 buffer pool，pages_visited 不变」 | 改断言文本 |
| `PILOT-T1-005` | R2 | 需断言嵌套 begin 失败后存储状态未变；或把断言文本改成「嵌套事务被显式拒绝」 | 改断言文本 |
| `PILOT-T1-008` | R1 | 需在事务内写入数据并断言回滚后恢复；或把断言文本改成「事务号分配与 in_transaction 复位」 | 改断言文本 |
| `PILOT-T1-009` | R1 | 函数体没有任何断言：先补一条可证伪断言（例如 current_tx_id() == 42），再让断言文本与之对齐 | 需改测试（补断言） |
| `PILOT-T1-010` | R1 | 需在事务内写入数据并断言回滚/提交后的可见性；或把断言文本改成「begin/commit 后事务标志复位」 | 改断言文本 |
| `PILOT-T1-011` | R2 | 需涉及回滚并比较前后状态；或把断言文本改成「新建实例的事务状态」 | 改断言文本 |
| `PILOT-T1-012` | R2 | 需涉及回滚并比较前后状态；或把断言文本改成「set_current_tx_id 生效」 | 改断言文本 |
| `PILOT-T1-013` | R1 | 需在事务内写入数据、回滚后断言存储状态恢复（当前注释明说状态断言被跳过）；或把断言文本改成「begin/commit/rollback 管线可用」 | 改断言文本 |
| `PILOT-T1-014` | R2 | 需在用例内执行一次真正的 rollback 并比较存储前后状态；或把断言文本改成「PITR 恢复不应用未提交事务的行」 | 改断言文本 |
| `PILOT-T1-015` | R2 | 需断言回滚后的存储状态；或把断言文本改成「BEGIN TRANSACTION 被分类为 Transaction」 | 改断言文本 |
| `PILOT-T1-016` | R2 | 需断言回滚后的存储状态；或把断言文本改成「semi-sync 事务计数」 | 改断言文本 |
| `PILOT-T1-017` | R1 | 需在事务内写入数据并断言回滚后恢复；或把断言文本改成「事务生命周期透传」 | 改断言文本 |
| `PILOT-T1-018` | R2 | 需涉及回滚并比较前后状态；或把断言文本改成「tx id 决定 in_transaction」 | 改断言文本 |
| `PILOT-T1-019` | R2 | 需涉及回滚并比较前后状态；或把断言文本改成「事务内 DML 安全检查放行」 | 改断言文本 |
| `PILOT-T1-020` | R2 | 需涉及回滚并比较前后状态；或把断言文本改成「guard 透传 tx 状态」 | 改断言文本 |
| `PILOT-T1-021` | R2 | 需涉及回滚并比较前后状态；或把断言文本改成「N 个已提交事务产生 M 条 WAL 记录」 | 改断言文本 |
| `PILOT-T1-022` | R1 | 需在回滚后比较存储状态（撤销了几条、状态是否等于事务前）；只断言 WAL 条数与条目类型无法判定「撤销」 | 改断言文本 |
| `PILOT-T1-023` | R2 | 需涉及回滚并比较前后状态；或把断言文本改成「WAL 记录条数」 | 改断言文本 |
| `PILOT-T1-024` | R1 | 需比较回滚前后的存储状态；或把断言文本改成「回滚后事务退出 active_txs」 | 改断言文本 |
| `PILOT-T1-025` | R2 | 需涉及回滚并比较前后状态；或把断言文本改成「初始无活动事务」 | 改断言文本 |
| `PILOT-T1-026` | R2 | 需涉及回滚并比较前后状态；或把断言文本改成「set_current_tx_id 生效」 | 改断言文本 |
| `PILOT-T1-027` | R2 | 需涉及回滚并比较前后状态；或把断言文本改成「WAL 提交记录条数」 | 改断言文本 |
| `PILOT-T1-028` | R1 | 需在回滚后比较存储状态（插入的行是否消失）；只断言 WAL 里有 Rollback 记录无法判定「撤销」 | 改断言文本 |
| `PILOT-T1-101` | R1 | 文档注释声明的「undo log 清空」必须落成断言：回滚到 savepoint 之前后 undo_log_len() == 0，并断言 applied 回调被调用恰好两次（每条 undo 一次）。当前唯一的长度断言与之相反，且 rollback_to_noop 的返回值被 .ok() 丢弃。 | 需改测试（断言与声明相反） |
| `PILOT-T2-002` | R2 | 需插入两行相同值但不同 RowId 的记录并断言两条都在；或把断言文本改成「同名 trigger 重复创建不报错」并把 >= 1 收紧为确切条数 | 改断言文本 |
| `PILOT-T2-003` | R2 | 需插入相同值的两行并断言两条都可见；或把断言文本改成「空表快照」 | 改断言文本 |
| `PILOT-T2-004` | R2 | 需在有多版本的链上断言同值不同键各自可见；或把断言文本改成「空版本链查找返回 None」 | 改断言文本 |
| `PILOT-T2-005` | R2 | 需断言回收后同一逻辑行仍只有一条可见版本；或把断言文本改成「GC 回收旧版本」 | 改断言文本 |
| `PILOT-T2-006` | R2 | 需插入相同值的第二行并断言两次 PK 查找互不覆盖；或把断言文本改成「按 PK 取可见版本」 | 改断言文本 |
| `PILOT-T3-001` | R2 | 需把断言改成「页失效后该页在索引缓存中不再可见」并把 invariant_ids 收窄到真正对应的不变量（当前语料 INV-01..INV-18 里没有一条讲页缓存失效的，最接近的 INV-15 讲的是「索引/聚类/cache 与同一 committed RowStore 发布/撤销一致」，也不是本用例）；或改测试去触及 INV-07 / INV-10。 | 需改映射（收窄 invariant_ids）+ 改断言文本 |
| `PILOT-T3-002` | R1 | 需先写入数据把 dirty 置真，再 flush 后断言 !s.dirty；否则该断言对「flush 清 dirty」是空过，任何断言文本都无法让它变得可判定。 | 需改测试（断言空过） |
| `PILOT-T3-003` | R1 | 需在 flush 后断言可观测结果（例如重新打开后数据仍在、或 dirty 已清）；当前函数没有任何断言。 | 需改测试（无断言） |
| `PILOT-T3-004` | R3 | 需断言页在装载后对读者可见的内容；只钉 disk_reads 计数无法判定可见性或写回时机。或把断言文本改成「buffer pool 的磁盘读记账」。 | 改断言文本 |
| `PILOT-T3-005` | R3 | 需断言写回确实发生（例如落到文件后重开仍能读到）；只钉 disk_writes 计数无法判定写回时机。或把断言文本改成「buffer pool 的磁盘写记账」。 | 改断言文本 |
| `PILOT-T3-006` | R3 | 需断言写回路径的可观测结果；当前的 pages_visited / cache_misses / cache_hits 三个计数与同文件里被判 R3 的三条同类。或把断言文本改成「buffer pool 的读路径记账与命中守卫」并收窄 invariant_ids。 | 需改映射（收窄 invariant_ids）+ 改断言文本 |
| `PILOT-T3-007` | R3 | 需在真实读写之后断言这些计数反映实际动作；只断言初值 0 无法判定可见性或写回时机。或把断言文本改成「统计字段初值」。 | 改断言文本 |
| `PILOT-T3-008` | R2 | 需涉及 flush/可见性并比较前后状态；或把断言文本改成「缓冲池容量与初始为空」 | 改断言文本 |

**统计**：改断言文本 32 条；需改测试 4 条；需改映射 + 改断言文本 2 条。

## 6. 本次审查暴露的结构性问题（供 Task 9 汇总）

1. **`declared_invariant` 是「按 feature 分桶」的标签，不是「按用例」的断言。**
   全部 75 条的 `declared_invariant` 只有 **13** 种取值（L1 子集 44 条里 **5** 种）；
   按 feature 的不同取值数为 **T1 3 / T2 1 / T3 1 / T4 3 / T5 2 / T6 3**。
   T1 的 30 条里 **28 条**共用同一句「undo log 语义：回滚后状态 == 事务前」；
   T2 的 6 条共用「逻辑行身份：相同值不等同同一行」，T3 的 8 条共用「buffer/flush：页可见性与写回时机」。
   Task 5 的映射规则明写「declared_invariant 相同的整组用同一组 id」——也就是说 `invariant_ids` 是
   **按组**贴上去的，从来没有按用例核对过。本轮 44 条里只有 6 条对得上。
   （注：`verify_matrix.py` M5 打印的「the mapping covers 9 word(s)」里的 9 是
   `status_vocabulary_mapping` 的**状态词**数，与 `declared_invariant` 的种数是两个量；
   Round 0 把 9 误当成 `declared_invariant` 的种数，本轮更正。）

2. **`PILOT-T1-002` 的测试自身就写明了它与所声称的不变量相反**：
   「Pages inserted during a transaction that is later rolled back will remain in the cache」
   （BufferPool 没有事务性排空）。把它算作「undo log 语义：回滚后状态 == 事务前」的覆盖，是把反例当成了证据。

3. **`PILOT-T3-002` 的断言是空过的**：`flush()` 之前从未写入任何数据，`dirty` 从未被置真，
   所以 `assert!(!s.dirty)` 即使 flush 什么都不做也通过。

4. **`PILOT-T1-101` 的文档注释与其断言相反**：注释写「INSERT 后 ROLLBACK 必须撤销该插入（undo log 清空）」，
   而唯一的断言是「回滚到一个刚建的 savepoint 不会弹出任何 undo」。

5. **T3 的 `INV-10` 映射值得单独看**：INV-10 的原文是「checkpoint 证书基于 fsync 数据/目录的 generation，
   **不是 flush 调用布尔**」，而 T3 组里最贴近的两条（T3-002 `assert!(!s.dirty)`、T3-003 `flush().unwrap()`）
   恰恰就是 flush 调用布尔。Round 1 的判定口径新增了 (d)（主体须与登记表原文是同一个主体），
   `PILOT-T3-001` / `PILOT-T3-006` 因此由「可判定」改判为 `ASSERTION_UNREVIEWABLE`（R2 / R3）。

## 7. 声明

- 本人在判定过程中**没有读取**被测函数的实现代码；
- **没有读取**任何执行日志（`L1-RERUN-fd849ae20541.json`、`L1-DIFF-fd849ae20541.md`、`L1-BINDING-fd849ae20541.json`、cargo 原始日志）；
- 矩阵里 `actual` 与 `artifact_sha256` 两个字段在判定期间**未被用于任何判定**。
  **如实披露两处边界**：
  1. **开工时为了摸清矩阵结构，打印过 `PILOT-T1-001` 与 `PILOT-T1-002` 两条的完整 case 对象**，
     当时的输出里带有这两条的 `actual`（`… transaction_lifecycle ... ok` 一行）与 `artifact_sha256`。
     这发生在建立「不打印 `actual`/`artifact_sha256`」规则之前。这两条的判定结果都是
     `ASSERTION_UNREVIEWABLE`，判定依据完全来自测试源码，**没有用到这两个字段**；
     且 Round 1 的独立复核从测试源码单独重推这两条，结论逐字相同。
  2. **读过 `evidence/case-mapping-20261120.json`**（试点自己的 case 索引，含 `rerun_status` 字符串），
     目的是确认 `declared_invariant` 是按用例还是按分桶写的——该文件里**没有** `declared_invariant` 字段，
     这个「缺失」正是分桶根因的证据。它不含任何执行输出，也未影响任何判定。
- 本人**没有改动**矩阵里 `execution_status` / `actual` / `artifact_sha256` / `red_identity` / `invariant_ids` / `source` 中的任何一个字段，也没有碰 T4/T5/T6 的任何一条；
- 矩阵里 `red_identity` 的取值**逐字节未变**（已用改动前的快照逐条比对确认）。

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

### 8.1 计数核对（脚本从矩阵反算，与本日志 §2 一致）

```
L1 场景总数: 44
已审查（带 assertion_review_verdict）: 44
已副签（带 assertion_reviewed_by）: 0
JUDGED_PENDING_COUNTERSIGNATURE: 6
ASSERTION_UNREVIEWABLE: 38   (R1 12 / R2 22 / R3 4)
L1 的 PASS 总数: 42
PASS 且已副签: 0
PASS 且判定可独立判定、待副签: 5
PASS 且判定不可独立判定: 37
```

### 8.2 计划 Step 2 / Step 4 自查脚本的实际输出

计划 Step 2 找的是「有 `assertion_reviewed_by` 的 PASS」。Round 1 之后没有任何一条有副签人，
所以该脚本现在如实报告**全部 42 条 PASS 未副签**：

```
$ python3 -c "... [c['case_id'] for c in t1 if c['execution_status']=='PASS' and not (c.get('assertion_reviewed_by') and ...)] ..."
T1 未签署的 PASS: [T1 的 24 条 PASS 全部列出]
```

计划 Step 4 的 `sorted({... assertion_reviewed_by ...})` 在有未副签条目时同样会因 `None` 混排抛
`TypeError`；过滤 `None` 后的等价读法：

```python
cs = json.load(open('docs/releases/v4.1.0/refactoring/TEST_COVERAGE_MATRIX.json'))['cases']
l1 = [c for c in cs if c['feature'] in ('T1','T2','T3')]
signed = [c for c in l1 if c.get('assertion_reviewed_by')]
judged = [c for c in l1 if c.get('assertion_review_verdict') == 'JUDGED_PENDING_COUNTERSIGNATURE']
print('L1 有终态:', sum(1 for c in l1 if c.get('execution_status')), '/', len(l1))
print('已审查:', len([c for c in l1 if c.get('assertion_review_verdict')]), ' 已副签:', len(signed))
print('签署人集合:', sorted({c['assertion_reviewed_by'] for c in signed}))
```

输出：

```
L1 有终态: 44 / 44
已审查: 44  已副签: 0
签署人集合: []
```

签署人集合为空——**没有任何一条被冒充为已签**。

## 9. Task 9 残余登记

按 `V410-READINESS-IMPLEMENTATION-PLAN.md` L772 与 `V410-REFACTOR-READINESS-DESIGN.md` L490-497，
Oracle 签署步骤 BLOCKED 的残余项如下，请在 Task 9 的放行裁决记录中逐项列明：

| 编号 | 残余项 | 需要谁裁决 | 影响 |
|---|---|---|---|
| R-T7-1 | **Oracle 副签缺失**：44 条 L1 场景的断言判定已由 `task7-executor/minimax-m3.1-flash` 完成，但没有属主授权的独立 Reviewer 副签，`assertion_reviewed_by` 在 44 条上全部缺席 | **属主**（指定 Codex 或 DSH Reviewer 池成员，或由属主本人副签） | L1 的 42 条 PASS **无一**满足 `ASSERTION_REVIEW` 激活条件；按设计 L493 的口径，「若实施者不得自签、又没有指定签名人，75 条将全部停在 `NOT_RUN`」——本任务正处在这个状态 |
| R-T7-2 | **断言可判定性缺陷**：38 条判定为 `ASSERTION_UNREVIEWABLE`（T1 25 / T2 5 / T3 8），其中 4 条必须改测试、2 条必须先修映射 | 控制者 + 实施者（修测试/修映射），属主裁决是否接受 | 按 `V410-REFACTOR-READINESS-DESIGN.md` L516-517，T1 的 `PASS` 不得计入阶段出口，须先修正断言的可判定性 |
| R-T7-3 | **本日志不在 manifest 哈希范围内**：`ORACLE-REVIEW-LOG-T1.md` 位于 `docs/refactor/v4.1.0-c/takeover/` 顶层，`verify_handoff.py` 的 `MANIFEST_ROOTS` 只覆盖 `handoff/`、`takeover/dsh-b/` 与 `docs/releases/v4.1.0/refactoring/`（`tools/verify_handoff.py:79-83`），顶层按设计是「活动工作区」不参与哈希 | **属主**（C1：扩大 manifest 范围是属主专属动作） | **签名本身是被绑定的**（矩阵在范围内，314 个文件之一），但**叙述性记录不被绑定**；本日志被篡改不会触发 `--check` 失败 |

### 9.1 R-T7-1 的处置选项（供属主选择）

1. **属主授权 Codex 或 DSH Reviewer 池的某名成员**按本日志 §1.1 的口径对 6 条 `JUDGED_PENDING_COUNTERSIGNATURE` （以及修正后的 38 条）逐条副签，写入 `assertion_reviewed_by`；
2. **属主本人副签**（计划把签署人限定在 Reviewer 池，但设计文档同时写了 `oracle_owner: "Owner/Independent Reviewer not implementer"`，属主本人副签在「非实现者」这一条上是成立的——**这一条需要属主自己裁定是否接受**）；
3. 先修 38 条断言的可判定性（§5 的清单），再重新走一次判定 + 副签；
4. 接受 L1 停在「已审查、未副签」，不计入阶段出口，进入 Task 9 裁决。

在 (1) 或 (2) 落地之前，**本日志与矩阵都不构成任何已完成签署的证据**。

