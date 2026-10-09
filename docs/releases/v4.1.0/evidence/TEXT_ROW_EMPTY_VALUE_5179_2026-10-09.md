# 文本行首列为空被当成二进制行（#5179）

- source_agent: mcode (MiniMax-M3.1-Flash-Preview)
- source_run: 2026-10-09 会话
- 基线: `72e4a60817`（PR #5180 合并后）
- 分支: `fix/5179-text-row-empty-value`
- Refs #5179

## 起点：一个被归错因的失败

PR #5178 把 11 个启动 server 二进制的测试迁进 owning crate。迁移前后 `mysqladmin_e2e_test`
都是 7 passed / 1 failed，失败的是：

```
wire_client_status_returns_well_formed_report
  panicked at mysqladmin_e2e_test.rs:116:
  status query: Query("global_status query:
    Protocol error: Binary row: data too short for null bitmap")
```

B2 禁用注册表里它躺了很久，备注是「pre-existing failure / **Investigate mysqladmin e2e
flow**」。这个归因是错的 —— 缺陷不在 admin 的 e2e 流程里，在所有 e2e 流程共用的那个客户端
行解析器里。

## 根因：用首字节猜协议，而这个字节天生二义

`parse_result_set_with_infile` 原本这样决定一行是文本还是二进制：

```rust
let first_byte = row_pkt.payload[0];
let is_binary = first_byte == 0x00;
```

二进制行（`COM_STMT_EXECUTE` 的响应）以 `0x00` 开头，行内其余字节是 NULL bitmap + 原始值。

问题在于文本协议里每个值都是 length-encoded string，而**长度为 0 的字符串编码成单个字节
`0x00`**。所以一个再普通不过的文本行 —— 只是首列恰好是 `''` —— 首字节同样是 `0x00`，
被送进 `parse_binary_row`，那里把剩下的字节当 NULL bitmap 读，然后越界：

```
Binary row: data too short for null bitmap
```

整条查询失败，不是那一行失败。

## 可达面比报出来的更宽

报出来的触发点是 `WireAdmin::status()` 发的 `SELECT @@global_status`。逐条实测（每条独立
连接，避免串扰）：

| 查询 | 修复前 | 原因 |
|------|--------|------|
| `SELECT @@global_status` | ERR | 未知变量回落到空 `Text` |
| `SELECT @@sql_mode` | ERR | 显式建模为空 `Text` |
| `SELECT ''` | ERR | 空字面量 |
| `SELECT 'x'` | ok | 首字节非零，侥幸通过 |
| `SELECT NULL` | ok | NULL 编码为 `0xfb`，不碰这个坑 |

也就是说 `SELECT ''` 一样挂。**`SELECT 'x'` 通过不是因为它对，而是因为它碰巧躲开了** ——
这一列只要有一个合法查询返回空字符串，客户就整个拿不到结果集。

## 修法：让调用方告诉解析器，而不是猜

调用方永远知道协议 —— 它刚发出的那条命令就是。`COM_QUERY` 回文本行，
`COM_STMT_EXECUTE` 回二进制行，这是命令的固有属性，不需要从载荷里反推。

给 `parse_result_set_with_infile` 加 `binary_rows: bool`，三个调用点各自如实声明：

| 调用点 | 命令 | 传入 |
|--------|------|------|
| `execute` / `execute_with_local_infile` | COM_QUERY | `false` |
| `execute_multi` | COM_QUERY（批量） | `false` |
| `execute_prepared` | COM_STMT_EXECUTE | `true` |

原来那行嗅探删除。

`parse_result_set`（只读便捷入口）和 `local_infile_test.rs` 的 8 个调用点一并显式传值，
不留「默认猜」的旁路。

## 顺带修掉一个独立的 flake

修完协议问题后 B2 门禁仍然红，但红的已经是另一个测试：

```
wire_admin_handles_connection_failure_gracefully
  panicked: expected Connect error
```

它连 `server.addr.port() + 1`，假定那个端口没人占。并行跑时另一个测试的 ephemeral
server 经常正好占着，连上成功 → 断言失败。实测 3 次全量跑挂了 1 次。

**「活端口 +1」不等于「关着的端口」**。改为绑 `127.0.0.1:0` 读出 OS 分配的端口再
`drop` —— 在 drop 返回那一刻，这个端口确实是关着的，且因为监听器刚才是本进程持有，
期间不会有别人抢走。连续 5 次全量跑：5/5 稳定。

两个缺陷互相独立，都写在这里是因为它们把同一个 binary 挡住了。

## 回归测试

`crates/mysql-server/tests/text_row_empty_first_value_5179.rs`，5 条，走真实 wire 到真实
server（因此服务端行编码也在覆盖范围内，不只是孤立测解析器）：

| 测试 | 钉住的行为 |
|------|------------|
| `empty_string_as_the_only_value_parses` | `SELECT ''` |
| `unknown_system_variable_returning_empty_text_parses` | `SELECT @@global_status`（原失败点） |
| `modelled_empty_text_variable_parses` | `SELECT @@sql_mode` |
| `empty_value_in_a_later_column_parses` | `SELECT 'a', ''` —— 整行都要解出，不能在第 0 列截断 |
| `non_empty_first_value_still_parses` | 反向护栏：修复前这类是「侥幸通过」，必须仍然通过 |

## 从禁用清单移除

`mysqladmin_e2e_test` 已从 B2 `DISABLED_BINARIES` 移除（`run_b2_per_binary.py` 与
`check_beta_v3.12.0.sh` 两处同步清单都改，同步守卫实测 89 = 89 一致）。门禁实跑结果：

```
→ running mysqladmin_e2e_test
  [✓] PASS  elapsed=1.2s  passed=8  failed=0
```

移出禁用清单才是「修好了」的含义 —— 只让测试变绿但继续挂在禁用名单上，等于把同一个
缺陷换个地方藏起来。注册表条目已改为 REACTIVATED 并记上真实根因，替代那句错误的
「Investigate mysqladmin e2e flow」。

## 验证

| 检查 | 结果 |
|------|------|
| `cargo test -p sqlrustgo-mysql-client` | 114 passed |
| `cargo test -p sqlrustgo-mysql-server --test text_row_empty_first_value_5179` | 5 passed |
| `cargo test -p sqlrustgo-mysql-server --test mysqladmin_e2e_test` | 8 passed；并行连跑 5 次 5/5 |
| wire 回归（prepared_stmt / multi_stmt / mysql_server_tests / issue_4516_wire / smoke / local_infile） | 123 passed |
| B2 门禁 `mysqladmin_e2e_test` | PASS 8/8（且不再 SKIP_DISABLED） |
| 两处禁用清单同步守卫 | 89 = 89，一致 |

二进制路径的护栏在 `prepared_stmt_params_test`（7 passed）里：改判定依据若弄错了协议方向，
prepared statement 的行会先炸。