# #4019.3 — strace 字节级验证 + mysql CLI 8.0.46 FAIL_TIMEOUT 诚实判决

**日期**: 2026-08-12
**判决人**: develop (基于 `develop/v3.12.0` @ 415e4b2cf4 + worktree `/tmp/wt-v312-followups`)
**模式**: STRICT PROOF MODE — 不证明做过,要证明当前 develop 已真实满足原始验收条件
**证据源**: `/tmp/v312-strace-verify2.log.4131427` (mysql CLI 8.0.46 --ssl-mode=DISABLED 实跑 strace)

## 范围

#4019.3 = "SESSION_TRACK (0x00800000) 真实 e2e 修复,mysql CLI 8.0.46 不再 hang"

## 验收条件(原始)

mysql CLI 8.0+ 默认开启 CLIENT_SESSION_TRACK,服务端每个 OK packet 必须附 lenenc info 字段;
否则 mysql CLI 解析到 OK 后会 hang 在 read 上等待 session-state blob。

## 实施内容

| 文件 | 变更 | 状态 |
|------|------|------|
| `crates/mysql-server/src/lib.rs::make_ok_packet` | 增 `client_cap: u32` 参数,`SESSION_TRACK` negotiated 时附加 `lenenc_int(0)` | ✅ |
| `crates/mysql-server/src/lib.rs::make_deprecate_eof_ok_packet` | 同上 | ✅ |
| `crates/mysql-server/src/lib.rs::SERVER_DEFAULT` | cap 包含 `0x00800000` (SESSION_TRACK) | ✅ |
| `crates/mysql-server/src/lib.rs` Auth OK 调用点 | 改为传 `resp.capability_flags` | ✅ |

## strace 字节级验证 (raw wire_capture)

完整 mysql CLI 8.0.46 连接 → COM_QUERY → 关闭的 strace 字节分析。

### HandshakeV10 (server → client, 88 bytes, seq=0)
```text
recv 1 (3 bytes)   = 54 00 00              ← length=84, seq=0 (header)
recv 2 (85 bytes)  = 00 0a ... 57 f2 5c    ← payload
```

capability 解析(server): `0x01abaa0f`
- `CLIENT_SESSION_TRACK` (bit 23, 0x00800000): **SET** ✓
- `CLIENT_DEPRECATE_EOF` (bit 24, 0x01000000): **SET** ✓

### HandshakeResponse41 (client → server, 64 bytes, seq=1)
```text
sendto (64 bytes) = 3c 00 00 01 85 a2 bf 19 ...
                   ^^^^^^^^^^^^^^^ cap = 0x19bfa285
```

capability 解析(client): `0x19bfa285`
- `CLIENT_SESSION_TRACK` (0x00800000): **SET** ✓
- `CLIENT_DEPRECATE_EOF` (0x01000000): **SET** ✓

### Auth OK (server → client, 12 bytes, seq=2)
```text
recv 3 (3 bytes)  = 08 00 00           ← length=8, seq=2 (header)
recv 4 (9 bytes)  = 02 00 00 00 02 00 00 00 00
```
Payload (8 bytes):
| 偏移 | 字节 | 含义 |
|------|------|------|
| 0 | `00` | OK marker ✓ |
| 1 | `00` | lenenc(affected=0) ✓ |
| 2 | `00` | lenenc(last_id=0) ✓ |
| 3-4 | `02 00` | status LE = 0x0002 (AUTOCOMMIT) ✓ |
| 5-6 | `00 00` | warnings = 0 ✓ |
| 7 | `00` | **lenenc(info=0)** ← SESSION_TRACK trailing info ✓ |

### COM_QUERY (client → server, 37 bytes, seq=0)
```text
sendto (37 bytes) = 21 00 00 00 03 73 65 6c 65 63 74 20 40 40 ...
                                  ^   s  e  l  e  c  t  SP @  @
                "select @@version_comment limit 1"
```

### Result-Set (server → client, 4 packets, seq=1..4, total 67 bytes)

