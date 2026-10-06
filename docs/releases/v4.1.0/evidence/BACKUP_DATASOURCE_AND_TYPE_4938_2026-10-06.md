# #4938 备份工具：数据源接线与备份类型标签契约

> 记录 `crates/tools/src/backup.rs` 两个备份入口的**真实行为契约**。
> 这份文档由 #4938 的 AC2/AC3/AC5 整改产生——修代码的过程发现，
> 函数名、CLI 子命令名、manifest 标签三者曾经互相矛盾，
> 写下契约是为了让下一个人不必再从代码里反推。
>
> source_agent: mcode (minimax-M3.1)
> base: 54e5a5fa83 (develop/v4.1.0)
> timestamp: 2026-10-06

## 一句话结论

**这个工具目前没有真正的增量备份。**`incremental` 子命令导出的是全部表，
manifest 标 `Full`。名字是历史包袱，不是能力描述。

## 两个入口的真实行为

| 入口 | 读什么 | 导出范围 | manifest `backup_type` | `parent_lsn` |
|---|---|---|---|---|
| `create_full_backup(dir, format, data_dir)` | 调用方的 `data_dir`（`FileStorage`） | 全部表 | `Full` | 无 |
| `create_incremental_backup(parent, dir, format, data_dir)` | 调用方的 `data_dir` | **全部表** | `Full` | 有（= parent 的 lsn） |
| `create_incremental_backup_with_changeset(parent, dir, ctx)` | `ctx` 里的 ChangeSet | **仅变化** | `Incremental` | 有 |
| `create_full_backup_from_demo(dir, format)` | 内置 demo 数据集 | 全部表 | `Full` | 无 |
| `create_incremental_backup_from_demo(parent, dir, format)` | 内置 demo 数据集 | 全部表 | `Full` | 有 |

注意第三行：唯一名副其实的增量入口是 `create_incremental_backup_with_changeset`，
它导出 `.inc.sql` 差量文件并记录 `Incremental`。**它带 `#[allow(dead_code)]`，
没有生产调用者**——目前只能由调用方自己构造 `IncrementalBackupContext` 并直接调用。

## 为什么 `create_incremental_backup` 不能是真增量

存储引擎没有**版本化**变更捕获（versioned change capture）。没有 WAL 游标、
没有 CDC、没有 per-table 变更日志，工具层就无从知道"自 parent LSN 以来哪些行变了"。

> 更正说明：初稿此处写的是"没有变更捕获"，过于绝对。`StorageEngine` 确实有
> `table_change_stamp`（`crates/storage/src/engine.rs:1309`），但它**不能**用于增量备份——
> 全仓库只有 `MemoryStorage` 实现了它（`engine.rs:2609`），而备份工具打开的是
> `FileStorage`（后者继承默认返回 `0`）；且其语义是缓存失效标记，trait 文档明说
> "callers must treat the value as opaque and only compare it for equality"，
> 拿不到行级变更内容。详见 #5048。

这意味着诚实的做法只有两个：要么接上真正的变更源，要么**如实标注它不是增量**。
本轮选了后者。给一个全量导出贴 `Incremental` 标签的危害是具体的：

> 运维拿到 manifest 看到 `backup_type: incremental`，会认为恢复时只需重放
> 少量变更。实际上目录里是第二份完整副本，`.sql` 里是全量 `INSERT`。
> 恢复路径、时间估算、存储容量规划全部基于错误前提。

标签从假话改成真话，代价是 CLI 子命令名仍然叫 `incremental`——这一点在
`run()` 的分派处打印了显式警告，因为子命令名是 CLI 契约，不在本轮可改范围。

## 标签的连锁影响（改动前实测）

改 `backup_type` 前先查了谁依赖它，结论：

- `restore_incremental_chain`（`backup.rs:816`）**按传入的目录列表遍历**，
  不按 `backup_type` 过滤 → 改标签不影响链式恢复
- `apply_retention_policy`（`backup.rs:928`）按标签分流 full / incremental
  的保留计数 → 这个函数带 `#[allow(dead_code)]`、无生产调用者，
  分类偏差不影响运行时行为
