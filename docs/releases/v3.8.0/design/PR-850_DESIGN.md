# PR-850: mysql-server 与 LocalExecutor 统一

> **版本**: v1.0
> **日期**: 2026-06-01
> **PR 系列**: PR-850A/B, PR-2696
> **状态**: 部分完成

---

## 1. 功能概述

### 1.1 目标

PR-850 旨在统一 mysql-server 和 LocalExecutor 的执行路径，确保：
1. 两者使用相同的 ExecutionEngine 实例
2. WAL 状态和事务上下文在会话中正确维护
3. Statement 执行复用共享的 engine

### 1.2 涉及的功能

| 功能 | PR | 描述 | 状态 |
|------|-----|------|------|
| A | PR-850A/B | Stateless WAL + Tx Context Route A | ✅ 已合并 |
| B | PR-2696 | Shared engine in STMT EXECUTE path | ✅ 已合并 |

---

## 2. 接口定义

### 2.1 核心接口

```rust
// crates/mysql-server/src/lib.rs - do_command_loop 函数

fn do_command_loop<S: Read + Write>(
    stream: &mut S,
    addr: SocketAddr,
    storage: Arc<RwLock<FileStorage>>,
    engine: Arc<RwLock<ExecutionEngine<FileStorage>>>,  // ← 共享 engine
    cap: u32,
    mut seq: u8,
    ps_manager: &mut PreparedStatementManager,
) -> MySqlResult<()>
```

### 2.2 关键变更 (PR-2696)

**变更前**:
```rust
let mut eng = ExecutionEngine::new(storage.clone());  // ← 每次创建新 engine
```

**变更后**:
```rust
let mut eng = engine.write().unwrap();  // ← 复用共享 engine
```

---

## 3. 数据结构

### 3.1 ExecutionEngine 包装

```rust
// 类型: Arc<RwLock<ExecutionEngine<FileStorage>>>
// 位置: crates/mysql-server/src/lib.rs

Arc<RwLock<ExecutionEngine<FileStorage>>>
```

### 3.2 PreparedStatementManager

```rust
struct PreparedStatementManager {
    statements: HashMap<u32, (String, u32)>,  // stmt_id -> (sql, column_count)
}
```

---

## 4. 边界条件与错误处理

### 4.1 错误处理

| 场景 | 错误类型 | 处理方式 |
|------|----------|----------|
| Engine 获取锁失败 | RwLock 错误 | 返回 MySqlError::Sql |
| SQL 解析失败 | Parse 错误 | 返回错误消息 |
| 执行失败 | SqlError | 返回错误消息 |

### 4.2 边界条件

| 条件 | 处理 |
|------|------|
| 无事务上下文 | DML 返回错误 |
| 空 WHERE 子句 | 全表操作 |
| 并发连接 | 每个连接独立 engine |

---

## 5. 依赖关系

### 5.1 外部依赖

| 依赖 | 版本 | 用途 |
|------|------|------|
| tokio | 1.x | 异步运行时 |
| tracing | 0.1 | 日志追踪 |

### 5.2 内部依赖

| 模块 | 依赖关系 |
|------|----------|
| sqlrustgo-executor | ExecutionEngine |
| sqlrustgo-storage | FileStorage |
| sqlrustgo-parser | SQL 解析 |

---

## 6. 测试设计

### 6.1 单元测试

| 测试 | 描述 | 状态 |
|------|-------|------|
| `test_execution_engine_state_persistence` | 验证 engine 状态持久化 | ✅ 已添加 |

### 6.2 集成测试

| 测试 | 描述 | 状态 |
|------|-------|------|
| mysql_server_tests | Packet I/O, MySqlError | ✅ 14 tests |

---

## 7. 已知问题

### 7.1 未完成项

| 问题 | 优先级 | 说明 |
|------|--------|------|
| 无独立的 E2E 测试 | P2 | 需要完整的 mysql-server 协议测试 |

### 7.2 限制

1. 当前只测试了 ExecutionEngine 状态持久化，未测试实际的 MySQL 协议交互
2. 需要完整的 TCP 连接测试才能验证 mysql-server 实际行为

---

## 8. 验收标准

### 8.1 功能验收

| 验收点 | 标准 |
|--------|------|
| Shared engine | STMT EXECUTE 复用传入的 engine |
| State persistence | 连续执行看到前一条的结果 |
| Error propagation | 错误正确返回 |

### 8.2 测试验收

| 验收点 | 标准 |
|--------|------|
| Unit test | `test_execution_engine_state_persistence` 通过 |
| Integration test | 14 mysql_server_tests 通过 |

---

*本文档为 PR-850 功能设计报告 v1.0*