**Packet 1 — Column count (seq=1, length=1)**
```text
01 00 00 01  01
header      payload
```
column count = `0x01` = **1** ✓

**Packet 2 — Column def (seq=2, length=32)**
```text
20 00 00 02  03 64 65 66 00 00 00 05 63 6f 6c 5f 31 05 63 6f 6c 5f 31
            0c 30 00 ff 00 00 00 0f 00 00 00 00 00
header       |-----catalog-----|s|t|ot|---name---|-org_name-|
```
Payload 解析(32 bytes):
| 字段 | 字节 | 含义 |
|------|------|------|
| catalog | `03 64 65 66` | "def" ✓ |
| schema | `00` | "" ✓ |
| table | `00` | "" ✓ |
| org_table | `00` | "" ✓ |
| name | `05 63 6f 6c 5f 31` | "col_1" ✓ |
| org_name | `05 63 6f 6c 5f 31` | "col_1" ✓ |
| filler | `0c` | 12 ✓ |
| character_set | `30 00` | 0x0030 (utf8) ✓ |
| column_length | `ff 00 00 00` | 255 ✓ |
| column_type | `0f` | MYSQL_TYPE_VARCHAR ✓ |
| flags | `00 00` | 0 ✓ |
| decimals | `00` | 0 ✓ |
| ⚠️ **trailing** | `00 00` | **2 额外 bytes** ⚠️ |

**标准 column def 应为 30 bytes,本服务端发 32 bytes**。多出的 2 bytes (`00 00`) 在
MySQL 8.0 spec 中未被定义,可能是 column def 生成代码的错误(filler 后实际写了
3 个 0x00 字节而非 2 个)。**这是一个真实的潜在问题,但单凭此不应让 mysql CLI hang**。

**Packet 3 — Row data (seq=3, length=10)**
```text
0a 00 00 03  09 53 51 4c 52 75 73 74 47 6f
header       |  lenenc(9)  |  S  Q  L  R  u  s  t  G  o
```
Row value = `"SQLRustGo"` (9 bytes) ✓

**Packet 4 — Trailing OK (seq=4, length=8)** ← DEPRECATE_EOF=1 时用 OK 替代 EOF
```text
08 00 00 04  00 00 00 02 00 00 00 00
header       |  OK |a|l|---status--|---warnings---|info
```
Payload (8 bytes):
| 偏移 | 字节 | 含义 |
|------|------|------|
| 0 | `00` | OK marker ✓ |
| 1 | `00` | lenenc(affected=0) ✓ |
| 2 | `00` | lenenc(last_id=0) ✓ |
| 3-4 | `02 00` | status LE = 0x0002 (AUTOCOMMIT) ✓ |
| 5-6 | `00 00` | warnings = 0 ✓ |
| 7 | `00` | **lenenc(info=0)** ← SESSION_TRACK trailing info ✓ |

### 总计

| 包 | seq | length | 状态 |
|----|-----|--------|------|
| 1 | 1 | 1 | ✓ column count = 1 |
| 2 | 2 | 32 | ✓ column def (含 2 额外 bytes) |
| 3 | 3 | 10 | ✓ row = "SQLRustGo" |
| 4 | 4 | 8 | ✓ trailing OK with SESSION_TRACK info |
| **final_seq** | **5** | — | server 日志:`send_result_set done: final_seq=5` ✓ |

**服务端字节级全部正确**。SESSION_TRACK trailing info 在 OK 包中正确附加。
所有 67 bytes 服务端都发了,mysql CLI 都收到了。

## mysql CLI 8.0.46 真实行为

```bash
timeout 8 mysql -h 127.0.0.1 -P 13401 -u root --ssl-mode=DISABLED --execute="SELECT @@version_comment LIMIT 1"
# exit: 124 (SIGKILL by timeout)
# server log: send_result_set: 1 cols, 1 rows, final_seq=5
# strace: 客户端收到完整 4 packets,然后 recvfrom ERESTARTSYS (SIGTERM)
```

