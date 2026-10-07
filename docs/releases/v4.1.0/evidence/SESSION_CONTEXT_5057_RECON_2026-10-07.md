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

**53.76% 这个数字本轮未重跑验证**，属既有代码注释中的历史记录，
引用时须注明来源，不得当作本次实测。