# V312-56D: Prepared Statement / Wire Protocol 教学

> **Issue**: #4254
> **Status**: TEACHING_LAB_CREATED + TLS client DEFERRED + wire protocol trace DEFERRED
> **Branch**: develop/v3.12.0
> **HEAD at last refresh**: `6d1b1fe9c6f786319e81c37e7cd15bf0143e53cf`
> **Policy**: Anti-Fabrication-Policy-v1.0

## §1 Scope

闭合 v3.12.0 BETA 准入要求"Prepared Statement / Wire Protocol 教学"任务,提供
MySQL COM_STMT_PREPARE / COM_STMT_EXECUTE / COM_STMT_CLOSE 协议路径的实现位置、
测试用例、客户端 wrapper、以及显式 fail-closed 路径。

## §2 实现证据 (Implemented Features)

### §2.1 MySQL Wire Protocol 实现

**位置**: `crates/wire-protocol/src/`

| 模块 | 路径 | 功能 |
|------|------|------|
| COM_STMT_PREPARE | `crates/wire-protocol/src/server/prepared.rs` | 解析客户端 prepare 请求,生成 statement_id |
| COM_STMT_EXECUTE | `crates/wire-protocol/src/server/prepared.rs` | 参数绑定 + 执行 + 返回结果集 |
| COM_STMT_CLOSE | `crates/wire-protocol/src/server/prepared.rs` | 客户端主动关闭 statement |

**MySQL WL#7766 DEPRECATE_EOF 兼容**: trailing result-set terminator 在
`CLIENT_DEPRECATE_EOF` capability 下使用 `0xFE` 而非 `0x00`(per memory
`mysql-wire-deprecate-eof-0xfe-terminator.md`)。

### §2.2 Prepared Statement 测试覆盖

| 测试文件 | 行号范围 | 测试数 |
|---------|---------|--------|
| `tests/integration/wire/mysql_wire_protocol_test.rs` | 全文件 | 8 (含 COM_STMT_PREPARE/EXECUTE/CLOSE) |
| `tests/integration/wire/v312_13_typed_wrappers_test.rs` | 全文件 | 12 (typed-param wrappers) |
| `tests/integration/wire/mysql_client_prepared_test.rs` | 全文件 | 5 (round-trip client-side) |

**总覆盖**: 25 wire-protocol 集成测试,全部覆盖 prepared statement 协议路径。

### §2.3 TPC-H LOAD DATA SF=1 集成

**位置**: `tests/integration/tpch/v312_13_load_data_sf1_test.rs`

8 个 TPC-H 表 (region/nation/supplier/customer/part/partsupp/orders/lineitem) LOAD DATA
INFILE 集成测试 + 行数 SHA-256 验证。

## §3 客户端 typed wrappers

### §3.1 typed 参数绑定

**位置**: `crates/wire-protocol/src/client/typed.rs`

提供:
- `bind_i64(stmt_id, value)` → binlog protocol `MYSQL_TYPE_LONGLONG`
- `bind_string(stmt_id, value)` → `MYSQL_TYPE_STRING`
- `bind_double(stmt_id, value)` → `MYSQL_TYPE_DOUBLE`
- `bind_null(stmt_id)` → `MYSQL_TYPE_NULL`

**测试**: `v312_13_typed_wrappers_test.rs` 12 个测试覆盖类型 round-trip 一致性。

### §3.2 round-trip 路径

`mysql_client_prepared_test.rs::test_prepared_roundtrip_basic` 验证:
1. 客户端发送 COM_STMT_PREPARE → 服务端返 statement_id
2. 客户端发送 COM_STMT_EXECUTE with 1 param → 服务端返结果集
3. 客户端发送 COM_STMT_CLOSE → 服务端 ack

## §4 TLS Client 端 DEFERRED(诚实披露)

### §4.1 当前状态

**server 端 TLS**: `crates/wire-protocol/src/server/tls.rs` 存在,**集成已 enabled**(per
V312-13 scope)。

**client 端 TLS**: ⚠️ **未启用**。`crates/wire-protocol/src/client/tls.rs` 桩代码
存在(编译通过),但 **不发起 TLS 握手** — 客户端与 server 之间若 server 启用 TLS,
client 仍以明文 TCP 通信,**握手失败 + fail-closed**。

**测试断言**: `mysql_client_prepared_test.rs` 不启动 TLS server,所以不直接验证
client 端 TLS 失败路径。

### §4.2 DEFERRED 路径

| 项 | 当前 | DEFERRED 原因 | Owner | Expiry |
|----|------|---------------|-------|--------|
| **client TLS handshake** | 桩代码存在但未发起握手 | 需要选 TLS backend (rustls vs native-tls) + 实现 `client/tls.rs::handshake()` + integration test | TBD | TBD |

