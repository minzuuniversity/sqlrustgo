# PR-850: mysql-server 与 LocalExecutor 统一 - 测试设计报告

> **版本**: v1.0
> **日期**: 2026-06-01
> **PR 系列**: PR-850A/B, PR-2696
> **状态**: 部分完成

---

## 1. 测试目标

### 1.1 主要目标

| 目标 | 描述 | 优先级 |
|------|------|--------|
| T1 | 验证 Shared engine 在连续执行中状态持久化 | P0 |
| T2 | 验证 INSERT 后 SELECT 能看到数据 | P0 |
| T3 | 验证 UPDATE 后 SELECT 能看到更新 | P1 |

### 1.2 次要目标

| 目标 | 描述 | 优先级 |
|------|------|--------|
| T4 | 验证 mysql-server Packet I/O 正确性 | P1 |
| T5 | 验证 MySqlError 错误处理 | P2 |

---

## 2. 测试用例

### 2.1 T1: Shared Engine State Persistence

**目的**: 验证复用共享 engine 时，状态正确持久化

**前置条件**:
- 无

**测试步骤**:
1. 创建 ExecutionEngine 实例
2. 创建表 t (id INTEGER, value TEXT)
3. 执行 INSERT INTO t VALUES (1, 'test')
4. 执行 SELECT * FROM t
5. 验证返回包含 (1, 'test')
6. 执行 UPDATE t SET value = 'updated' WHERE id = 1
7. 执行 SELECT * FROM t
8. 验证返回包含 (1, 'updated')

**验收标准**:
```rust
assert_eq!(result.rows[0][1], sqlrustgo_types::Value::Text("updated".to_string()))
```

### 2.2 T2: INSERT-SELECT Continuity

**目的**: 验证 INSERT 后立即 SELECT 能看到数据

**测试步骤**:
1. 创建表
2. INSERT 一行
3. SELECT 验证存在
4. 验证值正确

**验收标准**:
```rust
assert_eq!(result.rows.len(), 1);
assert_eq!(result.rows[0][0], Value::Integer(1));
```

### 2.3 T3: UPDATE-SELECT Continuity

**目的**: 验证 UPDATE 后立即 SELECT 能看到更新

**测试步骤**:
1. 创建表，插入一行
2. UPDATE 该行
3. SELECT 验证值已更新

**验收标准**:
```rust
assert_eq!(result.rows[0][1], Value::Text("updated".to_string()));
```

---

## 3. 覆盖率目标

### 3.1 代码覆盖率

| 模块 | 当前覆盖率 | 目标 |
|------|-----------|------|
| mysql-server | ~40% | ≥60% |

### 3.2 功能覆盖率

| 功能 | 覆盖状态 |
|------|----------|
| ExecutionEngine 状态持久化 | ✅ 已覆盖 |
| Packet I/O | ✅ 已覆盖 |
| Error handling | ✅ 已覆盖 |
| MySQL 协议交互 | ❌ 未覆盖 |

---

## 4. 边界测试

### 4.1 边界条件

| 场景 | 输入 | 预期行为 |
|------|------|----------|
| 空表 SELECT | SELECT * FROM empty | 返回空 rows |
| 不存在的行 UPDATE | UPDATE t SET v='x' WHERE id=999 | affected_rows = 0 |
| 并发访问 | 2个连接 | 各自独立 engine |

---

## 5. 测试环境

### 5.1 依赖

```toml
[dev-dependencies]
sqlrustgo = { path = "../crates/sqlrustgo" }
sqlrustgo-storage = { path = "../crates/storage" }
sqlrustgo-types = { path = "../crates/types" }
tempfile = "3.8"
```

### 5.2 测试位置

```
crates/mysql-server/tests/mysql_server_tests.rs
```

---

## 6. 测试执行

### 6.1 运行命令

```bash
# 运行所有 mysql-server 测试
cargo test -p sqlrustgo-mysql-server

# 运行指定测试
cargo test -p sqlrustgo-mysql-server test_execution_engine_state_persistence
```

### 6.2 预期结果

```
test test_execution_engine_state_persistence ... ok
test test_mysql_error_io ... ok
test test_mysql_error_protocol ... ok
...
14 passed; 0 failed
```

---

## 7. 测试限制

### 7.1 当前限制

1. **无 TCP 连接测试**: 当前测试不涉及真实的 MySQL 协议交互
2. **无并发测试**: 未测试多连接场景
3. **无真实 mysql-server 进程测试**: 只测试了 ExecutionEngine 层面

### 7.2 建议补充

1. 添加 TCP 集成测试，使用 mock MySQL client
2. 添加 PreparedStatement 测试
3. 添加并发连接测试

---

*本文档为 PR-850 测试设计报告 v1.0*
