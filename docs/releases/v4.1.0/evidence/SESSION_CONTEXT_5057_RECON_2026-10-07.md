# #5057 SessionContext —— 侦察报告与方案修正

- **日期**: 2026-10-07
- **分支**: `feat/5057-session-context`（基于 `gitea252/develop/v4.1.0` = `0136139707`）
- **source_agent**: mavis (MiniMax-M3.1-Flash-Preview)
- **source_run**: 本会话 `mvs_70c2cc01e3564280804e7a54c6dbfca5`
- **性质**: 侦察 + 实测证据。本文件不含实现结论。

---

## 1. 实测：`USE` 作为 SQL 在 server 路径上完全无效

探针：`crates/mysql-server/tests/probe_5057_use_sql_path.rs`

```
running 3 tests
before USE, SELECT DATABASE() -> default
USE d1 -> Ok { affected_rows: 0, ... }
after USE d1, SELECT DATABASE() -> default
OBSERVED: `USE d1` was accepted but the next command reports "default".
```

**`USE d1` 返回 OK packet，随后任何语句仍然在 `default` 上解析。**

这比 issue #5057 描述的两个问题更严重：issue 只提到 TOCTOU 与 `DATABASE()` 恒返回
`default`，而这里 `USE` 本身就是 no-op。

### 成因（读码定位，非推测）

| 位置 | 行为 |
|------|------|
| `crates/mysql-server/src/lib.rs:5316-5321` | 每条命令前 `storage.write().set_current_db(conn_db)` 重断言 |
| `crates/mysql-server/src/lib.rs:5358-5380` | `COM_INIT_DB` **同时**更新 `storage.current_db` 和 `*conn_db` |
| `src/execution_engine_methods.rs:781` | `execute_use_database` 只做 `self.storage.write().set_current_db(db)?`，**从不碰 `conn_db`** |

`USE` 走 `COM_QUERY`，到不了 `COM_INIT_DB` 分支。于是：USE 把共享值改成 `d1` →
下一条命令的重断言拿旧 `conn_db`（`default`）覆盖回去。

`tests/e2e/multi_db_isolation_5025.rs` 测不到它：该文件第 23 行自述用
`MemoryExecutionEngine` 进程内驱动、**不起 server**，因此没有 `conn_db` 重断言。

---

## 2. 侦察发现的 4 项与 issue 描述的偏差

### 偏差一：`USE` 缺陷比 issue 说的更严重

见上。这不是 TOCTOU，是"开关本身失效"。

### 偏差二：第四步的前提已不成立

issue 第四步称 "`expr_utils.rs:708` 的 `DATABASE()` 硬编码在 `eval_fn` 里，
应归位到 `resolve_system_variable` 路径"。

实际情况：`crates/executor/src/expr/mod.rs:1423` 里
`"DATABASE" | "SCHEMA" => Value::Text("default".to_string())` 仍在，
但 #5025 已经在 `#5025` 之后加了一层 AST 预处理：

- `src/execution_engine.rs:1044` `substitute_current_database_in_expr`
- `src/engine_select.rs:783` 调用它，替换成 `Expression::Literal("'d1'")`

并且有 4 个测试覆盖（`src/execution_engine_tests.rs:1761-1830`）。
所以第四步不是"新增能力"，而是"把现有实现的取数来源从共享 storage 改成会话状态"。

### 偏差三：`ExecutionEngine` 已经就是每连接实例

issue 的方案前提是"storage 共享 → 状态必须显式传参"。但代码里：

```
crates/mysql-server/src/lib.rs:7011  thread::spawn(move || handle_connection(...))
crates/mysql-server/src/lib.rs:6434  ExecutionEngine::new(storage.clone())   // 在 handle_connection 内
```

`storage` 是 `Arc` 共享的，`ExecutionEngine` 是**每连接新建**的，而且它已经持有
每连接状态 `session_vars: Arc<RwLock<HashMap<String, SqlValue>>>`（`execution_engine.rs:190`）。

