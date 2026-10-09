# 变异审计：4 条 PENDING 结清，#5015 的 SURVIVED 是假象

- source_agent: mcode (MiniMax-M3.1-Flash-Preview)
- source_run: 2026-10-09 会话
- 基线: `1e7b3d053e`（PR #5164 合并后，develop/v4.1.0 HEAD）
- 分支: `test/5015-mvcc-list-databases`
- 工具: `scripts/gate/mutation_runner.py`
- Refs #5015, #5009, #5013

## 背景

v4.1.0 的 fix PR 要证明「我的修复被测试钉住了」，办法是把该 PR 改过的那一行
生产代码反向变异一次，看注册的测试目标会不会变红。红 = KILLED（修复是真的且被
钉住），绿 = SURVIVED（修复可能是空的）。

上一轮结束时 27 条里还剩 4 条 PENDING：

| PR | 变异目标 |
|----|----------|
| #5007 | `MvccStorage` 的 ODKU 更新后重读 |
| #5015 | `MvccStorage::list_databases` 转发 |
| #5037 | ROLLUP/CUBE 分组键改用 `Value` 前缀 |
| #5001 | 5 层列名解析器去掉了大小写不敏感那层 |

## 结果

| PR | 结果 | 杀掉它的测试 |
|----|------|--------------|
| #5007 | KILLED | `odku_update_visibility_4995`（4 条全红） |
| #5037 | KILLED | `rollup_cube_test`（2 条红） |
| #5001 | KILLED | `engine::tests::test_find_column_index_*`（3 条红） |
| #5015 | **KILLED（补测后）** | 新增 `issue_5009_mvcc_list_databases_test`（3 条全红） |

审计全量结论：**16 KILLED / 0 SURVIVED / 11 REVIEW / 0 PENDING**。

#5007 / #5037 / #5001 三条被现有测试直接杀掉，无需改动。

## #5015：SURVIVED 的两层原因

### 第一层：测试跑的根本不是当前代码

`tests/integration/ddl/show_databases_test.rs` 挂在**根 package** 下，用
`bin_path()` 去找 server 二进制。但 `sqlrustgo-mysql-server` 这个 bin 属于
`crates/mysql-server`，Cargo 只为拥有该 bin 的 package 设置
`CARGO_BIN_EXE_<name>` —— 根 package 下这个变量恒为 `Err(NotPresent)`
（已用探针实测确认）。

于是 `bin_path()` 回退到扫描磁盘：

```rust
"target/release/sqlrustgo-mysql-server",
"target/debug/sqlrustgo-mysql-server",   // <-- 审计现场就是这个
```

实测该文件 mtime 为 `2026-10-06 14:45`，而 HEAD 是 `2026-10-08`。测试报告
PASS，执行的是一个比 HEAD 旧 8 天的二进制。里面当然没有这次变异。

这是「测试通过了」与「测试跑过什么」被混为一谈。#4974 建立的判断在这里再次
成立：缺陷住在包装层，而包装层的测试会绿。

**改法**：文件移入 `crates/mysql-server/tests/`，`bin_path()` 换成
`env!("CARGO_BIN_EXE_sqlrustgo-mysql-server")`。`env!` 是编译期求值 —— 没有
这个 bin，测试编译不过。保证从「运行时看磁盘」变成「编译期可证」。

### 第二层：换成新二进制，repl 路径依然看不见这条转发

`run_repl` 构造的是 `MemoryStorage`：

```
crates/mysql-server/src/main.rs:552
    let storage = Arc::new(RwLock::new(sqlrustgo::MemoryStorage::new()));
```

`MvccStorage` 只在 server 的 TCP 路径上被构造
（`crates/mysql-server/src/lib.rs:6820`，V400-MVCC-ENABLE 把 `FileStorage`
包进去）。repl 子进程永远不会走到 `MvccStorage::list_databases`，所以这次
变异**按构造必然存活**。

该测试保留 —— 它覆盖 bin 的参数解析和 REPL 分发，是唯一做这件事的测试。
但文件头写明它结构上看不见 MVCC 层。此前它被当作 #5009 的主要防线，这个说法
站不住。

## 新增测试