- `list_backups` 的显示走 `manifest.backup_type` → 变为如实显示

## 历史（为什么会有这个坑）

1. `#4938` 的原始问题是 `backup.rs` 是**孤儿文件**——`lib.rs` 声明的是
   `pub mod backup_restore;`，指向另一个文件。`backup.rs` 当时不参与编译，
   21 个 `#[test]` 从未执行过。
2. PR `6c8d428879` 接通了 `mod`，文件进入编译图，暴露出七层类型漂移并修掉。
   从这一刻起代码可编译、`cargo test --lib` 153 通过。
3. 但**行为缺陷没被修**：`data_dir` 参数名是 `_data_dir`（下划线前缀 =
   从未使用），函数体内部被 `let storage = create_demo_storage();` 同名遮蔽。
   21 个单元测试没有一个创建过备份文件或执行过恢复——它们只覆盖 LSN 生成、
   manifest 序列化、Value→SQL、change-context 记账。
4. 于是 #4938 在 2026-10-05 10:19 关闭时，AC2/AC3 已经由 e2e 测试满足，
   但"增量名不副实"这个更根本的问题是在**关闭 6 小时后**（16:52）才被发现的。

这是本次整改反复撞上的同一形状：**组件级测试全绿 ≠ 系统行为正确**。
可编译的文件、零生产调用的函数、只测内部 helper 的单元测试——三者都曾经是绿的。

## 写这类测试时踩到的坑

`FileStorage` 的近期写入在 insert buffer 里，`scan`（备份导出用的就是它）
**只读已提交行**。seed 数据后必须 `flush()`，否则：

- 备份是空的
- 测试会"证明"导出器丢数据，而实际是数据还没落盘
- 空 manifest 又会让"标签检查"这类断言**空洞通过**（没内容可检查时断言不成立）

新测试 `issue_4938_all_tables_export_is_not_labelled_incremental` 显式断言
两个目录的 `data/` 都有表文件，就是为了防止这种空洞通过。

## 测试与变异验证

```
crates/tools/tests/incremental_backup_e2e_test.rs        5 passed
crates/tools/tests/incremental_backup_datasource_4938.rs 4 passed
```

| 变异 | 描述 | 结果 | 判定 |
|---|---|---|---|
| P | `backup_type` 退回 `Incremental` | 2 FAILED | **有效** |
| Q | 改标签时顺手丢 `parent_lsn` | 3 FAILED | **有效** |
| O | `open_source_storage` 退回 demo 兜底 | 3 FAILED | **有效**（上一轮 PR #5042） |

变异 Q 是特意做的：如果有人日后把标签"修正"成 `Full` 的同时认为
"既然是全量了，parent_lsn 也就没用了"，必须失败。链式链接是独立于类型的属性。

## 遗留（不在本轮范围）

1. **CLI 子命令名**仍叫 `incremental`，实际是全量。改名是 CLI 破坏性变更，
   需单独决策。
2. **`create_incremental_backup_with_changeset` 零生产调用者**。要让
   `incremental` 子命令名副其实，需要存储层提供变更源（LSN 游标 / CDC），
   属于存储层工作，不是 tools 层能单独解决的。
3. `apply_retention_policy` 的标签分类在标签修正后语义变化，未修正（无调用者）。
4. `restore_incremental_chain` 打印 "operations applied" 但**不真正执行 SQL**
   （`backup.rs:907` 的注释自认："In a real implementation, we would execute
   the SQL / For demo, we just log the operations"）。链式恢复目前只验证
   遍历顺序，不验证数据落盘。

## 关联

- Issue #4938（P1 接通增量备份 tools/src/backup.rs）
- Issue #4（v2.0 高可用与数据可靠性 — 增量备份）
- PR #5040（e2e 测试，`5b76c35a4d9d`）
- PR #5042（真实 data_dir 接线，merge `9dfb919419`）
- PR `6c8d428879`（接通 mod）