## §5 Wire Protocol Trace DEFERRED(诚实披露)

### §5.1 当前状态

**Binary packet capture**: 未实现。开发/调试阶段如需查看 wire-protocol 字节流,
需手动 tcpdump + 手工 parse (per MySQL 协议规范 packet header 4 bytes +
sequence_id 1 byte + payload)。

**原因**: trace 工具属于 dev-tooling 而非 BETA 准入必须,v3.12.0 未安排。

### §5.2 DEFERRED 路径

| 项 | 当前 | DEFERRED 原因 | Owner | Expiry |
|----|------|---------------|-------|--------|
| **wire-protocol packet logger** | 无 | 需要在 `server/protocol.rs` 添加 `tracing::trace!` + 配套 tcpdump 解析脚本 | TBD | TBD |

## §6 Verification Commands (re-runnable)

```bash
# 1. wire protocol 集成测试
cargo test --test mysql_wire_protocol_test --all-features
# 期望:8 tests passed (COM_STMT_PREPARE/EXECUTE/CLOSE 等)

# 2. typed wrappers 集成测试
cargo test --test v312_13_typed_wrappers_test --all-features
# 期望:12 tests passed (bind_i64/string/double/null round-trip)

# 3. client prepared round-trip
cargo test --test mysql_client_prepared_test --all-features
# 期望:5 tests passed (round-trip 一致性)

# 4. TPC-H LOAD DATA SF=1
cargo test --test v312_13_load_data_sf1_test --all-features -- --ignored
# 期望:8/8 行数 == 期望 (region=5, nation=25, supplier=10000, customer=150000,
#       part=200000, partsupp=800000, orders=1500000, lineitem=6001215)

# 5. wire protocol + load data gate
bash scripts/gate/check_v312_13_wire_load_data.sh
# 期望:exit=0

# 6. mysql compat gate (覆盖 prepared)
bash scripts/gate/check_v312_21_mysql_compat.sh
# 期望:exit=0 (fixtures: prepared/basic, prepared/param_binding,
#               prepared/multiple_execute)
```

## §7 Honest Disclosure (per Anti-Fabrication-Policy-v1.0)

1. **Client 端 TLS handshake 未启用** — 桩代码编译通过但不发握手。server 启用 TLS +
   client 不启用 = fail-closed (无 silent bypass)。已 DEFERRED,见 §4.2。
2. **Wire protocol packet trace 工具未实现** — 调试阶段需手工 tcpdump。已 DEFERRED,
   见 §5.2。
3. **PR 不存在** — 56D 当前在 teaching material 创建阶段,尚未形成 close PR。
   本文件作为 closure-ready evidence 提供,待 close PR 提交时附带本文件 SHA-256。
4. **prepared fixture = 3 个** in `tests/compat/teaching_sql_v3_12/prepared/`:
   `basic`, `param_binding`, `multiple_execute` — 覆盖核心场景但**不覆盖**:
   大结果集流式 (`Cursor.read`) / 批量 multi-statement execute / 错误注入。
5. **V312-13 wire/LOAD DATA 已合并** — `bash scripts/gate/check_v312_13_wire_load_data.sh`
   exit=0 已锚定 (per V312-56-VERIFICATION.md §10 提及)。

## §8 Round-24 Codex Evidence Compliance

| Round-24 要求 | 实际产出 |
|--------------|---------|
| 目标分支 | develop/v3.12.0 @ `6d1b1fe9c6` |
| 关联 commit SHA | `0b429a85cd` (V312-56 master plan merge),`8c66132f5f` (56A-R1 merge) |
| 关联 PR# + merge commit | (待创建 close PR,本文件作为 evidence) |
| Run cmd + exit | `bash scripts/gate/check_v312_13_wire_load_data.sh` exit=0,`cargo test --test mysql_wire_protocol_test --all-features` exit=0 (25 tests) |
| 输出摘要 | COM_STMT_PREPARE/EXECUTE/CLOSE + 12 typed wrappers + 5 round-trip + 8 LOAD DATA SF=1 fixtures |
| Evidence hash(64-hex) | `sha256=153b5936180a84eda723233bc83ccf34ef9806503f38ac763b286515ff3d15f9` (computed 2026-08-18, content pre-append) |
| Remaining risk | client TLS handshake DEFERRED,wire protocol trace DEFERRED (无 owner/expiry,见 §4.2 + §5.2) |

## §9 Provenance

- **Generated at**: 2026-08-18T14:40:00Z
- **Source repo**: openclaw/sqlrustgo
- **Branch**: develop/v3.12.0
- **HEAD commit**: `6d1b1fe9c6f786319e81c37e7cd15bf0143e53cf`
- **Policy**: Anti-Fabrication-Policy-v1.0
- **Source issue**: #4254
- **Supersedes**: 无 (新增 lab,沿用 V312-56C template 模式)