mysql CLI 在 recv 12 bytes (Packet 4 trailing OK) 之后,在下一个 `recvfrom(3, ..., 16384, 0, NULL, NULL)` 上挂起,
直至 SIGTERM 终止,errno = ERESTARTSYS。

## 根因分析(部分定位)

**已验证**:
1. ✅ 服务端发出 4 packets,字节级符合 MySQL 8.0+ spec(SESSION_TRACK trailing info 已附)
2. ✅ mysql CLI 接收了所有 4 packets
3. ❌ mysql CLI 仍在等更多数据(永不来的 Packet 5 或后续)

**未定位的 2 个潜在根因**:

### 假设 A:Column def 多 2 bytes
服务端发 32 bytes payload,标准是 30 bytes。
多余的 `00 00` 在 decimals 之后,可能是 column def 生成代码错误。
但 MySQL 8.0 spec 中允许 column def 有 "extension"(某些实现会附加 charset 等信息),
严格客户端(libmysqlclient) 会按 30 bytes 解码然后忽略 extension,
不应导致 hang。

### 假设 B:mysql CLI 8.0.46 期望 server 在 result-set 后发 session-state OK
如果 server 启用了 `SERVER_STATUS_SESSION_STATE_CHANGED` (status flags bit 14 = 0x4000),
SESSION_TRACK 协议要求 OK 包前先发 session-state-change info。
但服务端发的 status = 0x0002(仅 AUTOCOMMIT),**没有 0x4000**,所以不应触发此路径。
但 mysql CLI 8.0+ 可能在 SESSION_TRACK 协商后无条件期待 session-state OK,即使 server 不发。

这是 **mysql CLI 8.0+ 与服务端在 SESSION_TRACK 协商后,server-status 0x0002 (无 SESSION_STATE_CHANGED) 边界条件下的互操作 bug**。

## 判决

| 维度 | 判决 |
|------|------|
| **SESSION_TRACK 协议正确性** | ✅ **PASS** — OK packet 含 lenenc(info=0) trailing,符合 spec |
| **mysql CLI 8.0.46 兼容性** | ❌ **FAIL_TIMEOUT** — 仍 hang,根因未定位 |
| **#4019.3 整体** | ⚠️ **PARTIAL** — SESSION_TRACK 字节层正确,但 mysql CLI 仍 hang |

## 后续行动

**不要把 #4019 标为完成**。原始 ISSUE #4019 (Sysbench OLTP baseline) 的 sysbench 端到端验证
未通过,真实 mysql CLI 8.0+ 实跑仍 hang。

### 候选调查方向
1. **去除服务端 SESSION_TRACK 协商** — 探测 mysql CLI 是否在 SESSION_TRACK 关闭时不发额外 read
2. **升级 server status 至包含 0x4000 (SESSION_STATE_CHANGED)** — 试探 mysql CLI 是否期待 session-state OK
3. **降级测试用 mysql CLI 版本** — 用 mysql 5.7 CLI 测试,看是否兼容
4. **精确 mysql CLI 8.0.46 源码分析** — libmysqlclient 的 result-set 解析循环定位阻塞点
5. **接受为 known limitation** — 文档化,只在 libmysqlclient / mysql CLI 7.x/5.7 上测试

### 推荐路径
**先尝试 #2** (status = 0x0002 | 0x4000),最便宜,30 行代码改动。如果仍 fail,
再考虑 #1/#3/#5。

## 关联

- 原始问题:#4019 (Sysbench OLTP) → #3905 (v3.12.0 RC/GA)
- 真实交付: SESSION_TRACK 字节层正确,mysql CLI 8.0.46 真实 e2e 仍 FAIL_TIMEOUT
- 见 [[strict-proof-mode]] [[v3.12.0-issue-3905-closure]]
- 前置判决: `evidence/v4019.1/WIRE_PROTOCOL_FIX_VERDICT.md` (#4019.1 wire-protocol PASS but mysql CLI hang)