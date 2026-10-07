# SessionContext executor 侧：为什么本批只交测试，不交实现

- source_agent: mcode (MiniMax-M3.1-Flash-Preview)
- source_run: 2026-10-07 会话
- 基线: `496531a00fe6`（PR #5095 合并后）
- 分支: `feat/5057-session-db`
- Refs #5057

## 结论

按最小步子交付了**下一批的验收标准**，代码改动整体回退。原因写在下文，不是进度问题，是半修会造成新缺陷。

## 侦察结果：读取点比预估集中

原以为 executor 侧要改 55+ 处 `current_db`。实际：

| 位置 | 数量 | 性质 |
|------|------|------|
| `src/engine_select.rs:777` | **1** | 真正的读取点，其余 54 处都是把这个局部变量往下传参 |
| `src/engine_dml.rs` | 3 | 已做过语句级快照（`stmt_db`） |
| `src/execution_engine_methods.rs:792` | 1 | `execute_use_database` 的写入点 |

`ExecutionEngine` 本来就是每连接一个（`mysql-server` 的 `handle_connection` 各建各的），所以加一个 `session_db` 字段天然就是每连接值。

实现部分我写完了：`ExecutionEngine.session_db` 字段（7 个构造点同步初始化）、`engine_select.rs:777` 改读 `session_db`、`engine_dml.rs` 3 处改读 `session_db`、`execute_use_database` 同时写两者并说明锁序（`session_db` → `storage`，与读路径一致）。编译通过。

## 为什么回退

测试立刻暴露了第二层缺口。

### 缺陷（基线实测）

`tests/session_db_isolation_5057.rs` 构造生产拓扑 —— **两个 engine 共用一个 storage**。基线上 4/5 失败：

```
test select_reads_the_connections_own_database ... FAILED
  connection A (in d1) read the wrong database
  left: [Integer(2)]
 right: [Integer(1)]
```

连接 A 在 d1，B 在 d2，A 读到了 d2 的行。这是 `USE` 未连接级隔离的直接后果。

### 半修会制造新的不一致

把 `engine_select.rs:777` 换成 `session_db` 之后，表**元信息**解析用连接值，但取**数据**仍走共享值：

```
engine_select.rs:3536    self.scan_for_reader_with(&**storage, table)
                                          ^^^ 已持有 current_db 参数却没用它
engine_select.rs:1420    storage.scan_pk(lookup_table, &pk_column, &pk_value)
engine_select.rs:3566    storage.scan_with_index(table, ...)
```

`scan_for_reader_with` → `storage.scan_in(table, reader_tx)` —— `scan_in` 只带事务快照，不带库；`FileStorage` 内部用 `current_db` 解析。

于是同一张表的 schema 来自 d1、数据来自 d2。可观测结果与修复前一样错（都返回 d2 的行），但**内部机制从「自洽地错」变成「不自洽地错」**。后者更危险：下一次有人修数据路径时，会误以为元信息侧已经正确。

这正是 #5086 的教训 —— 当时 `flush_all_buffers` 的半修会把「跳过不写」变成「写错地方」。

### 补齐需要的范围

要让读路径真正参数化，得动：

1. `StorageEngine::scan_in` 增加 `(db, table, tx)` 变体 —— 现有 `scan_in_db(db, table)` 不带事务快照，直接替换会丢 MVCC 语义
2. `FileStorage` 的 MVCC 读取实现按库分流
3. `scan_pk` / `scan_with_index` 的库变体接线（`scan_with_index_in_db` 已在 #5089 存在，`scan_pk` 的库版本还没有）
4. **4 层包装层转发**：`BoxStorageEngine` / `ParallelWalStorage` / `MvccStorage` / `WalStorage` / `BinaryStorage`
5. `scan_with_ahi` 已持有 `current_db` 参数却未使用 —— 接线点在这里，改动集中但依赖前四项

这是一个完整批次，不是收尾。半路的任何一点都不该进主干。

## 交付物

### `tests/session_db_isolation_5057.rs`

5 个用例，4 个 `#[ignore]`，是下一批的验收标准：

- `select_reads_the_connections_own_database` —— 核心
- `interleaved_use_does_not_redirect_the_other_connection` —— 反复交替 `USE`，单次切换测不出来
- `concurrent_reads_stay_isolated` —— 两线程各 50 次
- `last_use_wins_on_the_mirror_but_not_on_the_connection` —— 镜像归最后一个 `USE`，但连接读自己的库
- `shared_current_db_still_agrees_with_the_last_use` —— **保持 live**，对照组

对照组为何必须 live：它断言 `storage.current_db` 仍与最后一次 `USE` 一致。这不是目标，是**未参数化路径的前提**（`FileStorage::tbl`、DDL rename/truncate 仍读它）。若将来有人让 `USE` 只写 `session_db`，这些路径会静默地指向最后写入者 —— 而上面 4 个用例全走 `*_in_db`，仍会通过。只有这一条能抓住。

`last_use_wins_on_the_mirror_but_not_on_the_connection` 是配套的另一半：镜像确实是最后写入者，且连接确实不受它影响。只断言前半句会在缺陷存在时通过。

## 下一批的范围

按依赖顺序：

1. `scan_in` 的 `(db, table, tx)` 变体 + FileStorage 实现
2. 4 层包装层转发
3. `scan_pk` 的库变体
4. `scan_with_ahi` 接线（`current_db` 参数已在手，只需不再忽略它）
5. `ExecutionEngine::session_db` + `execute_use_database`（本次已写好，随第 1 步一起接）
6. `mysql-server` 删掉 #5084 加的每命令重断言 —— 最后做，因为它当前是 `USE` 能生效的保障

第 6 步放最后：#5084 的重断言 + `*conn_db = storage.read().current_db()` 是 `USE` 修复的支柱。在 `session_db` 真正成为权威之前删掉它，`USE` 会立刻退回 no-op。

## 验证

- 基线 `cargo test --test session_db_isolation_5057`：1 passed / 4 ignored / 0 failed
- `cargo test -- --ignored` 可复现 4 项失败，输出即规格
- `cargo fmt --check --all` 通过
- 代码改动已全部回退，工作区除本测试文件外无改动