即：**会话状态的正确宿主已经存在**，只是 `db` 被放错了地方（storage 上）。

### 偏差四：显式传 `db` 的先例已在 trait 里

`StorageEngine` trait 已有三个方法显式收 `db`：

- `scan_in_db(&self, db: &str, table: &str)` (`engine.rs:1204`)
- `has_table_in(&self, db: &str, table: &str)`
- `get_table_info_in(&self, db: &str, table: &str)`

所以"db 显式传参"在 storage 层已经是既有模式，不需要新发明。

---

## 3. 工程规模（实测引用数）

| 文件 | `current_db` 引用数 |
|------|--------------------|
| `crates/storage/src/file_storage.rs` | 72（其中 `self.current_db` 48 处） |
| `src/engine_select.rs` | 55 |
| `crates/storage/src/mvcc_storage.rs` | 11 |
| `crates/mysql-server/src/lib.rs` | 7 |
| `crates/storage/src/wal_storage.rs` / `parallel_wal_storage.rs` / `binary_storage.rs` | 各 4 |
| `src/execution_engine_methods.rs` | 1 |
| 测试（`probe_5025_conn_db_race.rs` / `file_storage_db_isolation_5025.rs` / `index_and_table_survive_restart_5025.rs`） | 26 |

`StorageEngine` trait 里需要加 `db` 参数的表操作方法：约 12 个
（`scan` / `scan_with_filter` / `scan_with_filter_in` / `scan_with_index` /
`parallel_scan` / `insert` / `force_insert` / `delete` / `delete_collect_pks` /
`delete_if` / `update` / `update_if` / `create_table` / `drop_table` /
`get_table_info` / `has_table`）。

实现方：`FileStorage` / `MemoryStorage` / `MvccStorage` / `WalStorage` /
`ParallelWalStorage` / `BinaryStorage` / `BoxStorageEngine`。

编译基线（本机，内存紧张）：`cargo build -p sqlrustgo --all-features` = **4m27s**。

---

## 4. 方案修正：不单独发 PR 修 `USE`

考虑过先单独修 `USE`（让 server 在 `COM_QUERY` 后同步 `conn_db`），
**放弃该方案**：SessionContext 落地后 `storage.current_db` 不复存在，
`conn_db` 不再需要每命令重断言，这个修复会被整体重写。发出去即为 churn。

改为一次性完成 SessionContext 改造，把 `USE` 失效作为它的可观测后果一并消掉。

### 不可回避的核心约束

`db` 一旦移出 storage，**表操作必须显式收到 db**。否则：

- `SELECT DATABASE()` 从会话状态读 `d1`
- 但 `SELECT COUNT(*) FROM t` 的表作用域仍来自 storage 的全局值 `default`

两者不一致，比现状更糟。所以"给表操作加 `db` 参数"不是可选项。

---

## 5. 未复现的部分（如实记录）

探针 3（双连接串扰）本轮**未复现**：

```
connection B, handshake -D d_b, SELECT DATABASE() -> d_b
connection A after USE d_a, SELECT DATABASE() -> default
connection B now reports SELECT DATABASE() -> d_b
not observed this run
```

原因是探针 3 的时序是顺序的：连接 B 每条命令前的重断言把自己的 `conn_db`
恢复了，串扰窗口没被打开。

TOCTOU 的结构性论证不依赖本次复现：`FileStorage::tbl()` 每次调用重读
`current_db`（`file_storage.rs:1783`），重断言与语句执行之间无锁，窗口存在于
结构上。已有 `crates/storage/tests/probe_5025_conn_db_race.rs`（无断言，仅打印）
记录该窗口，`src/engine_select.rs:775-777` 的注释记录了 53.76% 误导向的实测值。

**53.76% 这个数字本轮未重跑验证**，属既有代码注释中的历史记录
（出处：`crates/storage/src/engine.rs:1289`，`get_table_info_in` 的文档注释），
引用时须注明来源，不得当作本次实测。

---

## 6. 实施过程中发现的：`db` 必须穿过整条持久化链（P1-b 的实际规模）

