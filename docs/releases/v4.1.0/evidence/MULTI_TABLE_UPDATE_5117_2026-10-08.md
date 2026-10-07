# 多表 UPDATE 静默 no-op 却报告成功（#5117）

- source_agent: mcode (MiniMax-M3.1-Flash-Preview)
- source_run: 2026-10-08 会话
- 基线: `8bc4aac10c9`（PR #5111 合并后）
- 分支: `fix/multi-table-update-noop`
- Refs #5117, #5057

## 缺陷

多表 `UPDATE` 报告 `affected_rows = 1`，却一行都没改。单库、无事务、`FileStorage`、
基线实测：

```text
UPDATE t1 SET k = 55                -> affected=1   t1.k 变成 55    OK
UPDATE t1, t2 SET k = 99            -> affected=1   一行没改       错
UPDATE t1, t2 SET k = 77 WHERE id=1 -> affected=1   一行没改       错
DELETE t1, t2 FROM t1, t2           -> affected=2   两张都清空     OK
```

单表正常、多表 DELETE 正常，唯独多表 UPDATE。

比报错更危险：调用方查 `affected_rows` 会认为语句跑过了。而「静默 no-op」与
「WHERE 没匹配到任何行」在返回值上**无法区分** —— 后者是正常结果，不需要排查。
于是每一次这样的 UPDATE 都会悄悄丢掉一个写入。

## 根因：两个相邻的缺陷

### 1. 裸列名不匹配任何表

```rust
let table_has_update = resolved_set
    .iter()
    .any(|(col, _)| col.starts_with(&format!("{}.", table_prefix)));
if !table_has_update { continue; }
```

`UPDATE t1, t2 SET k = 99` 里的 `k` 是**裸列名** —— 不以 `t1.` 或 `t2.` 开头，
两张表都被 `continue` 跳过。

而 MySQL 的语义是：多表 UPDATE 中未限定的列**应用于所有表**。代码要求写全
`t1.k`，于是唯一符合 MySQL 语法的写法反而什么都不做。

`total_count += 1` 在 `continue` 之外，照常执行 → 报告 1。

### 2. SET 写进错误的列槽位

```rust
combined_cols.iter().find(|(name, _)| name.ends_with(&format!(".{}", target_col)))
```

组合 schema 的列名是 `prefix.col`，`ends_with` 只找到**第一个**匹配。所以即使
写全限定名 `SET t1.k = 1, t2.k = 2`，两次写入也都落在 `t1.k` 的槽位，
`t2.k` 从未被写。

第二条在基线上单独可复现：`qualified_names_update_their_own_tables` 在基线失败。

## 改动

`src/engine_dml.rs` 的 `execute_update_multi_table`：

1. **SET 应用改为按表**：遍历每张表的列，用组合 schema 的全名 `prefix.col` 精确
   匹配 SET 项，写进该表自己的列偏移区间（`col_offsets[t] + i`）。
   `col == "t1.k"` 或 `col == "k"` 都算命中该列 —— 前者限定，后者按 MySQL 语义
   应用于所有表
2. **表是否有更新目标的判定**：SET 项带前缀则只匹配该表；不带前缀则**匹配所有表**

顺带删掉 `combined_cols` —— 它只被已移除的 `ends_with` 搜索使用。

## 变异验证

### M-L：恢复 requires-prefix 判定 — CAUGHT

```
bare_column_name_updates_every_table ... FAILED
bare_column_name_with_where_updates_every_table ... FAILED
```

### 基线对照：7 条中 4 条失败

```
qualified_names_update_their_own_tables ... FAILED      <- 缺陷 2
bare_column_name_updates_every_table ... FAILED         <- 缺陷 1
bare_column_name_with_where_updates_every_table ... FAILED
aliases_are_resolved_to_their_tables ... FAILED         <- 别名前缀同理
```

修复后 7/7 通过。

别名那条值得单列：`UPDATE t1 AS a, t2 AS b SET a.k = 7, b.k = 8` 在基线上同样
全部失败，因为 `table_prefix` 取的是 alias，而 `ends_with(".k")` 无法区分
`a.k` 与 `b.k`。

## 测试（`tests/multi_table_update_5117.rs`，7 条）

| 用例 | 钉住的 |
|------|--------|
| `bare_column_name_updates_every_table` | 裸列名应用到所有表（缺陷 1） |
| `bare_column_name_with_where_updates_every_table` | 同上，带 WHERE |
| `qualified_names_update_their_own_tables` | 两个限定名写到各自的列（缺陷 2） |
| `qualified_name_leaves_the_other_table_alone` | V312-84：限定名不溢出到别的表 |
| `aliases_are_resolved_to_their_tables` | 别名前缀 |
| `a_where_that_matches_nothing_reports_zero` | **对照组**：真没匹配上时必须报 0 |
| `single_table_update_still_works` | **对照组**：单表路径不受影响 |

两条对照组的必要性：原缺陷的全部危害是「什么都没改」和「报告 1」同时出现。
只修路由而不钉住「真的没匹配上时报 0」，调用方仍无法区分成功与无操作 ——
修完这个 bug，等于把同一个歧义留给下一个人。

## 验证

```
cargo test -p sqlrustgo --lib --all-features                161 passed / 0 failed
cargo test --test multi_table_update_5117                    7 passed
```

`test_v410_atomic_parallel_degree_getter_clamps_to_one` 在一次全量并发中失败过一次。
单独跑通过；基线 3/3 通过、带回改动 3/3 也通过。该用例断言新 engine 的
`parallel_degree()` 初值为 1，与本改动无关（不涉及多表 UPDATE 或库解析）；
#5105 已记录它读 `SQLRUSTGO_EXECUTOR_PARALLELISM` 与其他测试竞争。判为既有 flaky。

## 后续

多表 `DELETE` 在 `MvccStorage` 上提交后其他连接仍不可见（`dml_read_db_5106.rs`
的 ignored 用例），未在本 PR 处理 —— 那是 MVCC 层对多表 DML 的可见性问题，
与本缺陷正交。