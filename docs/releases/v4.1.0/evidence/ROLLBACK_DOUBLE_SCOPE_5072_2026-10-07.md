# #5072 — MemoryStorage 的 INSERT 撤销静默失效 + update 撤销改多行

- **Date**: 2026-10-07
- **Branch**: `fix/5072-rollback-double-scope`
- **Base**: `gitea252/develop/v4.1.0` = `441d69ce90`（PR #5074 合并后）
- **Issue**: #5072（由 #4948 的实施过程带出）

---

## 1. 缺陷一：INSERT 的 undo 双重作用域

### 根因：TxLog 的契约没写在类型上，且有一个生产者违反它

`TxLog`（`crates/storage/src/engine.rs`）的契约是「**存裸表名**，回放时用 `tbl()` 按回放时的活动库解析」—— 这样日志才能被应用到产生它之外的连接上。两个消费者都遵守这个契约：

- `rollback_transaction`（`engine.rs:2188/2192/2201`）：`let key = self.tbl(table);`
- `apply_committed_log`（`engine.rs:1947/1963/1975`）：同样 `self.tbl(table)`

但**生产者不一致**：

| 生产者 | push 的值 | 作用域状态 |
|---|---|---|
| `insert`（2353） | `table_key` | **已作用域** ← 唯一异类 |
| `delete`（2376/2396/2427/2454/2480） | `table.to_string()` | 裸名 ✓ |
| `update`（2518/2546/2595） | `table.to_string()` | 裸名 ✓ |

于是 INSERT 的回放算出 `default\x01default\x01t1`，`get_mut` 返回 `None`，循环体不执行 —— **不报错，不撤销，悄无声息**。

### 实测（三条撤销路径）

```text
P insert-in-tx rows  = 2
P insert after RB    = 2  (expect 1)     <-- 失效
D delete-in-tx rows  = 1
D delete after RB    = 2  (expect 2)     <-- 正常
U update-in-tx       = [[Integer(1), Integer(99)]]
U update after RB    = [[Integer(1), Integer(10)]]  (expect v=10)  <-- 正常
```

DELETE / UPDATE 正常、只有 INSERT 失效，正是「只有 `insert` push 了作用域化键」的直接后果。

### 既有测试为何没抓到

`test_rollback_removes_inserted_rows` 用**未作用域的裸键** `"t"` 预置 `storage.tables`，而 `insert` 写到 `"default\x01t"`：

```rust
s.tables.insert("t".to_string(), ...);      // 裸键 "t"
s.insert("t", ...);                          // 写 "default\x01t"
s.rollback_transaction().unwrap();           // 找 "default\x01default\x01t" —— 啥也没做
let rows = s.scan("t").unwrap();             // 读 "default\x01t" -> 1 行
assert_eq!(rows.len(), 1);                   // 成立，但与 rollback 无关
```

**这是一个空断言**：无论 rollback 有没有执行，`scan("t")` 都只看到 1 行。已改为经 `create_table` 建表，键对齐后才真正测到撤销。

### 修复

`insert` 改为 push 裸表名，与 `delete` / `update` 一致；并把契约写进 `TxLog` 字段的文档注释（这是能防第三次违反的部分 —— 前两次都是因为契约只存在于阅读者脑子里）。

## 2. 缺陷二：UPDATE 的撤销缺 `break`

同一个回放循环里：

```rust
for record in records.iter_mut() {
    if *record == _new {
        *record = prior.clone();
        // #5072: break restored
    }
}
```

没有 `break` 时，若表里**本来就有**一行等于 `new`，它会被一并改写成 `prior`，该行被破坏。

可复现场景：表初始为 `[1,1], [1,99]`；`UPDATE ... WHERE v=1 SET v=99` 把首行改成 `[1,99]`，此时两行都是 `[1,99]`，而 TxLog 只有一条 `(prior=[1,1], new=[1,99])`。回放时无 `break` 会把**两行**都改成 `[1,1]`，把原有的 `[1,99]` 销毁。

同文件里 `apply_committed_log` 的同类循环**本来就有 `break`**，此处是没有 —— 两处不一致本身就是线索。

## 3. 影响面（据实修正 issue 的措辞）

issue #5072 初稿写的是「用户在显式事务里 INSERT 后 ROLLBACK，行不会被撤销」，读起来像生产数据问题。实测核对后**下调**：

- `FileStorage` **根本没有 `tx_log`**（`grep tx_log crates/storage/src/file_storage.rs` 无结果）—— 生产服务器与 REPL 走的 `FileStorage` 不经过这条路径。
- `ExecutionEngine::with_memory()` 的全部调用点都在 `tests/` 与 `crates/bench/tests/`，**没有任何生产二进制调用**。

所以这是**测试 / 基准用存储后端的正确性缺陷**，不是线上数据丢失。但仍然要修：TPC-H / OLTP 基准与大量集成测试都建立在 `MemoryStorage` 的事务语义之上，rollback 契约坏着意味着这些测试在静默地给出错误结论。

## 4. 变异验证

| ID | 变异 | 结果 | 命中 |
|---|---|---|---|
| M23 | `insert` 退回 push `table_key`（重新双重作用域） | **CAUGHT** — 2 FAILED | `rollback_actually_undoes_an_insert_4948`、`test_rollback_removes_inserted_rows` |
| M24 | 撤掉 update 回放循环的 `break` | **CAUGHT** — 1 FAILED | `rollback_of_update_restores_only_the_changed_row_5072` |

无无效变异。

### 4.1 M24 首轮存活 —— 补测才算修

第一次跑 M24 时**存活**：全量 lib 893 passed，0 failed。也就是说缺陷二的修复当时**没有任何测试覆盖**，若就此收工就会宣称一个未经验证的修复。

补的测试第一版也是错的：`table_with_rows_4948` 建的是单列表，而赋值写的是第 1 列（越界），UPDATE 根本没生效 —— 失败点落在中间断言而非回滚断言。改为显式建双列表 `(id, v)` 后才测到真正的行为。

这条记在这里，是因为它与本轮 #4948 的 M22 是同一个模式：**变异存活先怀疑测试夹具，再怀疑实现**。

## 5. 触及面

- `crates/storage/src/engine.rs`
  - `TxLog.inserted` 字段文档：写明「存裸表名」的契约与违反后的症状
  - `MemoryStorage::insert`：push 裸表名（原 push 已作用域的 `table_key`）
  - `rollback_transaction` 的 update 分支：恢复 `break`
  - `test_rollback_removes_inserted_rows`：由空断言改为经 `create_table` 建表
  - 新增 `rollback_of_update_restores_only_the_changed_row_5072`
  - `rollback_actually_undoes_an_insert_4948`（PR #5071 引入时标 `#[ignore]`）：摘掉 `#[ignore]`

## 6. 验证

| 项 | 结果 |
|---|---|
| storage `--lib` | **894 passed / 0 failed**（基线 889，+5） |
| storage 全量 | 见下 |
| executor 全量 | 见下 |
| clippy `-p sqlrustgo-storage` | 0 warning |
| fmt | clean |

## 7. 证据字段（ADR-014）

- `source_agent`: mcode（MCode desktop session）
- `source_run`: 变异 M23 / M24；基线 lib 894 passed / 0 failed / 0 ignored
- `timestamp`: 2026-10-07
- `conflict_resolution`: issue 初稿称「用户在显式事务里 INSERT 后 ROLLBACK，行不会被撤销」，隐含生产影响。实测 `FileStorage` 无 `tx_log`、`with_memory()` 无生产调用点后，下调为「测试 / 基准用后端的正确性缺陷」，并在 issue 评论中同步更正。