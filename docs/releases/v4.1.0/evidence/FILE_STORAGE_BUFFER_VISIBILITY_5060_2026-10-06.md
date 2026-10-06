# #5059 / #5060：FileStorage 的 insert_buffer 可见性与回滚撤销

- **日期**：2026-10-06
- **Issue**：#5059（回滚后 200 行而非 100）、#5060（update/delete 对 insert_buffer 不可见）
- **分支**：`fix/5060-5059-storage-write-paths`
- **测试**：`crates/storage/tests/file_storage_dml_visibility_5060.rs`（13 项）

---

## 1. 根因

`insert_buffer` 是表状态的**第二份拷贝**（延迟落盘的待写行），而只有 `scan` 知道它的存在。
`update` / `delete` 只在 `tables` 这一份上找行，于是对尚未 flush 的行完全失明。

探针（修复前）：

```
P1 update affected = 0   (应 1)   scan 仍是旧值 "a"
P2 delete affected = 0   scan 空了  ← 更糟：从 buffer 剥了行但没计数，
                                 dirty_tables 未设 → 删除永不落盘，
                                 行在内存消失、重开又回来
P3 事务内 update affected = 0      after rollback 行还在
P4 rollback+flush scan = [[Integer(1), Text("doomed")]]  ← #5059 根因
P5 update_if affected = 0
```

### #5059 的精确根因

`rollback_transaction` 末尾有一段「belt-and-suspenders」清扫：遍历 `s.tables.keys()`
再套一层 `scoped_key`。但那些 key **本身已经是** `db\x01table` 形态的作用域键，
二次作用域化后寻址 `default\x01default\x01tx_t`，匹配不到任何 buffer。
于是每个回滚的 INSERT 都存活 —— 100 行变 200 行。

**修复不能靠「扫得更狠」**：buffer 是实例级共享的（跨连接），无差别清扫会把别的写者的
数据也删掉，把回滚变成数据丢失。必须**按值撤销**。

---

## 2. 修复

`crates/storage/src/file_storage.rs`

### `WriteState` 两个新原语（表有两个存储这件事，只有这里知道）

- `mutate_matching(scoped_table, matches, mutate) -> Vec<(Record, Record)>`
  同时遍历 `tables` 与 `insert_buffer`，返回每行的 (前像, 后像)。后像给 WAL（#5055）
  和 change log（#5048）——两者事后都重建不出来。
- `remove_matching(scoped_table, matches) -> Vec<Record>`
  从两处都移除并返回被删行。

### `UndoOp` 两个新变体

- `BufferedDelete { table, row }`（#5059）—— 按值撤销，不用索引。
- `BufferedUpdate { table, post, original }`（#5060）—— 按后像定位再替换回前像；
  后像已消失时无需撤销 buffer（`tables` 那份由 `UpdateRow` 覆盖）。

`insert` / `delete` / `delete_if` / `update` / `update_if` 全部改用上述原语；
`rollback_transaction` 删掉无效清扫，改为按值处理三种 buffered undo；`apply_undo` 同步。

---

## 3. 测试中的一处自我修正

最初写的「插入后删除再回滚」用例，断言写错了：
的正确结果是**空**（恢复到事务前状态），不是「行回来了」。已拆成两个用例：

- `rollback_restores_a_committed_row_deleted_in_the_transaction` —— 测 `DeleteRow`
- `rollback_of_insert_then_delete_restores_the_empty_table` —— 两个 value-based undo 抵消

---

## 4. 变异验证

测试文件：`file_storage_dml_visibility_5060.rs`（13 项）
辅助：`phase_c_1_race`（#5059 的原始失败用例）、`tx_isolation_and_escape_hatch_test`

| 变异 | 做法 | 结果 | 判定 |
|---|---|---|---|
| **M4** | `mutate_matching` 不再遍历 `insert_buffer`（退回 #5060 之前） | 3 项 FAILED：`update_after_autocommit_insert_affects_the_row`、`update_inside_a_transaction_reaches_the_uncommitted_row`、`update_if_after_autocommit_insert_affects_the_row` | **CAUGHT** |
| **M5** | `remove_matching` 不再从 `insert_buffer` 移除 | 3 项 FAILED：`delete_after_autocommit_insert_removes_the_row_from_disk`、`delete_if_after_autocommit_insert_removes_the_row_from_disk`、`rollback_of_insert_then_delete_restores_the_empty_table` | **CAUGHT** |
| **M6** | `BufferedDelete` 撤销改为空操作 | 首轮**存活**（0 项失败）；补测试后 1 项 FAILED | **CAUGHT（补测试后）** |
| **M7** | `BufferedUpdate` 撤销改为空操作 | 1 项 FAILED：`rollback_undoes_an_update_applied_to_an_uncommitted_row` | **CAUGHT** |
| **M8** | `BufferedInsert` 撤销换回 #5059 之前的二次作用域 blanket sweep | 5 项 FAILED（含 `rollback_removes_the_buffered_insert`、`rollback_keeps_rows_it_did_not_insert`、`committed_and_rolled_back_inserts_are_counted_exactly`）；`phase_c_1_race::c1_concurrent_begin_commit_rollback` 亦 FAILED | **CAUGHT** |

### M6 的处理 —— 一次真实的覆盖缺口

`BufferedDelete` 的撤销臂改成空操作后，**13 个测试全部照过**。这不是变异无效，是测试
没覆盖到它唯一承重的那条路径。

原因：文件里所有 delete 测试在删之前都调了 `flush_all_buffers()`，行因此已经搬进
`tables`，回滚走的是 `DeleteRow` 而非 `BufferedDelete`。

新增 `rollback_restores_a_committed_but_unflushed_row_deleted_in_the_transaction`：
插入（自动提交，**故意不 flush**，行仍在 `insert_buffer`）→ 开事务 → 删除 → 回滚。
此时行没有 `tables` 下标可寻址，`DeleteRow` 救不回来，只有 `BufferedDelete` 能。

补上后 M6 被抓住。

（补测试过程中一度误判成「修复代码本身有 bug」—— 实际是此前一条恢复命令被权限门禁拦下、
整条未执行，变异还留在文件里。恢复干净代码后 13/13 通过。）

---

## 5. 全量回归

`cargo test -p sqlrustgo-storage --all-features`：**0 失败**。

其中 `tx_isolation_and_escape_hatch_test::rolled_back_rows_disappear_for_all_readers`
（#5059 的连带症状）由 FAILED 转 PASS。

`cargo fmt --check --all` 通过；`cargo clippy -p sqlrustgo-storage --all-features --all-targets`
未引入新告警（改造后变为零调用的 `load_index` / `load_table` / `load_table_delta`
包装函数已删除）。
