<!-- env:blocked:no-ci -->

# openspec/3517 - STMT_PREPARE Malformed Packet (Bug #3)

> **Issue**: [#3517](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3517)
> **作者**: Hermes Agent
> **日期**: 2026-06-18
> **Phase**: 1 (W1-2)
> **工作量**: 6h (~1 day)
> **优先级**: P0 (v3.9.0 blocker)
> **Milestone**: v3.9.0 (id=32, due 2026-09-23)
> **Label**: P0, ai-task, integration, mysql-server

## 一、问题分析

### 1.1 背景

mysql CLI 8.0.46 在执行 STMT_PREPARE 时返回 "Malformed packet" 错误。

**影响**:
- 预处理语句工作流完全失效
- mysql CLI 无法使用 prepared statements

### 1.2 症状

```
mysql> PREPARE stmt1 FROM 'SELECT 1';
ERROR 2013 (HY000): Lost connection to MySQL server during query
# 或
ERROR 1047 (08S01): Malformed packet
```

### 1.3 相关 Issue

- Issue #3423: 基础 wire protocol 问题
- Issue #3516: DEPRECATE_EOF 问题 (关联)
- wire protocol STMT_PREPARE/STMT_EXECUTE 实现

## 二、问题定位

### 2.1 需要分析的函数 (gitnexus_impact)

```
COM_STMT_PREPARE handler
├── parse_stmt_execute_params
├── ps_manager.add
├── write_column_def
├── make_deprecate_eof_ok_packet / make_eof_packet
└── Packet::write_to
```

### 2.2 已知问题点

**Packet 结构问题**:
```rust
// 当前可能的错误结构
p.push(0x00);                                    // OK packet type
p.write_u32::<LittleEndian>(stmt_id).unwrap();    // 但应该是 lenenc_int
p.write_u16::<LittleEndian>(column_count).unwrap();
p.write_u16::<LittleEndian>(param_count).unwrap();
p.push(0x00);                                    // 这行可能多余
```

**正确结构** (MySQL wire protocol):
```
COM_STMT_PREPARE Response:
- 1 byte: 0x00 (OK packet type)
- 4 bytes: statement_id
- 2 bytes: column_count
- 2 bytes: param_count
- 1 byte: 0x00 (reserved)
- 2 bytes: warning_count
```

### 2.3 STMT_PREPARE Response 格式

**问题**: packet sequence 可能不正确

```rust
Packet {
    length: p.len() as u32,
    sequence: seq,        // ← 可能没有正确递增
    payload: p,
}
.write_to(stream)?;
seq = seq.wrapping_add(1);

// 后面还有 param defs 和 column defs，sequence 需要连续
```

## 三、修复策略

### 3.1 修复 COM_STMT_PREPARE Response

**文件**: `crates/mysql-server/src/lib.rs`

**当前代码** (~line 2458):
```rust
Packet {
    length: p.len() as u32,
    sequence: seq,
    payload: p,
}
.write_to(stream)?;
seq = seq.wrapping_add(1);
```

**问题**: param_count > 0 时发送 param defs，但 sequence 可能在某些路径没正确递增。

### 3.2 修复 Param Def Packet

```rust
for i in 0..param_count as usize {
    let ptype = param_types.get(i).copied().unwrap_or(col_type::VARSTRING);
    let mut param_def = Vec::new();
    write_lenenc_string(&mut param_def, b"def").unwrap();
    // ... 写入完整的 param def
    Packet {
        length: param_def.len() as u32,
        sequence: seq,  // ← 必须正确递增
        payload: param_def,
    }
    .write_to(stream)?;
    seq = seq.wrapping_add(1);
}
```

### 3.3 修复 Column Def Packet

同样问题可能在 column defs 发送时出现。

## 四、测试策略

### 4.1 单元测试

| 测试 | 验证 |
|------|------|
| `test_stmt_prepare_response_format` | STMT_PREPARE response packet 格式正确 |
| `test_stmt_prepare_sequence_increments` | sequence id 连续 |
| `test_stmt_prepare_with_params` | 带参数的 STMT_PREPARE |

### 4.2 集成测试

| 测试 | 验证 |
|------|------|
| `test_mysql_cli_stmt_prepare` | mysql CLI 执行 PREPARE 不报错 |
| `test_stmt_execute_after_prepare` | PREPARE 后 EXECUTE 正常 |

### 4.3 抓包对比

```bash
# 抓取 MySQL 8.0.46 server 的 STMT_PREPARE 响应
tcpdump -i lo -w mysql_stmt_prepare.pcap port 3306
mysql -h 127.0.0.1 -e "PREPARE stmt1 FROM 'SELECT 1'"

# 对比 sqlrustgo 的响应
```

## 五、风险评估

| 风险 | 影响 | 缓解 |
|------|------|------|
| 修改 STMT_PREPARE 可能破坏 STMT_EXECUTE | 高 | 现有 prepared statement tests |
| sequence id 管理复杂 | 中 | 严格追踪每个 packet |
| 与 DEPRECATE_EOF 路径交互 | 中 | 分别测试两个路径 |

## 六、实施步骤

| # | 步骤 | 文件 | 工作量 |
|---|------|------|--------|
| 1 | 分析 STMT_PREPARE handler (gitnexus_impact) | crates/mysql-server/src/lib.rs | 2h |
| 2 | 修复 packet sequence 管理 | 同上 | 2h |
| 3 | 添加单元测试 | tests/stmt_prepare_test.rs (新建) | 1h |
| 4 | 验证 mysql CLI | — | 1h |
| **合计** | | | **6h** |

## 七、交付物清单

| 类别 | 文件 | 大小预估 |
|------|------|----------|
| Fix | `crates/mysql-server/src/lib.rs` (STMT_PREPARE) | +40 行 |
| Test | `tests/stmt_prepare_test.rs` (新建) | 150 行 |
| **合计** | | **~190 行** |

## 八、Issue 关闭条件

满足 3 项:
1. ✅ mysql CLI 执行 `PREPARE stmt1 FROM 'SELECT 1'` 不报错
2. ✅ `cargo test --test stmt_prepare_test` 全部 PASS
3. ✅ `cargo test --test wire_deprecate_eof_test` 全部 PASS (关联)

## 九、参考

- Issue #3517: <http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3517>
- Issue #3516: DEPRECATE_EOF 问题 (关联)
- Issue #3423: wire protocol 基础问题
