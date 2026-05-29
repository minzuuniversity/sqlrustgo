# Isolated Modules

> v3.7.0 Core Integrity Release - 孤岛模块列表
> 这些模块存在但未被主路径使用，需要收敛或删除

## 快速索引

| 模块 | 状态 | 问题 | 行动 |
|------|------|------|------|
| `parallel_executor` | **ISOLATED** | 63KB，无主路径调用 | 删除/合并 |
| `parallel_vector_executor` | **ISOLATED** | 无主路径调用 | 删除 |
| `local_executor_dml` | **ISOLATED** | PLACEHOLDER | 必须实现 |
| `mysql-server` | **MIGRATING** | session 独立 | 合并到 network |
| `expr` | **MIGRATING** | 未接入主路径 | 收敛 |
| `expression` | **ISOLATED** | 与 expr 重叠 | 删除 |
| `expr-legacy` | **DEPRECATED** | 待删除 | 删除 |
| `distributed` | **FROZEN** | 分布式实验 | P2 暂停 |
| `graph` | **FROZEN** | 图存储 | P2 暂停 |
| `vector` | **FROZEN** | 向量存储 | P2 暂停 |
| `vec_simd` | **EXPERIMENTAL** | SIMD 优化 | feature-gated |
| `wal-verification` | **PRODUCTION_SUPPORT** | Verification tooling | 不是主路径 |
| `qmd-bridge` | **ISOLATED** | 不确定 | 评估 |

## 详细问题

### 1. parallel_executor.rs (63KB)

**问题**: 存在但未被使用

```rust
// crates/executor/src/lib.rs
pub mod parallel_executor;  // 导出但未在主路径使用
```

**证据**:
- `local_executor.rs` (73KB) 是主执行器
- `transactional_executor.rs` 未调用 parallel_executor
- 无集成测试使用 parallel_executor

**行动**: TASK-R2-2 - 删除或合并到主 Executor

---

### 2. local_executor_dml.rs (PLACEHOLDER)

**问题**: DML 操作是占位符

```rust
// local_executor_dml.rs - 全部是 TODO
pub struct LocalExecutorDml;  // 只有 new() 和 default()
```

**行动**: TASK-R1-2 - StorageEngine Trait 升级，DML 必须经过事务

---

### 3. vec_simd.rs (1KB)

**问题**: SIMD  isolated

```rust
pub mod vec_simd;  // 无主路径调用
```

**行动**: 禁止新 SIMD 功能，v3.7.0 不是 SIMD 版本

---

### 4. mysql-server (5 crates)

**问题**: 独立 session，未集成

```
mysql-server/
├── session.rs      # 独立 session
├── connection.rs  # 独立连接
└── ...
```

**行动**: TASK-R4-1 - 合并到 network/

---

### 5. expr crate 孤岛

**问题**: 多个表达式 crate 重叠

| Crate | 问题 |
|-------|------|
| `expr` | 未被主路径使用 |
| `expr-eval` | 与 expr 重叠 |
| `expression` | 与 expr 重叠 |
| `expr-legacy` | 待删除 |

**行动**: TASK-R3-1 - 合并为统一 expr crate

---

### 6. distributed (分布式实验)

**问题**: 实验性质，未稳定

**行动**: P2，暂停

---

### 7. graph (图存储)

**问题**: 孤岛模块

**行动**: P2，暂停

---

### 8. vector (向量存储)

**问题**: 孤岛模块

**行动**: P2，暂停

---

## CI 检测

### Dead Module Scan

```bash
# 检测孤立模块（无主路径依赖）
cargo xtask dead-modules
```

### 主路径验证

```bash
# 验证 DML 经过事务
grep -r "storage.insert" crates/executor/src/ | grep -v "txn\|transaction"
# 应该无输出
```

## 相关 Issue

- #2599: v3.7.0 Master Issue
- #2590: expr crate 孤岛
- #2591: mysql-server 未集成
- #2603: R2 - 执行引擎统一
- #2604: R3 - expr crate 收敛
- #2605: R4 - mysql-server 统一