预估 `P1-b` 是"给 `FileStorage` 的表方法加 `db` 参数"。实际实施时发现比这深一层：
`db` 不只被方法**入口**用，它沿着调用链一路传下去：

| 层 | 位置 | 对 `current_db` 的依赖 |
|----|------|----------------------|
| 表方法入口 | `insert`(5076) / `delete`(5152) / `update`(5438) / `create_table`(5576) / `drop_table`(5617) | `self.tbl(...)` |
| 写入 helper | `insert_direct`(4188) / `insert_buffered`(4231) / `flush_buffer`(4288) | 方法体内直接内联 `scoped_key(&self.current_db.read().unwrap(), ...)` |
| 索引维护 | `update_pk_index`(4335) / `update_pk_index_window`(4369) | `self.tbl(table)` |
| 持久化 | `save_table_window`(1443) / `save_table`(1388) | 间接经 `db_dir()`(1789) → `self.current_db_name()`(1809) |
| 磁盘读 | `load_table_in`(1325) / `load_table_delta_in`(1554) | **已经**接受显式 `db` 参数 |

`FileStorage` 共有 **40 处 `self.tbl(...)`** + **4 处 `self.db_dir()`**。
其中只有 `load_table_in` / `load_table_delta_in` 已经参数化 —— 磁盘读路径是对的，
内存缓存与写入路径没跟上。

所以 `P1-b` 的真实规模是：把 `db` 从表方法入口贯穿到写入 helper、索引维护、
持久化三层，约 500–1000 行的单文件精细重构。这不是机械替换：
每一层的方法体都要确认「这个 `db` 是调用方给的那个，不是 storage 当前持有的那个」。

### 附带观察：`last_saved_row_count` 按未作用域的表名索引

`save_table_window`(1455-1460) 用**裸** `table_name` 查
`self.last_saved_row_count`，而 `st.tables` 用的是 `scoped_key(db, table)`。
两个数据库里同名表 `t` 会共享同一个增量保存计数。

本轮**未构造用例验证**，仅从读码得出。推演方向是保守的：
`d1.t` 与 `d2.t` 互相抬高计数只会让 `total_rows <= last_saved` 更早成立，
从而走 re-derive 全量保存分支 —— 不会丢数据，是性能问题不是正确性问题。
即便如此，`db` 参数化之后这个 key 应当一并作用域化。记录在此，
待 `P1-b` 落地时顺带处理或单开 issue。

---

## 7. PR A：先让 `USE` 恢复可用

### 7.1 变更

在 server 的**两处** dispatch 之后，把 storage 的当前库同步回本连接的
`conn_db`：

| dispatch | 位置 | 覆盖的语句 |
|----------|------|-----------|
| `COM_QUERY` | `crates/mysql-server/src/lib.rs` `do_command_loop` | 绝大多数 SQL，含 `USE` |
| `COM_STMT_EXECUTE` | 同上，`COM_STMT_EXECUTE` 分支 | 预编译语句 |

两处都先 `drop(eng)` 再取 storage 锁，与 engine 内部既有的
engine → storage 锁序一致，不引入新的嵌套持锁。

### 7.2 修复后的实测

探针 `probe_5057_use_sql_path.rs` 三个用例的输出全部转为 NOT-REPRODUCED：

```
after USE d1, SELECT DATABASE() -> d1
after USE d2, SELECT DATABASE() -> d2
connection A after USE d_a, SELECT DATABASE() -> d_a
connection B now reports SELECT DATABASE() -> d_b
```

注意第三行：修复前连接 A 报 `default`（USE 失效），修复后报 `d_a`；
连接 B 始终报 `d_b`，说明连接间隔离未被这个改动破坏。

### 7.3 缺陷的真实后果（比「USE 无效」更严重）

变异 M-A 的失败输出给出了铁证 —— 去掉同步后，
`use_routes_table_creation_to_the_selected_database` 打印的是：

```
Tables_in_default, rows: [["only_in_d1"]]
```

