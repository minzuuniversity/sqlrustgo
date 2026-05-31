# ARCH-900: StorageEngine Trait 拆分研究
**Issue**: #2655  
**Author**: Hermes C  
**Date**: 2026-05-31  
**Branch**: `origin/docs/v380-arch-900-storage-engine-split`  
**Status**: COMPLETED — for PR review

---

## 1. 背景与目标

**Issue #2655** 要求研究将 `StorageEngine` trait（20+ 方法）拆分为独立接口的可行性和影响，为 PR-900 ExecutionEngine 拆分清理提供蓝图。

**约束**：
- 不破坏当前代码编译
- 不改变运行时行为
- 产出物为 Markdown 研究报告

---

## 2. 当前 StorageEngine trait 分析

### 2.1 方法清单

`crates/storage/src/engine.rs` 中 `StorageEngine` trait 当前方法（按语义分组）：

**读操作（ReadEngine 接口）**
| 方法 | 返回类型 | 说明 |
|------|----------|------|
| `scan(table)` | Scan抱回 | 全表扫描 |
| `query(plan)` | ResultSet | 查询执行 |
| `read(key)` | Option<Value> | 单点读 |

**写操作（WriteEngine 接口）**
| 方法 | 返回类型 | 说明 |
|------|----------|------|
| `insert(row)` | Result | 插入 |
| `update(row)` | Result | 更新 |
| `delete(key)` | Result | 删除 |
| `write(batch)` | Result | 批量写 |

**事务接口（TransactionManager 绑定）**
| 方法 | 返回类型 | 说明 |
|------|----------|------|
| `begin()` | Transaction | 开启事务 |
| `commit()` | Result | 提交 |
| `rollback()` | Result | 回滚 |
| `is_in_transaction()` | bool | 事务状态 |

**WAL 接口（WALManager 绑定）**
| 方法 | 返回类型 | 说明 |
|------|----------|------|
| `wal()` | Option<WalManager> | WAL 访问 |
| `flush()` | Result | 刷盘 |
| `recover()` | Result | 恢复 |

**Catalog 元数据**
| 方法 | 返回类型 | 说明 |
|------|----------|------|
| `catalog()` | CatalogRef | 目录访问 |

### 2.2 当前实现者

| 实现者 | StorageEngine impl | 继承哪些方法 |
|--------|------------------|-------------|
| `MemoryStorage` | 完整 | 所有除了 WAL |
| `WalStorage<S>` | 完整 | WAL + flush + recover |
| `FileStorage` | 完整 | 所有方法 |

---

## 3. 拆分方案

### 3.1 方案 A：三级接口分层（推荐）

```
trait ReadEngine {
    fn scan(&self, table: &str) -> Result<ScanResult, Error>;
    fn query(&self, plan: &PhysicalPlan) -> Result<ResultSet, Error>;
    fn read(&self, key: &Key) -> Result<Option<Value>, Error>;
}

trait WriteEngine: ReadEngine {
    fn insert(&self, row: Row) -> Result<(), Error>;
    fn update(&self, row: Row) -> Result<(), Error>;
    fn delete(&self, key: &Key) -> Result<(), Error>;
    fn write_batch(&self, batch: WriteBatch) -> Result<(), Error>;
}

trait TransactionalWrite: WriteEngine {
    fn begin(&self) -> Result<Transaction, Error>;
    fn commit(&self) -> Result<(), Error>;
    fn rollback(&self) -> Result<(), Error>;
    fn is_in_transaction(&self) -> bool;
}

trait RecoveryEngine {
    fn wal(&self) -> Option<WalManager>;
    fn flush(&self) -> Result<(), Error>;
    fn recover(&self) -> Result<(), Error>;
}
```

**约束**：
- `StorageEngine = ReadEngine + WriteEngine + TransactionalWrite + RecoveryEngine`（trait 对象或自动 impl）
- 当前实现者保持 `impl StorageEngine for X` 不变（向后兼容）
- 拆分后的子接口用于 PR-900 阶段 `ExecutionEngine` 按职责委托

### 3.2 方案 B：分离 trait 对象（breaking change）

```rust
type Storage = dyn ReadEngine + WriteEngine + TransactionalWrite;
```

**问题**：所有调用点需要改为 trait object，增加动态分发开销，当前代码库不准备接受此变更。

### 3.3 方案 C：模块化拆分（最小改动）

将 `StorageEngine` 方法分组到独立 trait，保留 `StorageEngine` 为组合 superset：

```rust
trait ReadEngine { ... }
trait WriteEngine { ... }
trait Transactional { ... }
trait Recovery { ... }

trait StorageEngine:
    ReadEngine + WriteEngine + Transactional + Recovery {}

impl<T: StorageEngine> ReadEngine for T {}
impl<T: StorageEngine> WriteEngine for T {}
// ... 自动委派
```

---

## 4. 实施路径

### Phase 1（v3.8.0 RC 之前）：仅研究，不改代码
- [x] 本研究文档
- [ ] 验证方案 A 在 `MemoryStorage` / `WalStorage` 上可实现

### Phase 2（PR-900）：代码实施
1. 在 `crates/storage/src/engine.rs` 添加子 trait 定义
2. 为 `MemoryStorage` / `WalStorage` 实现子 trait
3. 在 `ExecutionEngine` 中用具体类型而非 trait object
4. 删除 `StorageEngine` 的冗余方法（如果有）

### Phase 3（v3.9.0）：WAL 重构
- `RecoveryEngine` 与 `WalManager` 交互方式重构
- WAL replay 隔离到独立模块

---

## 5. 风险评估

| 风险 | 等级 | 缓解 |
|------|------|------|
| 拆分后二进制大小增加 | 低 | 静态分发（方案 A） |
| 实现者需要实现多个 trait | 中 | 提供默认实现 |
| PR-900 执行引擎拆分依赖此研究 | 低 | 研究先行是前置条件 |

---

## 6. 结论

**推荐方案 A（三级接口分层）**：
- ReadEngine / WriteEngine / TransactionalWrite / RecoveryEngine 四层
- 自动 impl 保持向后兼容
- 静态分发，无运行时开销
- 可在 PR-900 中直接使用

**下一步**：等待 PR-830~PR-840 完成后，PR-900 阶段实施。

---

## 附录：相关文件

- `crates/storage/src/engine.rs` — StorageEngine trait
- `crates/storage/src/wal_storage.rs` — WalStorage 实现
- `crates/storage/src/memory_storage.rs` — MemoryStorage 实现
- `src/execution_engine.rs` — ExecutionEngine 当前使用 StorageEngine 的方式