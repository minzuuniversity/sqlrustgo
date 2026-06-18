<!-- env:blocked:no-ci -->

# openspec/3516 - MySQL CLI Hangs on Result Set with Trailing Terminator

> **Issue**: [#3516](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3516)
> **作者**: Hermes Agent
> **日期**: 2026-06-18
> **Phase**: 1 (W1-2)
> **工作量**: 8h (~1 day)
> **优先级**: P0 (v3.9.0 blocker)
> **Milestone**: v3.9.0 (id=32, due 2026-09-23)
> **Label**: P0, ai-task, integration, mysql-server

## 一、问题分析

### 1.1 背景

mysql 8.0.46 CLI 在 result set 响应时挂起 60 秒，导致 ER_MALFORMED 或 timeout。

**影响范围**:
- SHOW TABLES
- SELECT 1
- LOAD DATA + COUNT

### 1.2 根本原因

DEPRECATE_EOF capability 处理不一致:
- MySQL 8.0.46 客户端默认设置 `DEPRECATE_EOF = 0x01000000`
- sqlrustgo 在 `DEPRECATE_EOF=1` 时仍发送 inter-record EOF，导致客户端混淆
- PR #3514 已做 partial fix，但仍有遗留问题

### 1.3 现有状态

| 组件 | 状态 |
|------|------|
| `send_result_set` (lib.rs) | 🟡 部分修复，仍有问题 |
| DEPRECATE_EOF capability check | 🟡 存在但不完整 |
| inter-record EOF 处理 | ❌ 有 bug |
| PR #3514 | ✅ merged，partial fix |

## 二、变更设计

### 2.1 问题定位

**需要修复的函数** (需通过 gitnexus_impact 分析):

1. `send_result_set` - 结果集发送逻辑
2. `make_deprecate_eof_ok_packet` - DEPRECATE_EOF OK packet 构建
3. `make_eof_packet` - classic EOF packet 构建
4. Result set terminator 处理

### 2.2 修复策略

**Classic 路径 (DEPRECATE_EOF=0)**:
```
Packet sequence: column count → column defs → EOF(5B) → rows → EOF(5B)
```

**DEPRECATE_EOF 路径 (DEPRECATE_EOF=1)**:
```
Packet sequence: column count → column defs → [无 inter-record EOF] → rows → OK(7B)
```

**关键修复**:
1. 当 `cap & DEPRECATE_EOF != 0` 时，**不发送** inter-record EOF
2. 使用 `make_ok_packet` 而非 `make_eof_packet` 作为 trailing terminator
3. 确保 packet sequence id 正确递增

### 2.3 验证方法

**tcpdump 抓包对比**:
```bash
# MySQL 8.0.46 server (正确行为)
tcpdump -i lo -w mysql_correct.pcap port 3306
mysql -h 127.0.0.1 -e "SELECT 1"

# sqlrustgo (当前行为)
tcpdump -i lo -w sqlrustgo_bug.pcap port 3306
# 使用 mysql CLI 连接 sqlrustgo
```

**gdb 调试 mysql client**:
```bash
gdb --args mysql -h 127.0.0.1 -e "SELECT 1"
# 在 libmysqlclient 接收数据包处设置断点
```

## 三、风险评估

| 风险 | 影响 | 缓解 |
|------|------|------|
| 修改 wire protocol 可能破坏现有功能 | 高 | 现有 test suite 覆盖 |
| DEPRECATE_EOF=0 路径被意外修改 | 高 | 分离代码路径 |
| Packet sequence id 管理 | 中 | 严格递增检查 |

## 四、测试策略

### 4.1 单元测试

| 测试 | 验证 |
|------|------|
| `test_deprecate_eof_inter_record_no_eof` | DEPRECATE_EOF=1 时不发送 inter-record EOF |
| `test_classic_inter_record_has_eof` | DEPRECATE_EOF=0 时发送 inter-record EOF |
| `test_packet_sequence_increments` | sequence id 正确递增 |

### 4.2 集成测试

| 测试 | 验证 |
|------|------|
| `wire_deprecate_eof_test::classic_caps_round_trip` | DEPRECATE_EOF=0 路径 |
| `wire_deprecate_eof_test::deprecated_eof_caps_round_trip` | DEPRECATE_EOF=1 路径 |
| `mysql_cli_smoke` | mysql 8.0.46 CLI 不挂起 |

### 4.3 外部 CLI 测试

```bash
# scripts/wire_smoke_mysql_cli.sh
mysql -h 127.0.0.1 -P 3306 -u tester -ptester -e "SELECT 1"
# 期望: 立即返回，无 hang，无 ER_MALFORMED
```

## 五、门禁

**位置**: `scripts/gate/check_wire_deprecate_eof.sh` (新建)

**检查项**:
1. `cargo test --test wire_deprecate_eof_test` 全部 PASS
2. mysql CLI smoke script exit 0
3. 现有 TPC-H 22/22 不退化

## 六、实施步骤

| # | 步骤 | 文件 | 工作量 |
|---|------|------|--------|
| 1 | 分析 wire protocol 代码 (gitnexus_impact) | crates/mysql-server/src/lib.rs | 2h |
| 2 | 修复 inter-record EOF 逻辑 | 同上 | 2h |
| 3 | 添加单元测试 | tests/wire_deprecate_eof_test.rs | 1h |
| 4 | 验证 mysql CLI 不挂起 | scripts/wire_smoke_mysql_cli.sh | 1h |
| 5 | 运行 TPC-H 22/22 回归测试 | — | 2h |
| **合计** | | | **8h** |

## 七、交付物清单

| 类别 | 文件 | 大小预估 |
|------|------|----------|
| Fix | `crates/mysql-server/src/lib.rs` (wire protocol) | +50 行 |
| Test | `tests/wire_deprecate_eof_test.rs` (扩展) | +30 行 |
| Gate | `scripts/gate/check_wire_deprecate_eof.sh` | 50 行 |
| **合计** | | **~130 行** |

## 八、Issue 关闭条件

满足 3 项:
1. ✅ mysql 8.0.46 CLI 连接 sqlrustgo 不挂起
2. ✅ `cargo test --test wire_deprecate_eof_test` 全部 PASS
3. ✅ TPC-H 22/22 不退化

## 九、参考

- Issue #3516: <http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3516>
- Issue #3517: STMT_PREPARE Malformed packet (关联)
- PR #3514: fix/wire-deprecate-eof-partial
- 现有 openspec: `openspec/changes/2026-06-18-wire-deprecate-eof/`