即：**在 `d1` 里 `CREATE TABLE` 的表，实际落进了 `default` 库**，
而且 `SHOW TABLES` 诚实地报告了 `Tables_in_default`。
这不是「切换没生效」，是**写到了错误的库**，且全程无任何报错。

### 7.4 回归测试与变异验证

新增 `crates/mysql-server/tests/use_database_regression_5057.rs`，7 个用例：

| 用例 | 作用 |
|------|------|
| `use_as_sql_switches_the_connection_database` | 核心回归 |
| `repeated_use_lands_on_the_last_database` | 连续切换落在最后一个 |
| `use_survives_many_intervening_commands` | 扛住 5 次逐命令重断言 |
| `use_routes_table_creation_to_the_selected_database` | 表作用域随之切换（§7.3 的铁证） |
| `use_of_an_unknown_database_is_rejected_and_leaves_the_current_one_intact` | 被拒的 USE 不改变当前库 |
| `handshake_selected_database_still_works` | 对照组：握手选库原本就正常，防误伤 |
| `use_through_prepared_statement_switches_the_connection_database` | 第二处 dispatch |

变异：

| 编号 | 变异内容 | 结果 |
|------|---------|------|
| M-A | 移除 `COM_QUERY` 处的同步（还原成缺陷态） | **CAUGHT**，6 个中 5 个失败 |
| M-B | 保留 `COM_QUERY`、移除 `COM_STMT_EXECUTE` 处的同步 | **CAUGHT**，精确只失败在 prepared 用例，其余 6 个照常通过 |

M-B 的意义：它证明 prepared 用例真的在测第二处 dispatch，而不是顺带通过。
也顺带证明 server **确实接受 `USE` 作为预编译语句**，因此第二处 dispatch
不是死代码 —— 若不接受，该测试会以「server 必须接受 USE」失败，而不是空过。

### 7.5 这个 PR 不解决什么

跨连接 TOCTOU 仍在。重断言机制在命令不交错的场景下有效（§7.2 的连接 B
始终报 `d_b`），但重断言与语句表解析之间的窗口依旧存在。
消除它需要 §6 描述的 `SessionContext` 重构，也就是 #5057 的主体。

本 PR 的两处同步在 SessionContext 落地时需要改写一次 —— 届时
`conn_db` 不再需要每命令重断言，`db` 从会话上下文直接下传。
这是已知的 churn，不是意外。

---

## 8. P1-b 实施：把 `db` 贯穿到持久化层，以及三处同源的未作用域键

### 8.1 已完成的部分

在 `FileStorage` 上：

| 改动 | 位置 |
|------|------|
| 新增 `tbl_in(db, name)` —— 显式库名的作用域键 | inherent |
| `insert_direct` / `insert_buffered` / `flush_buffer` / `update_pk_index` / `update_pk_index_window` 加 `db` 参数 | inherent |
| 新增 `insert_at(db, …)`（从 trait 侧 `insert` 搬进 inherent，`insert` 与 `insert_in_db` 都转调它） | inherent |
| `save_table` / `save_table_window` / `save_table_full` 加 `db` 参数 | inherent |
| 新增 `table_path_for_write_in(db, table)` —— 写侧路径的显式库名版本 | inherent |
| `scan_in_db` 补上 `insert_buffer` 合并 | trait |
| `table_path_for_write` 并入 `table_path_for_write_in`（原版已无调用者） | inherent |

`insert_in_db` / `force_insert_in_db` 已覆写；**其余 `*_in_db` 仍是 trait 默认
实现（转调无 db 的方法），故本次提交中尚不可依赖。**

### 8.2 顺带修好的：`scan_in_db` 看不见未落盘的行

`#5025` 加的 `scan_in_db` 只读 `tables`，不像 `scan` 那样合并 `insert_buffer`。
于是同一行数据对 `scan` 可见、对 `scan_in_db` 不可见。这个分歧在 #5025 引入，
本轮测试首次撞上（见 §8.4 的夹具教训）。已补齐。