`crates/mysql-server/tests/issue_5009_mvcc_list_databases_test.rs`，走
`start_ephemeral`（`storage: None`，即出货默认），真实链路：

```
FileStorage -> MvccStorage -> WalStorage -> Box<dyn StorageEngine>
```

三条断言，变异后全红：

| 测试 | 钉住的行为 | 变异后 |
|------|------------|--------|
| `mvcc_wrapped_storage_enumerates_a_created_database` | 建了就查得到 | FAILED |
| `dropped_database_disappears_from_the_mvcc_enumeration` | 删了就不在 | FAILED |
| `show_tables_from_a_created_database_is_not_reported_unknown` | 第二个消费者 | FAILED |

第三条走 `ensure_database_known`（`src/engine_ddl.rs:342`），它读的是同一份
枚举 —— `SHOW TABLES FROM <db>` / `SHOW FULL TABLES` / `SHOW TABLE STATUS`
都会经过它。这是这条转发的第二个消费者，也是断言能区分两层的原因：只测
`SHOW DATABASES` 的话，坏的是枚举；加上这条，坏的是「存在性校验」。

### 一条被删掉的断言

写测试时我先断言「重复 `CREATE DATABASE` 应当报错」，理由是 `CREATE` 的守卫
也读 `list_databases()`。**实测失败**：转发完好时，重复创建照样成功 ——
引擎根本没有这个守卫。

```
Got Ok(Ok { affected_rows: 0, last_insert_id: 0, status_flags: 2, ... })
```

这条断言钉不住任何东西（有无转发都通过），已删除。留着它只会让这个文件显得
比实际覆盖更多 —— 而「覆盖比声称的多」正是本次审计要消除的那类偏差。

## mutation_runner.py 的两个假判决来源

审计过程中发现 runner 自身也会造出假分数，一并修掉。

### 1200s 超时伪装成 SURVIVED

`crates/storage` 的变异要重建整个 workspace（`sqlrustgo-mysql-server` 单个
bin 冷编约 4 分钟）。1200s 默认值把 run 砍在编译中途，进程退出码非 0 →
`outcome = KILLED`…… 不对，是超时抛异常，但**看起来**和「测试跑完了」没有
区别。实测这次直接崩在 traceback 里。

改为 `MUT_TIMEOUT` 环境变量（默认 3600s）。

### 超时必须显式，不能算进任何分数

超时现在返回独立 outcome `TIMEOUT`，并计入汇总行：

```
=== Summary ===
  TIMEOUT:  0
```

一个从未跑完的 run 不产出判决。把它算成 SURVIVED 是凭空造证据 —— 正是这个
仓库反复在治理文档里禁止的事。

## 遗留：同样的模式还有约 11 处

以下根 package 测试都用同一套 `bin_path()` 回退逻辑启动 server 二进制，
本次只修了审计证伪的那一个：

```
tests/integration/dml/insert_odku_test.rs
tests/integration/ddl/alter_table_test.rs
tests/integration/ddl/truncate_table_test.rs
tests/integration/wire/cli0{1,2,3}_*.rs
tests/integration/wire/server_threads_cli_test.rs
tests/integration/sql/server01_v2_test.rs
tests/integration/sql/server01_server_test.rs
tests/integration/stress/concurrent_insert_test.rs
tests/integration/admin/mysqladmin_e2e_test.rs
```

它们**未经证实**，也**未被证伪**：只要工作区里碰巧有一个足够旧的 bin 在，
它们就会拿旧代码给出 PASS。逐个改法与本次相同（移入
`crates/mysql-server/tests/` + `env!`），但要逐个确认迁移后仍然编译通过，
不宜顺手批量替换。建议单开一个 issue。

## 复现

```bash
python3 scripts/gate/mutation_runner.py --pending   # 4 条
python3 scripts/gate/mutation_runner.py --pr 5015   # 单条
MUT_TIMEOUT=5400 python3 scripts/gate/mutation_runner.py --pr 5001
```

判定标准：`KILLED` = 修复被钉住；`SURVIVED` = 测试有洞；`SKIP` / `INVALID` /
`TIMEOUT` = 没跑出判决，都不算通过。