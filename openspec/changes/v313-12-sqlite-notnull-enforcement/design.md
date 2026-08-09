# Design: SQLite 兼容层 NOT NULL 约束强制执行

## Context

### 问题描述

SQLite 兼容层的 NOT NULL 约束未被强制执行。以下测试用例失败：

```sql
-- constraints__test_not_null.test
CREATE TABLE integers(i INTEGER NOT NULL);
INSERT INTO integers VALUES (NULL);  -- 应返回 error，实际成功

-- test_constraint_with_updates.test
CREATE TABLE integers(i INTEGER NOT NULL, j INTEGER NOT NULL);
UPDATE integers SET i=NULL;  -- 应返回 error，实际成功
```

### 当前状态

1. `ColumnDefinition` 结构正确解析 `NOT NULL` 修饰符（`nullable = false`）
2. 执行器在 INSERT/UPDATE 时未检查目标列的 `nullable` 属性
3. SQLite 模式使用 `rusqlite` 存储后端

### 约束

- 修改应仅影响 SQLite 兼容层（使用 `rusqlite` 的路径）
- 错误信息应明确指出违反的约束类型
- 性能影响应最小化

## Goals / Non-Goals

**Goals:**
- 在 INSERT 执行路径中添加 NOT NULL 约束检查
- 在 UPDATE 执行路径中添加 NOT NULL 约束检查
- 违反约束时返回明确的 `SqlError::ConstraintViolation` 错误
- 修复后 `constraints__test_not_null.test` 和 `test_constraint_with_updates.test` 应 PASS

**Non-Goals:**
- 不修改非 SQLite 路径的执行逻辑（除非共享代码）
- 不实现 CHECK 约束或其他约束类型（仅 NOT NULL）
- 不修改查询规划器或优化器

## Decisions

### Decision 1: 约束检查放置位置

**选项：**
- A. 在执行器顶层（`InsertExecutor`/`UpdateExecutor`）检查
- B. 在存储层（写入前）检查
- C. 在 DML 语句编译时检查

**选择：A - 执行器顶层**

**理由：**
- 执行器已有表结构信息（`TableInfo`），可直接访问列定义
- 检查发生在数据转换后、写入前，是最清晰的失败点
- 与现有错误处理模式一致（返回 `SqlError`）
- 避免修改共享存储层代码（可能影响其他执行路径）

### Decision 2: 错误传播方式

**选项：**
- A. 返回 `SqlError::ConstraintViolation("NOT NULL constraint violated: column 'x'")`
- B. 返回 `SqlError::ExecutionError("...NOT NULL...")`
- C. 打印日志但不返回错误

**选择：A - 专用约束违规错误**

**理由：**
- 便于客户端区分约束错误与其他执行错误
- 与 SQL 标准错误码（SQLITE_CONSTRAINT_NOTNULL）一致
- 便于测试断言错误类型

### Decision 3: 检查时机

**选项：**
- A. 每行写入前检查
- B. 批量检查（写入前对所有行）
- C. 延迟到写入时由 rusqlite 检测

**选择：A - 每行检查**

**理由：**
- 与现有 `constraints__test_not_null.test` 的单行语义一致
- 简单明确，易于理解和维护
- 批量优化可在后续迭代中添加

## 实现方案

### 3.1 INSERT 执行路径

```
InsertExecutor::execute()
  │
  ▼
对每一行数据：
  │
  ▼
对每个目标列：
  │  获取 ColumnDefinition
  │  检查 nullable 属性
  │  若 !nullable 且 value == Null → 返回 ConstraintViolation error
  ▼
写入存储层
```

### 3.2 UPDATE 执行路径

```
UpdateExecutor::execute()
  │
  ▼
对每一行（满足 WHERE 条件）：
  │
  ▼
对每个 SET 赋值：
  │  获取目标列的 ColumnDefinition
  │  检查 nullable 属性
  │  若 !nullable 且 new_value == Null → 返回 ConstraintViolation error
  ▼
写入存储层
```

### 3.3 关键数据结构

```rust
// crates/catalog/src/column.rs
pub struct ColumnDefinition {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,  // NOT NULL → false
}

// crates/types/src/lib.rs
pub enum SqlError {
    ConstraintViolation(String),  // 新增变体或扩展
}
```

### 3.4 修复位置（推断）

基于代码结构分析，关键路径可能在：

```
crates/executor/src/
├── local_executor_dml.rs    ← DML 语句分派
├── stored_proc.rs           ← 存储过程执行器（包含 Insert/Update 分支）
└── trigger.rs              ← 触发器执行（可能影响 Before Insert/Update）
```

具体需搜索 `Insert` / `Update` 枚举 variant 的处理代码。

## 验证方式

```bash
# 1. 构建
cargo build --all-features

# 2. 运行约束测试
cargo test -p sqlrustgo_sqllogictest constraints__test_not_null
cargo test -p sqlrustgo_sqllogictest test_constraint_with_updates

# 3. 运行完整测试套件
cargo test --all-features

# 4. lint 和格式检查
cargo clippy --all-features -- -D warnings
cargo fmt --check
```

## 失败模式

- **约束仍被忽略**：说明检查点不在执行器，可能在存储层需添加检查
- **误报错误**：说明 nullable 属性解析有误或默认值处理不当
- **其他 fixture 回归**：说明约束检查范围过宽，影响了允许 NULL 的场景
