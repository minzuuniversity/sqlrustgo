# #4019.1 wire-protocol 修复诚实判决

**日期**: 2026-08-12
**判决人**: develop (@ `/tmp/wt-v312-followups`,基于 `develop/v3.12.0` @ 415e4b2cf4)
**模式**: STRICT PROOF MODE — 不证明做过,要证明当前 develop 已真实满足原始验收条件

## 范围

#4019.1 = "修复 wire-protocol CLIENT_SSL 回归(sysbench gate 测试发现)"

## 验收条件(原始)

sysbench OLTP wire 协议应能正常工作,mysql CLI 8.0+ 应能完整接收 query 结果(不被 hang)。

## 实施内容

| 文件 | 变更 | 状态 |
|------|------|------|
| `crates/mysql-server/src/lib.rs` | `send_result_set` + `send_binary_result_set`: DEPRECATE_EOF=1 时**不发送** inter-record separator (移除错误 OK packet) | ✅ |
| `tests/common/mod.rs` | `MySqlTestClient::query_rows`: 根据 `client_caps & 0x01000000` 决定是否读 separator | ✅ |

## 单元测试

```bash
cargo test --test deprecate_eof_wire --all-features
# 期望: 18/18 passed
```

实际结果:18/18 passed,D6 inventory `wire_deprecate_eof_test` 在 D6 全量跑中通过。

## 真实字节验证 (raw wire_capture)

用裸 socket 客户端复现 mysql CLI 行为(cap=0x19bfa285, 含 DEPRECATE_EOF=0x01000000):

**Query: `SELECT 1`**:
| Packet | seq | len | content |
|--------|-----|-----|---------|
| 1 | 1 | 1 | column count = 1 ✓ |
| 2 | 2 | 32 | column def `col_1` |
| 3 | 3 | 2 | row data `[0x01, 0x31]` = "1" |
| 4 | 4 | 7 | trailing OK ✓ |

**Query: `SELECT @@version_comment LIMIT 1`**:
| Packet | seq | len | content |
|--------|-----|-----|---------|
| 1 | 1 | 1 | column count = **3** ✗ (应=1) |
| 2 | 2 | 32 | column def `col_1` |
| 3 | 3 | 32 | column def `col_2` |
| 4 | 4 | 32 | column def `col_3` |
| 5 | 5 | 20 | row data `[0x01, 0x40, 0x01, 0x40, 0x0F, "version_comment"]` = 1 value |
| 6 | 6 | 7 | trailing OK ✓ |

**Schema 不匹配**: column count = 3,row 只有 1 个 value。所 mysql CLI 8.0+ 在 read packet 5 后**等待**更多 value 来填满 3 个 column,看到 packet 6 是 OK 而不是 row continuation,hang。

## 真实 mysql CLI 8.0.46 行为

```bash
timeout 8 mysql -h 127.0.0.1 -P 13309 -u root --ssl-mode=DISABLED --execute="SELECT @@version_comment LIMIT 1"
# exit: 124 (SIGKILL by timeout)
# server log: send_result_set: 3 cols, 1 rows, final_seq=7
# strace: 客户端收到完整 6 packets,然后 recvfrom ERESTARTSYS (SIGTERM)
```

mysql CLI 在收到 trailing OK (packet 6) 后被强制中断,因为它在等第 4 个 column value(永远不来的)。

## 根因分析

**wire-protocol 修复正确**(packet 结构精确符合 MySQL 8.0+ spec)。

**真正未修复的 bug 是 SQL 解析器**:
- `lexer.rs` 遇到 `@` 字符没有专门处理,落到 fallback 分支:
  ```rust
  _ => {
      self.position += 1;
      Token::Identifier(ch.to_string())
  }
  ```
- `@@version_comment` 被 tokenize 成 3 个 Identifier: `@`, `@`, `version_comment`
- parser 把它当成 3 个 column reference,生成 3 个 column definitions
- executor 取最后 1 个 identifier (`version_comment`) 作为数据值
- 结果: 3 列 × 1 行 × 1 value → schema mismatch → mysql CLI hang

## 判决

| 维度 | 判决 |
|------|------|
| **wire-protocol 修复** | ✅ **PASS** — 单元/集成测试通过,raw wire 字节符合 MySQL 8.0+ spec |
| **mysql CLI 8.0.46 兼容性** | ❌ **FAIL** — 仍 hang,因为 SQL 引擎不识别 `@@` 系统变量语法 |
| **#4019.1 整体** | ⚠️ **PARTIAL** — wire 子问题修复了,SQL 解析器子问题待修复 |

## 后续行动

**#4019.2 (新)**: 修复 SQL parser 中 `@@` 系统变量解析,使 `SELECT @@version_comment LIMIT 1` 返回 1 列 1 行。详见 task #59。

**改进建议**: 
- lexer 增加 `Token::AtAt` + `Token::SystemVariable(String)` 
- parser 识别 `@@xyz` 作为单表达式
- executor 返回标量值
- 添加 e2e 单元测试

## 关联

- 原始问题:#4019 "Sysbench OLTP baseline" → 母 ISSUE #3905
- 真实交付: wire-protocol 正确,SQL 解析器仍 bug
- 见 [[v3-12-0-issue-3905-closure]] [[strict-proof-mode]]
