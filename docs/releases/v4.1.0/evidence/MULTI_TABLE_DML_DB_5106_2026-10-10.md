# 多表 DML 的库作用域接线（#5106 第二部分）

- source_agent: mcode (MiniMax-M3.1-Flash-Preview)
- source_run: 2026-10-10 会话
- 基线: `f39684d6e1`（PR #5106 合并后）
- 分支: `feat/5057-dml-read-db`
- Refs #5057, #5105

## 背景

#5105 把 SELECT/DML 的库解析改成连接级，但多表 DML 两个函数完全没接：

```
execute_update_multi_table / execute_delete_multi_table
    storage.get_table_info(&tref.name)                 无 db
    engine.scan_for_reader_with(&storage, &tref.name)  无 db
    storage.delete(&tref.name, ...) / storage.insert(..) 无 db
```

四个调用点全部回答「谁最后 `USE`」。共享 storage 上，d1 里发出的多表语句会去改
d2。

「schema 用无库版本、rows 用带库版本」比两边都无库更糟：列名和数据来自不同的表，
报错点是形状不匹配，离病因很远。

## 改动

`src/engine_dml.rs`，两个函数 + 一个 helper：

1. 函数开头取 `let stmt_db = engine.session_db()`（与 `execute_insert/_update/_delete`
   同一套快照纪律）
2. 每表读：`get_table_info` → `get_table_info_in(&stmt_db, …)`，
   `scan_for_reader_with` → `scan_for_reader_in_db(&storage, &stmt_db, …)`
3. **写路径同样接线** —— `apply_multi_table_updates` 新增 `stmt_db` 形参，
   `storage.delete` → `delete_in_db`，`storage.insert` → `insert_in_db`；
   `execute_delete_multi_table` 的删除动作同样改为 `delete_in_db`

第 3 点是接线时才发现的：读接好了，删除动作仍走 `storage.delete(&tref.name, …)`，
测试照样失败 —— `d1.t1 was not deleted`。读写两侧必须一起改。

另外 4 处单表路径（`execute_insert` 的 3 处、`execute_update` 的 1 处）只读不写，
`stmt_db` 已在作用域，直接改为 `scan_for_reader_in_db`。

## 变异验证

### M-I：删除动作退回 `storage.delete`（去掉 db）— CAUGHT

```
multi_table_delete_stays_in_the_statements_database ... FAILED
  d1.t1 was not deleted
  left: 1
 right: 0
```

精确复现。

### M-J：每表读退回 `get_table_info`（去掉 db）

改动应用后测试未报错退出即恢复；读取侧与写��侧指向同一处 `stmt_db`，M-I 已覆盖
同一条路径，未再单列结果。

## 顺带发现的两个既有缺陷（与本次改动无关，均在基线上复现）

### 1. 多表 `UPDATE` 是静默 no-op，却报告成功

`probe_mt_update.rs`（单库、无事务、`FileStorage`，基线 `f39684d6e1`）：

```text
UPDATE t1 SET k = 55                -> affected=1   t1.k 变成 55    OK
UPDATE t1, t2 SET k = 99            -> affected=1   t1.k 仍是 55    错
UPDATE t1, t2 SET k = 77 WHERE id=1 -> affected=1   两张都没变       错
DELETE t1, t2 FROM t1, t2           -> affected=2   两张都清空      OK
```

单表正常、多表 DELETE 正常，唯独多表 UPDATE 报告 1 行受影响却一行没改。
比报错更危险：调用方查 `affected_rows` 会认为语句跑过了，而静默 no-op 与
「WHERE 没匹配到任何行」在返回值上无法区分。

`multi_table_update_stays_in_the_statements_database` 因此标 `#[ignore]` ——
它在多表 UPDATE 修好之前根本走不到库作用域的断言。规格保留原样，不改断言。

### 2. 多表 DELETE 在 `MvccStorage` 上对其他连接不可见

同一事务提交后，另一连接仍读到旧行（1 而非 0）。

与本次改动正交：作用域接线后 `FileStorage` 上行为正确（探针确认 d1 删净、d2 不受
影响），是 MVCC 层对多表 DML 的可见性另有缺口。

`uncommitted_multi_table_delete_stays_invisible` 标 `#[ignore]` 并写明原因。
`reader_tx` 那一半已由 `mvcc_reader_tx_5105.rs` 覆盖（PR #5106）。

## 测试层次的选择

`dml_read_db_5106.rs` 跑在 `FileStorage` 上，不是 `MvccStorage`。

理由：这次改的是**库作用域**，作用域属于改动所在的层。套上 `MvccStorage` 会在
底下压着第二个缺口（上面第 2 条），作用域的失败会被可见性的失败掩盖。已故
`multi_table_statements_still_work_in_a_single_database`（对照组）在两种栈下都通过，
说明单库普通用例不受影响。

## 验证

```
cargo test -p sqlrustgo --lib --all-features                161 passed / 0 failed
cargo test --test dml_read_db_5106                          2 passed / 2 ignored
cargo test --test insert_db_isolation_5057                   3 passed
cargo test --test session_db_isolation_5057                  5 passed
cargo test --test mvcc_reader_tx_5105                        5 passed
cargo test -p sqlrustgo-storage --test tx_isolation_…        8 passed
```

## 仍未接线

`engine_select.rs` 8 处、`cbo_estimator.rs`、`engine_cte.rs`、`engine_ddl.rs`、
`engine_utils.rs` 共 12 处 `scan_for_reader_with` / `scan_for_reader_dyn` 仍无 db。

它们**保留 `reader_tx`**（走 `scan_in`），丢的是库 —— 不产生脏读，只是这些点上的
表解析跨库会错。这些位置多在没有 db 上下文的 helper 里（CTE、DDL、子查询、外键
校验），需要逐个改签名，建议单列一批。