这条看似小，但它卡住整件事：一旦读路径改走 `scan_in_db`，**每一条尚未 flush
的行都会凭空消失**。

### 8.3 三处同源的未作用域键（本轮定位，尚未修）

`FileStorage` 里有三个以**裸表名**为 key、而表本身按 `scoped_key(db, table)`
存储的结构。它们共享同一个成因 —— #5025 把表作用域化时，只改了 `tables` 与
`insert_buffer`，没改这几个「按名字索引表」的旁路结构：

| 结构 | key | 后果 |
|------|-----|------|
| `dirty_tables` | **混用**：insert 路径存裸名，delete/update 路径存 `scoped_key(...)` | flush 时无法判断某张 dirty 表属于哪个库 |
| `last_saved_row_count` | 裸表名 | 两个库的同名表共享增量保存计数 |
| `append_table_delta` / `delta_path` | 裸表名 → 路径 | delta 文件落在 `current_db` 目录 |

### 8.4 `flush` 的跨库行为：**实测写错了目录**

诊断输出（`flush_all_buffers` 之后的数据目录快照）：

```
["a.json", "a_idx_id.json", "b.json", "b_idx_id.json", "d1/", "d2/"]
```

`d1/` 与 `d2/` 都是**空目录**，而 `a.json` / `b.json` 落在了共享根目录 ——
`a` 是在 d1 里建的，`b` 是在 d2 里建的。

成因：`flush()` 的 `drain_dirty_windowed()` 返回 `(name, …)`，随后用
**单一的 `current_db`** 为所有 dirty 表调 `save_table_window`。加上
`dirty_tables` 的 key 混用（§8.3），跨库 flush 无法知道一张表属于哪个库，
于是全部按当前库写。

**这直接决定了 `flush_all_buffers` 怎么处理。** 我一度把它改成遍历所有库的
buffer，但那样只修了一半：`flush_buffer` 会找到正确的行，`flush` 的
`drain_dirty_windowed` 仍会把它们写到当前库目录 —— 从「跳过不写」变成
「写错地方」，后者更糟。

所以本次提交把 `flush_all_buffers` **回退为行为等价**（仍只 flush 当前库的
buffer，只是把 `db` 传对），并在函数注释里写明为什么不做跨库。跨库 flush
必须等 `dirty_tables` 作用域化之后。

为此我一度新增的 `unscoped_key`（`scoped_key` 的逆运算）随回退一起删除 ——
没有调用者的辅助函数不该进仓库，它会在跨库 flush 真正实现时随那次改动再加。

### 8.5 夹具教训：先怀疑夹具

第一版 `file_storage_db_param_5057.rs` 8 个用例**全红**，包括一个测的是
**完全没改动过**的路径（`insert` 跟随 `current_db`）。

按惯例先查夹具，结论是两层：

1. `scan_in_db` 不合并 buffer（§8.2，真缺陷，已修）；
2. `count()` 夹具里先调 `flush()`，而 flush 对**命名库**无效（§8.4）——
   于是计数读到 0，看起来像 `insert_in_db` 没生效。

对照组（未改动路径）先变绿，才确认夹具这一层的判断方向是对的。

### 8.6 验证

- `cargo check -p sqlrustgo-storage --all-features` — 通过
- `cargo clippy -p sqlrustgo-storage --all-features -- -D warnings` — 通过
- `cargo test -p sqlrustgo-storage` — **EXIT=0，1404 passed / 0 failed / 9 ignored**
- `cargo test -p sqlrustgo-storage --test file_storage_db_param_5057` — 4 passed

### 8.7 本次提交不包含

- `create_table_in_db` / `drop_table_in_db` / `delete_in_db` /
  `delete_if_in_db` / `update_in_db` / `update_if_in_db` 的 `FileStorage` 覆写
- `dirty_tables` / `last_saved_row_count` / delta 路径的作用域化（§8.3）
- `flush` 的跨库正确性（§8.4）

`storage` 全量回归结果见 §9。